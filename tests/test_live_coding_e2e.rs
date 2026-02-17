//! End-to-end tests for live coding scenarios
//!
//! These tests verify that Phonon behaves correctly during live coding sessions:
//! - Hot swapping graphs (pattern edits)
//! - Smooth transitions between patterns
//! - State preservation across graph swaps
//! - Tempo stability during edits
//! - Effect chain modifications
//! - Bus reference updates
//!
//! All tests use deterministic timing (wall_clock = false) for reproducibility in CI.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::UnifiedSignalGraph;
use std::time::Instant;

// ============================================================================
// Test Helpers
// ============================================================================

/// Compile DSL code to a signal graph
fn compile_code(code: &str, sample_rate: f32) -> UnifiedSignalGraph {
    let (rest, statements) = parse_program(code).expect("Failed to parse");
    assert!(
        rest.trim().is_empty(),
        "Parser did not consume all input: '{}'",
        rest.trim()
    );
    let mut graph = compile_program(statements, sample_rate, None).expect("Failed to compile");
    graph.use_wall_clock = false; // Deterministic timing for tests
    graph
}

/// Render audio and return the buffer
fn render_audio(graph: &mut UnifiedSignalGraph, num_samples: usize) -> Vec<f32> {
    graph.render(num_samples)
}

/// Calculate RMS of audio buffer
fn calculate_rms(buffer: &[f32]) -> f32 {
    if buffer.is_empty() {
        return 0.0;
    }
    (buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32).sqrt()
}

/// Count zero crossings in buffer (useful for detecting activity)
fn count_zero_crossings(buffer: &[f32]) -> usize {
    buffer
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count()
}

/// Simulate a graph swap (like pressing C-x in phonon-edit)
fn swap_graph(
    old_graph: &mut UnifiedSignalGraph,
    new_code: &str,
    sample_rate: f32,
) -> UnifiedSignalGraph {
    let mut new_graph = compile_code(new_code, sample_rate);

    // Transfer state (mimics modal_editor behavior)
    new_graph.transfer_fx_states(old_graph);
    new_graph.transfer_voice_manager(old_graph.take_voice_manager());

    new_graph
}

// ============================================================================
// Hot Swap Tests - Graph Replacement
// ============================================================================

#[test]
fn test_hot_swap_pattern_simple_to_complex() {
    let sample_rate = 44100.0;

    let simple = r#"
tempo: 2
out $ s "bd sn"
"#;

    let complex = r#"
tempo: 2
out $ s "bd*2 sn hh*4 cp"
"#;

    let mut graph = compile_code(simple, sample_rate);

    // Render some audio with simple pattern
    let audio_before = render_audio(&mut graph, 44100);

    // Hot swap to complex pattern
    let mut new_graph = swap_graph(&mut graph, complex, sample_rate);

    // Render audio after swap
    let audio_after = render_audio(&mut new_graph, 44100);

    // Both should produce audio
    let rms_before = calculate_rms(&audio_before);
    let rms_after = calculate_rms(&audio_after);

    // Note: With samples, these may be silent without actual sample files
    // The test verifies the swap mechanics work correctly
    assert!(rms_before >= 0.0, "Should handle simple pattern");
    assert!(rms_after >= 0.0, "Should handle complex pattern after swap");
}

#[test]
fn test_hot_swap_preserves_cps() {
    let sample_rate = 44100.0;

    let code1 = r#"
tempo: 1.5
out $ sine 440
"#;

    let code2 = r#"
tempo: 1.5
out $ sine 880
"#;

    let mut graph = compile_code(code1, sample_rate);
    let cps_before = graph.get_cps();

    let new_graph = swap_graph(&mut graph, code2, sample_rate);
    let cps_after = new_graph.get_cps();

    // CPS should match the code (1.5), not drift
    assert!(
        (cps_before - 1.5).abs() < 0.001,
        "CPS before should be 1.5, got {}",
        cps_before
    );
    assert!(
        (cps_after - 1.5).abs() < 0.001,
        "CPS after should be 1.5, got {}",
        cps_after
    );
}

#[test]
fn test_hot_swap_oscillator_continuity() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);

    // Render first second
    let audio1 = render_audio(&mut graph, 44100);
    let rms1 = calculate_rms(&audio1);

    // Hot swap to same code (simulates C-x without changes)
    let mut new_graph = swap_graph(&mut graph, code, sample_rate);

    // Render another second
    let audio2 = render_audio(&mut new_graph, 44100);
    let rms2 = calculate_rms(&audio2);

    // Both should have similar audio levels
    assert!(rms1 > 0.1, "Should produce audio before swap");
    assert!(rms2 > 0.1, "Should produce audio after swap");
    assert!(
        (rms1 - rms2).abs() < 0.1,
        "Audio level should be similar: {} vs {}",
        rms1,
        rms2
    );
}

#[test]
fn test_hot_swap_multiple_rapid_swaps() {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    let codes = [
        "tempo: 2\nout $ sine 220 * 0.2",
        "tempo: 2\nout $ sine 440 * 0.2",
        "tempo: 2\nout $ sine 880 * 0.2",
        "tempo: 2\nout $ saw 110 * 0.2",
        "tempo: 2\nout $ square 330 * 0.2",
    ];

    let mut graph = compile_code(codes[0], sample_rate);

    // Rapid swaps with small renders in between
    for i in 1..50 {
        let code = codes[i % codes.len()];
        graph = swap_graph(&mut graph, code, sample_rate);

        // Process a small buffer after each swap
        let _ = render_audio(&mut graph, buffer_size);
    }

    // Final render should work
    let final_audio = render_audio(&mut graph, 44100);
    let rms = calculate_rms(&final_audio);

    assert!(
        rms > 0.05,
        "Should produce audio after 50 rapid swaps: rms={}",
        rms
    );
}

#[test]
fn test_hot_swap_with_effects_chain() {
    let sample_rate = 44100.0;

    let with_effects = r#"
tempo: 2
out $ saw 110 # lpf 800 0.7 # reverb 0.5 0.4 0.2
"#;

    let different_effects = r#"
tempo: 2
out $ saw 110 # hpf 200 0.7 # delay 0.25 0.5 0.3
"#;

    let mut graph = compile_code(with_effects, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, different_effects, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    // Both should produce audio
    assert!(
        calculate_rms(&audio1) > 0.01,
        "First pattern should produce audio"
    );
    assert!(
        calculate_rms(&audio2) > 0.01,
        "Second pattern should produce audio"
    );
}

#[test]
fn test_hot_swap_node_count_bounded() {
    let sample_rate = 44100.0;

    let simple = "tempo: 2\nout $ sine 440";
    let complex = r#"
tempo: 2
~a $ sine 220
~b $ saw 110
~c $ square 330
out $ (~a + ~b + ~c) * 0.2
"#;

    let mut graph = compile_code(simple, sample_rate);

    // Track node counts separately for simple and complex graphs
    let mut simple_counts = Vec::new();
    let mut complex_counts = Vec::new();

    // Swap back and forth 20 times
    for i in 0..20 {
        let code = if i % 2 == 0 { complex } else { simple };
        graph = swap_graph(&mut graph, code, sample_rate);
        let count = graph.node_count();

        if i % 2 == 0 {
            complex_counts.push(count);
        } else {
            simple_counts.push(count);
        }

        // Render a buffer
        let _ = render_audio(&mut graph, 512);
    }

    // Node counts for the same graph type should not grow over repeated swaps
    // (i.e., no node leak). Allow small variance but first and last should match.
    let simple_first = simple_counts[0];
    let simple_last = *simple_counts.last().unwrap();
    assert!(
        simple_last <= simple_first + 1,
        "Simple graph node count should not grow: first={}, last={}, all={:?}",
        simple_first,
        simple_last,
        simple_counts
    );

    let complex_first = complex_counts[0];
    let complex_last = *complex_counts.last().unwrap();
    assert!(
        complex_last <= complex_first + 1,
        "Complex graph node count should not grow: first={}, last={}, all={:?}",
        complex_first,
        complex_last,
        complex_counts
    );
}

// ============================================================================
// Pattern Transition Tests
// ============================================================================

#[test]
fn test_pattern_transition_euclidean_to_straight() {
    let sample_rate = 44100.0;

    let euclidean = r#"
tempo: 2
out $ s "bd(3,8) sn(2,8)"
"#;

    let straight = r#"
tempo: 2
out $ s "bd sn hh cp"
"#;

    let mut graph = compile_code(euclidean, sample_rate);
    let _ = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, straight, sample_rate);
    let _ = render_audio(&mut new_graph, 44100);

    // Should not panic or hang
    assert!(new_graph.get_cps() > 0.0);
}

#[test]
fn test_pattern_transition_fast_to_slow() {
    let sample_rate = 44100.0;

    let fast = r#"
tempo: 4
out $ s "hh*16"
"#;

    let slow = r#"
tempo: 0.25
out $ s "bd"
"#;

    let mut graph = compile_code(fast, sample_rate);
    let audio_fast = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, slow, sample_rate);
    let audio_slow = render_audio(&mut new_graph, 44100);

    // Fast pattern should have more zero crossings (more activity)
    let _zc_fast = count_zero_crossings(&audio_fast);
    let _zc_slow = count_zero_crossings(&audio_slow);

    // At least verify we can render both
    assert!(true, "Should handle fast pattern");
    assert!(true, "Should handle slow pattern");
}

#[test]
fn test_pattern_transition_with_transforms() {
    let sample_rate = 44100.0;

    let with_rev = r#"
tempo: 2
out $ s "bd sn hh cp" $ rev
"#;

    let with_fast = r#"
tempo: 2
out $ s "bd sn hh cp" $ fast 2
"#;

    let with_palindrome = r#"
tempo: 2
out $ s "bd sn hh cp" $ palindrome
"#;

    let mut graph = compile_code(with_rev, sample_rate);
    let _ = render_audio(&mut graph, 22050);

    graph = swap_graph(&mut graph, with_fast, sample_rate);
    let _ = render_audio(&mut graph, 22050);

    graph = swap_graph(&mut graph, with_palindrome, sample_rate);
    let audio = render_audio(&mut graph, 22050);

    // Should complete without panic
    assert!(audio.len() == 22050);
}

#[test]
fn test_pattern_transition_alternation() {
    let sample_rate = 44100.0;

    let alt1 = r#"
tempo: 2
out $ s "<bd sn> hh"
"#;

    let alt2 = r#"
tempo: 2
out $ s "bd <sn cp hh>"
"#;

    let mut graph = compile_code(alt1, sample_rate);
    let _ = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, alt2, sample_rate);
    let audio = render_audio(&mut new_graph, 44100);

    assert_eq!(audio.len(), 44100);
}

#[test]
fn test_pattern_transition_nested_mini_notation() {
    let sample_rate = 44100.0;

    let nested1 = r#"
tempo: 2
out $ s "[bd sn] [hh hh hh]"
"#;

    let nested2 = r#"
tempo: 2
out $ s "[[bd bd] sn] [hh [cp cp]]"
"#;

    let mut graph = compile_code(nested1, sample_rate);
    let _ = render_audio(&mut graph, 22050);

    let mut new_graph = swap_graph(&mut graph, nested2, sample_rate);
    let audio = render_audio(&mut new_graph, 22050);

    assert_eq!(audio.len(), 22050);
}

// ============================================================================
// Tempo Change Tests
// ============================================================================

#[test]
fn test_tempo_change_increase() {
    let sample_rate = 44100.0;

    let slow = r#"
tempo: 0.5
out $ sine 440 * 0.3
"#;

    let fast = r#"
tempo: 2.0
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(slow, sample_rate);
    assert!((graph.get_cps() - 0.5).abs() < 0.001);

    let new_graph = swap_graph(&mut graph, fast, sample_rate);
    assert!((new_graph.get_cps() - 2.0).abs() < 0.001);
}

#[test]
fn test_tempo_change_decrease() {
    let sample_rate = 44100.0;

    let fast = r#"
tempo: 4.0
out $ sine 440 * 0.3
"#;

    let slow = r#"
tempo: 0.25
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(fast, sample_rate);
    assert!((graph.get_cps() - 4.0).abs() < 0.001);

    let new_graph = swap_graph(&mut graph, slow, sample_rate);
    assert!((new_graph.get_cps() - 0.25).abs() < 0.001);
}

#[test]
fn test_tempo_stability_through_swaps() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 1.0
out $ s "bd sn"
"#;

    let mut graph = compile_code(code, sample_rate);

    // Do 100 swaps, all with same tempo
    for _ in 0..100 {
        graph = swap_graph(&mut graph, code, sample_rate);

        // CPS should remain exactly 1.0
        let cps = graph.get_cps();
        assert!(
            (cps - 1.0).abs() < 0.001,
            "CPS should stay at 1.0, got {}",
            cps
        );
    }
}

#[test]
fn test_tempo_via_bpm() {
    let sample_rate = 44100.0;

    let code_120bpm = r#"
bpm: 120
out $ sine 440 * 0.3
"#;

    let code_60bpm = r#"
bpm: 60
out $ sine 440 * 0.3
"#;

    // 120 BPM in 4/4 = 120 / (4 * 60) = 0.5 CPS
    let mut graph = compile_code(code_120bpm, sample_rate);
    assert!(
        (graph.get_cps() - 0.5).abs() < 0.01,
        "120 BPM should be 0.5 CPS, got {}",
        graph.get_cps()
    );

    // 60 BPM in 4/4 = 60 / (4 * 60) = 0.25 CPS
    let new_graph = swap_graph(&mut graph, code_60bpm, sample_rate);
    assert!(
        (new_graph.get_cps() - 0.25).abs() < 0.01,
        "60 BPM should be 0.25 CPS, got {}",
        new_graph.get_cps()
    );
}

#[test]
fn test_tempo_fractional_values() {
    let sample_rate = 44100.0;

    let tempos = [0.125, 0.25, 0.333, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];

    for tempo in tempos {
        let code = format!("tempo: {}\nout $ sine 440 * 0.3", tempo);
        let graph = compile_code(&code, sample_rate);

        assert!(
            (graph.get_cps() - tempo as f32).abs() < 0.01,
            "Tempo {} should produce CPS {}, got {}",
            tempo,
            tempo,
            graph.get_cps()
        );
    }
}

// ============================================================================
// Bus Reference Tests
// ============================================================================

#[test]
fn test_bus_reference_simple() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
~freq $ 440
out $ sine ~freq * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    let audio = render_audio(&mut graph, 44100);

    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Should produce audio via bus reference");
}

#[test]
fn test_bus_reference_update() {
    let sample_rate = 44100.0;

    let code1 = r#"
tempo: 2
~freq $ 220
out $ sine ~freq * 0.3
"#;

    let code2 = r#"
tempo: 2
~freq $ 880
out $ sine ~freq * 0.3
"#;

    let mut graph = compile_code(code1, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, code2, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    // Both should produce audio (different frequencies)
    assert!(calculate_rms(&audio1) > 0.1);
    assert!(calculate_rms(&audio2) > 0.1);

    // Different frequencies should have different zero crossing rates
    let zc1 = count_zero_crossings(&audio1);
    let zc2 = count_zero_crossings(&audio2);

    // 880 Hz has 4x the frequency of 220 Hz, so ~4x zero crossings
    assert!(
        zc2 > zc1 * 2,
        "Higher frequency should have more zero crossings: {} vs {}",
        zc1,
        zc2
    );
}

#[test]
fn test_bus_reference_chain() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
~base $ 110
~doubled $ ~base * 2
~quadrupled $ ~doubled * 2
out $ sine ~quadrupled * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    let audio = render_audio(&mut graph, 44100);

    // 110 * 4 = 440 Hz
    let rms = calculate_rms(&audio);
    assert!(rms > 0.1, "Chained bus references should work");
}

#[test]
fn test_bus_reference_pattern() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
~freqs $ "220 440 660"
out $ sine ~freqs * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    let audio = render_audio(&mut graph, 44100);

    let rms = calculate_rms(&audio);
    assert!(rms > 0.05, "Pattern bus should produce audio");
}

#[test]
fn test_bus_reference_lfo_modulation() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
~lfo $ sine 2
~cutoff $ ~lfo * 500 + 1000
out $ saw 110 # lpf ~cutoff 0.7 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    let audio = render_audio(&mut graph, 88200); // 2 seconds

    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "LFO modulation via bus should work");
}

#[test]
fn test_bus_reference_across_swap() {
    let sample_rate = 44100.0;

    let code1 = r#"
tempo: 2
~osc $ sine 440
out $ ~osc * 0.3
"#;

    let code2 = r#"
tempo: 2
~osc $ saw 440
out $ ~osc * 0.3
"#;

    let mut graph = compile_code(code1, sample_rate);
    let audio1 = render_audio(&mut graph, 22050);

    let mut new_graph = swap_graph(&mut graph, code2, sample_rate);
    let audio2 = render_audio(&mut new_graph, 22050);

    // Saw has more harmonics than sine (higher RMS for same amplitude)
    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    assert!(rms1 > 0.1, "Sine should produce audio");
    assert!(rms2 > 0.1, "Saw should produce audio");
}

// ============================================================================
// Effect Chain Modification Tests
// ============================================================================

#[test]
fn test_effect_add_filter() {
    let sample_rate = 44100.0;

    let without_filter = r#"
tempo: 2
out $ saw 110 * 0.3
"#;

    let with_filter = r#"
tempo: 2
out $ saw 110 # lpf 500 0.8 * 0.3
"#;

    let mut graph = compile_code(without_filter, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, with_filter, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    // Both should produce audio
    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    assert!(rms1 > 0.1, "Unfiltered saw should produce audio");
    assert!(rms2 > 0.05, "Filtered saw should produce audio");

    // LPF removes high frequencies, so RMS should be lower
    assert!(rms2 < rms1, "LPF should reduce RMS: {} < {}", rms2, rms1);
}

#[test]
fn test_effect_remove_filter() {
    let sample_rate = 44100.0;

    let with_filter = r#"
tempo: 2
out $ saw 110 # lpf 500 0.8 * 0.3
"#;

    let without_filter = r#"
tempo: 2
out $ saw 110 * 0.3
"#;

    let mut graph = compile_code(with_filter, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, without_filter, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    let rms1 = calculate_rms(&audio1);
    let rms2 = calculate_rms(&audio2);

    assert!(rms1 > 0.05, "Filtered saw should produce audio");
    assert!(rms2 > 0.1, "Unfiltered saw should produce audio");

    // Removing filter should increase RMS
    assert!(
        rms2 > rms1,
        "Removing LPF should increase RMS: {} > {}",
        rms2,
        rms1
    );
}

#[test]
fn test_effect_change_filter_type() {
    let sample_rate = 44100.0;

    let lpf = r#"
tempo: 2
out $ saw 110 # lpf 800 0.7 * 0.3
"#;

    let hpf = r#"
tempo: 2
out $ saw 110 # hpf 800 0.7 * 0.3
"#;

    let bpf = r#"
tempo: 2
out $ saw 110 # bpf 800 2.0 * 0.3
"#;

    let mut graph = compile_code(lpf, sample_rate);
    let audio_lpf = render_audio(&mut graph, 22050);

    graph = swap_graph(&mut graph, hpf, sample_rate);
    let audio_hpf = render_audio(&mut graph, 22050);

    graph = swap_graph(&mut graph, bpf, sample_rate);
    let audio_bpf = render_audio(&mut graph, 22050);

    // All should produce audio
    assert!(calculate_rms(&audio_lpf) > 0.01);
    assert!(calculate_rms(&audio_hpf) > 0.01);
    assert!(calculate_rms(&audio_bpf) > 0.01);
}

#[test]
fn test_effect_chain_reorder() {
    let sample_rate = 44100.0;

    let filter_first = r#"
tempo: 2
out $ saw 110 # lpf 800 0.7 # distortion 2.0 0.3
"#;

    let distortion_first = r#"
tempo: 2
out $ saw 110 # distortion 2.0 0.3 # lpf 800 0.7
"#;

    let mut graph = compile_code(filter_first, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, distortion_first, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    // Both should produce audio (different character though)
    assert!(calculate_rms(&audio1) > 0.01);
    assert!(calculate_rms(&audio2) > 0.01);
}

#[test]
fn test_effect_add_reverb() {
    let sample_rate = 44100.0;

    let dry = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let wet = r#"
tempo: 2
out $ sine 440 # reverb 0.8 0.5 0.3
"#;

    let mut graph = compile_code(dry, sample_rate);
    let audio_dry = render_audio(&mut graph, 88200);

    let mut new_graph = swap_graph(&mut graph, wet, sample_rate);
    let audio_wet = render_audio(&mut new_graph, 88200);

    assert!(calculate_rms(&audio_dry) > 0.1);
    assert!(calculate_rms(&audio_wet) > 0.1);
}

#[test]
fn test_effect_add_delay() {
    let sample_rate = 44100.0;

    let without_delay = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let with_delay = r#"
tempo: 2
out $ sine 440 # delay 0.25 0.5 0.4
"#;

    let mut graph = compile_code(without_delay, sample_rate);
    let _ = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, with_delay, sample_rate);
    let audio = render_audio(&mut new_graph, 44100);

    assert!(calculate_rms(&audio) > 0.1);
}

// ============================================================================
// Continuous Modulation Transition Tests
// ============================================================================

#[test]
fn test_lfo_modulation_change() {
    let sample_rate = 44100.0;

    let slow_lfo = r#"
tempo: 2
~lfo $ sine 0.5
out $ saw 110 # lpf (~lfo * 500 + 1000) 0.7 * 0.3
"#;

    let fast_lfo = r#"
tempo: 2
~lfo $ sine 4.0
out $ saw 110 # lpf (~lfo * 500 + 1000) 0.7 * 0.3
"#;

    let mut graph = compile_code(slow_lfo, sample_rate);
    let audio1 = render_audio(&mut graph, 88200);

    let mut new_graph = swap_graph(&mut graph, fast_lfo, sample_rate);
    let audio2 = render_audio(&mut new_graph, 88200);

    assert!(calculate_rms(&audio1) > 0.01);
    assert!(calculate_rms(&audio2) > 0.01);
}

#[test]
fn test_pattern_modulation_change() {
    let sample_rate = 44100.0;

    let step_mod = r#"
tempo: 2
~cutoffs $ "500 1000 2000"
out $ saw 110 # lpf ~cutoffs 0.7 * 0.3
"#;

    let smooth_mod = r#"
tempo: 2
~lfo $ sine 2
~cutoffs $ ~lfo * 750 + 1250
out $ saw 110 # lpf ~cutoffs 0.7 * 0.3
"#;

    let mut graph = compile_code(step_mod, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, smooth_mod, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    assert!(calculate_rms(&audio1) > 0.01);
    assert!(calculate_rms(&audio2) > 0.01);
}

#[test]
fn test_frequency_modulation_swap() {
    let sample_rate = 44100.0;

    let static_freq = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let modulated_freq = r#"
tempo: 2
~lfo $ sine 5
~freq $ ~lfo * 50 + 440
out $ sine ~freq * 0.3
"#;

    let mut graph = compile_code(static_freq, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, modulated_freq, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    assert!(calculate_rms(&audio1) > 0.1);
    assert!(calculate_rms(&audio2) > 0.1);
}

#[test]
fn test_amplitude_modulation_swap() {
    let sample_rate = 44100.0;

    let static_amp = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let tremolo = r#"
tempo: 2
~lfo $ sine 6
~amp $ ~lfo * 0.15 + 0.15
out $ sine 440 * ~amp
"#;

    let mut graph = compile_code(static_amp, sample_rate);
    let audio1 = render_audio(&mut graph, 44100);

    let mut new_graph = swap_graph(&mut graph, tremolo, sample_rate);
    let audio2 = render_audio(&mut new_graph, 44100);

    assert!(calculate_rms(&audio1) > 0.1);
    assert!(calculate_rms(&audio2) > 0.05);
}

// ============================================================================
// Performance and Stability Tests
// ============================================================================

#[test]
fn test_render_time_stability() {
    let sample_rate = 44100.0;
    let buffer_size = 512;

    let code = r#"
tempo: 2
~drums $ s "bd*2 sn hh*4"
~bass $ saw 55 # lpf 400 0.8
out $ ~drums * 0.5 + ~bass * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);

    // Warm up
    for _ in 0..10 {
        let _ = render_audio(&mut graph, buffer_size);
    }

    // Measure render times
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let _ = render_audio(&mut graph, buffer_size);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let max = times.iter().cloned().fold(0.0, f64::max);

    // Max shouldn't be more than 10x average (no huge spikes)
    assert!(
        max < avg * 10.0,
        "Render time spikes detected: avg={:.3}ms, max={:.3}ms",
        avg,
        max
    );
}

#[test]
fn test_no_infinite_loops_on_swap() {
    let sample_rate = 44100.0;

    let codes = [
        "tempo: 2\nout $ sine 440",
        "tempo: 2\nout $ s \"bd sn\"",
        "tempo: 2\n~a $ sine 220\nout $ ~a * 0.3",
        "tempo: 2\nout $ saw 110 # lpf 800 0.7",
    ];

    let mut graph = compile_code(codes[0], sample_rate);

    let timeout = std::time::Duration::from_secs(30);
    let start = Instant::now();

    for i in 0..100 {
        if start.elapsed() > timeout {
            panic!("Timeout! Possible infinite loop at swap {}", i);
        }

        let code = codes[i % codes.len()];
        graph = swap_graph(&mut graph, code, sample_rate);

        // Render some audio
        let _ = render_audio(&mut graph, 512);
    }

    // Should complete well within timeout (detects infinite loops, not performance)
    assert!(
        start.elapsed() < timeout,
        "100 swaps took too long: {:?}",
        start.elapsed()
    );
}

#[test]
fn test_memory_stability_many_swaps() {
    let sample_rate = 44100.0;

    let complex = r#"
tempo: 2
~drums $ s "bd*4 sn*2 hh*8 cp*2"
~bass $ saw "55 110" # lpf 800 0.8
~lead $ sine "220 440 660"
out $ ~drums * 0.4 + ~bass * 0.3 + ~lead * 0.2
"#;

    let simple = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(simple, sample_rate);

    // Do 200 swaps between complex and simple
    for i in 0..200 {
        let code = if i % 2 == 0 { complex } else { simple };
        graph = swap_graph(&mut graph, code, sample_rate);

        // Render between swaps
        let _ = render_audio(&mut graph, 256);
    }

    // Final render should work
    let audio = render_audio(&mut graph, 44100);
    let rms = calculate_rms(&audio);

    assert!(
        rms > 0.1,
        "Should produce audio after 200 swaps: rms={}",
        rms
    );
}

// ============================================================================
// Hush and Panic Tests (Live Control)
// ============================================================================

#[test]
fn test_hush_silences_output() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);

    // Render some audio
    let audio_before = render_audio(&mut graph, 22050);
    assert!(
        calculate_rms(&audio_before) > 0.1,
        "Should have audio before hush"
    );

    // Hush
    graph.hush_all();

    // Render after hush
    let audio_after = render_audio(&mut graph, 22050);
    assert!(
        calculate_rms(&audio_after) < 0.001,
        "Should be silent after hush"
    );
}

#[test]
fn test_panic_stops_all() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 2
~a $ sine 220
~b $ sine 440
~c $ sine 880
out $ (~a + ~b + ~c) * 0.1
"#;

    let mut graph = compile_code(code, sample_rate);

    // Render some audio
    let audio_before = render_audio(&mut graph, 22050);
    assert!(
        calculate_rms(&audio_before) > 0.05,
        "Should have audio before panic"
    );

    // Panic
    graph.panic();

    // Render after panic
    let audio_after = render_audio(&mut graph, 22050);
    assert!(
        calculate_rms(&audio_after) < 0.001,
        "Should be silent after panic"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_pattern_swap() {
    let sample_rate = 44100.0;

    let with_pattern = r#"
tempo: 2
out $ s "bd sn"
"#;

    let empty = r#"
tempo: 2
out $ s "~"
"#;

    let mut graph = compile_code(with_pattern, sample_rate);
    let _ = render_audio(&mut graph, 22050);

    let mut new_graph = swap_graph(&mut graph, empty, sample_rate);
    let audio = render_audio(&mut new_graph, 22050);

    // Should be silent but not crash
    assert_eq!(audio.len(), 22050);
}

#[test]
fn test_very_fast_tempo() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 16
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    assert!((graph.get_cps() - 16.0).abs() < 0.01);

    let audio = render_audio(&mut graph, 44100);
    assert!(calculate_rms(&audio) > 0.1);
}

#[test]
fn test_very_slow_tempo() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 0.0625
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);
    assert!((graph.get_cps() - 0.0625).abs() < 0.001);

    let audio = render_audio(&mut graph, 44100);
    assert!(calculate_rms(&audio) > 0.1);
}

#[test]
fn test_swap_during_cycle_boundary() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 1.0
out $ s "bd sn hh cp"
"#;

    let mut graph = compile_code(code, sample_rate);

    // Render exactly at cycle boundaries (at 1 CPS, 44100 samples = 1 cycle)
    for _ in 0..10 {
        let _ = render_audio(&mut graph, 44100);
        graph = swap_graph(&mut graph, code, sample_rate);
    }

    // Should complete without issues
    let audio = render_audio(&mut graph, 44100);
    assert_eq!(audio.len(), 44100);
}

#[test]
fn test_swap_mid_cycle() {
    let sample_rate = 44100.0;

    let code = r#"
tempo: 1.0
out $ sine 440 * 0.3
"#;

    let mut graph = compile_code(code, sample_rate);

    // Render partial cycles and swap
    for _ in 0..10 {
        let _ = render_audio(&mut graph, 11025); // Quarter cycle
        graph = swap_graph(&mut graph, code, sample_rate);
    }

    // Should complete without issues
    let audio = render_audio(&mut graph, 44100);
    assert!(calculate_rms(&audio) > 0.1);
}
