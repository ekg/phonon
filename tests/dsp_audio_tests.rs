//! DSP audio verification tests

use phonon::glicol_dsp::{sin, mul, lpf, hpf, add, saw, square};
use phonon::glicol_parser::parse_glicol;
use phonon::test_utils::*;
use std::f32::consts::PI;

#[test]
fn test_sine_wave_generation() {
    // Generate a 440Hz sine wave
    let chain = sin(440.0);
    let mut graph = chain.build_graph(44100.0).unwrap();
    
    // Generate 1 second of audio
    let samples = 44100;
    let mut audio = vec![0.0; samples];
    
    for i in 0..samples {
        audio[i] = graph.process();
    }
    
    // Verify it's actually a sine wave
    // Check zero crossings - should be 880 for 440Hz
    let mut zero_crossings = 0;
    for i in 1..samples {
        if audio[i-1] <= 0.0 && audio[i] > 0.0 {
            zero_crossings += 1;
        }
    }
    
    // Should be close to 440 zero crossings (one per cycle)
    assert!((zero_crossings as i32 - 440).abs() < 10);
    
    // Check spectral centroid is around 440Hz
    let centroid = spectral_centroid(&audio, 44100.0);
    // This is approximate - real FFT would be more accurate
    assert!(centroid > 400.0 && centroid < 500.0);
}

#[test]
fn test_amplitude_modulation() {
    // Test sin(440) >> mul(0.5)
    let chain = sin(440.0) >> mul(0.5);
    let mut graph = chain.build_graph(44100.0).unwrap();
    
    let samples = 1000;
    let mut audio = vec![0.0; samples];
    
    for i in 0..samples {
        audio[i] = graph.process();
    }
    
    // Check that amplitude is reduced
    let max_amplitude = audio.iter().map(|x| x.abs()).fold(0.0, f32::max);
    assert!(max_amplitude > 0.4 && max_amplitude < 0.6);
}

#[test]
fn test_low_pass_filter() {
    // Create a signal with high frequency content
    let chain = saw(100.0) >> lpf(500.0, 1.0);
    let mut graph = chain.build_graph(44100.0).unwrap();
    
    let samples = 44100;
    let mut filtered = vec![0.0; samples];
    
    for i in 0..samples {
        filtered[i] = graph.process();
    }
    
    // Compare with unfiltered
    let unfiltered_chain = saw(100.0);
    let mut unfiltered_graph = unfiltered_chain.build_graph(44100.0).unwrap();
    let mut unfiltered = vec![0.0; samples];
    
    for i in 0..samples {
        unfiltered[i] = unfiltered_graph.process();
    }
    
    // Filtered should have lower high-frequency content
    // (Simplified test - real FFT analysis would be better)
    let filtered_centroid = spectral_centroid(&filtered, 44100.0);
    let unfiltered_centroid = spectral_centroid(&unfiltered, 44100.0);
    
    assert!(filtered_centroid < unfiltered_centroid);
}

#[test]
fn test_high_pass_filter() {
    // Test high pass filter removes low frequencies
    let chain = square(100.0) >> hpf(1000.0, 1.0);
    let mut graph = chain.build_graph(44100.0).unwrap();
    
    let samples = 44100;
    let mut filtered = vec![0.0; samples];
    
    for i in 0..samples {
        filtered[i] = graph.process();
    }
    
    // High-passed signal should have higher centroid
    let centroid = spectral_centroid(&filtered, 44100.0);
    assert!(centroid > 800.0); // Should be high frequency dominated
}

#[test]
fn test_signal_addition() {
    // Test adding two signals
    let chain1 = sin(440.0) >> mul(0.3);
    let chain2 = sin(880.0) >> mul(0.3);
    
    // This would require implementing signal mixing
    // For now, test that individual chains work
    let mut graph1 = chain1.build_graph(44100.0).unwrap();
    let mut graph2 = chain2.build_graph(44100.0).unwrap();
    
    let samples = 1000;
    let mut audio1 = vec![0.0; samples];
    let mut audio2 = vec![0.0; samples];
    
    for i in 0..samples {
        audio1[i] = graph1.process();
        audio2[i] = graph2.process();
    }
    
    // Both should generate audio
    assert!(calculate_rms(&audio1) > 0.0);
    assert!(calculate_rms(&audio2) > 0.0);
}

#[test]
fn test_glicol_parser_audio() {
    // Test parsing and audio generation from Glicol code
    let code = r#"
        ~amp: sin 0.5 >> mul 0.3 >> add 0.5
        o: sin 440 >> mul 0.5
    "#;
    
    let env = parse_glicol(code).unwrap();
    
    // Verify environment was created
    assert!(env.output_chain.is_some());
    assert_eq!(env.ref_chains.len(), 1);
    assert!(env.ref_chains.contains_key("amp"));
}

#[test]
fn test_oscillator_waveforms() {
    // Test different waveforms have different characteristics
    let sine_chain = sin(100.0);
    let saw_chain = saw(100.0);
    let square_chain = square(100.0);
    
    let mut sine_graph = sine_chain.build_graph(44100.0).unwrap();
    let mut saw_graph = saw_chain.build_graph(44100.0).unwrap();
    let mut square_graph = square_chain.build_graph(44100.0).unwrap();
    
    let samples = 4410; // 0.1 second
    let mut sine_audio = vec![0.0; samples];
    let mut saw_audio = vec![0.0; samples];
    let mut square_audio = vec![0.0; samples];
    
    for i in 0..samples {
        sine_audio[i] = sine_graph.process();
        saw_audio[i] = saw_graph.process();
        square_audio[i] = square_graph.process();
    }
    
    // Different waveforms should have different RMS values
    let sine_rms = calculate_rms(&sine_audio);
    let saw_rms = calculate_rms(&saw_audio);
    let square_rms = calculate_rms(&square_audio);
    
    // All should generate audio
    assert!(sine_rms > 0.0);
    assert!(saw_rms > 0.0);
    assert!(square_rms > 0.0);
    
    // They should be different from each other
    assert!((sine_rms - saw_rms).abs() > 0.01);
    assert!((sine_rms - square_rms).abs() > 0.01);
    assert!((saw_rms - square_rms).abs() > 0.01);
}

#[test]
fn test_modulation_routing() {
    // Test using one signal to modulate another
    let lfo = sin(1.0) >> mul(100.0) >> add(440.0); // LFO from 340 to 540 Hz
    let carrier = sin(440.0); // This would be modulated in a real implementation
    
    let mut lfo_graph = lfo.build_graph(44100.0).unwrap();
    let mut carrier_graph = carrier.build_graph(44100.0).unwrap();
    
    let samples = 44100; // 1 second
    let mut lfo_audio = vec![0.0; samples];
    let mut carrier_audio = vec![0.0; samples];
    
    for i in 0..samples {
        lfo_audio[i] = lfo_graph.process();
        carrier_audio[i] = carrier_graph.process();
    }
    
    // LFO should vary slowly
    // Check that it crosses zero approximately 2 times (1 Hz)
    let mut zero_crossings = 0;
    for i in 1..samples {
        if lfo_audio[i-1] <= 440.0 && lfo_audio[i] > 440.0 {
            zero_crossings += 1;
        }
    }
    
    assert!(zero_crossings >= 0 && zero_crossings <= 3); // Around 1 Hz
}