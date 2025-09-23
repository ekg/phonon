//! Simple test for audio generation

use phonon::glicol_parser_v2::parse_glicol_v2;
use phonon::simple_dsp_executor_v2::SimpleDspExecutorV2;

#[test]
fn test_simple_sine_generation() {
    println!("\n=== Testing Simple Sine Wave Generation ===");

    let code = "o: sin 440 >> mul 0.5";
    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    // Generate 0.1 seconds
    let audio = executor.render(&env, 0.1).expect("Failed to render");

    println!("Generated {} samples", audio.len());

    // Check we have audio
    assert_eq!(audio.len(), 4410);

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.4, "Should have signal");

    // Check it's not NaN
    assert!(audio.iter().all(|s| !s.is_nan()), "Should not have NaN");

    // Count zero crossings to estimate frequency
    let mut crossings = 0;
    for i in 1..audio.len() {
        if audio[i-1] <= 0.0 && audio[i] > 0.0 {
            crossings += 1;
        }
    }
    println!("Zero crossings: {}", crossings);
    // Should be around 44 for 440Hz in 0.1s
    assert!(crossings > 35 && crossings < 50, "Frequency should be roughly 440Hz");
}

#[test]
fn test_saw_wave_generation() {
    println!("\n=== Testing Saw Wave Generation ===");

    let code = "o: saw 220";
    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    let audio = executor.render(&env, 0.01).expect("Failed to render");

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.5, "Should have signal");

    // Check it's not NaN
    assert!(audio.iter().all(|s| !s.is_nan()), "Should not have NaN");
}

#[test]
fn test_reference_chain() {
    println!("\n=== Testing Reference Chain ===");

    let code = r#"
        ~osc: sin 330
        o: ~osc >> mul 0.3
    "#;

    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    let audio = executor.render(&env, 0.01).expect("Failed to render");

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.2 && max_val < 0.4, "Should be scaled by 0.3");

    // Check it's not NaN
    assert!(audio.iter().all(|s| !s.is_nan()), "Should not have NaN");
}

#[test]
fn test_pattern_frequency() {
    println!("\n=== Testing Pattern Frequency ===");

    let code = r#"o: sin "220 440 330""#;

    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);
    executor.set_cps(1.0); // 1 cycle per second

    let audio = executor.render(&env, 1.5).expect("Failed to render");

    // In 1.5 seconds at 1 cps, we should hear:
    // 0.0-0.33s: 220Hz
    // 0.33-0.67s: 440Hz
    // 0.67-1.0s: 330Hz
    // 1.0-1.33s: 220Hz (repeat)
    // 1.33-1.5s: 440Hz (partial)

    println!("Generated {} samples for pattern test", audio.len());

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.5, "Should have signal");

    // Check it's not NaN
    assert!(audio.iter().all(|s| !s.is_nan()), "Should not have NaN");
}