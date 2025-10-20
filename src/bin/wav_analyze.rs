use rustfft::{FftPlanner, num_complex::Complex};
use std::env;
use std::f32::consts::PI;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <wav_file> [--verbose]", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let verbose = args.len() > 2 && args[2] == "--verbose";

    match analyze_wav(filename, verbose) {
        Ok(analysis) => {
            println!("{}", analysis.format_report());

            // Exit with error code if audio is empty
            if analysis.is_empty {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error analyzing {filename}: {e}");
            std::process::exit(2);
        }
    }
}

#[derive(Debug)]
struct AudioAnalysis {
    filename: String,
    sample_rate: u32,
    duration_secs: f32,
    num_samples: usize,

    // Basic metrics
    rms: f32,
    peak: f32,
    dc_offset: f32,
    zero_crossings: usize,

    // Spectral
    spectral_centroid: f32,
    dominant_frequency: f32,

    // Rhythm
    estimated_bpm: Option<f32>,
    onset_count: usize,

    // Overall
    is_empty: bool,
    is_clipping: bool,
}

impl AudioAnalysis {
    fn format_report(&self) -> String {
        let mut report = String::new();

        report.push_str(&format!("=== WAV Analysis: {} ===\n", self.filename));
        report.push_str(&format!("Duration:    {:.3} seconds\n", self.duration_secs));
        report.push_str(&format!("Sample Rate: {} Hz\n", self.sample_rate));
        report.push_str(&format!("Samples:     {}\n", self.num_samples));
        report.push('\n');

        if self.is_empty {
            report.push_str("❌ EMPTY AUDIO (silence detected)\n");
        } else {
            report.push_str("✅ Contains audio signal\n");
        }

        report.push_str("\n[Level Analysis]\n");
        report.push_str(&format!(
            "RMS Level:   {:.3} ({:.1} dB)\n",
            self.rms,
            20.0 * self.rms.log10()
        ));
        report.push_str(&format!(
            "Peak Level:  {:.3} ({:.1} dB)\n",
            self.peak,
            20.0 * self.peak.log10()
        ));
        report.push_str(&format!("DC Offset:   {:.6}\n", self.dc_offset));

        if self.is_clipping {
            report.push_str("⚠️  CLIPPING DETECTED\n");
        }

        report.push_str("\n[Frequency Analysis]\n");
        report.push_str(&format!("Zero Crossings:     {}\n", self.zero_crossings));
        report.push_str(&format!(
            "Est. Base Freq:     {:.1} Hz\n",
            self.zero_crossings as f32 / (2.0 * self.duration_secs)
        ));
        report.push_str(&format!(
            "Dominant Freq:      {:.1} Hz\n",
            self.dominant_frequency
        ));
        report.push_str(&format!(
            "Spectral Centroid:  {:.1} Hz\n",
            self.spectral_centroid
        ));

        report.push_str("\n[Rhythm Analysis]\n");
        report.push_str(&format!("Onset Events: {}\n", self.onset_count));
        if let Some(bpm) = self.estimated_bpm {
            report.push_str(&format!("Estimated BPM: {bpm:.1}\n"));
        } else {
            report.push_str("Estimated BPM: N/A\n");
        }

        report
    }
}

fn analyze_wav(filename: &str, verbose: bool) -> Result<AudioAnalysis, Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(filename)?;
    let spec = reader.spec();

    if verbose {
        println!(
            "Loading WAV: {} channels, {} Hz, {} bits",
            spec.channels, spec.sample_rate, spec.bits_per_sample
        );
    }

    // Read all samples
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap_or(0) as f32 / max_val)
                .collect()
        }
    };

    // If multi-channel, mix to mono for analysis
    let mono_samples: Vec<f32> = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
            .collect()
    } else {
        samples
    };

    let num_samples = mono_samples.len();
    let duration_secs = num_samples as f32 / spec.sample_rate as f32;

    // Basic metrics
    let rms = calculate_rms(&mono_samples);
    let peak = mono_samples.iter().map(|x| x.abs()).fold(0.0, f32::max);
    let dc_offset = mono_samples.iter().sum::<f32>() / num_samples as f32;
    let zero_crossings = count_zero_crossings(&mono_samples);

    // Check if empty
    let is_empty = rms < 0.0001 && peak < 0.001;
    let is_clipping = mono_samples.iter().any(|&x| x.abs() >= 0.999);

    // Spectral analysis
    let (dominant_frequency, spectral_centroid) = if !is_empty {
        analyze_spectrum(&mono_samples, spec.sample_rate)
    } else {
        (0.0, 0.0)
    };

    // Rhythm analysis
    let (onset_count, estimated_bpm) = if !is_empty {
        analyze_rhythm(&mono_samples, spec.sample_rate)
    } else {
        (0, None)
    };

    Ok(AudioAnalysis {
        filename: filename.to_string(),
        sample_rate: spec.sample_rate,
        duration_secs,
        num_samples,
        rms,
        peak,
        dc_offset,
        zero_crossings,
        spectral_centroid,
        dominant_frequency,
        onset_count,
        estimated_bpm,
        is_empty,
        is_clipping,
    })
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}

fn count_zero_crossings(samples: &[f32]) -> usize {
    if samples.len() < 2 {
        return 0;
    }

    let mut crossings = 0;
    let mut last_sign = samples[0] >= 0.0;

    for &sample in &samples[1..] {
        let current_sign = sample >= 0.0;
        if current_sign != last_sign {
            crossings += 1;
            last_sign = current_sign;
        }
    }

    crossings
}

fn analyze_spectrum(samples: &[f32], sample_rate: u32) -> (f32, f32) {
    // Use rustfft for efficient FFT computation
    let window_size = 2048.min(samples.len());
    let window = &samples[..window_size];

    // Apply Hamming window
    let windowed: Vec<f32> = window
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let window_val = 0.54 - 0.46 * (2.0 * PI * i as f32 / (window_size - 1) as f32).cos();
            x * window_val
        })
        .collect();

    // Convert to complex numbers for FFT
    let mut buffer: Vec<Complex<f32>> = windowed
        .iter()
        .map(|&x| Complex { re: x, im: 0.0 })
        .collect();

    // Perform FFT
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);
    fft.process(&mut buffer);

    // Calculate magnitudes
    let num_bins = window_size / 2; // Only use positive frequencies
    let magnitudes: Vec<f32> = buffer[..num_bins]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();

    // Find dominant frequency (skip DC component at bin 0)
    let (dominant_bin, max_magnitude) = magnitudes[1..]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, &mag)| (i + 1, mag))
        .unwrap_or((0, 0.0));

    // Calculate dominant frequency
    let bin_width = sample_rate as f32 / window_size as f32;
    let dominant_frequency = dominant_bin as f32 * bin_width;

    // Calculate spectral centroid (weighted average of frequencies)
    let mut weighted_sum = 0.0;
    let mut magnitude_sum = 0.0;

    for (i, &mag) in magnitudes.iter().enumerate().skip(1) {
        // Skip DC
        let freq = i as f32 * bin_width;
        weighted_sum += freq * mag;
        magnitude_sum += mag;
    }

    let spectral_centroid = if magnitude_sum > 0.0 {
        weighted_sum / magnitude_sum
    } else {
        0.0
    };

    (dominant_frequency, spectral_centroid)
}

fn analyze_rhythm(samples: &[f32], sample_rate: u32) -> (usize, Option<f32>) {
    // Better onset detection using spectral flux
    let window_size = (sample_rate as usize / 50).max(128); // 20ms windows
    let hop_size = window_size / 2;

    let mut energies = Vec::new();
    let mut i = 0;

    // Calculate energy in each window
    while i + window_size < samples.len() {
        let window = &samples[i..i + window_size];
        let energy = window.iter().map(|x| x * x).sum::<f32>() / window_size as f32;
        energies.push(energy);
        i += hop_size;
    }

    // Smooth energies to reduce noise
    let mut smoothed = Vec::new();
    for i in 0..energies.len() {
        let start = i.saturating_sub(2);
        let end = (i + 3).min(energies.len());
        let avg = energies[start..end].iter().sum::<f32>() / (end - start) as f32;
        smoothed.push(avg);
    }

    // Find peaks using adaptive threshold
    let mean_energy: f32 = smoothed.iter().sum::<f32>() / smoothed.len() as f32;
    let mut std_dev = 0.0;
    for &e in &smoothed {
        std_dev += (e - mean_energy).powi(2);
    }
    std_dev = (std_dev / smoothed.len() as f32).sqrt();

    // Dynamic threshold based on statistics
    let threshold = mean_energy + std_dev * 1.5;

    let mut peaks = Vec::new();
    let mut in_peak = false;
    let mut peak_start = 0;

    // Minimum time between peaks (prevents double detection)
    let min_peak_distance = (sample_rate as usize / 10) / hop_size; // 100ms

    for i in 1..smoothed.len() {
        if smoothed[i] > threshold && smoothed[i] > smoothed[i - 1] {
            if !in_peak {
                in_peak = true;
                peak_start = i;
            }
        } else if in_peak && smoothed[i] < smoothed[i - 1] {
            // Peak ended, record it
            in_peak = false;

            // Check minimum distance from last peak
            if peaks.is_empty() || i - peaks.last().unwrap() > min_peak_distance {
                peaks.push(peak_start);
            }
        }
    }

    let onset_count = peaks.len();

    // Estimate BPM from peak intervals
    let estimated_bpm = if peaks.len() > 4 {
        let mut intervals = Vec::new();
        for i in 1..peaks.len() {
            let interval_samples = (peaks[i] - peaks[i - 1]) * hop_size;
            let interval_secs = interval_samples as f32 / sample_rate as f32;
            intervals.push(interval_secs);
        }

        // Find most common interval (simple histogram)
        intervals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_interval = intervals[intervals.len() / 2];

        // Convert to BPM
        if median_interval > 0.1 && median_interval < 2.0 {
            Some(60.0 / median_interval)
        } else {
            None
        }
    } else {
        None
    };

    (onset_count, estimated_bpm)
}
