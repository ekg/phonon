/// FDN Reverb Demo
///
/// This example demonstrates the Feedback Delay Network reverb.
/// It processes a simple impulse and saves the reverb tail to a WAV file.

use phonon::nodes::FdnState;
use std::fs::File;
use std::io::Write;

fn main() {
    println!("FDN Reverb Demo");
    println!("===============\n");

    let sample_rate = 44100.0;
    let mut reverb = FdnState::new(sample_rate);

    // Parameters
    let decay = 0.98;      // Long decay time (0.0 to 0.9999)
    let damping = 0.3;     // Moderate high-frequency damping (0.0 to 1.0)

    println!("Sample rate: {} Hz", sample_rate);
    println!("Decay: {}", decay);
    println!("Damping: {}", damping);
    println!();

    // Generate impulse response
    let duration_seconds = 3.0;
    let num_samples = (sample_rate * duration_seconds) as usize;
    let mut output = Vec::with_capacity(num_samples);

    // Send impulse
    let first_sample = reverb.process(1.0, decay, damping);
    output.push(first_sample);

    // Process silence to capture reverb tail
    for _ in 1..num_samples {
        let sample = reverb.process(0.0, decay, damping);
        output.push(sample);
    }

    // Calculate some statistics
    let max_amplitude = output.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    let rms = (output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32).sqrt();

    println!("Output statistics:");
    println!("  Max amplitude: {:.6}", max_amplitude);
    println!("  RMS level: {:.6}", rms);
    println!("  Samples: {}", output.len());
    println!();

    // Normalize to prevent clipping
    let normalization = 1.0 / max_amplitude.max(1e-6);
    for sample in &mut output {
        *sample *= normalization * 0.8; // Scale to 80% to leave headroom
    }

    // Save to WAV file
    let filename = "fdn_reverb_impulse.wav";
    match save_wav(filename, &output, sample_rate as u32) {
        Ok(_) => println!("Saved reverb impulse response to: {}", filename),
        Err(e) => eprintln!("Error saving WAV file: {}", e),
    }

    println!();
    println!("The FDN reverb uses:");
    println!("  - 8 delay lines with coprime lengths");
    println!("  - Householder mixing matrix for efficient diffusion");
    println!("  - Per-channel lowpass damping for natural decay");
    println!();
    println!("Try different parameters:");
    println!("  - decay: 0.8 (short), 0.95 (medium), 0.99 (long)");
    println!("  - damping: 0.0 (bright), 0.5 (neutral), 0.9 (dark)");
}

/// Simple WAV file writer (mono, 16-bit PCM)
fn save_wav(filename: &str, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
    let mut file = File::create(filename)?;

    let num_samples = samples.len() as u32;
    let num_channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = num_channels * (bits_per_sample / 8);
    let data_size = num_samples * num_channels as u32 * (bits_per_sample / 8) as u32;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Chunk size
    file.write_all(&1u16.to_le_bytes())?;  // Audio format (1 = PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Write samples as 16-bit PCM
    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
        file.write_all(&sample_i16.to_le_bytes())?;
    }

    Ok(())
}
