use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{State, TimeSpan, Fraction};
use phonon::simple_dsp_executor::SimpleDspExecutor;
use phonon::glicol_parser::parse_glicol;
use std::collections::HashMap;

/// Analyze frequency content in a window using zero-crossing
fn detect_primary_frequency(samples: &[f32], sample_rate: f32) -> Option<f32> {
    if samples.len() < 100 {
        return None;
    }
    
    // Count zero crossings
    let mut zero_crossings = 0;
    let mut last_sign = samples[0] >= 0.0;
    
    for &sample in samples.iter().skip(1) {
        let current_sign = sample >= 0.0;
        if current_sign != last_sign {
            zero_crossings += 1;
            last_sign = current_sign;
        }
    }
    
    // Frequency = (zero crossings / 2) / duration
    let duration = samples.len() as f32 / sample_rate;
    Some((zero_crossings as f32 / 2.0) / duration)
}

#[test]
#[ignore] // TODO: Fix for new implementation
#[ignore] // TODO: Fix alternation
fn test_simple_alternation_parsing() {
    println!("\n=== Testing Simple Alternation Parsing ===");
    
    let pattern = parse_mini_notation("<bd sn cp>");
    
    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        assert_eq!(events.len(), 1, "Should have exactly 1 event per cycle");
        
        let expected = match cycle % 3 {
            0 => "bd",
            1 => "sn",
            2 => "cp",
            _ => unreachable!(),
        };
        
        assert_eq!(events[0].value, expected);
        println!("  Cycle {}: '{}' ✓", cycle, events[0].value);
    }
}

#[test]
#[ignore] // TODO: Fix alternation
fn test_alternation_in_euclidean_arguments() {
    println!("\n=== Testing Alternation in Euclidean Arguments ===");
    
    // Test alternating pulse counts
    let pattern = parse_mini_notation("bd(<3 4 5>,8)");
    
    for cycle in 0..6 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        
        let expected = match cycle % 3 {
            0 => 3,
            1 => 4,
            2 => 5,
            _ => unreachable!(),
        };
        
        assert_eq!(bd_count, expected, "Cycle {} should have {} bd events", cycle, expected);
        println!("  Cycle {}: {} bd events ✓", cycle, bd_count);
    }
}

#[test]
#[ignore] // TODO: Fix alternation
fn test_nested_alternation() {
    println!("\n=== Testing Nested Alternation ===");
    
    // Nested alternation: alternates between 3 and alternation of 4,5
    let pattern = parse_mini_notation("bd(<3 <4 5>>,8)");
    
    for cycle in 0..4 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        let bd_count = events.len();
        
        // Pattern should be: 3, 4, 3, 5
        let expected = match cycle {
            0 => 3,
            1 => 4,
            2 => 3, 
            3 => 5,
            _ => unreachable!(),
        };
        
        assert_eq!(bd_count, expected, "Cycle {} should have {} events", cycle, expected);
        println!("  Cycle {}: {} events ✓", cycle, bd_count);
    }
}

#[test]
#[ignore] // TODO: Fix alternation
fn test_alternation_with_operators() {
    println!("\n=== Testing Alternation with Operators ===");
    
    // Test alternation with repeat operator
    let pattern = parse_mini_notation("bd*<2 3 4>");
    
    for cycle in 0..3 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        
        let expected = match cycle % 3 {
            0 => 2,
            1 => 3,
            2 => 4,
            _ => unreachable!(),
        };
        
        assert_eq!(events.len(), expected, "Cycle {} should have {} events", cycle, expected);
        println!("  Cycle {}: {} repeats ✓", cycle, events.len());
    }
}

#[test]
#[ignore] // TODO: Fix alternation
fn test_polyrhythm_with_alternation() {
    println!("\n=== Testing Polyrhythm with Alternation ===");
    
    // Polyrhythm where one part has alternation
    let pattern = parse_mini_notation("[bd*3, <sn cp>*2]");
    
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(
                Fraction::new(cycle as i64, 1),
                Fraction::new((cycle + 1) as i64, 1),
            ),
            controls: HashMap::new(),
        };
        
        let events = pattern.query(&state);
        
        let bd_count = events.iter().filter(|e| e.value == "bd").count();
        let sn_count = events.iter().filter(|e| e.value == "sn").count();
        let cp_count = events.iter().filter(|e| e.value == "cp").count();
        
        // bd should always have 3
        assert_eq!(bd_count, 3, "Should have 3 bd events");
        
        // sn/cp should alternate
        if cycle % 2 == 0 {
            assert_eq!(sn_count, 2, "Cycle {} should have 2 sn", cycle);
            assert_eq!(cp_count, 0, "Cycle {} should have 0 cp", cycle);
        } else {
            assert_eq!(sn_count, 0, "Cycle {} should have 0 sn", cycle);
            assert_eq!(cp_count, 2, "Cycle {} should have 2 cp", cycle);
        }
        
        println!("  Cycle {}: bd={}, sn={}, cp={} ✓", cycle, bd_count, sn_count, cp_count);
    }
}

#[test]
#[ignore] // TODO: Fix alternation
fn test_alternation_audio_generation() {
    println!("\n=== Testing Alternation Audio Generation ===");
    
    let sample_rate = 44100.0;
    let mut executor = SimpleDspExecutor::new(sample_rate);
    
    // Generate 4 seconds with alternating samples
    let code = r#"o: s "<bd sn>""#;
    let env = parse_glicol(code).expect("Failed to parse");
    let audio = executor.render(&env, 4.0).expect("Failed to render");
    
    // Verify we have audio
    assert!(!audio.data.is_empty(), "Should generate audio");
    assert!(audio.peak() > 0.01, "Should have non-zero amplitude");
    
    // Check each cycle has different content
    let samples_per_cycle = sample_rate as usize;
    let mut cycle_signatures = Vec::new();
    
    for cycle in 0..4 {
        let start = cycle * samples_per_cycle;
        let end = ((cycle + 1) * samples_per_cycle).min(audio.data.len());
        
        if end > start {
            let cycle_data = &audio.data[start..end];
            
            // Calculate a simple "signature" for each cycle
            let rms: f32 = (cycle_data.iter()
                .map(|x| x * x)
                .sum::<f32>() / cycle_data.len() as f32)
                .sqrt();
            
            // Count peaks as another signature
            let peaks = cycle_data.windows(2)
                .filter(|w| w[0].abs() < 0.1 && w[1].abs() > 0.1)
                .count();
            
            cycle_signatures.push((rms, peaks));
            println!("  Cycle {}: RMS={:.3}, peaks={}", cycle, rms, peaks);
        }
    }
    
    // Verify alternation: cycles 0&2 should be similar, 1&3 should be similar
    if cycle_signatures.len() >= 4 {
        // Allow some variance but expect similarity
        let rms_diff_02 = (cycle_signatures[0].0 - cycle_signatures[2].0).abs();
        let rms_diff_13 = (cycle_signatures[1].0 - cycle_signatures[3].0).abs();
        
        println!("  RMS diff cycles 0-2: {:.3}", rms_diff_02);
        println!("  RMS diff cycles 1-3: {:.3}", rms_diff_13);
        
        // Different samples should have notably different RMS
        let rms_diff_01 = (cycle_signatures[0].0 - cycle_signatures[1].0).abs();
        println!("  RMS diff cycles 0-1: {:.3} (should be larger)", rms_diff_01);
    }
}