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
        out $ sine 220 * 0.3
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
        out $ sine 220 * 0.3
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
        out $ s "bd ~ bd ~"
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
        out $ s "bd ~ bd ~"
        o2: s "~ sn ~ sn"
        o3: s "hh hh hh hh"
    "#;

    let duration = 176400; // 4 seconds at 44100 Hz (8 cycles at 0.5 CPS)

    // This test asserts LINEAR channel summation: (o1+o2+o3 together) == sum of
    // each rendered alone. The output stage's brick-wall master limiter (0.95
    // ceiling) is a deliberate GLOBAL safety nonlinearity that clamps the combined
    // mix when the summed peak exceeds the ceiling (here bd+sn+hh peaks at ~1.06),
    // which by design makes the together-mix != linear sum at exactly the clamp
    // points. That is the limiter working, NOT a channel-isolation bug. Disable it
    // on every render so the test exercises the channel-mix path linearly; a real
    // sample-switch bug would still surface as mismatches away from any clamp.
    let compile_linear = |code: &str| {
        let (_, statements) = parse_dsl(code).unwrap();
        let mut graph = DslCompiler::new(44100.0).compile(statements);
        graph.set_master_limiter_ceiling(1.0); // disable master limiter
        graph
    };

    // Render individual channels
    let mut graph = compile_linear(o1_code);
    let o1_alone = graph.render(duration);
    let o1_rms = calculate_rms(&o1_alone);
    let o1_peak = find_peak(&o1_alone);
    println!("o1 alone (bd): RMS = {:.6}, Peak = {:.6}", o1_rms, o1_peak);

    let mut graph = compile_linear(o2_code);
    let o2_alone = graph.render(duration);
    let o2_rms = calculate_rms(&o2_alone);
    let o2_peak = find_peak(&o2_alone);
    println!("o2 alone (sn): RMS = {:.6}, Peak = {:.6}", o2_rms, o2_peak);

    let mut graph = compile_linear(o3_code);
    let o3_alone = graph.render(duration);
    let o3_rms = calculate_rms(&o3_alone);
    let o3_peak = find_peak(&o3_alone);
    println!("o3 alone (hh): RMS = {:.6}, Peak = {:.6}", o3_rms, o3_peak);

    // Render all together
    let mut graph = compile_linear(all_code);
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
    // Regression for the user report that `808bd` seemed to switch to a different
    // sample (a "tom") when rendered alongside other channels. We verify channel
    // isolation DIRECTLY: o2's exact contribution to the full mix must equal o2
    // rendered alone, sample-for-sample. If the sample switched, the recovered
    // contribution would diverge sharply from the isolated render.
    println!("\n=== Testing user-reported bug (808bd switching to tom?) ===");

    // NOTE on the `out` channel syntax: the original repro used
    //   out $ struct "t(3,8,1)" $ sine "66" # env 0.01 1 1 0.2
    // but `parse_dsl` (the unified_graph_parser front-end this test exercises) does
    // NOT support chaining a source into `struct` via a second `$` — it parses
    // `out $ struct "t(3,8,1)"` and silently DROPS the unparsed tail, which here also
    // swallowed the `o2:`/`o3:` statements, leaving a struct-with-no-source (total
    // silence) and making the whole comparison meaningless. That parser gap is tracked
    // separately; the 808bd-consistency behaviour under test is independent of what
    // the OTHER channel plays, so we use a plain synth on `out` that parse_dsl accepts.
    let o2_code = r#"
        tempo: 0.5
        o2: s "808bd(3,8)"
    "#;

    // Full mix: a synth on `out` plus the 808bd (o2) and hats (o3).
    let all_code = r#"
        tempo: 0.5
        out $ sine 66 * 0.3
        o2: s "808bd(3,8)"
        o3: s "hh hh hh hh" * 0.3
    "#;

    // Same mix WITHOUT o2, so (all - without_o2) recovers o2's exact contribution.
    let without_o2_code = r#"
        tempo: 0.5
        out $ sine 66 * 0.3
        o3: s "hh hh hh hh" * 0.3
    "#;

    let duration = 176400; // 4 seconds

    // Disable the master limiter (global 0.95 brick-wall safety limiter) so the output
    // stage is LINEAR and the subtraction below recovers o2 exactly. The default output
    // mix mode is an unscaled sum, so all == out + o2 + o3 and without_o2 == out + o3.
    let compile_linear = |code: &str| {
        let (_, statements) = parse_dsl(code).unwrap();
        let mut graph = DslCompiler::new(44100.0).compile(statements);
        graph.set_master_limiter_ceiling(1.0);
        graph
    };

    let o2_alone = compile_linear(o2_code).render(duration);
    let all_together = compile_linear(all_code).render(duration);
    let without_o2 = compile_linear(without_o2_code).render(duration);

    // Recover o2's contribution to the mix by subtracting the o2-free mix.
    let o2_in_mix: Vec<f32> = all_together
        .iter()
        .zip(without_o2.iter())
        .map(|(a, b)| a - b)
        .collect();

    // Tolerance 1e-3 absorbs f32 rounding from the extra mix additions (magnitudes are
    // ~1.0, so rounding is ~1e-7); a real sample switch would differ by ~0.5.
    let (matches, max_diff, mismatch_count) = buffers_match(&o2_alone, &o2_in_mix, 1e-3);

    println!(
        "o2 alone peak = {:.6}, recovered-in-mix peak = {:.6}",
        find_peak(&o2_alone),
        find_peak(&o2_in_mix)
    );
    println!("  Max difference: {:.10}", max_diff);
    println!("  Mismatches: {} / {}", mismatch_count, duration);

    assert!(
        matches,
        "CRITICAL BUG: The 808bd sample changed when rendered with other channels!\n\
         o2's recovered contribution to the mix differs from o2 rendered alone.\n\
         Max difference: {:.10}, Mismatches: {} / {}",
        max_diff, mismatch_count, duration
    );

    println!("✅ 808bd sample is consistent across renders (channel-isolated)");
}
