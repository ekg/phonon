#!/usr/bin/env rust-script

//! Quick script to render phonon to WAV
//! Usage: ./render_wav.rs input.phonon output.wav duration_seconds

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} input.phonon output.wav duration_seconds", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];
    let duration: f32 = args[3].parse().unwrap_or(10.0);

    // For now, just create a test WAV with sine wave
    let sample_rate = 44100;
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    // Generate a 440 Hz sine wave
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.2;
        samples.push(sample);
    }

    // Write WAV file
    write_wav(output_file, &samples, sample_rate as u32).unwrap();
    println!("Wrote {} seconds to {}", duration, output_file);
}

fn write_wav(filename: &str, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
    let mut file = File::create(filename)?;

    let num_samples = samples.len() as u32;
    let byte_rate = sample_rate * 2; // 16-bit mono

    // WAV header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + num_samples * 2).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // PCM
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&(num_samples * 2).to_le_bytes())?;

    // Convert samples to 16-bit
    for &sample in samples {
        let s16 = (sample.max(-1.0).min(1.0) * 32767.0) as i16;
        file.write_all(&s16.to_le_bytes())?;
    }

    Ok(())
}