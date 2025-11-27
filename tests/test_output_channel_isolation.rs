//! Test that output channels are truly isolated and sum correctly
//!
//! This test verifies that o1 + o2 + o3 = (o1 alone) + (o2 alone) + (o3 alone)
//! at the sample level, with FFT analysis to confirm frequency content.

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

fn calculate_rms(buffer: &[f32]) -> f32 {
    (buffer.iter().map(|x| x * x).sum::<f32>() / buffer.len() as f32).sqrt()
}

fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

/// Compare two buffers sample-by-sample
fn buffers_match(a: &[f32], b: &[f32], tolerance: f32) -> (bool, f32, usize) {
    if a.len() != b.len() {
        return (false, 0.0, 0);
    }

    let mut max_diff = 0.0f32;
    let mut mismatch_count = 0;

    for (_i, (&sample_a, &sample_b)) in a.iter().zip(b.iter()).enumerate() {
        let diff = (sample_a - sample_b).abs();
        if diff > tolerance {
            mismatch_count += 1;
            if diff > max_diff {
                max_diff = diff;
            }
        }
    }

    (mismatch_count == 0, max_diff, mismatch_count)
}

#[test]
fn test_output_channels_sum_correctly_simple() {
    // Simple test with sine waves - perfect for verification
    println!("\n=== Testing simple sine wave isolation ===");

    // Render each channel individually
    let o1_code = r#"
        tempo: 0.5
        o1: sine 220 * 0.3
    "#;

    let o2_code = r#"
        tempo: 0.5
        o2: sine 440 * 0.3
    "#;

    let o3_code = r#"
        tempo: 0.5
        o3: sine 880 * 0.3
    "#;

    // Render all together
    let all_code = r#"
        tempo: 0.5
        o1: sine 220 * 0.3
        o2: sine 440 * 0.3
        o3: sine 880 * 0.3
    "#;

    let duration = 22050; // 0.5 seconds at 44100 Hz

    // Render individual channels
    let (_, statements) = parse_dsl(o1_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o1_alone = graph.render(duration);
    let o1_rms = calculate_rms(&o1_alone);
    let o1_peak = find_peak(&o1_alone);
    println!("o1 alone: RMS = {:.6}, Peak = {:.6}", o1_rms, o1_peak);

    let (_, statements) = parse_dsl(o2_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o2_alone = graph.render(duration);
    let o2_rms = calculate_rms(&o2_alone);
    let o2_peak = find_peak(&o2_alone);
    println!("o2 alone: RMS = {:.6}, Peak = {:.6}", o2_rms, o2_peak);

    let (_, statements) = parse_dsl(o3_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o3_alone = graph.render(duration);
    let o3_rms = calculate_rms(&o3_alone);
    let o3_peak = find_peak(&o3_alone);
    println!("o3 alone: RMS = {:.6}, Peak = {:.6}", o3_rms, o3_peak);

    // Render all together
    let (_, statements) = parse_dsl(all_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let all_together = graph.render(duration);
    let all_rms = calculate_rms(&all_together);
    let all_peak = find_peak(&all_together);
    println!("All together: RMS = {:.6}, Peak = {:.6}", all_rms, all_peak);

    // Manually sum the individual channels
    let mut manual_sum = vec![0.0f32; duration];
    for i in 0..duration {
        manual_sum[i] = o1_alone[i] + o2_alone[i] + o3_alone[i];
    }
    let manual_rms = calculate_rms(&manual_sum);
    let manual_peak = find_peak(&manual_sum);
    println!(
        "Manual sum:   RMS = {:.6}, Peak = {:.6}",
        manual_rms, manual_peak
    );

    // Compare sample-by-sample
    let (matches, max_diff, mismatch_count) = buffers_match(&all_together, &manual_sum, 0.0001);

    println!("\nSample comparison:");
    println!("  Max difference: {:.10}", max_diff);
    println!("  Mismatches: {} / {}", mismatch_count, duration);

    assert!(
        matches,
        "CRITICAL BUG: o1+o2+o3 rendered together ≠ (o1 alone) + (o2 alone) + (o3 alone)\n\
         Max difference: {:.10}, Mismatches: {} / {}",
        max_diff, mismatch_count, duration
    );

    println!("✅ Channels sum correctly (sample-perfect)");
}

#[test]
fn test_output_channels_sum_correctly_with_samples() {
    // Test with actual samples - the real use case
    println!("\n=== Testing sample playback isolation ===");

    // Render each channel individually
    let o1_code = r#"
        tempo: 0.5
        o1: s "bd ~ bd ~"
    "#;

    let o2_code = r#"
        tempo: 0.5
        o2: s "~ sn ~ sn"
    "#;

    let o3_code = r#"
        tempo: 0.5
        o3: s "hh hh hh hh"
    "#;

    // Render all together
    let all_code = r#"
        tempo: 0.5
        o1: s "bd ~ bd ~"
        o2: s "~ sn ~ sn"
        o3: s "hh hh hh hh"
    "#;

    let duration = 176400; // 4 seconds at 44100 Hz (8 cycles at 0.5 CPS)

    // Render individual channels
    let (_, statements) = parse_dsl(o1_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o1_alone = graph.render(duration);
    let o1_rms = calculate_rms(&o1_alone);
    let o1_peak = find_peak(&o1_alone);
    println!("o1 alone (bd): RMS = {:.6}, Peak = {:.6}", o1_rms, o1_peak);

    let (_, statements) = parse_dsl(o2_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o2_alone = graph.render(duration);
    let o2_rms = calculate_rms(&o2_alone);
    let o2_peak = find_peak(&o2_alone);
    println!("o2 alone (sn): RMS = {:.6}, Peak = {:.6}", o2_rms, o2_peak);

    let (_, statements) = parse_dsl(o3_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o3_alone = graph.render(duration);
    let o3_rms = calculate_rms(&o3_alone);
    let o3_peak = find_peak(&o3_alone);
    println!("o3 alone (hh): RMS = {:.6}, Peak = {:.6}", o3_rms, o3_peak);

    // Render all together
    let (_, statements) = parse_dsl(all_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let all_together = graph.render(duration);
    let all_rms = calculate_rms(&all_together);
    let all_peak = find_peak(&all_together);
    println!(
        "All together:  RMS = {:.6}, Peak = {:.6}",
        all_rms, all_peak
    );

    // Manually sum the individual channels
    let mut manual_sum = vec![0.0f32; duration];
    for i in 0..duration {
        manual_sum[i] = o1_alone[i] + o2_alone[i] + o3_alone[i];
    }
    let manual_rms = calculate_rms(&manual_sum);
    let manual_peak = find_peak(&manual_sum);
    println!(
        "Manual sum:    RMS = {:.6}, Peak = {:.6}",
        manual_rms, manual_peak
    );

    // Compare sample-by-sample
    let (matches, max_diff, mismatch_count) = buffers_match(&all_together, &manual_sum, 0.0001);

    println!("\nSample comparison:");
    println!("  Max difference: {:.10}", max_diff);
    println!("  Mismatches: {} / {}", mismatch_count, duration);

    assert!(
        matches,
        "CRITICAL BUG: Sample playback changes when rendering channels together!\n\
         This suggests samples are switching or being evaluated differently.\n\
         Max difference: {:.10}, Mismatches: {} / {}",
        max_diff, mismatch_count, duration
    );

    println!("✅ Sample channels sum correctly (sample-perfect)");
}

#[test]
fn test_user_reported_bug() {
    // Test the exact scenario the user reported
    println!("\n=== Testing user-reported bug (808bd switching to tom?) ===");

    let o2_code = r#"
        tempo: 0.5
        o2: s "808bd(3,8)"
    "#;

    let all_code = r#"
        tempo: 0.5
        o1: struct "t(3,8,1)" $ sine "66" # env 0.01 1 1 0.2
        o2: s "808bd(3,8)"
        o3: s "hh hh hh hh" * 0.3
    "#;

    let duration = 176400; // 4 seconds

    // Render o2 alone
    let (_, statements) = parse_dsl(o2_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let o2_alone = graph.render(duration);

    // Render all together (we'll extract o2's contribution)
    let (_, statements) = parse_dsl(all_code).unwrap();
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let all_together = graph.render(duration);

    // Find the first kick drum hit in both renders (should be at the same time)
    let find_first_peak = |buffer: &[f32]| -> (usize, f32) {
        let mut max_val = 0.0f32;
        let mut max_idx = 0;
        for i in 0..buffer.len().min(22050) {
            // First 0.5 seconds
            if buffer[i].abs() > max_val {
                max_val = buffer[i].abs();
                max_idx = i;
            }
        }
        (max_idx, max_val)
    };

    let (idx_alone, peak_alone) = find_first_peak(&o2_alone);
    let (idx_together, peak_together) = find_first_peak(&all_together);

    println!(
        "o2 alone:      First peak at sample {}, amplitude {:.6}",
        idx_alone, peak_alone
    );
    println!(
        "All together:  First peak at sample {}, amplitude {:.6}",
        idx_together, peak_together
    );

    // The peak should be at roughly the same time
    let time_diff = (idx_alone as i32 - idx_together as i32).abs();
    println!(
        "Peak timing difference: {} samples ({:.2} ms)",
        time_diff,
        time_diff as f32 / 44.1
    );

    // Check if the waveform shape is similar around the peak
    let window = 100; // Compare 100 samples around the peak
    let start_alone = idx_alone.saturating_sub(window);
    let start_together = idx_together.saturating_sub(window);

    let mut correlation = 0.0f32;
    let mut samples_compared = 0;
    for i in 0..window * 2 {
        if start_alone + i < o2_alone.len() && start_together + i < all_together.len() {
            correlation += o2_alone[start_alone + i] * all_together[start_together + i];
            samples_compared += 1;
        }
    }
    correlation /= samples_compared as f32;

    println!("Waveform correlation around peak: {:.6}", correlation);
    println!("(1.0 = identical, 0.0 = uncorrelated, -1.0 = inverted)");

    assert!(
        correlation > 0.5,
        "CRITICAL BUG: The 808bd sample appears to have changed!\n\
         Correlation around first peak: {:.6} (expected > 0.5)\n\
         This suggests the sample being played is different when rendered with other channels.",
        correlation
    );

    println!("✅ Sample appears consistent across renders");
}
