//! Demonstration of the noise generators for reverb modulation
//!
//! This example shows how to use the different noise generators
//! (white, pink, brown, and fractal Brownian motion) and their
//! characteristics.
//!
//! Run with: cargo run --example noise_generators_demo

use phonon::nodes::noise_generators::{BrownNoise, FractalBrownianMotion, PinkNoise, WhiteNoise};

fn main() {
    println!("=== Noise Generators Demo ===\n");

    // White Noise - flat spectrum
    println!("1. White Noise (flat spectrum):");
    let mut white = WhiteNoise::new(12345);
    print!("   First 10 samples: ");
    for _ in 0..10 {
        print!("{:.3} ", white.next_sample());
    }
    println!("\n");

    // Pink Noise - 1/f spectrum (~-3dB/octave)
    println!("2. Pink Noise (1/f spectrum, -3dB/octave):");
    let mut pink = PinkNoise::new(12345);
    print!("   First 10 samples: ");
    for _ in 0..10 {
        print!("{:.3} ", pink.next_sample());
    }
    println!("\n");

    // Brown Noise - 1/f² spectrum (~-6dB/octave)
    println!("3. Brown Noise (1/f² spectrum, -6dB/octave):");
    let mut brown = BrownNoise::new(12345);
    print!("   First 10 samples: ");
    for _ in 0..10 {
        print!("{:.3} ", brown.next_sample());
    }
    println!("\n");

    // Fractal Brownian Motion - multi-octave noise
    println!("4. Fractal Brownian Motion (fBM):");
    println!("   Parameters: 4 octaves, lacunarity=2.0, persistence=0.5");
    let mut fbm = FractalBrownianMotion::new(12345, 4, 2.0, 0.5);
    print!("   First 10 samples: ");
    for _ in 0..10 {
        print!("{:.3} ", fbm.next_sample());
    }
    println!("\n");

    // fBM with different parameters
    println!("5. fBM with high persistence (0.75) - more detail:");
    let mut fbm_detailed = FractalBrownianMotion::new(12345, 6, 2.0, 0.75);
    print!("   First 10 samples: ");
    for _ in 0..10 {
        print!("{:.3} ", fbm_detailed.next_sample());
    }
    println!("\n");

    // fBM with rate control (LFO-style)
    println!("6. fBM as LFO (0.5 Hz at 44.1kHz):");
    let mut fbm_lfo = FractalBrownianMotion::new(54321, 3, 2.0, 0.5);
    print!("   Samples at 0.1s intervals: ");
    for i in 0..10 {
        // Generate samples for 0.1 second intervals
        for _ in 0..4410 {
            fbm_lfo.next_sample_at_rate(0.5, 44100.0);
        }
        if i < 9 {
            print!("{:.3} ", fbm_lfo.next_sample_at_rate(0.5, 44100.0));
        }
    }
    println!("\n");

    // Statistical analysis
    println!("7. Statistical Analysis (10000 samples each):");

    let analyze = |name: &str, samples: Vec<f32>| {
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        let rms = (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt();
        let min = samples.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = samples.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        println!("   {}:", name);
        println!("      Mean:  {:.6} (should be near 0)", mean);
        println!("      RMS:   {:.6}", rms);
        println!("      Range: [{:.3}, {:.3}]", min, max);
    };

    let mut white = WhiteNoise::new(99999);
    let white_samples: Vec<f32> = (0..10000).map(|_| white.next_sample()).collect();
    analyze("White Noise", white_samples);

    let mut pink = PinkNoise::new(99999);
    let pink_samples: Vec<f32> = (0..10000).map(|_| pink.next_sample()).collect();
    analyze("Pink Noise", pink_samples);

    let mut brown = BrownNoise::new(99999);
    let brown_samples: Vec<f32> = (0..10000).map(|_| brown.next_sample()).collect();
    analyze("Brown Noise", brown_samples);

    println!("\n=== Use Cases for Reverb ===\n");
    println!("• White Noise:  Early reflections, diffusion");
    println!("• Pink Noise:   Natural room character, warm reverb tails");
    println!("• Brown Noise:  Smooth modulation, gentle chorus effects");
    println!("• fBM:          Complex, natural-sounding modulation");
    println!("                - Low octaves: slow drift");
    println!("                - High octaves: texture and shimmer");
}
