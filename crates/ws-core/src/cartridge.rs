use crate::Error;

/// The deliberately supported cartridge subset: power-of-two ROM, no saves, mapper 2001.
#[derive(Clone, Debug)]
pub struct Cartridge {
    pub bytes: Vec<u8>,
    pub color: bool,
}

impl Cartridge {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 131_072 || bytes.len() > 4_194_304 || !bytes.len().is_power_of_two() {
            return Err(Error::new(
                "Expected a 128 KiB–4 MiB power-of-two WonderSwan ROM.",
            ));
        }
        let h = &bytes[bytes.len() - 16..];
        if h[0] != 0xea || h[5] & 15 != 0 || h[7] > 1 {
            return Err(Error::new("Invalid cartridge entry jump or header."));
        }
        let sizes = [
            131_072, 262_144, 524_288, 1_048_576, 2_097_152, 3_145_728, 4_194_304,
        ];
        if sizes.get(h[10] as usize) != Some(&bytes.len()) {
            return Err(Error::new("ROM size does not match cartridge header."));
        }
        if h[11] != 0 || h[13] != 0 || h[12] & !0x0d != 0 {
            return Err(Error::new(
                "Unsupported save hardware, mapper, or cartridge flags.",
            ));
        }
        let sum = bytes[..bytes.len() - 2]
            .iter()
            .fold(0u16, |sum, b| sum.wrapping_add(*b as u16));
        if sum != u16::from_le_bytes([h[14], h[15]]) {
            return Err(Error::new("Cartridge checksum mismatch."));
        }
        Ok(Self {
            bytes: bytes.to_vec(),
            color: h[7] != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEMO: &[u8] = include_bytes!("../../../web/public/demo/beacon-trail.ws");
    fn checksum(bytes: &mut [u8]) {
        let len = bytes.len();
        let sum = bytes[..len - 2]
            .iter()
            .fold(0u16, |n, b| n.wrapping_add(*b as u16));
        bytes[len - 2..].copy_from_slice(&sum.to_le_bytes());
    }
    #[test]
    fn footer_validation_rejects_corruption_and_unsupported_hardware() {
        assert!(Cartridge::parse(DEMO).is_ok());
        let mut corrupt = DEMO.to_vec();
        corrupt[123] ^= 1;
        assert!(Cartridge::parse(&corrupt)
            .unwrap_err()
            .to_string()
            .contains("checksum"));
        for (offset, value) in [
            (0, 0),
            (5, 1),
            (7, 2),
            (10, 1),
            (11, 0x10),
            (12, 0x80),
            (13, 1),
        ] {
            let mut rom = DEMO.to_vec();
            let len = rom.len();
            rom[len - 16 + offset] = value;
            checksum(&mut rom);
            assert!(Cartridge::parse(&rom).is_err(), "header offset {offset}");
        }
    }
}
