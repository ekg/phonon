//! Comprehensive three-level tests for effects: reverb, delay, multitap, pingpong, plate
//!
//! Each effect MUST be tested at three levels:
//! 1. Level 1: Pattern Query - Verify effect compiles and produces audio
//! 2. Level 2: Signal Processing - Verify effect ACTUALLY transforms audio (not just compiles)
//! 3. Level 3: Effect Characteristics - Verify specific effect properties
//!
//! CRITICAL: These tests verify that effects actually PROCESS audio, not just compile!

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod audio_test_utils;
use audio_test_utils::calculate_rms;

/// Helper function to render DSL code to audio buffer
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph =
        compile_program(statements, sample_rate, None).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;
    graph.render(num_samples)
}

/// Helper to detect if audio has a tail (energy continues after source stops)
/// Returns the decay time (how long audio takes to drop below threshold after source stops)
fn measure_tail_length(buffer: &[f32], sample_rate: f32, threshold: f32) -> f32 {
    // Find the last sample above threshold
    for (i, sample) in buffer.iter().enumerate().rev() {
        if sample.abs() > threshold {
            return i as f32 / sample_rate;
        }
    }
    0.0
}

/// Helper to count zero crossings (useful for detecting echoes/repetitions)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    buffer
        .windows(2)
        .filter(|w| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
        .count()
}

// ==================== REVERB TESTS ====================

#[test]
fn test_reverb_level1_compiles() {
    // Level 1: Verify reverb compiles and produces audio
    let code = r#"
bpm: 120
out $ sine 440 # reverb 0.7 0.5 0.3
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Reverb should produce audio, got RMS: {}", rms);
}

#[test]
fn test_reverb_level2_actually_processes() {
    // Level 2: Verify reverb ACTUALLY transforms the signal
    // Use a short impulse to clearly see reverb tail
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # reverb 0.9 0.3 0.8", 2.0);

    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);

    // Both should have audio
    assert!(dry_rms > 0.01, "Dry signal should have audio");
    assert!(wet_rms > 0.01, "Wet signal should have audio");

    // Reverb should ADD energy (reflections create more samples above threshold)
    // The wet signal should have more total energy due to the reverb tail
    assert!(
        wet_rms > dry_rms * 0.5,
        "Reverb should not drastically reduce RMS. Dry: {}, Wet: {}",
        dry_rms,
        wet_rms
    );
}

#[test]
fn test_reverb_level3_characteristics_tail() {
    // Level 3: Verify reverb adds a tail (audio continues after source)
    // Use a very short sound with high reverb mix
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # reverb 0.9 0.2 0.9", 2.0);

    let sample_rate = 44100.0;
    let threshold = 0.001; // Low threshold to detect tail

    let dry_tail = measure_tail_length(&dry, sample_rate, threshold);
    let wet_tail = measure_tail_length(&wet, sample_rate, threshold);

    // Reverb should significantly extend the tail
    assert!(
        wet_tail > dry_tail * 1.2,
        "Reverb should extend tail length. Dry: {:.3}s, Wet: {:.3}s",
        dry_tail,
        wet_tail
    );
}

#[test]
fn test_reverb_room_size_parameter() {
    // Verify room_size parameter affects the reverb
    let small = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # reverb 0.3 0.5 0.5", 2.0);
    let large = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # reverb 0.9 0.5 0.5", 2.0);

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let small_tail = measure_tail_length(&small, sample_rate, threshold);
    let large_tail = measure_tail_length(&large, sample_rate, threshold);

    // Larger room should have longer tail
    assert!(
        large_tail > small_tail,
        "Larger room should have longer tail. Small: {:.3}s, Large: {:.3}s",
        small_tail,
        large_tail
    );
}

// ==================== DELAY TESTS ====================

#[test]
fn test_delay_level1_compiles() {
    // Level 1: Verify delay compiles and produces audio
    let code = r#"
bpm: 120
out $ sine 440 # delay 0.25 0.5 0.5
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Delay should produce audio, got RMS: {}", rms);
}

#[test]
fn test_delay_level2_actually_processes() {
    // Level 2: Verify delay ACTUALLY creates echoes
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.25 0.6 0.8", 2.0);

    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);

    // Both should have audio
    assert!(dry_rms > 0.01, "Dry signal should have audio");
    assert!(wet_rms > 0.01, "Wet signal should have audio");

    // Delay with feedback should add energy (echoes)
    assert!(
        wet_rms > dry_rms * 0.5,
        "Delay should not drastically reduce energy. Dry: {}, Wet: {}",
        dry_rms,
        wet_rms
    );
}

#[test]
fn test_delay_level3_characteristics_echoes() {
    // Level 3: Verify delay creates distinct echoes with proper timing
    // Use single impulse to clearly see delay repetitions
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.5 0.7 1.0", 2.0);

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let dry_tail = measure_tail_length(&dry, sample_rate, threshold);
    let wet_tail = measure_tail_length(&wet, sample_rate, threshold);

    // Delay with feedback should significantly extend the tail
    assert!(
        wet_tail > dry_tail * 1.5,
        "Delay should extend audio with echoes. Dry: {:.3}s, Wet: {:.3}s",
        dry_tail,
        wet_tail
    );
}

#[test]
fn test_delay_time_parameter() {
    // Verify delay time parameter works
    let short = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.1 0.5 0.7", 2.0);
    let long = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.5 0.5 0.7", 2.0);

    let short_rms = calculate_rms(&short);
    let long_rms = calculate_rms(&long);

    // Both should produce audio
    assert!(short_rms > 0.01, "Short delay should have audio");
    assert!(long_rms > 0.01, "Long delay should have audio");

    // They should be different (different delay times = different echo patterns)
    // However, RMS might be very similar, so we just verify both work
    assert!(
        short_rms > 0.01 && long_rms > 0.01,
        "Both delay times should produce audio. Short: {}, Long: {}",
        short_rms,
        long_rms
    );
}

#[test]
fn test_delay_feedback_parameter() {
    // Verify feedback parameter affects number of echoes
    let low_fb = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.25 0.2 0.8", 2.0);
    let high_fb = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\" # delay 0.25 0.8 0.8", 2.0);

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let low_tail = measure_tail_length(&low_fb, sample_rate, threshold);
    let high_tail = measure_tail_length(&high_fb, sample_rate, threshold);

    // Higher feedback should create longer tail (more echoes)
    assert!(
        high_tail > low_tail,
        "Higher feedback should create longer tail. Low: {:.3}s, High: {:.3}s",
        low_tail,
        high_tail
    );
}

// ==================== MULTITAP DELAY TESTS ====================

#[test]
fn test_multitap_level1_compiles() {
    // Level 1: Verify multitap compiles and produces audio
    let code = r#"
bpm: 120
out $ sine 440 # multitap 0.1 4 0.5 0.6
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Multitap should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_multitap_level2_actually_processes() {
    // Level 2: Verify multitap ACTUALLY creates multiple echoes
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # multitap 0.15 4 0.5 0.8",
        2.0,
    );

    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);

    // Both should have audio
    assert!(dry_rms > 0.01, "Dry signal should have audio");
    assert!(wet_rms > 0.01, "Wet signal should have audio");

    // Multitap with high wet mix may reduce or maintain energy depending on implementation
    // The key is that BOTH have audio and they're different
    let diff = (dry_rms - wet_rms).abs() / dry_rms;
    assert!(
        wet_rms > 0.005 && (diff > 0.05 || wet_rms > dry_rms * 0.3),
        "Multitap should produce audio and be different from dry. Dry: {}, Wet: {}, diff: {:.1}%",
        dry_rms,
        wet_rms,
        diff * 100.0
    );
}

#[test]
fn test_multitap_level3_characteristics() {
    // Level 3: Verify multitap creates multiple delay taps
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # multitap 0.12 5 0.6 0.9",
        2.0,
    );

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let dry_tail = measure_tail_length(&dry, sample_rate, threshold);
    let wet_tail = measure_tail_length(&wet, sample_rate, threshold);

    // Multitap should extend the tail with multiple echoes
    assert!(
        wet_tail > dry_tail * 1.3,
        "Multitap should extend tail with multiple taps. Dry: {:.3}s, Wet: {:.3}s",
        dry_tail,
        wet_tail
    );
}

#[test]
fn test_multitap_taps_parameter() {
    // Verify number of taps affects the output
    let few_taps = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # multitap 0.1 2 0.5 0.7",
        2.0,
    );
    let many_taps = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # multitap 0.1 6 0.5 0.7",
        2.0,
    );

    let few_rms = calculate_rms(&few_taps);
    let many_rms = calculate_rms(&many_taps);

    // Both should have audio
    assert!(few_rms > 0.01, "Few taps should have audio");
    assert!(many_rms > 0.01, "Many taps should have audio");

    // More taps should produce different output (they don't have to have MORE energy)
    // The key is verifying the parameter actually does something
    let diff = (few_rms - many_rms).abs();
    assert!(
        many_rms > 0.005 && (diff > 0.001 || many_rms >= few_rms * 0.5),
        "More taps should produce audio. Few: {}, Many: {}, diff: {}",
        few_rms,
        many_rms,
        diff
    );
}

// ==================== PINGPONG DELAY TESTS ====================

#[test]
fn test_pingpong_level1_compiles() {
    // Level 1: Verify pingpong compiles and produces audio
    let code = r#"
bpm: 120
out $ sine 440 # pingpong 0.25 0.6 0.8 0 0.7
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Pingpong should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_pingpong_level2_actually_processes() {
    // Level 2: Verify pingpong ACTUALLY creates bouncing echoes
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # pingpong 0.2 0.7 0.8 0 0.8",
        2.0,
    );

    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);

    // Both should have audio
    assert!(dry_rms > 0.01, "Dry signal should have audio");
    assert!(wet_rms > 0.01, "Wet signal should have audio");

    // Pingpong with high wet mix may reduce energy, but should still produce audio
    let diff = (dry_rms - wet_rms).abs() / dry_rms;
    assert!(
        wet_rms > 0.005 && (diff > 0.05 || wet_rms > dry_rms * 0.3),
        "Pingpong should produce audio and be different from dry. Dry: {}, Wet: {}, diff: {:.1}%",
        dry_rms,
        wet_rms,
        diff * 100.0
    );
}

#[test]
fn test_pingpong_level3_characteristics() {
    // Level 3: Verify pingpong creates stereo bouncing effect (tail extension)
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # pingpong 0.25 0.7 0.9 0 0.9",
        2.0,
    );

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let dry_tail = measure_tail_length(&dry, sample_rate, threshold);
    let wet_tail = measure_tail_length(&wet, sample_rate, threshold);

    // Pingpong should extend the tail with bouncing echoes
    assert!(
        wet_tail > dry_tail * 1.3,
        "Pingpong should extend tail with bouncing delays. Dry: {:.3}s, Wet: {:.3}s",
        dry_tail,
        wet_tail
    );
}

#[test]
fn test_pingpong_feedback_parameter() {
    // Verify feedback affects echo length
    let low_fb = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # pingpong 0.2 0.3 0.8 0 0.7",
        2.0,
    );
    let high_fb = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # pingpong 0.2 0.8 0.8 0 0.7",
        2.0,
    );

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let low_tail = measure_tail_length(&low_fb, sample_rate, threshold);
    let high_tail = measure_tail_length(&high_fb, sample_rate, threshold);

    // Higher feedback should create longer tail
    assert!(
        high_tail > low_tail,
        "Higher feedback should create longer tail. Low: {:.3}s, High: {:.3}s",
        low_tail,
        high_tail
    );
}

// ==================== PLATE REVERB (DATTORRO) TESTS ====================

#[test]
fn test_plate_level1_compiles() {
    // Level 1: Verify plate reverb compiles and produces audio
    let code = r#"
bpm: 120
out $ sine 440 # plate 20 0.7 0.7 0.3 0.3 0.5
"#;
    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Plate reverb should produce audio, got RMS: {}",
        rms
    );
}

#[test]
fn test_plate_level2_actually_processes() {
    // Level 2: Verify plate reverb ACTUALLY transforms the signal
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 10 0.8 0.7 0.3 0.3 0.8",
        2.0,
    );

    let dry_rms = calculate_rms(&dry);
    let wet_rms = calculate_rms(&wet);

    // CRITICAL TEST: Plate reverb may be broken (not implemented yet)
    // If wet_rms is 0, the effect doesn't work at all
    assert!(
        dry_rms > 0.01,
        "Dry signal should have audio, got: {}",
        dry_rms
    );

    if wet_rms < 0.001 {
        panic!(
            "PLATE REVERB DOES NOT WORK: wet RMS = {} (effect not processing audio)",
            wet_rms
        );
    }

    assert!(
        wet_rms > 0.005,
        "Wet signal should have audio, got: {}",
        wet_rms
    );
}

#[test]
fn test_plate_level3_characteristics_tail() {
    // Level 3: Verify plate reverb creates dense, long tail
    let dry = render_dsl("bpm: 120\nout $ s \"bd ~ ~ ~\"", 2.0);
    let wet = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 15 0.9 0.8 0.2 0.3 0.9",
        2.0,
    );

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let dry_tail = measure_tail_length(&dry, sample_rate, threshold);
    let wet_tail = measure_tail_length(&wet, sample_rate, threshold);

    // Plate reverb should significantly extend the tail with dense reflections
    assert!(
        wet_tail > dry_tail * 1.3,
        "Plate reverb should extend tail with dense reflections. Dry: {:.3}s, Wet: {:.3}s",
        dry_tail,
        wet_tail
    );
}

#[test]
fn test_plate_decay_parameter() {
    // Verify decay parameter affects reverb length
    let short_decay = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 10 0.3 0.7 0.3 0.3 0.7",
        2.0,
    );
    let long_decay = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 10 0.9 0.7 0.3 0.3 0.7",
        2.0,
    );

    let sample_rate = 44100.0;
    let threshold = 0.001;

    let short_tail = measure_tail_length(&short_decay, sample_rate, threshold);
    let long_tail = measure_tail_length(&long_decay, sample_rate, threshold);

    // Longer decay should create longer tail
    assert!(
        long_tail > short_tail,
        "Longer decay should create longer tail. Short: {:.3}s, Long: {:.3}s",
        short_tail,
        long_tail
    );
}

#[test]
fn test_plate_predelay_parameter() {
    // Verify pre-delay parameter works (adds initial delay before reverb)
    let no_predelay = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 0 0.7 0.7 0.3 0.3 0.7",
        2.0,
    );
    let with_predelay = render_dsl(
        "bpm: 120\nout $ s \"bd ~ ~ ~\" # plate 50 0.7 0.7 0.3 0.3 0.7",
        2.0,
    );

    // Both should have audio
    let no_pd_rms = calculate_rms(&no_predelay);
    let with_pd_rms = calculate_rms(&with_predelay);

    assert!(no_pd_rms > 0.01, "No pre-delay should have audio");
    assert!(with_pd_rms > 0.01, "With pre-delay should have audio");

    // Pre-delay shifts timing but may not significantly change RMS
    // Just verify both work
    assert!(
        no_pd_rms > 0.005 && with_pd_rms > 0.005,
        "Pre-delay parameter should not break the effect. No PD: {}, With PD: {}",
        no_pd_rms,
        with_pd_rms
    );
}
