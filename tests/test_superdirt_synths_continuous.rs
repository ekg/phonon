//! Test SuperDirt synths - are they continuous or gated?

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

mod audio_test_utils;
use audio_test_utils::calculate_rms;

#[test]
fn test_superkick_is_continuous_or_gated() {
    let input = r#"
        tempo 1.0
        out superkick(60, 0.5, 0.15, 0.2) * 0.6
    "#;

    let (_, statements) = parse_dsl(input).expect("Parse failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 4 seconds
    let buffer = graph.render(176400);

    // Analyze segments to see if it's continuous or gated
    let segment1 = &buffer[0..22050]; // 0.0-0.5s
    let segment2 = &buffer[22050..44100]; // 0.5-1.0s
    let segment3 = &buffer[88200..110250]; // 2.0-2.5s
    let segment4 = &buffer[132300..154350]; // 3.0-3.5s

    let rms1 = calculate_rms(segment1);
    let rms2 = calculate_rms(segment2);
    let rms3 = calculate_rms(segment3);
    let rms4 = calculate_rms(segment4);

    println!("\n=== Superkick Continuity Test ===");
    println!("Segment 1 (0.0-0.5s) RMS: {:.4}", rms1);
    println!("Segment 2 (0.5-1.0s) RMS: {:.4}", rms2);
    println!("Segment 3 (2.0-2.5s) RMS: {:.4}", rms3);
    println!("Segment 4 (3.0-3.5s) RMS: {:.4}", rms4);

    // Check if all segments have similar energy (continuous) or if it decays (gated)
    let avg_rms = (rms1 + rms2 + rms3 + rms4) / 4.0;
    let variance = (rms1 - avg_rms)
        .abs()
        .max((rms2 - avg_rms).abs())
        .max((rms3 - avg_rms).abs())
        .max((rms4 - avg_rms).abs());

    println!("\nAverage RMS: {:.4}", avg_rms);
    println!("Max variance from average: {:.4}", variance);

    // Check first 100ms vs last 100ms
    let first_100ms = &buffer[0..4410];
    let last_100ms = &buffer[172000..176400];
    let rms_first = calculate_rms(first_100ms);
    let rms_last = calculate_rms(last_100ms);

    println!("\nFirst 100ms RMS: {:.4}", rms_first);
    println!("Last 100ms RMS: {:.4}", rms_last);

    if rms_last > rms_first * 0.5 {
        println!(
            "\n⚠️  CONTINUOUS: Sound persists throughout (last RMS is {:.1}% of first)",
            (rms_last / rms_first * 100.0)
        );
        println!("   User is correct - superkick is NOT gated, it's continuous!");
    } else {
        println!(
            "\n✅ GATED: Sound decays (last RMS is {:.1}% of first)",
            (rms_last / rms_first * 100.0)
        );
    }
}

#[test]
fn test_supersaw_is_continuous_or_gated() {
    let input = r#"
        tempo 1.0
        out supersaw(110, 0.5, 5) * 0.3
    "#;

    let (_, statements) = parse_dsl(input).expect("Parse failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    // Render 4 seconds
    let buffer = graph.render(176400);

    let segment1 = &buffer[0..44100]; // First second
    let segment2 = &buffer[44100..88200]; // Second second
    let segment3 = &buffer[88200..132300]; // Third second
    let segment4 = &buffer[132300..176400]; // Fourth second

    let rms1 = calculate_rms(segment1);
    let rms2 = calculate_rms(segment2);
    let rms3 = calculate_rms(segment3);
    let rms4 = calculate_rms(segment4);

    println!("\n=== Supersaw Continuity Test ===");
    println!("Second 1 RMS: {:.4}", rms1);
    println!("Second 2 RMS: {:.4}", rms2);
    println!("Second 3 RMS: {:.4}", rms3);
    println!("Second 4 RMS: {:.4}", rms4);

    let avg_rms = (rms1 + rms2 + rms3 + rms4) / 4.0;
    let max_rms = rms1.max(rms2).max(rms3).max(rms4);
    let min_rms = rms1.min(rms2).min(rms3).min(rms4);

    println!("\nAverage RMS: {:.4}", avg_rms);
    println!("Max RMS: {:.4}", max_rms);
    println!("Min RMS: {:.4}", min_rms);
    println!(
        "RMS variance: {:.4} ({:.1}%)",
        max_rms - min_rms,
        ((max_rms - min_rms) / avg_rms * 100.0)
    );

    if (max_rms - min_rms) < avg_rms * 0.1 {
        println!("\n⚠️  CONTINUOUS: RMS is nearly constant across all seconds");
        println!("   User is correct - supersaw is NOT gated, it's continuous!");
    } else {
        println!("\n✅ Has variation - may have envelope behavior");
    }
}

#[test]
fn test_superkick_with_samples_comparison() {
    // Compare superkick vs sample-based kick
    let input_superkick = r#"
        tempo 1.0
        out superkick(60, 0.5, 0.15, 0.2) * 0.6
    "#;

    let input_sample = r#"
        tempo 1.0
        out s "bd"
    "#;

    let (_, statements1) = parse_dsl(input_superkick).expect("Parse failed");
    let compiler1 = DslCompiler::new(44100.0);
    let mut graph1 = compiler1.compile(statements1);
    let buffer_superkick = graph1.render(88200); // 2 seconds

    let (_, statements2) = parse_dsl(input_sample).expect("Parse failed");
    let compiler2 = DslCompiler::new(44100.0);
    let mut graph2 = compiler2.compile(statements2);
    let buffer_sample = graph2.render(88200); // 2 seconds

    println!("\n=== Superkick vs Sample Comparison ===");

    // Analyze decay behavior
    let segments_superkick: Vec<f32> = (0..8)
        .map(|i| {
            let start = i * 11025;
            let end = start + 11025;
            calculate_rms(&buffer_superkick[start..end])
        })
        .collect();

    let segments_sample: Vec<f32> = (0..8)
        .map(|i| {
            let start = i * 11025;
            let end = start + 11025;
            calculate_rms(&buffer_sample[start..end])
        })
        .collect();

    println!("\nSuperkick RMS over time (250ms segments):");
    for (i, rms) in segments_superkick.iter().enumerate() {
        println!("  Segment {}: {:.4}", i, rms);
    }

    println!("\nSample 'bd' RMS over time (250ms segments):");
    for (i, rms) in segments_sample.iter().enumerate() {
        println!("  Segment {}: {:.4}", i, rms);
    }

    // A proper kick should have strong attack, then decay to near-silence
    let sample_decays = segments_sample[0] > segments_sample[7] * 2.0;
    let superkick_decays = segments_superkick[0] > segments_superkick[7] * 2.0;

    println!("\nDecay analysis:");
    println!(
        "  Sample decay ratio: {:.2}x",
        segments_sample[0] / segments_sample[7].max(0.0001)
    );
    println!(
        "  Superkick decay ratio: {:.2}x",
        segments_superkick[0] / segments_superkick[7].max(0.0001)
    );

    if sample_decays {
        println!("  ✅ Sample 'bd' decays properly (expected)");
    } else {
        println!("  ⚠️  Sample 'bd' doesn't decay");
    }

    if superkick_decays {
        println!("  ✅ Superkick decays");
    } else {
        println!("  ⚠️  Superkick is continuous (user is correct!)");
    }
}
