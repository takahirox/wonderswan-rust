// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model, SAMPLE_RATE};
const PROBES: [&[u8]; 3] = [
    include_bytes!("../../../fixtures/generated/sound-channel2.ws"),
    include_bytes!("../../../fixtures/generated/sound-hyper.ws"),
    include_bytes!("../../../fixtures/generated/sound-direct.ws"),
];
fn word(m: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
}
#[test]
fn original_machine_code_emits_exact_pcm_from_all_three_paths_and_restarts() {
    for (path, rom) in PROBES.iter().enumerate() {
        for model in [Model::Mono, Model::Color, Model::Crystal] {
            let mut m = Machine::new(model);
            m.load_rom(rom).unwrap();
            assert_eq!(SAMPLE_RATE, 24000);
            m.run_frame().unwrap();
            let first = m.audio_samples();
            assert_eq!(
                word(&m, 0x400),
                if model == Model::Mono { 0x4d4f } else { 0x600d },
                "path {path}, {model:?}"
            );
            assert_eq!(word(&m, 0x402), u16::from(model != Model::Mono));
            assert_eq!(m.cpu().halted, model == Model::Mono || path < 2);
            assert!(first.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
            m.run_frame().unwrap();
            let steady = m.audio_samples();
            assert_eq!(steady.len(), 636); // 40704 / 128 stereo samples per frame
            if model == Model::Mono {
                assert!(steady.iter().all(|s| *s == 0.0));
            } else {
                assert_eq!(&m.framebuffer()[..4], &[0, 255, 0, 255]);
                let expected = match path {
                    0 => [0.03125, 0.21875],
                    1 => [-0.125, 0.125],
                    _ => [-0.25, 0.25],
                };
                for sample in expected {
                    assert!(steady.contains(&sample), "path {path} missing {sample}");
                }
                if path < 2 {
                    assert!(steady.chunks_exact(2).all(|v| v[0] == v[1]));
                } else {
                    // Separate CPU OUTs can straddle an output tick; pairs are
                    // opposite except for brief transitions between writes.
                    let opposite = steady.chunks_exact(2).filter(|v| v[0] == -v[1]).count();
                    assert!(opposite > 280, "{opposite} opposite stereo pairs");
                    assert!(steady
                        .iter()
                        .all(|v| [0.125, -0.125, 0.25, -0.25].contains(v)));
                }
                if path == 2 {
                    assert!(steady.contains(&0.125));
                    assert!(steady.contains(&-0.125));
                }
            }
            assert!(m.audio_samples().is_empty(), "drain must not duplicate PCM");
            m.restart();
            assert!(
                m.audio_samples().is_empty(),
                "restart must not retain old PCM"
            );
            m.run_frame().unwrap();
            assert_eq!(m.audio_samples(), first, "reset is deterministic");
        }
    }
}
