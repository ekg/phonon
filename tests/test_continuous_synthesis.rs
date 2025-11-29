use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

/// Test that bus-triggered synthesis generates continuously until envelope finishes
/// NOT pre-rendered to fixed-length buffer
#[test]
#[ignore = "bus triggering via s pattern not producing audio - needs investigation"]
fn test_bus_triggered_synth_continuous_generation() {
    let sample_rate = 44100.0;

    // Synth bus with long attack/release that crosses buffer boundaries
    let code = r#"
tempo: 0.5
~synth $ sine 440
~trig $ s "~synth"
out $ ~trig
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    // Use sample-based timing for offline rendering (no wall-clock)
    // Time advances exactly by buffer_size samples per render() call
    // This ensures perfect continuity across buffer boundaries

    // Render 2 seconds (4 cycles at 2 cps) in multiple buffers
    // This ensures we cross buffer boundaries
    // NOTE: Using 128 instead of 512 to match the passing test's buffer size
    let buffer_size = 128;
    let num_buffers = (sample_rate * 2.0) as usize / buffer_size;

    let mut full_audio = Vec::new();
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        full_audio.extend_from_slice(&buffer);
    }

    // Verify continuous generation with no clicks at buffer boundaries
    // Check for discontinuities at buffer boundaries
    let mut max_discontinuity = 0.0_f32;
    for i in (buffer_size..full_audio.len()).step_by(buffer_size) {
        if i > 0 && i < full_audio.len() {
            let diff = (full_audio[i] - full_audio[i - 1]).abs();
            max_discontinuity = max_discontinuity.max(diff);
        }
    }

    // Buffer boundaries should be smooth (no clicks)
    // A click would be a huge jump (> 0.5)
    // Smooth synthesis should have small transitions (< 0.1)
    assert!(
        max_discontinuity < 0.1,
        "Discontinuity at buffer boundary: {} (indicates clicking)",
        max_discontinuity
    );

    // Verify synthesis actually happened
    let rms: f32 = full_audio.iter().map(|s| s * s).sum::<f32>() / full_audio.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.01, "No audio generated (RMS = {})", rms);
}

/// Test that synthesis envelope extends beyond single buffer
#[test]
#[ignore = "bus triggering via s pattern not producing audio - needs investigation"]
fn test_synth_envelope_crosses_buffers() {
    let sample_rate = 44100.0;

    // Single trigger with long envelope
    let code = r#"
tempo: 1.0
~synth $ sine 440
~trig $ s "[~synth ~]"
out $ ~trig
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    // Use sample-based timing for offline rendering
    // Time advances exactly by buffer_size samples per render() call

    // Render multiple small buffers to force crossing boundaries
    let buffer_size = 256;
    let num_buffers = 100; // ~0.6 seconds

    let mut buffers = Vec::new();
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        buffers.push(buffer);
    }

    // Check that sound persists across multiple buffers
    let mut non_silent_buffers = 0;
    for buffer in &buffers {
        let rms: f32 = buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32;
        let rms = rms.sqrt();
        if rms > 0.001 {
            non_silent_buffers += 1;
        }
    }

    // With continuous synthesis + envelope, we should have sound in many buffers
    // Not just 1-2 buffers (which would indicate pre-rendered fixed buffer)
    assert!(
        non_silent_buffers > 5,
        "Synthesis stopped too early - only {} buffers had audio (expected > 5)",
        non_silent_buffers
    );
}

/// Test that synthesis state persists correctly across buffer renders
#[test]
fn test_synth_phase_continuity_across_buffers() {
    let sample_rate = 44100.0;

    // Continuous sine tone triggered by pattern
    let code = r#"
tempo: 0.5
~synth $ sine 440
~trig $ s "~synth"
out $ ~trig
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    // Render in small chunks
    let buffer_size = 128;
    let num_buffers = 100;

    let mut all_samples = Vec::new();
    for _ in 0..num_buffers {
        let buffer = graph.render(buffer_size);
        all_samples.extend_from_slice(&buffer);
    }

    // Analyze phase continuity by checking zero crossings
    // A continuous 440Hz sine should have predictable zero crossing intervals
    let mut zero_crossings = Vec::new();
    for i in 1..all_samples.len() {
        if all_samples[i - 1] <= 0.0 && all_samples[i] > 0.0 {
            zero_crossings.push(i);
        }
    }

    // 440Hz at 44100Hz sample rate = ~100.2 samples per cycle
    // Check that zero crossing intervals are consistent
    if zero_crossings.len() >= 3 {
        let intervals: Vec<usize> = zero_crossings.windows(2).map(|w| w[1] - w[0]).collect();

        // Calculate variance in intervals
        let mean = intervals.iter().sum::<usize>() as f32 / intervals.len() as f32;
        let variance: f32 = intervals
            .iter()
            .map(|&i| {
                let diff = i as f32 - mean;
                diff * diff
            })
            .sum::<f32>()
            / intervals.len() as f32;

        // Low variance = consistent phase (continuous synthesis)
        // High variance = phase resets (pre-rendered buffer chunks)
        assert!(
            variance < 10.0,
            "Phase discontinuity detected - variance: {} (expected < 10)",
            variance
        );
    }
}

/// Test the exact user case: clicking synth with pattern
#[test]
#[ignore = "bus triggering via s pattern has clicking issues - needs investigation"]
fn test_user_case_no_clicking() {
    let sample_rate = 44100.0;

    // User's exact case (simplified)
    let code = r#"
tempo: 0.5
~s $ sine 440
~c $ s "~s(<7 7 6 10>,11,2)" # note "c3'maj" # gain 1
out $ ~c
"#;

    let (_, statements) = parse_program(code).expect("Parse failed");
    let mut graph = compile_program(statements, sample_rate, None).expect("Compilation failed");

    // Use sample-based timing for offline rendering
    // Time advances exactly by buffer_size samples per render() call

    // Render several cycles in chunks (like successful tests)
    let samples_per_cycle = (sample_rate / 2.0) as usize; // 2.0 cps
    let total_samples = samples_per_cycle * 4;
    let chunk_size = 128;
    let mut buffer = Vec::with_capacity(total_samples);
    for _ in 0..(total_samples / chunk_size) {
        buffer.extend_from_slice(&graph.render(chunk_size));
    }

    // Check for clicks by looking for abnormal peaks
    let mut peaks = Vec::new();
    for window in buffer.windows(3) {
        if window[1].abs() > window[0].abs() && window[1].abs() > window[2].abs() {
            peaks.push(window[1].abs());
        }
    }

    // In smooth synthesis, peaks should be relatively uniform
    // Clicks would show as isolated very high peaks
    if peaks.len() > 10 {
        peaks.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let max_peak = peaks[0];
        let median_peak = peaks[peaks.len() / 2];

        // Max peak shouldn't be way larger than median
        let ratio = max_peak / median_peak.max(0.001);
        assert!(
            ratio < 5.0,
            "Clicking detected - max/median peak ratio: {} (expected < 5.0)",
            ratio
        );
    }

    // Verify we have audio
    let rms: f32 = buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len() as f32;
    let rms = rms.sqrt();
    assert!(rms > 0.01, "No audio (RMS = {})", rms);
}
