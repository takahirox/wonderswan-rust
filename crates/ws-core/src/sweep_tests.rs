// SPDX-License-Identifier: CC0-1.0
// Independently calculated vectors. Unmeasured boundaries: docs/fast-sweep.md.
use super::{bus::Bus, Model, CYCLES_PER_FRAME};

const MODELS: [Model; 3] = [Model::Mono, Model::Color, Model::Crystal];

fn setup(model: Model, fast: u8, time: u8, step: u8, frequency: u16) -> Bus {
    let mut b = Bus::new(model);
    for (port, value) in [
        (0x84, frequency as u8),
        (0x85, (frequency >> 8) as u8),
        (0x8c, step),
        (0x8d, time),
        (0x90, 0x44),
        (0x95, fast),
    ] {
        b.output(port, value).unwrap();
    }
    b
}
fn frequency(b: &mut Bus) -> u16 {
    u16::from_le_bytes([b.input(0x84).unwrap(), b.input(0x85).unwrap()])
}

#[test]
fn signed_steps_wrap_both_directions_at_exact_fast_and_ordinary_boundaries() {
    for model in MODELS {
        for (fast, time, period) in [
            (2, 0, 1),
            (2, 1, 2),
            (2, 31, 32),
            (0, 0, 8192),
            (0, 1, 16384),
            (0, 31, 262144),
        ] {
            // initial, encoded signed step, first result, second result
            for (initial, step, first, second) in [
                (1000, 7, 1007, 1014),
                (1000, 0xfd, 997, 994),
                (2047, 1, 0, 1),
                (0, 0xff, 2047, 2046),
                (2040, 127, 119, 246),
                (3, 0x80, 1923, 1795),
                (517, 0, 517, 517),
            ] {
                let mut b = setup(model, fast, time, step, initial);
                b.tick(period - 1);
                assert_eq!(frequency(&mut b), initial);
                b.tick(1);
                assert_eq!(
                    frequency(&mut b),
                    first,
                    "{model:?}, fast={fast}, time={time}, step={step}"
                );
                b.tick(period - 1);
                assert_eq!(frequency(&mut b), first);
                b.tick(1);
                assert_eq!(frequency(&mut b), second);
            }
        }
    }
}

#[test]
fn both_enable_prerequisites_and_provisional_pause_policy() {
    for model in MODELS {
        for (fast, period) in [(0, 16384), (2, 2)] {
            for control in [0, 4, 0x40, 0x44] {
                let mut b = setup(model, fast, 1, 1, 30);
                b.tick(period - 1);
                b.output(0x90, control).unwrap();
                b.tick(period * 2);
                assert_eq!(frequency(&mut b), if control == 0x44 { 32 } else { 30 });
                b.output(0x90, 0x44).unwrap();
                b.tick(1);
                assert_eq!(frequency(&mut b), if control == 0x44 { 33 } else { 31 });
            }
        }
    }
}

#[test]
fn zero_step_advances_timer_and_step_writes_preserve_its_phase() {
    for model in MODELS {
        for (fast, period) in [(0, 16384), (2, 2)] {
            let mut b = setup(model, fast, 1, 0, 600);
            b.tick(period * 2 + period - 1);
            assert_eq!(frequency(&mut b), 600);
            b.output(0x8c, 7).unwrap();
            b.tick(1);
            assert_eq!(frequency(&mut b), 607);
        }
    }
}

#[test]
fn time_writes_reset_both_counters_even_for_the_same_value() {
    for model in MODELS {
        for (fast, time, period) in [(0, 0, 8192), (0, 1, 16384), (2, 0, 1), (2, 3, 4)] {
            let mut b = setup(model, fast, time, 1, 90);
            b.tick(period - 1);
            b.output(0x8d, time).unwrap();
            b.tick(period - 1);
            assert_eq!(frequency(&mut b), 90);
            b.tick(1);
            assert_eq!(frequency(&mut b), 91);
            b.output(0x8d, 1).unwrap();
            let changed_period = if fast == 2 { 2 } else { 16384 };
            b.tick(changed_period - 1);
            assert_eq!(frequency(&mut b), 91);
            b.tick(1);
            assert_eq!(frequency(&mut b), 92);
        }
    }
}

#[test]
fn provisional_mode_transitions_preserve_programmable_count_and_prescaler_phase() {
    for model in MODELS {
        let mut b = setup(model, 0, 2, 1, 700);
        b.tick(8192 + 100); // one programmable tick, 100 prescaler clocks
        b.output(0x95, 2).unwrap();
        b.tick(1); // second programmable tick, prescaler remains 100
        assert_eq!(frequency(&mut b), 700);
        b.output(0x95, 2).unwrap(); // repeated writes must not reset phase
        b.output(0x95, 0).unwrap();
        b.tick(8091);
        assert_eq!(frequency(&mut b), 700);
        b.tick(1); // third tick, completing one update
        assert_eq!(frequency(&mut b), 701);
        b.output(0x95, 2).unwrap();
        b.tick(2);
        b.output(0x95, 0).unwrap();
        b.output(0x95, 0).unwrap();
        b.tick(8191);
        assert_eq!(frequency(&mut b), 701);
        b.tick(1);
        assert_eq!(frequency(&mut b), 702);
        // A time write while fast also clears a retained ordinary prescaler.
        b.tick(8191);
        b.output(0x95, 2).unwrap();
        b.output(0x8d, 0).unwrap();
        b.output(0x95, 0).unwrap();
        b.tick(8191);
        assert_eq!(frequency(&mut b), 702);
        b.tick(1);
        assert_eq!(frequency(&mut b), 703);
    }
}

#[test]
fn control_readback_reset_policy_and_every_unsupported_value_preserve_state() {
    for model in MODELS {
        let mut b = Bus::new(model);
        assert_eq!(b.input(0x95).unwrap(), 0);
        for accepted in [0, 2, 0, 2] {
            b.output(0x95, accepted).unwrap();
            assert_eq!(b.input(0x95).unwrap(), accepted);
        }
        for fast in [0, 2] {
            for bad in (0..=255).filter(|v| v & !2 != 0) {
                let mut b = setup(model, fast, 0, 1, 42);
                let period = if fast == 2 { 1 } else { 8192 };
                b.tick(period - 1);
                let before = b.ports;
                let cycles = b.cycles;
                let error = b.output(0x95, bad).unwrap_err().to_string();
                assert!(
                    error.contains("0x95")
                        && error.contains("FAST_SWEEP")
                        && error.contains("HOLD_CH")
                );
                assert_eq!(b.ports, before);
                assert_eq!(b.cycles, cycles);
                b.tick(1);
                assert_eq!(frequency(&mut b), 43);
            }
        }
        // Unsupported writes also preserve the partly elapsed programmable count.
        let mut b = setup(model, 2, 3, 1, 10);
        b.tick(2);
        assert!(b.output(0x95, 3).is_err());
        b.tick(1);
        assert_eq!(frequency(&mut b), 10);
        b.tick(1);
        assert_eq!(frequency(&mut b), 11);
    }
}

#[test]
fn single_clocks_and_chunks_match_frequency_pcm_and_other_devices_across_frame() {
    for model in MODELS {
        for (fast, expected) in [(0, 11), (2, 1604)] {
            let prepare = || {
                let mut b = setup(model, fast, 3, 7, 2045);
                // Select two ordinary updates during this 40709-clock vector.
                if fast == 0 {
                    b.output(0x8d, 1).unwrap();
                }
                b.output(0x90, 0x45).unwrap(); // unrelated wavetable channel 1
                b.output(0x80, 0xc0).unwrap();
                b.output(0x81, 7).unwrap();
                b.output(0x88, 0xff).unwrap();
                b.output(0x8a, 0xff).unwrap();
                b.output(0x91, 8).unwrap();
                b.ram[8..16].fill(255);
                b.ram[40..48].fill(255);
                b
            };
            let (mut one, mut chunks) = (prepare(), prepare());
            let total = CYCLES_PER_FRAME as u32 + 5; // 40709 clocks
            for _ in 0..total {
                one.tick(1);
            }
            for n in [127, 1, 8191, 17, total - 8336] {
                chunks.tick(n);
            }
            assert_eq!(frequency(&mut one), expected);
            assert_eq!(one.ports, chunks.ports);
            assert_eq!(one.cycles, chunks.cycles);
            assert_eq!(one.completed_frames, 1);
            assert_eq!(one.completed_frames, chunks.completed_frames);
            assert_eq!(one.raster_line, chunks.raster_line);
            assert_eq!(one.audio.mix, chunks.audio.mix);
            let samples = one.audio.drain();
            assert_eq!(samples.len(), 636);
            assert!(samples.iter().any(|v| *v != 0.0));
            assert_eq!(samples, chunks.audio.drain());
            assert!(one.audio.drain().is_empty());
            // Continue in the other mode to compare the retained hidden phase.
            for b in [&mut one, &mut chunks] {
                b.output(0x95, fast ^ 2).unwrap();
            }
            for _ in 0..20000 {
                one.tick(1);
            }
            chunks.tick(20000);
            assert_eq!(one.ports, chunks.ports);
            assert_eq!(one.audio.drain(), chunks.audio.drain());
        }
    }
}
