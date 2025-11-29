//! Generate synthetic drum samples for testing
//!
//! Creates minimal synthetic samples: bd, sn, hh, cp, blip

use std::f32::consts::PI;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const SAMPLE_RATE: u32 = 44100;

fn main() {
    let samples_dir = Path::new("samples");

    // Generate kick drums (bd)
    generate_kick(&samples_dir.join("bd/0.wav"), 60.0, 0.3);
    generate_kick(&samples_dir.join("bd/1.wav"), 50.0, 0.4);
    generate_kick(&samples_dir.join("bd/2.wav"), 70.0, 0.25);

    // Generate snares (sn)
    generate_snare(&samples_dir.join("sn/0.wav"), 200.0, 0.2);
    generate_snare(&samples_dir.join("sn/1.wav"), 180.0, 0.25);

    // Generate hi-hats (hh)
    generate_hihat(&samples_dir.join("hh/0.wav"), 0.1, 0.8);
    generate_hihat(&samples_dir.join("hh/1.wav"), 0.05, 0.9);
    generate_hihat(&samples_dir.join("hh/2.wav"), 0.15, 0.7);

    // Generate claps (cp)
    generate_clap(&samples_dir.join("cp/0.wav"), 0.15);
    generate_clap(&samples_dir.join("cp/1.wav"), 0.2);

    // Generate blips
    generate_blip(&samples_dir.join("blip/0.wav"), 880.0, 0.05);
    generate_blip(&samples_dir.join("blip/1.wav"), 1320.0, 0.04);

    println!("Generated all samples in samples/");
}

fn write_wav(path: &Path, samples: &[f32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let file = File::create(path).expect("Failed to create file");
    let writer = BufWriter::new(file);
    let mut wav_writer = hound::WavWriter::new(writer, spec).expect("Failed to create WAV writer");

    for &sample in samples {
        let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        wav_writer.write_sample(s).expect("Failed to write sample");
    }

    wav_writer.finalize().expect("Failed to finalize WAV");
    println!("  Created: {:?}", path);
}

/// Generate a kick drum: sine wave with pitch envelope
fn generate_kick(path: &Path, base_freq: f32, duration: f32) {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; num_samples];

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (-t * 10.0).exp(); // Fast decay
        let pitch_env = 1.0 + 4.0 * (-t * 30.0).exp(); // Pitch drops quickly
        let freq = base_freq * pitch_env;
        let phase = 2.0 * PI * freq * t;
        samples[i] = (phase.sin() * env * 0.9).tanh(); // Soft clip
    }

    write_wav(path, &samples);
}

/// Generate a snare: noise + tone with envelope
fn generate_snare(path: &Path, tone_freq: f32, duration: f32) {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; num_samples];
    let mut noise_state: u32 = 12345;

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Noise component (simple LCG)
        noise_state = noise_state.wrapping_mul(1103515245).wrapping_add(12345);
        let noise = (noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0;

        // Tone component
        let tone = (2.0 * PI * tone_freq * t).sin();

        // Envelopes
        let noise_env = (-t * 20.0).exp();
        let tone_env = (-t * 40.0).exp();

        samples[i] = (noise * noise_env * 0.6 + tone * tone_env * 0.4) * 0.8;
    }

    write_wav(path, &samples);
}

/// Generate a hi-hat: filtered noise
fn generate_hihat(path: &Path, duration: f32, brightness: f32) {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; num_samples];
    let mut noise_state: u32 = 67890;
    let mut hp_state = 0.0f32;

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Noise
        noise_state = noise_state.wrapping_mul(1103515245).wrapping_add(12345);
        let noise = (noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0;

        // Simple highpass for brightness
        let cutoff = 0.1 + brightness * 0.3;
        hp_state = hp_state * (1.0 - cutoff) + noise * cutoff;
        let hp_out = noise - hp_state;

        // Envelope
        let env = (-t * 30.0).exp();

        samples[i] = hp_out * env * 0.7;
    }

    write_wav(path, &samples);
}

/// Generate a clap: multiple noise bursts
fn generate_clap(path: &Path, duration: f32) {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; num_samples];
    let mut noise_state: u32 = 11111;

    // Multiple micro-bursts for clap texture
    let bursts = [0.0, 0.01, 0.02, 0.025];

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        noise_state = noise_state.wrapping_mul(1103515245).wrapping_add(12345);
        let noise = (noise_state as f32 / u32::MAX as f32) * 2.0 - 1.0;

        let mut env = 0.0f32;
        for &burst_t in &bursts {
            if t >= burst_t {
                let dt = t - burst_t;
                env += (-dt * 50.0).exp() * 0.5;
            }
        }
        // Main tail
        env += (-t * 15.0).exp() * 0.5;

        samples[i] = noise * env * 0.8;
    }

    write_wav(path, &samples);
}

/// Generate a blip: short sine burst
fn generate_blip(path: &Path, freq: f32, duration: f32) {
    let num_samples = (duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; num_samples];

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = (-t * 40.0).exp();
        let osc = (2.0 * PI * freq * t).sin();
        samples[i] = osc * env * 0.8;
    }

    write_wav(path, &samples);
}
