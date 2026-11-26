//! Versioned full-machine states. See docs/save-states.md.
use crate::{bus::Bus, rtc::Rtc, video::PendingLine, Cpu, Error, Machine, Model, FRAME_BYTES};
use flate2::{write::ZlibEncoder, Compression, Decompress, FlushDecompress, Status};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;

const MAGIC: &[u8; 8] = b"WSRSTATE";
const VERSION: u16 = 1;
const PREFIX_LEN: usize = 51;
const HEADER_LEN: usize = PREFIX_LEN + 32;
const MAX_PAYLOAD: usize = 1024 * 1024;
pub const MAX_SNAPSHOT_BYTES: usize = HEADER_LEN + MAX_PAYLOAD + 1024;

#[derive(Serialize)]
struct StateRef<'a> {
    cpu: &'a Cpu,
    bus: &'a Bus,
    #[serde(with = "serde_bytes")]
    framebuffer: &'a [u8],
    pending_line: &'a Option<PendingLine>,
    #[serde(with = "serde_bytes")]
    save: &'a [u8],
    rtc: Option<&'a Rtc>,
    #[serde(with = "serde_bytes")]
    console: &'a [u8],
}

#[derive(Deserialize)]
struct State {
    cpu: Cpu,
    bus: Bus,
    #[serde(with = "serde_bytes")]
    framebuffer: Vec<u8>,
    pending_line: Option<PendingLine>,
    #[serde(with = "serde_bytes")]
    save: Vec<u8>,
    rtc: Option<Rtc>,
    #[serde(with = "serde_bytes")]
    console: Vec<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    model: Model,
    rom_hash: [u8; 32],
}

/// Fully decoded and validated against a loaded cartridge and physical console.
/// Does not own ROM/firmware or any references to the running machine.
pub struct PreparedSnapshot {
    identity: Identity,
    state: State,
}

impl Machine {
    fn snapshot_identity(&self) -> Result<Identity, Error> {
        let cart = self
            .bus
            .cartridge
            .as_ref()
            .ok_or_else(|| Error::new("Load a cartridge before using save states."))?;
        Ok(Identity {
            model: self.model(),
            rom_hash: cart.rom_hash(),
        })
    }

    fn check_snapshot_identity(&self, identity: Identity) -> Result<(), Error> {
        let current = self.snapshot_identity()?;
        if identity.rom_hash != current.rom_hash {
            return Err(Error::new("Save state belongs to a different ROM."));
        }
        if identity.model != current.model {
            return Err(Error::new(
                "Save state belongs to a different physical console model.",
            ));
        }
        Ok(())
    }

    /// Capture at power-on or between calls to run_frame. After a fault this
    /// also permits recovery of native progress from the stopped hardware.
    pub fn capture_snapshot(&self) -> Result<Vec<u8>, Error> {
        self.capture_with_rtc(self.rtc())
    }

    /// Include time since the last RTC sample without changing the live clock.
    pub fn capture_snapshot_at(&self, now_ms: u64) -> Result<Vec<u8>, Error> {
        self.capture_with_rtc(self.rtc_at(now_ms).as_ref())
    }

    fn capture_with_rtc(&self, rtc: Option<&Rtc>) -> Result<Vec<u8>, Error> {
        let identity = self.snapshot_identity()?;
        let cart = self.bus.cartridge.as_ref().unwrap();
        encode(
            identity,
            StateRef {
                cpu: &self.cpu,
                bus: &self.bus,
                framebuffer: &self.framebuffer,
                pending_line: &self.video.pending,
                save: &cart.save,
                rtc,
                console: self.console_data(),
            },
        )
    }

    /// Decode and verify the entire candidate. Errors leave the machine intact.
    pub fn prepare_snapshot(&self, bytes: &[u8]) -> Result<PreparedSnapshot, Error> {
        let (identity, length) = header(bytes)?;
        self.check_snapshot_identity(identity)?;
        let payload = decode(bytes, length)?;
        let (state, extra): (State, _) = postcard::take_from_bytes(&payload)
            .map_err(|_| Error::new("Invalid save-state payload."))?;
        let cart = self.bus.cartridge.as_ref().unwrap();
        self.check_snapshot_rtc(&state)?;
        if !extra.is_empty()
            || state.framebuffer.len() != FRAME_BYTES
            || state
                .framebuffer
                .chunks_exact(4)
                .any(|pixel| pixel[3] != 255)
            || state.save.len() != cart.header.save.len()
            || state.console.len() != self.console_data().len()
            || !state.cpu.valid_snapshot()
            || !state
                .bus
                .valid_snapshot(self.model(), cart, state.console.len())
            || state.rtc.as_ref().is_some_and(|rtc| !rtc.valid_snapshot())
            || state
                .pending_line
                .as_ref()
                .is_some_and(|line| !line.valid_snapshot(state.bus.raster_line))
        {
            return Err(Error::new("Invalid machine state in save-state payload."));
        }
        Ok(PreparedSnapshot { identity, state })
    }

    fn check_snapshot_rtc(&self, state: &State) -> Result<(), Error> {
        let attached = self.bus.cartridge.as_ref().is_some_and(|c| c.rtc.is_some());
        if state.rtc.is_some() != attached {
            return Err(Error::new(
                "Save state requires the original cartridge clock setting.",
            ));
        }
        Ok(())
    }

    /// Install a validated candidate without rebooting. The supplied wall-clock
    /// sample is the new RTC anchor. Only future elapsed time advances it.
    pub fn apply_snapshot(&mut self, snapshot: PreparedSnapshot, now_ms: u64) -> Result<(), Error> {
        self.check_snapshot_identity(snapshot.identity)?;
        self.check_snapshot_rtc(&snapshot.state)?;
        let State {
            cpu,
            mut bus,
            framebuffer,
            pending_line,
            save,
            mut rtc,
            console,
        } = snapshot.state;
        if let Some(clock) = &mut rtc {
            clock.rebase(now_ms);
        }
        let mut video = crate::video::Pipeline::default();
        video.pending = pending_line;
        bus.inherit_snapshot_environment(&mut self.bus);
        let cart = bus.cartridge.as_mut().unwrap();
        cart.save = save;
        cart.rtc = rtc;
        bus.console_data[bus.model.index()] = console;
        *self = Self {
            cpu,
            bus,
            framebuffer,
            video,
            fault: None,
        };
        Ok(())
    }
}

impl PreparedSnapshot {
    pub fn framebuffer(&self) -> &[u8] {
        &self.state.framebuffer
    }
    pub fn save_data(&self) -> &[u8] {
        &self.state.save
    }
    pub fn console_data(&self) -> &[u8] {
        &self.state.console
    }
    /// Encode the native RTC record as it will be when installed.
    pub fn rtc_data(&self, now_ms: u64) -> Vec<u8> {
        self.state.rtc.clone().map_or_else(Vec::new, |mut rtc| {
            rtc.rebase(now_ms);
            rtc.export()
        })
    }

    /// Make a recovery export containing any distinct, previously persisted
    /// records. Clear transfers for replaced records, as with native imports.
    pub fn replace_native_records(
        &mut self,
        save: Option<&[u8]>,
        rtc: Option<&[u8]>,
        console: Option<&[u8]>,
    ) -> Result<(), Error> {
        if save.is_some_and(|data| data.len() != self.state.save.len())
            || console.is_some_and(|data| data.len() != self.state.console.len())
            || (rtc.is_some() && self.state.rtc.is_none())
        {
            return Err(Error::new(
                "Native recovery records do not match this save state.",
            ));
        }
        let rtc = rtc.map(Rtc::import).transpose()?;
        if let Some(data) = save {
            self.state.save.copy_from_slice(data);
            self.state.bus.cancel_cartridge_eeprom_transfer();
        }
        if let Some(rtc) = rtc {
            self.state.rtc = Some(rtc);
            self.state.bus.cancel_rtc();
        }
        if let Some(data) = console {
            self.state.console.copy_from_slice(data);
            self.state.bus.cancel_internal_eeprom_transfer();
        }
        Ok(())
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let state = &self.state;
        encode(
            self.identity,
            StateRef {
                cpu: &state.cpu,
                bus: &state.bus,
                framebuffer: &state.framebuffer,
                pending_line: &state.pending_line,
                save: &state.save,
                rtc: state.rtc.as_ref(),
                console: &state.console,
            },
        )
    }
}

fn encode(identity: Identity, state: StateRef<'_>) -> Result<Vec<u8>, Error> {
    let payload =
        postcard::to_allocvec(&state).map_err(|_| Error::new("Could not encode save state."))?;
    if payload.len() > MAX_PAYLOAD {
        return Err(Error::new("Save state exceeds the format size limit."));
    }
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::fast());
    let compressed = zlib
        .write_all(&payload)
        .and_then(|()| zlib.finish())
        .map_err(|_| Error::new("Could not compress save state."))?;
    if compressed.len() > MAX_SNAPSHOT_BYTES - HEADER_LEN {
        return Err(Error::new(
            "Compressed save state exceeds the format size limit.",
        ));
    }
    let mut bytes = Vec::with_capacity(HEADER_LEN + compressed.len());
    bytes.extend(MAGIC);
    bytes.extend(VERSION.to_le_bytes());
    bytes.push(identity.model.index() as u8);
    bytes.extend(identity.rom_hash);
    bytes.extend((payload.len() as u32).to_le_bytes());
    bytes.extend((compressed.len() as u32).to_le_bytes());
    bytes.extend(checksum(&bytes, &compressed));
    bytes.extend(compressed);
    Ok(bytes)
}

fn header(bytes: &[u8]) -> Result<(Identity, usize), Error> {
    if !(HEADER_LEN..=MAX_SNAPSHOT_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC {
        return Err(Error::new("Invalid save-state file or file size."));
    }
    let version = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    if version != VERSION {
        return Err(Error::new(format!(
            "Unsupported save-state version {version}."
        )));
    }
    let model = match bytes[10] {
        0 => Model::Mono,
        1 => Model::Color,
        2 => Model::Crystal,
        _ => return Err(Error::new("Invalid save-state console model.")),
    };
    let rom_hash = bytes[11..43].try_into().unwrap();
    let decoded = u32::from_le_bytes(bytes[43..47].try_into().unwrap()) as usize;
    let compressed = u32::from_le_bytes(bytes[47..51].try_into().unwrap()) as usize;
    if !(1..=MAX_PAYLOAD).contains(&decoded) || compressed != bytes.len() - HEADER_LEN {
        return Err(Error::new("Invalid save-state lengths."));
    }
    Ok((Identity { model, rom_hash }, decoded))
}

fn checksum(prefix: &[u8], compressed: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(prefix);
    hasher.update(compressed);
    hasher.finalize().into()
}

fn decode(bytes: &[u8], length: usize) -> Result<Vec<u8>, Error> {
    let compressed = &bytes[HEADER_LEN..];
    if checksum(&bytes[..PREFIX_LEN], compressed) != bytes[PREFIX_LEN..HEADER_LEN] {
        return Err(Error::new("Save-state checksum mismatch."));
    }
    let mut zlib = Decompress::new(true);
    let mut payload = vec![0; length + 1];
    let result = zlib.decompress(compressed, &mut payload, FlushDecompress::Finish);
    if !matches!(result, Ok(Status::StreamEnd))
        || zlib.total_in() != compressed.len() as u64
        || zlib.total_out() != length as u64
    {
        return Err(Error::new(
            "Invalid compressed save-state data or decoded length.",
        ));
    }
    payload.truncate(length);
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Button;

    const SRAM: &[u8] = include_bytes!("../../../fixtures/generated/sram32-common.ws");
    const GDMA: &[u8] = include_bytes!("../../../fixtures/generated/gdma.ws");
    const EEPROM_RTC: &[u8] = include_bytes!("../../../fixtures/generated/eeprom2048-packed.ws");

    fn drive(machine: &mut Machine, clock_start: u64) -> Vec<f32> {
        let mut audio = Vec::new();
        for (step, button) in [Button::Right, Button::Y1, Button::A]
            .into_iter()
            .enumerate()
        {
            machine.sync_rtc(clock_start + (step as u64 + 1) * 377);
            machine.set_buttons(button as u16);
            machine.run_frame().unwrap();
            audio.extend(machine.audio_samples());
        }
        audio
    }

    #[test]
    fn all_physical_models_replay_at_power_on_and_after_ipl_lockout() {
        for (model, rom) in [
            (Model::Mono, SRAM),
            (Model::Color, GDMA),
            (Model::Crystal, EEPROM_RTC),
        ] {
            let mut m = Machine::new(model);
            m.load_rom(rom).unwrap();
            m.sync_rtc(1000);
            let at_power_on = m.capture_snapshot().unwrap();
            assert_eq!(m.bus.ports[0xa0] & 1, 0);
            for _ in 0..4 {
                m.run_frame().unwrap();
            }
            assert_ne!(m.bus.ports[0xa0] & 1, 0);
            let locked = m.capture_snapshot().unwrap();
            let mut other = Machine::new(model);
            other.load_rom(rom).unwrap();
            for (bytes, is_locked) in [(&at_power_on, false), (&locked, true)] {
                let mut candidate = other.prepare_snapshot(bytes).unwrap();
                assert_eq!(candidate.encode().unwrap(), *bytes);
                let original_console = candidate.console_data().to_vec();
                candidate
                    .replace_native_records(None, None, Some(&vec![42; original_console.len()]))
                    .unwrap();
                other.apply_snapshot(candidate, 12_000).unwrap();
                assert!(other.console_data().iter().all(|&byte| byte == 42));
                m.apply_snapshot(m.prepare_snapshot(bytes).unwrap(), 1000)
                    .unwrap();
                other
                    .apply_snapshot(other.prepare_snapshot(bytes).unwrap(), 12_000)
                    .unwrap();
                assert_eq!(other.bus.ports[0xa0] & 1 != 0, is_locked);
                assert_eq!(other.frame_number(), if is_locked { 4 } else { 0 });
                assert_eq!(other.console_data(), original_console);
                assert_eq!(other.framebuffer(), m.framebuffer());
                assert!(m.audio_samples().is_empty());
                let pcm = drive(&mut m, 1000);
                assert!(!pcm.is_empty());
                assert_eq!(drive(&mut other, 12_000), pcm);
                assert_eq!(other.framebuffer(), m.framebuffer());
                assert_eq!(other.save_data(), m.save_data());
                assert_eq!(
                    other.capture_snapshot().unwrap(),
                    m.capture_snapshot().unwrap(),
                    "{model:?}"
                );
                if !m.rtc_data().is_empty() {
                    assert_eq!(&other.rtc_data()[..16], &m.rtc_data()[..16]);
                    assert_ne!(&other.rtc_data()[16..], &m.rtc_data()[16..]);
                }
            }
        }
    }

    #[test]
    fn replay_in_flight_dma_raster_and_eeprom_rtc_commands() {
        let mut m = Machine::new(Model::Crystal);
        m.load_rom(EEPROM_RTC).unwrap();
        m.sync_rtc(1000);
        for _ in 0..4 {
            m.run_frame().unwrap();
        }
        // Compose a row and leave it waiting for the delayed palette stage.
        m.advance(256 - (m.bus.cycles % 256) as u32);
        assert!(m.video.pending.is_some());
        // Read a word via both EEPROM controllers and launch an RTC read.
        m.bus.cartridge.as_mut().unwrap().save[..2].copy_from_slice(&[0x34, 0x12]);
        m.bus.console_data[Model::Crystal.index()][..2].copy_from_slice(&[0x78, 0x56]);
        let source = m.bus.ram[0x400..0x420].to_vec();
        for (port, value) in [
            (0xc6, 0),
            (0xc7, 0x18),
            (0xc8, 0x10),
            (0xbc, 0),
            (0xbd, 0x18),
            (0xbe, 0x10),
            (0xca, 0x15),
            (0x40, 0),
            (0x41, 4),
            (0x42, 0),
            (0x44, 0),
            (0x45, 0x10),
            (0x46, 32),
            (0x47, 0),
            (0x48, 0x80),
        ] {
            m.bus.output(port, value).unwrap();
        }
        let cycles = m.bus.gdma_step().unwrap();
        m.advance(cycles);
        let from = m.capture_snapshot().unwrap();
        m.audio_samples();
        let expected = drive(&mut m, 1000);
        let result = m.capture_snapshot().unwrap();
        m.apply_snapshot(m.prepare_snapshot(&from).unwrap(), 5000)
            .unwrap();
        assert_eq!(drive(&mut m, 5000), expected);
        assert_eq!(m.capture_snapshot().unwrap(), result);
        assert_eq!(m.bus.input(0xc4).unwrap(), 0x34);
        assert_eq!(m.bus.input(0xba).unwrap(), 0x78);
        assert_eq!(m.bus.input(0xca).unwrap() & 0x80, 0x80);
        assert_eq!(m.bus.ram[0x1000..0x1020], source);
    }

    fn rehash(bytes: &mut [u8]) {
        let hash = checksum(&bytes[..PREFIX_LEN], &bytes[HEADER_LEN..]);
        bytes[PREFIX_LEN..HEADER_LEN].copy_from_slice(&hash);
    }

    #[test]
    fn invalid_files_do_not_mutate_the_running_machine() {
        let mut m = Machine::new(Model::Crystal);
        m.load_rom(EEPROM_RTC).unwrap();
        let baseline = m.capture_snapshot().unwrap();
        let (_, length) = header(&baseline).unwrap();
        let payload = decode(&baseline, length).unwrap();
        let cart = m.bus.cartridge.as_ref().unwrap();
        assert_eq!(&baseline[11..43], &Sha256::digest(EEPROM_RTC)[..]);
        assert!(!payload.windows(64).any(|part| part == &cart.bytes[..64]));

        let mut bad_digest = baseline.clone();
        *bad_digest.last_mut().unwrap() ^= 1;
        let mut version = baseline.clone();
        version[8] = 2;
        let mut model = baseline.clone();
        model[10] = 1;
        let mut invalid_model = baseline.clone();
        invalid_model[10] = 3;
        let mut rom = baseline.clone();
        rom[11] ^= 1;
        let mut trailing = baseline.clone();
        trailing.push(0);
        let compressed_len = (trailing.len() - HEADER_LEN) as u32;
        trailing[47..51].copy_from_slice(&compressed_len.to_le_bytes());
        rehash(&mut trailing);
        let mut expansion = baseline.clone();
        expansion[43..47].copy_from_slice(&1u32.to_le_bytes());
        rehash(&mut expansion);
        let mut oversized_payload = baseline.clone();
        oversized_payload[43..47].copy_from_slice(&((MAX_PAYLOAD + 1) as u32).to_le_bytes());
        rehash(&mut oversized_payload);
        let mut invalid_device = m.prepare_snapshot(&baseline).unwrap();
        invalid_device.state.bus.sprite_count = 255;
        let mut invalid_dma = m.prepare_snapshot(&baseline).unwrap();
        invalid_dma.state.bus.ports[0x44] = 255;
        let mut invalid_ram = m.prepare_snapshot(&baseline).unwrap();
        invalid_ram.state.bus.ram.truncate(10);
        for (data, error) in [
            (version, "Unsupported save-state version"),
            (bad_digest, "checksum"),
            (model, "different physical console"),
            (invalid_model, "Invalid save-state console"),
            (rom, "different ROM"),
            (trailing, "compressed save-state"),
            (expansion, "compressed save-state"),
            (oversized_payload, "lengths"),
            (vec![0; MAX_SNAPSHOT_BYTES + 1], "file or file size"),
            (baseline[..baseline.len() - 1].to_vec(), "lengths"),
            (invalid_device.encode().unwrap(), "Invalid machine state"),
            (invalid_dma.encode().unwrap(), "Invalid machine state"),
            (invalid_ram.encode().unwrap(), "Invalid machine state"),
        ] {
            assert!(m
                .prepare_snapshot(&data)
                .err()
                .unwrap()
                .to_string()
                .contains(error));
            assert_eq!(m.capture_snapshot().unwrap(), baseline);
        }
        let candidate = m.prepare_snapshot(&baseline).unwrap();
        m.set_rtc_present(false).unwrap();
        let absent = m.capture_snapshot().unwrap();
        assert!(m.apply_snapshot(candidate, 1000).is_err());
        assert_eq!(m.capture_snapshot().unwrap(), absent);
    }
}
