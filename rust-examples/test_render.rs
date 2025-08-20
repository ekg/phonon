//! End-to-end test of the Phonon render system
//! Run with: cargo run --example test_render

use fermion::render::{RenderConfig, Renderer};
use std::path::Path;

fn main() {
    println!("🧪 Phonon Render System Test");
    println!("=============================\n");
    
    // Test 1: Simple sine wave
    test_sine_wave();
    
    // Test 2: Complex modulation
    test_complex_modulation();
    
    // Test 3: Various durations
    test_durations();
    
    // Test 4: Edge cases
    test_edge_cases();
    
    println!("\n✅ All render tests passed!");
}

fn test_sine_wave() {
    println!("Test 1: Simple Sine Wave");
    println!("------------------------");
    
    let dsl = r#"
~osc: sine(440)
out: ~osc * 0.5
"#;
    
    let config = RenderConfig {
        duration: 1.0,
        ..Default::default()
    };
    
    let renderer = Renderer::new(config);
    let stats = renderer.render_to_file(dsl, Path::new("/tmp/test_sine_440.wav"))
        .expect("Failed to render");
    
    assert_eq!(stats.sample_count, 44100, "Should have 44100 samples for 1 second");
    assert!(stats.rms > 0.3 && stats.rms < 0.4, "RMS should be ~0.35");
    
    // Check frequency estimation
    let est_freq = stats.zero_crossings as f32 / 2.0;
    assert!((est_freq - 440.0).abs() < 10.0, "Frequency should be close to 440 Hz");
    
    println!("  ✓ Rendered 1 second of 440Hz sine wave");
    println!("    RMS: {:.3}, Peak: {:.3}, Freq: ~{:.0} Hz", 
             stats.rms, stats.peak, est_freq);
}

fn test_complex_modulation() {
    println!("\nTest 2: Complex Modulation");
    println!("--------------------------");
    
    let dsl = r#"
~lfo1: sine(2) * 100
~lfo2: sine(0.5) * 0.5 + 0.5
~carrier: sine(440 + ~lfo1)
~filtered: ~carrier >> lpf(500 + ~lfo2 * 1500, 1.0)
out: ~filtered * 0.4
"#;
    
    let config = RenderConfig {
        duration: 2.0,
        ..Default::default()
    };
    
    let renderer = Renderer::new(config);
    let stats = renderer.render_to_file(dsl, Path::new("/tmp/test_complex.wav"))
        .expect("Failed to render");
    
    assert_eq!(stats.sample_count, 88200, "Should have 88200 samples for 2 seconds");
    assert!(stats.peak > 0.0, "Should have non-zero output");
    assert!(stats.peak < 1.0, "Should not clip");
    
    println!("  ✓ Rendered 2 seconds of complex modulation");
    println!("    RMS: {:.3}, Peak: {:.3}", stats.rms, stats.peak);
}

fn test_durations() {
    println!("\nTest 3: Various Durations");
    println!("-------------------------");
    
    let dsl = r#"
~osc: saw(110)
out: ~osc * 0.3
"#;
    
    let durations = [0.1, 0.5, 1.0, 5.0, 10.0];
    
    for duration in durations {
        let config = RenderConfig {
            duration,
            ..Default::default()
        };
        
        let renderer = Renderer::new(config);
        let samples = renderer.render_to_buffer(dsl)
            .expect("Failed to render");
        
        let expected_samples = (duration * 44100.0) as usize;
        assert_eq!(samples.len(), expected_samples, 
                   "Wrong sample count for {} seconds", duration);
        
        println!("  ✓ {} seconds: {} samples", duration, samples.len());
    }
}

fn test_edge_cases() {
    println!("\nTest 4: Edge Cases");
    println!("------------------");
    
    // Test very short duration
    let config = RenderConfig {
        duration: 0.001, // 1ms
        fade_in: 0.0,
        fade_out: 0.0,
        ..Default::default()
    };
    
    let renderer = Renderer::new(config);
    let samples = renderer.render_to_buffer("out: sine(1000) * 0.5")
        .expect("Failed to render");
    
    assert_eq!(samples.len(), 44, "Should have 44 samples for 1ms");
    println!("  ✓ Very short duration (1ms)");
    
    // Test with high gain
    let config = RenderConfig {
        duration: 0.1,
        master_gain: 2.0, // Over-gain to test clipping prevention
        ..Default::default()
    };
    
    let renderer = Renderer::new(config);
    let samples = renderer.render_to_buffer("out: sine(440) * 0.8")
        .expect("Failed to render");
    
    let peak = samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(peak > 1.0, "With 2.0 gain, should exceed 1.0");
    println!("  ✓ High gain handling (peak: {:.2})", peak);
    
    // Test empty/silence
    let config = RenderConfig::default();
    let renderer = Renderer::new(config);
    let samples = renderer.render_to_buffer("out: sine(440) * 0.0")
        .expect("Failed to render");
    
    let peak = samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    assert!(peak < 0.01, "Should be nearly silent");
    println!("  ✓ Silence rendering");
}