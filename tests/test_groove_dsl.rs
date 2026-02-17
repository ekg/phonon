/// Tests for groove transform DSL integration
/// Tests apply_groove with preset templates: mpc, hiphop, reggae, jazz, drunken
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::groove::presets;
use phonon::groove::GrooveTemplate;
use phonon::mini_notation_v3::parse_mini_notation;
use phonon::pattern::{Fraction, Pattern, State, TimeSpan};
use std::collections::HashMap;
use std::sync::Arc;

mod audio_test_utils;
mod pattern_verification_utils;
use audio_test_utils::calculate_rms;
use pattern_verification_utils::detect_audio_events;

fn render_dsl(code: &str, cycles: usize) -> Vec<f32> {
    let (_, statements) = parse_program(code).expect("Parse failed");
    let sample_rate = 44100.0;
    let mut graph = compile_program(statements, sample_rate, None).expect("Compile failed");
    let samples_per_cycle = (sample_rate as f64 / 0.5) as usize;
    let total_samples = samples_per_cycle * cycles;
    graph.render(total_samples)
}

// ============================================================================
// LEVEL 1: Pattern Query Verification
// ============================================================================

#[test]
fn test_groove_level1_preserves_event_count() {
    // Groove should shift event timings, NOT add or remove events
    let pattern = parse_mini_notation("bd sn hh cp");
    let groove = Arc::new(presets::mpc_swing(0.5));

    let grooved = pattern.clone().apply_groove(groove, Pattern::pure(1.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let groove_haps = grooved.query(&state);

    assert_eq!(base_haps.len(), groove_haps.len(),
        "Groove should preserve event count: base={}, grooved={}",
        base_haps.len(), groove_haps.len());

    println!("✅ groove Level 1: Event count preserved ({} events)", base_haps.len());
}

#[test]
fn test_groove_level1_shifts_timing() {
    // MPC swing delays odd 16th-note positions, so we need 8 events (8th notes)
    // to land on both even and odd grid positions in the 16-grid.
    // 8 events at positions 0/8, 1/8, ..., 7/8 map to grid positions 0,2,4,6,8,10,12,14
    // That's still all even! Use 16 events instead to hit odd positions.
    let pattern = parse_mini_notation("a b c d e f g h i j k l m n o p");
    let groove = Arc::new(presets::mpc_swing(0.5));

    let grooved = pattern.clone().apply_groove(groove, Pattern::pure(1.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let groove_haps = grooved.query(&state);

    assert_eq!(base_haps.len(), 16, "Should have 16 events");
    assert_eq!(groove_haps.len(), 16, "Grooved should have 16 events");

    // At least some events should have shifted timing (odd 16th-note positions)
    let mut any_shifted = false;
    for (base, grooved) in base_haps.iter().zip(groove_haps.iter()) {
        let base_time = base.part.begin.to_float();
        let groove_time = grooved.part.begin.to_float();
        if (base_time - groove_time).abs() > 0.001 {
            any_shifted = true;
        }
    }

    assert!(any_shifted, "MPC swing should shift at least some event timings");
    println!("✅ groove Level 1: Timing shifts detected");
}

#[test]
fn test_groove_level1_amount_zero_is_identity() {
    // groove with amount=0.0 should not change timing at all
    let pattern = parse_mini_notation("bd sn hh cp");
    let groove = Arc::new(presets::mpc_swing(0.5));

    let grooved = pattern.clone().apply_groove(groove, Pattern::pure(0.0));

    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let base_haps = pattern.query(&state);
    let groove_haps = grooved.query(&state);

    for (base, grooved) in base_haps.iter().zip(groove_haps.iter()) {
        let base_time = base.part.begin.to_float();
        let groove_time = grooved.part.begin.to_float();
        assert!((base_time - groove_time).abs() < 0.001,
            "Amount 0.0 should not shift events: base={}, grooved={}", base_time, groove_time);
    }

    println!("✅ groove Level 1: Amount 0.0 is identity");
}

#[test]
fn test_groove_level1_all_presets() {
    // All presets should be usable with patterns
    let pattern = parse_mini_notation("bd sn hh cp");
    let state = State {
        span: TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 1)),
        controls: HashMap::new(),
    };

    let presets_to_test: Vec<(&str, Arc<GrooveTemplate>)> = vec![
        ("mpc_swing", Arc::new(presets::mpc_swing(0.5))),
        ("lazy_hiphop", Arc::new(presets::lazy_hiphop())),
        ("reggae_one_drop", Arc::new(presets::reggae_one_drop())),
        ("jazz_swing", Arc::new(presets::jazz_swing(0.5))),
        ("drunken", Arc::new(presets::drunken(0.5))),
    ];

    for (name, template) in presets_to_test {
        let grooved = pattern.clone().apply_groove(template, Pattern::pure(1.0));
        let haps = grooved.query(&state);
        assert_eq!(haps.len(), 4,
            "Preset '{}' should produce 4 events, got {}", name, haps.len());
    }

    println!("✅ groove Level 1: All 5 presets work");
}

#[test]
fn test_groove_level1_multi_cycle_consistency() {
    // Groove should produce consistent results across multiple cycles
    let pattern = parse_mini_notation("bd sn hh cp");
    let groove = Arc::new(presets::jazz_swing(0.5));
    let grooved = pattern.apply_groove(groove, Pattern::pure(1.0));

    let mut total_events = 0;
    for cycle in 0..8 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };
        let haps = grooved.query(&state);
        assert_eq!(haps.len(), 4,
            "Cycle {} should have 4 events, got {}", cycle, haps.len());
        total_events += haps.len();
    }

    assert_eq!(total_events, 32, "8 cycles × 4 events = 32 total");
    println!("✅ groove Level 1: Multi-cycle consistency (32 events over 8 cycles)");
}

// ============================================================================
// LEVEL 2: DSL Parsing and Compilation
// ============================================================================

#[test]
fn test_groove_dsl_parse_mpc() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "mpc"
"#;
    let (_, statements) = parse_program(code).expect("Parse failed for groove mpc");
    let graph = compile_program(statements, 44100.0, None);
    assert!(graph.is_ok(), "groove 'mpc' should compile: {:?}", graph.err());
    println!("✅ groove DSL: 'mpc' preset parses and compiles");
}

#[test]
fn test_groove_dsl_parse_all_presets() {
    let presets = vec!["mpc", "hiphop", "reggae", "jazz", "drunken"];

    for preset in &presets {
        let code = format!(
            "cps: 0.5\nout $ s \"bd sn hh cp\" $ groove \"{}\"",
            preset
        );
        let (_, statements) = parse_program(&code).expect(&format!("Parse failed for groove {}", preset));
        let graph = compile_program(statements, 44100.0, None);
        assert!(graph.is_ok(), "groove '{}' should compile: {:?}", preset, graph.err());
    }

    println!("✅ groove DSL: All presets parse and compile");
}

#[test]
fn test_groove_dsl_with_amount() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "mpc" 0.7
"#;
    let (_, statements) = parse_program(code).expect("Parse failed");
    let graph = compile_program(statements, 44100.0, None);
    assert!(graph.is_ok(), "groove with amount should compile: {:?}", graph.err());
    println!("✅ groove DSL: Preset with amount parses and compiles");
}

#[test]
fn test_groove_dsl_with_pattern_amount() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "jazz" "0.5 1.0"
"#;
    let (_, statements) = parse_program(code).expect("Parse failed");
    let graph = compile_program(statements, 44100.0, None);
    assert!(graph.is_ok(), "groove with pattern amount should compile: {:?}", graph.err());
    println!("✅ groove DSL: Pattern amount parses and compiles");
}

#[test]
fn test_groove_dsl_invalid_preset() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "nonexistent"
"#;
    let (_, statements) = parse_program(code).expect("Parse failed");
    let graph = compile_program(statements, 44100.0, None);
    assert!(graph.is_err(), "Unknown groove preset should fail compilation");
    let err = graph.err().unwrap();
    assert!(err.contains("Unknown groove preset"), "Error should mention unknown preset: {}", err);
    println!("✅ groove DSL: Unknown preset gives clear error");
}

// ============================================================================
// LEVEL 2: Onset Detection (Audio events at right times)
// ============================================================================

#[test]
fn test_groove_level2_renders_audio() {
    // Basic check: groove transform should render non-silent audio
    let code_base = r#"
cps: 0.5
out $ s "bd sn hh cp"
"#;
    let code_grooved = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "mpc"
"#;

    let audio_base = render_dsl(code_base, 4);
    let audio_grooved = render_dsl(code_grooved, 4);

    let rms_base = calculate_rms(&audio_base);
    let rms_grooved = calculate_rms(&audio_grooved);

    assert!(rms_base > 0.01, "Base pattern should have audio: RMS={}", rms_base);
    assert!(rms_grooved > 0.01, "Grooved pattern should have audio: RMS={}", rms_grooved);

    println!(
        "✅ groove Level 2: Both patterns render audio (base RMS={:.4}, grooved RMS={:.4})",
        rms_base, rms_grooved
    );
}

#[test]
fn test_groove_level2_onset_count_preserved() {
    // Groove should preserve the number of audio events
    let code_base = r#"
cps: 0.5
out $ s "bd sn hh cp"
"#;
    let code_grooved = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "mpc"
"#;

    let audio_base = render_dsl(code_base, 2);
    let audio_grooved = render_dsl(code_grooved, 2);

    let onsets_base = detect_audio_events(&audio_base, 44100.0, 0.1);
    let onsets_grooved = detect_audio_events(&audio_grooved, 44100.0, 0.1);

    // Event counts should be similar (groove shifts timing, doesn't add/remove events)
    let diff = (onsets_base.len() as i32 - onsets_grooved.len() as i32).abs();
    assert!(diff <= 2,
        "Groove should preserve event count: base={}, grooved={}, diff={}",
        onsets_base.len(), onsets_grooved.len(), diff);

    println!(
        "✅ groove Level 2: Onset count preserved (base={}, grooved={})",
        onsets_base.len(), onsets_grooved.len()
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics
// ============================================================================

#[test]
fn test_groove_level3_rms_similar() {
    // Groove should not significantly change the overall energy
    let code_base = r#"
cps: 0.5
out $ s "bd sn hh cp"
"#;
    let code_grooved = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "jazz"
"#;

    let audio_base = render_dsl(code_base, 4);
    let audio_grooved = render_dsl(code_grooved, 4);

    let rms_base = calculate_rms(&audio_base);
    let rms_grooved = calculate_rms(&audio_grooved);

    // RMS should be within 50% (groove changes timing, not volume)
    let ratio = rms_grooved / rms_base;
    assert!(ratio > 0.5 && ratio < 2.0,
        "RMS should be similar: base={:.4}, grooved={:.4}, ratio={:.2}",
        rms_base, rms_grooved, ratio);

    println!(
        "✅ groove Level 3: RMS similar (base={:.4}, grooved={:.4}, ratio={:.2})",
        rms_base, rms_grooved, ratio
    );
}

#[test]
fn test_groove_level3_no_clipping() {
    // Groove should not introduce clipping
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ groove "drunken" 1.0
"#;

    let audio = render_dsl(code, 4);
    let max_amplitude = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

    assert!(max_amplitude <= 1.0,
        "Groove should not cause clipping: max amplitude = {}", max_amplitude);

    println!("✅ groove Level 3: No clipping (max amplitude = {:.4})", max_amplitude);
}

// ============================================================================
// Composability with other transforms
// ============================================================================

#[test]
fn test_groove_dsl_composable_with_fast() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ fast 2 $ groove "mpc"
"#;
    let audio = render_dsl(code, 2);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "groove + fast should render audio: RMS={}", rms);
    println!("✅ groove composable with fast (RMS={:.4})", rms);
}

#[test]
fn test_groove_dsl_composable_with_every() {
    let code = r#"
cps: 0.5
out $ s "bd sn hh cp" $ every 2 (groove "jazz" 0.8)
"#;
    let audio = render_dsl(code, 4);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "every + groove should render audio: RMS={}", rms);
    println!("✅ groove composable with every (RMS={:.4})", rms);
}

#[test]
fn test_groove_preset_aliases() {
    // Test that all aliases work
    let aliases = vec![
        ("mpc_swing", "mpc"),
        ("lazy_hiphop", "hiphop"),
        ("one_drop", "one_drop"),
        ("reggae_one_drop", "reggae_one_drop"),
        ("jazz_swing", "jazz_swing"),
        ("drunk", "drunk"),
    ];

    for (alias, _desc) in &aliases {
        let code = format!(
            "cps: 0.5\nout $ s \"bd sn hh cp\" $ groove \"{}\"",
            alias
        );
        let result = parse_program(&code);
        assert!(result.is_ok(), "Alias '{}' should parse", alias);
        let (_, statements) = result.unwrap();
        let graph = compile_program(statements, 44100.0, None);
        assert!(graph.is_ok(), "Alias '{}' should compile: {:?}", alias, graph.err());
    }

    println!("✅ groove aliases all work");
}
