// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model, SAMPLE_RATE};

const ROM: &[u8] = include_bytes!("../../../fixtures/generated/sweep.ws");
fn word(m: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
}

#[test]
fn original_cpu_probe_clears_sweeps_restores_normal_generates_pcm_and_resets_on_every_model() {
    let mut m = Machine::default();
    m.load_rom(ROM).unwrap();
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.change_model(model).unwrap();
        let mut first_pcm = Vec::new();
        let mut first_markers = Vec::new();
        for iteration in 0..2 {
            assert!(m.audio_samples().is_empty());
            assert_eq!(m.frame_number(), 0);
            m.run_frame().unwrap();
            assert_eq!(word(&m, 0x400), 0x600d, "{model:?}");
            assert_eq!(word(&m, 0x402), 1);
            assert!((32..=1536).contains(&word(&m, 0x404)));
            assert_eq!(word(&m, 0x406), 0x100);
            assert_eq!(word(&m, 0x408), 0x101);
            assert_eq!(word(&m, 0x40a), 0, "control resets from 2 to 0");
            assert_eq!(word(&m, 0x40c), 2, "leave FAST set before restart");
            for address in 0..14 {
                assert_eq!(
                    m.save_data()[address],
                    m.read_memory(0x400 + address as u32)
                );
            }
            assert!(m.cpu().halted);
            let pcm = m.audio_samples();
            assert!(m.audio_samples().is_empty());
            if iteration == 0 {
                first_pcm = pcm;
                first_markers = m.save_data()[..14].to_vec();
            } else {
                assert_eq!(pcm, first_pcm);
                assert_eq!(&m.save_data()[..14], first_markers);
            }
            m.run_frame().unwrap();
            // Initialization finishes after the first scanline was rendered.
            assert_eq!(
                &m.framebuffer()[..4],
                if model == Model::Mono {
                    &[170, 170, 170, 255]
                } else {
                    &[0, 255, 0, 255]
                }
            );
            let steady = m.audio_samples();
            assert_eq!(SAMPLE_RATE, 24000);
            assert_eq!(steady.len(), 636);
            // 15 * 15 << 5 / 32768 = 0.2197265625; equal stereo square.
            assert!(steady.contains(&0.0) && steady.contains(&0.21972656));
            assert!(steady.iter().all(|v| [0.0, 0.21972656].contains(v)));
            assert!(steady.chunks_exact(2).all(|v| v[0] == v[1]));
            let edges = steady
                .chunks_exact(2)
                .map(|v| v[0])
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|pair| pair[0] != pair[1])
                .count();
            assert!(
                (38..=40).contains(&edges),
                "1500 Hz in a 318-sample frame: {edges}"
            );
            assert!(m.audio_samples().is_empty());
            m.restart();
            assert!(m.audio_samples().is_empty());
        }
    }
}
