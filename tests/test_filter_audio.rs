//! Test filter audio generation

use phonon::glicol_parser_v2::parse_glicol_v2;
use phonon::simple_dsp_executor_v2::SimpleDspExecutorV2;

#[test]
fn test_simple_lowpass() {
    println!("\n=== Testing Simple Lowpass Filter ===");

    // Saw wave through lowpass - should reduce brightness
    let code = "o: saw 220 >> lpf 500 0.7 >> mul 0.5";
    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    let audio = executor.render(&env, 0.1).expect("Failed to render");

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.1, "Should have signal");

    // Check it's not NaN
    let has_nan = audio.iter().any(|s| s.is_nan());
    if has_nan {
        // Print first few samples for debugging
        println!("First 20 samples:");
        for i in 0..20.min(audio.len()) {
            println!("  [{}]: {}", i, audio[i]);
        }
    }
    assert!(!has_nan, "Should not have NaN");
}

#[test]
fn test_filter_without_input() {
    println!("\n=== Testing Filter Without Input ===");

    // Just a filter - should be silent but not NaN
    let code = "o: lpf 1000 0.7";
    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    let audio = executor.render(&env, 0.01).expect("Failed to render");

    // Should be silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val < 0.001, "Should be silent");

    // Check it's not NaN
    assert!(audio.iter().all(|s| !s.is_nan()), "Should not have NaN");
}

#[test]
fn test_filter_with_reference() {
    println!("\n=== Testing Filter with Reference ===");

    let code = r#"
        ~source: saw 110
        o: ~source >> lpf 1000 0.5
    "#;

    let env = parse_glicol_v2(code).expect("Failed to parse");
    let mut executor = SimpleDspExecutorV2::new(44100.0);

    let audio = executor.render(&env, 0.01).expect("Failed to render");

    // Check it's not silent
    let max_val = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Max amplitude: {}", max_val);
    assert!(max_val > 0.1, "Should have signal");

    // Check it's not NaN
    let has_nan = audio.iter().any(|s| s.is_nan());
    if has_nan {
        println!("Found NaN in filtered reference output!");
    }
    assert!(!has_nan, "Should not have NaN");
}
