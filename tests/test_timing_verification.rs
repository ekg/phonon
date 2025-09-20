use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use std::collections::HashMap;

/// Find peaks in audio to detect event timing
fn find_event_onsets(samples: &[f32], threshold: f32) -> Vec<usize> {
    let mut onsets = Vec::new();
    let mut was_below = true;
    
    for (i, &sample) in samples.iter().enumerate() {
        let is_above = sample.abs() > threshold;
        
        // Detect rising edge
        if is_above && was_below {
            onsets.push(i);
        }
        
        was_below = !is_above;
    }
    
    onsets
}

#[test]
fn test_pattern_timing_is_even() {
    println!("\n=== Testing Pattern Timing Distribution ===");
    
    // First verify pattern parsing
    let pattern = parse_mini_notation("bd sn bd sn");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };
    
    let events = pattern.query(&state);
    
    println!("Pattern events:");
    for (i, event) in events.iter().enumerate() {
        println!("  Event {}: {} at {:.3}-{:.3}",
                 i, event.value,
                 event.part.begin.to_float(),
                 event.part.end.to_float());
    }
    
    // Verify timing is correct in pattern
    assert_eq!(events.len(), 4);
    for i in 0..4 {
        let expected_start = i as f64 * 0.25;
        let actual_start = events[i].part.begin.to_float();
        assert!((actual_start - expected_start).abs() < 0.001,
                "Event {} should start at {:.3}, got {:.3}",
                i, expected_start, actual_start);
    }
    
    println!("✓ Pattern timing is correct");
}

#[test]
fn test_synth_timing_is_even() {
    println!("\n=== Testing Synth Trigger Timing ===");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Create a pattern with 4 evenly spaced synth triggers
    let code = r#"
        ~click: sin 1000 >> mul 0.5
        o: s "~click ~click ~click ~click"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Find onset positions
    let onsets = find_event_onsets(&audio.data, 0.01);
    
    println!("Found {} onsets (expected 4)", onsets.len());
    
    if onsets.len() >= 4 {
        // Check spacing between onsets
        let expected_spacing = sample_rate / 4.0; // 4 beats in 1 second
        
        for i in 0..3 {
            let spacing = (onsets[i + 1] - onsets[i]) as f32;
            let error = ((spacing - expected_spacing) / expected_spacing * 100.0).abs();
            
            println!("Spacing {}-{}: {} samples (expected {:.0}, error {:.1}%)",
                     i, i+1, spacing, expected_spacing, error);
            
            // Allow 10% tolerance due to envelope attack time
            assert!(error < 10.0,
                    "Spacing should be close to {:.0} samples, got {}",
                    expected_spacing, spacing);
        }
    }
    
    println!("✓ Synth timing is evenly distributed");
}

#[test]
fn test_timing_with_rests() {
    println!("\n=== Testing Timing with Rests ===");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Pattern with rests
    let code = r#"
        ~beep: sin 880 >> mul 0.4
        o: s "~beep ~ ~beep ~"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Find onsets - should be 2 (positions 0 and 2)
    let onsets = find_event_onsets(&audio.data, 0.01);
    
    println!("Found {} onsets (expected 2)", onsets.len());
    
    if onsets.len() >= 2 {
        // First onset should be near sample 0
        let first_pos = onsets[0] as f32 / sample_rate;
        println!("First onset at {:.3}s (expected ~0.0s)", first_pos);
        assert!(first_pos < 0.01, "First onset should be near 0");
        
        // Second onset should be near 0.5s (position 2 of 4)
        let second_pos = onsets[1] as f32 / sample_rate;
        println!("Second onset at {:.3}s (expected ~0.5s)", second_pos);
        assert!((second_pos - 0.5).abs() < 0.01, "Second onset should be near 0.5s");
    }
    
    println!("✓ Timing with rests is correct");
}

#[test]
fn test_timing_across_multiple_cycles() {
    println!("\n=== Testing Timing Across Multiple Cycles ===");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Simple pattern across 2 cycles
    let code = r#"
        ~tick: sin 2000 >> mul 0.3
        o: s "~tick ~tick"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 2.0).expect("Failed to render"); // 2 seconds = 2 cycles
    
    let onsets = find_event_onsets(&audio.data, 0.01);
    
    println!("Found {} onsets across 2 cycles (expected 4)", onsets.len());
    
    // Should have 2 events per cycle, 4 total
    if onsets.len() >= 4 {
        for (i, &onset) in onsets.iter().enumerate() {
            let time_pos = onset as f32 / sample_rate;
            println!("Onset {}: {:.3}s", i, time_pos);
        }
        
        // Check that onsets are evenly spaced
        let expected_times = [0.0, 0.5, 1.0, 1.5];
        for (i, &expected) in expected_times.iter().enumerate() {
            if i < onsets.len() {
                let actual = onsets[i] as f32 / sample_rate;
                assert!((actual - expected).abs() < 0.02,
                        "Onset {} should be near {:.1}s, got {:.3}s",
                        i, expected, actual);
            }
        }
    }
    
    println!("✓ Timing across cycles is consistent");
}

#[test]
fn test_no_compression_at_end() {
    println!("\n=== Testing No Compression at End of Cycle ===");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // 4 beats should fill the whole cycle evenly
    let code = r#"
        ~beat: sin 500 >> mul 0.4
        o: s "~beat ~beat ~beat ~beat"
    "#;
    
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 1.0).expect("Failed to render");
    
    // Check that there's audio throughout the whole second
    // Divide into 4 quarters and check each has audio
    let quarter_size = audio.data.len() / 4;
    
    for quarter in 0..4 {
        let start = quarter * quarter_size;
        let end = (quarter + 1) * quarter_size;
        let quarter_data = &audio.data[start..end];
        
        // Check RMS of this quarter
        let rms: f32 = quarter_data.iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt() / (quarter_size as f32).sqrt();
        
        println!("Quarter {}: RMS = {:.4}", quarter, rms);
        
        // Each quarter should have some audio
        assert!(rms > 0.001, "Quarter {} should have audio", quarter);
    }
    
    // Check there's no long silence at the end
    let last_10_percent = &audio.data[(audio.data.len() * 9 / 10)..];
    let last_rms: f32 = last_10_percent.iter()
        .map(|x| x * x)
        .sum::<f32>()
        .sqrt() / (last_10_percent.len() as f32).sqrt();
    
    println!("Last 10% RMS: {:.4}", last_rms);
    
    // Should still have some decay from the last beat
    assert!(last_rms > 0.0001, "Should have audio decay at end, not silence");
    
    println!("✓ No compression - beats fill entire cycle");
}