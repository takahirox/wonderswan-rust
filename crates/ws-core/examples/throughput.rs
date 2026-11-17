//! Headless end-to-end core throughput, with video and drained audio enabled.
use std::{env, fs, hint::black_box, time::Instant};
use ws_core::{Machine, Model};

fn hash(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: throughput ROM WARMUP_FRAMES MEASURED_FRAMES mono|color|crystal".into(),
        );
    }
    let warmup: usize = args[2].parse()?;
    let frames: usize = args[3].parse()?;
    if frames == 0 {
        return Err("MEASURED_FRAMES must be positive".into());
    }
    let model = match args[4].as_str() {
        "mono" => Model::Mono,
        "color" => Model::Color,
        "crystal" => Model::Crystal,
        _ => return Err("unknown model".into()),
    };
    let mut machine = Machine::new(model);
    machine.load_rom(&fs::read(&args[1])?)?;
    for _ in 0..warmup {
        machine.run_frame()?;
        black_box(machine.audio_samples());
        black_box(machine.framebuffer());
    }
    let start_frame = machine.frame_number();
    let start = Instant::now();
    let mut sample_count = 0;
    let mut last_audio = Vec::new();
    for _ in 0..frames {
        machine.run_frame()?;
        last_audio = machine.audio_samples();
        sample_count += black_box(&last_audio).len();
        black_box(machine.framebuffer());
    }
    let seconds = start.elapsed().as_secs_f64();
    assert_eq!(machine.frame_number() - start_frame, frames as u64);
    let video = hash(machine.framebuffer().iter().copied());
    let audio = hash(last_audio.iter().flat_map(|s| s.to_bits().to_le_bytes()));
    let ram = hash((0..65536).map(|a| machine.read_memory(a)));
    let markers = [0x400, 0x402, 0x404]
        .map(|a| u16::from_le_bytes([machine.read_memory(a), machine.read_memory(a + 1)]));
    println!("{{\"frames\":{frames},\"seconds\":{seconds:.9},\"us_per_frame\":{:.3},\"fps\":{:.1},\"sample_values\":{sample_count},\"video_hash\":\"{video:016x}\",\"audio_hash\":\"{audio:016x}\",\"ram_hash\":\"{ram:016x}\",\"markers\":{markers:?}}}", seconds * 1e6 / frames as f64, frames as f64 / seconds);
    Ok(())
}
