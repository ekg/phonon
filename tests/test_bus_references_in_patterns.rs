/// Tests for bus references in sample patterns
/// Verifies that patterns can trigger buses (e.g., s "~mybuss(3,8)")
///
/// This is a CRITICAL feature that allows patterns to trigger any audio source,
/// not just samples from disk. Enables powerful compositional patterns.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

// Helper functions for testing
fn render_dsl(code: &str, duration: f32) -> Vec<f32> {
    let sample_rate = 44100.0;
    let (_, statements) = parse_program(code).expect("Failed to parse DSL code");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile DSL code");
    let num_samples = (duration * sample_rate) as usize;

    // CRITICAL: Render in small chunks (128 samples) like continuous synthesis tests
    // This is necessary for synthesis voices to work properly!
    let buffer_size = 128;
    let num_buffers = num_samples / buffer_size;
    let mut full_audio = Vec::with_capacity(num_samples);
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }
    full_audio
}

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

#[test]
fn test_bus_reference_simple() {
    // Test that a bus reference in a pattern triggers the bus correctly
    let code = r#"
bpm: 120
~sine: sine 440
~pattern: s "~sine*4"
out: ~pattern
"#;

    let audio = render_dsl(code, 2.0);

    // Should have audio output
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Bus reference should produce audio, got RMS={}", rms);

    // Verify we have distinct events (not solid tone)
    // With *4, we should have 4 events per cycle = 8 events total over 2 seconds at 120 BPM
    // Each event should be separated by silence
    // Check for amplitude variation (events trigger, then decay/release)
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    // Should NOT be 100% filled (that would indicate solid tone)
    // Should be < 50% filled (events with gaps between them)
    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.5, "Audio should have gaps between events, got fill_ratio={}", fill_ratio);
}

#[test]
fn test_bus_reference_euclidean() {
    // Test Euclidean rhythm with bus reference
    let code = r#"
bpm: 120
~saw: saw 220
~drums: s "~saw(3,8)"
out: ~drums
"#;

    let audio = render_dsl(code, 4.0);

    // Should produce audio
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Euclidean bus reference should produce audio, got RMS={}", rms);

    // With (3,8) we get 3 evenly-distributed events per cycle
    // Over 4 seconds at 120 BPM = 4 cycles, so 12 events total
    // Check that we don't have a solid tone (should have gaps)
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.4, "Euclidean pattern should have sparse events, got fill_ratio={}", fill_ratio);
}

#[test]
fn test_bus_reference_with_note_modifier() {
    // Test bus reference with note modifier
    // THIS IS THE USER'S EXACT ISSUE
    let code = r#"
bpm: 120
~s: sine 440
~c: s "~s*2" # note "c3 e3"
out: ~c
"#;

    let audio = render_dsl(code, 2.0);

    // Should produce audio with note modulation
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Bus reference with note modifier should produce audio, got RMS={}", rms);

    // Verify we don't have a solid tone
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.6, "Pattern with notes should have distinct events, got fill_ratio={}", fill_ratio);
}

#[test]
fn test_bus_reference_with_chord() {
    // Test bus reference with chord notation (user's exact use case)
    let code = r#"
bpm: 120
~s: sine 440
~c: s "~s*4" # note "c3'maj"
out: ~c
"#;

    let audio = render_dsl(code, 2.0);

    // Should produce audio with chord notes (C, E, G)
    let rms = calculate_rms(&audio);
    assert!(rms > 0.05, "Bus reference with chord should produce audio, got RMS={}", rms);

    // With chord triggering, each pattern event should trigger multiple notes
    // This creates denser harmonic content
    // But should still have gaps between chord events
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.8, "Chord pattern should have some dynamic range, got fill_ratio={}", fill_ratio);
}

#[test]
fn test_bus_reference_vs_regular_sample() {
    // Compare bus reference triggering to regular sample triggering
    // Both should produce similar rhythmic patterns

    let code_bus = r#"
bpm: 120
~sine: sine 440
out: s "~sine*4"
"#;

    let code_sample = r#"
bpm: 120
out: s "bd*4"
"#;

    let audio_bus = render_dsl(code_bus, 2.0);
    let audio_sample = render_dsl(code_sample, 2.0);

    // Both should produce non-zero audio
    assert!(calculate_rms(&audio_bus) > 0.01, "Bus pattern should produce audio");
    assert!(calculate_rms(&audio_sample) > 0.01, "Sample pattern should produce audio");

    // Both should have similar fill ratios (similar rhythmic density)
    let threshold = 0.1;

    let fill_bus = audio_bus.iter().filter(|&&s| s.abs() > threshold).count() as f32 / audio_bus.len() as f32;
    let fill_sample = audio_sample.iter().filter(|&&s| s.abs() > threshold).count() as f32 / audio_sample.len() as f32;

    // Both should be sparse (< 50% filled)
    assert!(fill_bus < 0.5, "Bus pattern should be sparse, got {}", fill_bus);
    assert!(fill_sample < 0.5, "Sample pattern should be sparse, got {}", fill_sample);
}

#[test]
fn test_bus_reference_with_gain() {
    // Test that gain modifier works with bus references
    let code = r#"
bpm: 120
~sine: sine 440
~loud: s "~sine*4" # gain 2.0
~quiet: s "~sine*4" # gain 0.2
out: ~loud
"#;

    let audio_loud = render_dsl(code, 1.0);

    let code_quiet = r#"
bpm: 120
~sine: sine 440
out: s "~sine*4" # gain 0.2
"#;

    let audio_quiet = render_dsl(code_quiet, 1.0);

    // Loud should be louder than quiet
    let rms_loud = calculate_rms(&audio_loud);
    let rms_quiet = calculate_rms(&audio_quiet);

    assert!(rms_loud > rms_quiet * 2.0,
        "Gain modifier should affect amplitude: loud={}, quiet={}", rms_loud, rms_quiet);
}

#[test]
fn test_bus_reference_multiple_buses() {
    // Test pattern that alternates between two different buses
    let code = r#"
bpm: 120
~low: sine 220
~high: sine 880
~pattern: s "~low ~high ~low ~high"
out: ~pattern
"#;

    let audio = render_dsl(code, 2.0);

    // Should produce audio alternating between two frequencies
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Multi-bus pattern should produce audio, got RMS={}", rms);

    // Should have 4 events per cycle
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.6, "Alternating pattern should have gaps, got fill_ratio={}", fill_ratio);
}

#[test]
fn test_bus_reference_nested() {
    // Test bus references in nested patterns
    let code = r#"
bpm: 120
~osc: sine 440
~inner: s "~osc*2"
~outer: s "~inner*2"
out: ~outer
"#;

    let audio = render_dsl(code, 1.0);

    // Should produce audio (testing nested bus triggering)
    let rms = calculate_rms(&audio);
    // This might not work as expected depending on implementation
    // At minimum it shouldn't crash and should produce some audio
    assert!(rms > 0.001, "Nested bus reference should produce some audio, got RMS={}", rms);
}

#[test]
fn test_bus_reference_not_found_warning() {
    // Test that missing bus produces warning but doesn't crash
    let code = r#"
bpm: 120
out: s "~nonexistent*4"
"#;

    // Should render without crashing
    let audio = render_dsl(code, 1.0);

    // Should be silent (bus not found)
    let rms = calculate_rms(&audio);
    assert!(rms < 0.001, "Missing bus should produce silence, got RMS={}", rms);

    // The code prints: "Warning: Bus 'nonexistent' not found for trigger"
    // But test output doesn't capture eprintln, so we can't verify the warning text
}

#[test]
fn test_user_exact_issue() {
    // THIS IS THE EXACT CODE THE USER REPORTED AS BROKEN
    let code = r#"
bpm: 120
~s: sine 440
~c: s "~s(<7 7 6 10>,11,2)" # note "c3'maj"
out: ~c
"#;

    let audio = render_dsl(code, 4.0);

    // Should produce audio with Euclidean pattern and chord notes
    let rms = calculate_rms(&audio);
    assert!(rms > 0.05, "User's exact code should produce audio, got RMS={}", rms);

    // Should NOT be a solid sine tone
    // Euclidean (<7 7 6 10>,11,2) creates sparse pattern
    // Should have clear gaps
    let mut samples_above_threshold = 0;
    let threshold = 0.1;
    for &sample in &audio {
        if sample.abs() > threshold {
            samples_above_threshold += 1;
        }
    }

    let fill_ratio = samples_above_threshold as f32 / audio.len() as f32;
    assert!(fill_ratio < 0.5,
        "Euclidean pattern with sparse events should NOT be solid tone, got fill_ratio={}", fill_ratio);

    // Verify we have distinct events (not continuous)
    // Count number of zero-crossing clusters (rough onset detection)
    let mut in_sound = false;
    let mut event_count = 0;
    let silence_threshold = 0.05;

    for &sample in &audio {
        if sample.abs() > silence_threshold {
            if !in_sound {
                event_count += 1;
                in_sound = true;
            }
        } else {
            in_sound = false;
        }
    }

    assert!(event_count > 5,
        "Should have multiple distinct events (got {}), not solid tone", event_count);
    assert!(event_count < 50,
        "Should have reasonable event count (got {}), not thousands", event_count);
}
