//! Comprehensive tests for nested function calls with space-separated syntax
//!
//! Tests various levels of nesting AND verifies that effects actually transform the audio

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

/// Helper to calculate RMS of audio buffer
fn calc_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

/// Helper to calculate spectral centroid (rough estimate of brightness)
fn calc_spectral_centroid(buffer: &[f32]) -> f32 {
    // Very simple approximation: measure high-frequency energy
    let mut high_freq_energy = 0.0;
    let mut total_energy = 0.0;

    for i in 1..buffer.len() {
        let diff = (buffer[i] - buffer[i - 1]).abs();
        high_freq_energy += diff;
        total_energy += buffer[i].abs();
    }

    if total_energy > 0.0 {
        high_freq_energy / total_energy
    } else {
        0.0
    }
}

#[test]
fn test_single_level_nesting() {
    // Compare: raw sine vs reverb(sine)
    // Reverb should change the envelope and add reflections

    let base_code = r#"
        bpm 120
        out: sine 440 * 0.2
    "#;

    let reverb_code = r#"
        bpm 120
        out: reverb (sine 440) 0.7 0.5 0.5 * 0.2
    "#;

    // Render base
    let (_, base_statements) = parse_dsl(base_code).unwrap();
    let base_compiler = DslCompiler::new(44100.0);
    let mut base_graph = base_compiler.compile(base_statements);
    let base_buffer = base_graph.render(8820); // 0.2 seconds
    let base_rms = calc_rms(&base_buffer);

    // Render with reverb
    let (remaining, reverb_statements) = parse_dsl(reverb_code).unwrap();
    assert!(remaining.trim().is_empty(), "Should consume all input");
    let reverb_compiler = DslCompiler::new(44100.0);
    let mut reverb_graph = reverb_compiler.compile(reverb_statements);
    let reverb_buffer = reverb_graph.render(8820);
    let reverb_rms = calc_rms(&reverb_buffer);

    // Both should have audio
    assert!(
        base_rms > 0.001,
        "Base should produce audio, got RMS={}",
        base_rms
    );
    assert!(
        reverb_rms > 0.001,
        "Reverb should produce audio, got RMS={}",
        reverb_rms
    );

    // Reverb should change the signal (not just pass through)
    // With 0.5 mix, reverb should affect the overall RMS
    let rms_ratio = (reverb_rms - base_rms).abs() / base_rms;
    assert!(
        rms_ratio > 0.01,
        "Reverb should noticeably change the audio (RMS ratio: {:.3})",
        rms_ratio
    );
}

#[test]
fn test_double_level_nesting() {
    // Compare: saw wave vs lpf(saw wave) with VERY low cutoff
    // LPF should significantly reduce overall amplitude

    let bright_code = r#"
        bpm 120
        out: saw 110 * 0.2
    "#;

    let filtered_code = r#"
        bpm 120
        out: lpf (saw 110) 200 0.5 * 0.2
    "#;

    // Render bright version
    let (_, bright_statements) = parse_dsl(bright_code).unwrap();
    let bright_compiler = DslCompiler::new(44100.0);
    let mut bright_graph = bright_compiler.compile(bright_statements);
    let bright_buffer = bright_graph.render(4410);
    let bright_rms = calc_rms(&bright_buffer);
    let bright_centroid = calc_spectral_centroid(&bright_buffer);

    // Render heavily filtered version (200 Hz cutoff on 110 Hz saw)
    let (remaining, filtered_statements) = parse_dsl(filtered_code).unwrap();
    assert!(remaining.trim().is_empty(), "Should consume all input");
    let filtered_compiler = DslCompiler::new(44100.0);
    let mut filtered_graph = filtered_compiler.compile(filtered_statements);
    let filtered_buffer = filtered_graph.render(4410);
    let filtered_rms = calc_rms(&filtered_buffer);
    let filtered_centroid = calc_spectral_centroid(&filtered_buffer);

    // Both should produce audio
    assert!(bright_rms > 0.001, "Bright saw should produce audio");
    assert!(filtered_rms > 0.001, "Filtered saw should produce audio");

    // Filtered version should be noticeably different
    // Either lower RMS OR lower spectral centroid (or both)
    let rms_reduced = bright_rms > filtered_rms * 1.1;
    let spectrum_reduced = bright_centroid > filtered_centroid * 1.05;

    assert!(rms_reduced || spectrum_reduced,
        "LPF should change the audio: bright_rms={:.4}, filtered_rms={:.4}, bright_centroid={:.4}, filtered_centroid={:.4}",
        bright_rms, filtered_rms, bright_centroid, filtered_centroid);
}

#[test]
fn test_triple_level_nesting() {
    // Three levels: reverb(lpf(sine(pattern), ...), ...)
    let code = r#"
        bpm 120
        out: reverb (delay (lpf (sine 440) 1000 0.8) 0.25 0.5 0.5) 0.7 0.5 0.5
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse triple-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_with_arithmetic() {
    // Compare: static filter vs LFO-modulated filter
    // LFO modulation should create time-varying spectral content

    let static_code = r#"
        bpm 120
        out: lpf (sine 440) 1250 0.8 * 0.3
    "#;

    let modulated_code = r#"
        bpm 120
        ~lfo: sine 0.5
        out: lpf (sine 440) (~lfo * 2000 + 500) 0.8 * 0.3
    "#;

    // Render static version
    let (_, static_statements) = parse_dsl(static_code).unwrap();
    let static_compiler = DslCompiler::new(44100.0);
    let mut static_graph = static_compiler.compile(static_statements);
    let static_buffer = static_graph.render(44100); // 1 second

    // Render modulated version
    let (remaining, modulated_statements) = parse_dsl(modulated_code).unwrap();
    assert!(remaining.trim().is_empty(), "Should consume all input");
    let modulated_compiler = DslCompiler::new(44100.0);
    let mut modulated_graph = modulated_compiler.compile(modulated_statements);
    let modulated_buffer = modulated_graph.render(44100);

    // Both should produce audio
    assert!(
        calc_rms(&static_buffer) > 0.001,
        "Static filter should produce audio"
    );
    assert!(
        calc_rms(&modulated_buffer) > 0.001,
        "Modulated filter should produce audio"
    );

    // Measure variation in the signal by comparing first half vs second half
    let static_first_half = &static_buffer[0..22050];
    let static_second_half = &static_buffer[22050..44100];
    let static_variation = (calc_rms(static_first_half) - calc_rms(static_second_half)).abs();

    let modulated_first_half = &modulated_buffer[0..22050];
    let modulated_second_half = &modulated_buffer[22050..44100];
    let modulated_variation =
        (calc_rms(modulated_first_half) - calc_rms(modulated_second_half)).abs();

    // Modulated version should show MORE variation over time
    // (LFO sweeps the filter, changing brightness)
    assert!(
        modulated_variation > static_variation * 1.5,
        "LFO modulation should create time-varying sound: static_var={:.4}, modulated_var={:.4}",
        static_variation,
        modulated_variation
    );
}

#[test]
fn test_multiple_nested_calls_in_expression() {
    // Compare: single filtered source vs sum of two differently filtered sources
    // The combined version should have different spectral characteristics

    let single_code = r#"
        bpm 120
        out: lpf (sine 440) 1000 0.8 * 0.5
    "#;

    let combined_code = r#"
        bpm 120
        out: (lpf (sine 440) 1000 0.8) + (lpf (sine 220) 2000 0.5) * 0.5
    "#;

    // Render single source
    let (_, single_statements) = parse_dsl(single_code).unwrap();
    let single_compiler = DslCompiler::new(44100.0);
    let mut single_graph = single_compiler.compile(single_statements);
    let single_buffer = single_graph.render(4410);
    let single_rms = calc_rms(&single_buffer);

    // Render combined sources
    let (remaining, combined_statements) = parse_dsl(combined_code).unwrap();
    assert!(remaining.trim().is_empty(), "Should consume all input");
    let combined_compiler = DslCompiler::new(44100.0);
    let mut combined_graph = combined_compiler.compile(combined_statements);
    let combined_buffer = combined_graph.render(4410);
    let combined_rms = calc_rms(&combined_buffer);

    // Both should produce audio
    assert!(single_rms > 0.001, "Single source should produce audio");
    assert!(
        combined_rms > 0.001,
        "Combined sources should produce audio"
    );

    // Combined version should have MORE energy (two sources added)
    assert!(
        combined_rms > single_rms * 1.2,
        "Combined sources should be louder than single: single={:.4}, combined={:.4}",
        single_rms,
        combined_rms
    );
}

#[test]
fn test_nesting_with_pattern_strings() {
    // Compare: constant frequency vs pattern-changing frequency
    // Pattern should create varying pitch

    let constant_code = r#"
        bpm 120
        out: lpf (sine 330) 1500 0.8 * 0.3
    "#;

    let pattern_code = r#"
        bpm 120
        out: lpf (sine "220 440 330") 1500 0.8 * 0.3
    "#;

    // Render constant frequency
    let (_, constant_statements) = parse_dsl(constant_code).unwrap();
    let constant_compiler = DslCompiler::new(44100.0);
    let mut constant_graph = constant_compiler.compile(constant_statements);
    let constant_buffer = constant_graph.render(44100); // 1 second

    // Render pattern frequency
    let (remaining, pattern_statements) = parse_dsl(pattern_code).unwrap();
    assert!(remaining.trim().is_empty(), "Should consume all input");
    let pattern_compiler = DslCompiler::new(44100.0);
    let mut pattern_graph = pattern_compiler.compile(pattern_statements);
    let pattern_buffer = pattern_graph.render(44100);

    // Both should produce audio
    assert!(
        calc_rms(&constant_buffer) > 0.001,
        "Constant frequency should produce audio"
    );
    assert!(
        calc_rms(&pattern_buffer) > 0.001,
        "Pattern frequency should produce audio"
    );

    // Divide into thirds and check variation
    let third = constant_buffer.len() / 3;
    let pattern_third1_rms = calc_rms(&pattern_buffer[0..third]);
    let pattern_third2_rms = calc_rms(&pattern_buffer[third..2 * third]);
    let pattern_third3_rms = calc_rms(&pattern_buffer[2 * third..]);

    let constant_third1_rms = calc_rms(&constant_buffer[0..third]);
    let constant_third2_rms = calc_rms(&constant_buffer[third..2 * third]);

    // Pattern should show variation between sections
    let pattern_variation = (pattern_third1_rms - pattern_third2_rms).abs()
        + (pattern_third2_rms - pattern_third3_rms).abs();
    let constant_variation = (constant_third1_rms - constant_third2_rms).abs();

    // Pattern should vary more than constant (due to frequency changes)
    assert!(
        pattern_variation > constant_variation * 1.5,
        "Pattern should create varying audio: constant_var={:.4}, pattern_var={:.4}",
        constant_variation,
        pattern_variation
    );
}

#[test]
fn test_nesting_with_sample_patterns() {
    // Nesting with sample patterns
    let code = r#"
        bpm 120
        out: reverb (s "bd sn") 0.5 0.3 0.4 * 50
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with samples");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_with_bus_refs() {
    // Nesting with bus references
    let code = r#"
        bpm 120
        ~osc: sine 440
        out: lpf ~osc 1000 0.8 * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with bus refs");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 3);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Nesting with bus refs should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_deeply_nested_with_mixed_types() {
    // Deep nesting with various argument types
    let code = r#"
        bpm 120
        ~lfo: sine 0.25
        out: reverb (delay (lpf (saw "110 220") (~lfo * 1000 + 500) 0.8) 0.25 0.5 0.3) 0.7 0.5 0.4 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse deeply nested mixed types");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 3);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(8820);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Deep nesting with mixed types should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_nesting_with_chaining() {
    // Nesting combined with # operator chaining
    let code = r#"
        bpm 120
        out: lpf (s "bd sn" # gain 0.8) 2000 0.5 * 50
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with chaining");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_quadruple_level_nesting() {
    // Four levels of nesting - stress test
    let code = r#"
        bpm 120
        out: reverb (delay (lpf (sine 440) 1000 0.8) 0.25 0.5 0.3) 0.7 0.5 0.5 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse quadruple-level nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles without panicking
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_parallel_nested_calls() {
    // Multiple nested calls at same level (in addition)
    let code = r#"
        bpm 120
        out: (reverb (sine 440) 0.5 0.3 0.4) + (delay (saw 220) 0.25 0.5 0.3) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse parallel nested calls");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Parallel nested calls should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_asymmetric_nesting() {
    // Different nesting depths in same expression
    let code = r#"
        bpm 120
        out: (lpf (sine 440) 1000 0.8) + (sine 220) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse asymmetric nesting");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Asymmetric nesting should produce audio, got RMS={}",
        rms
    );
}

#[test]
fn test_nesting_with_all_numeric_args() {
    // Ensure numeric args are parsed correctly in nested context
    let code = r#"
        bpm 120
        out: delay (sine 440) 0.25 0.5 0.3 * 0.2
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse nesting with numeric args");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let _graph = compiler.compile(statements);
}

#[test]
fn test_nesting_stops_at_operators() {
    // Ensure parser correctly stops at operators
    let code = r#"
        bpm 120
        out: (sine 440) * 0.5 + (saw 220) * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("Should parse with operator boundaries");
    assert!(remaining.trim().is_empty(), "Should consume all input");
    assert_eq!(statements.len(), 2);

    // Verify it compiles
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Verify it produces audio
    let buffer = graph.render(4410);
    let rms: f32 = (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt();
    assert!(
        rms > 0.001,
        "Operator boundaries should work correctly, got RMS={}",
        rms
    );
}
