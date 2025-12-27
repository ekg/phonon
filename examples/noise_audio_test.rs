//! Generate audio files for each noise type to verify characteristics
//!
//! This creates WAV files that can be analyzed in audio software to verify
//! the spectral characteristics of each noise generator.
//!
//! Run with: cargo run --example noise_audio_test
//!
//! Output files (in examples/ directory):
//! - white_noise.wav
//! - pink_noise.wav
//! - brown_noise.wav
//! - fbm_noise.wav

use hound;
use phonon::nodes::noise_generators::{BrownNoise, FractalBrownianMotion, PinkNoise, WhiteNoise};

fn main() {
    let sample_rate = 44100;
    let duration_seconds = 5;
    let num_samples = sample_rate * duration_seconds;

    println!("Generating {} seconds of audio at {} Hz...\n", duration_seconds, sample_rate);

    // WAV file spec
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // White Noise
    {
        println!("Generating white_noise.wav...");
        let mut writer = hound::WavWriter::create("examples/white_noise.wav", spec).unwrap();
        let mut gen = WhiteNoise::new(12345);

        for _ in 0..num_samples {
            let sample = gen.next_sample();
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).unwrap();
        }
        writer.finalize().unwrap();
        println!("  ✓ Written");
    }

    // Pink Noise
    {
        println!("Generating pink_noise.wav...");
        let mut writer = hound::WavWriter::create("examples/pink_noise.wav", spec).unwrap();
        let mut gen = PinkNoise::new(12345);

        for _ in 0..num_samples {
            let sample = gen.next_sample();
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).unwrap();
        }
        writer.finalize().unwrap();
        println!("  ✓ Written");
    }

    // Brown Noise
    {
        println!("Generating brown_noise.wav...");
        let mut writer = hound::WavWriter::create("examples/brown_noise.wav", spec).unwrap();
        let mut gen = BrownNoise::new(12345);

        for _ in 0..num_samples {
            let sample = gen.next_sample();
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).unwrap();
        }
        writer.finalize().unwrap();
        println!("  ✓ Written");
    }

    // Fractal Brownian Motion
    {
        println!("Generating fbm_noise.wav (4 octaves, lacunarity=2.0, persistence=0.5)...");
        let mut writer = hound::WavWriter::create("examples/fbm_noise.wav", spec).unwrap();
        let mut gen = FractalBrownianMotion::new(12345, 4, 2.0, 0.5);

        for _ in 0..num_samples {
            let sample = gen.next_sample();
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).unwrap();
        }
        writer.finalize().unwrap();
        println!("  ✓ Written");
    }

    // fBM as slow LFO
    {
        println!("Generating fbm_lfo.wav (0.2 Hz LFO for reverb modulation)...");
        let mut writer = hound::WavWriter::create("examples/fbm_lfo.wav", spec).unwrap();
        let mut gen = FractalBrownianMotion::new(54321, 3, 2.0, 0.5);

        for _ in 0..num_samples {
            let sample = gen.next_sample_at_rate(0.2, sample_rate as f32);
            let amplitude = (sample * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).unwrap();
        }
        writer.finalize().unwrap();
        println!("  ✓ Written");
    }

    println!("\n✅ All audio files generated successfully!");
    println!("\nTo analyze spectral characteristics:");
    println!("  - Load files into Audacity, Sonic Visualizer, or similar");
    println!("  - View frequency spectrum");
    println!("  - White: flat spectrum (all frequencies equal)");
    println!("  - Pink:  ~-3 dB/octave slope (natural sounding)");
    println!("  - Brown: ~-6 dB/octave slope (dark, smooth)");
    println!("  - fBM:   Complex multi-octave structure");
    println!("  - fBM LFO: Very slow modulation (for reverb)");
}
