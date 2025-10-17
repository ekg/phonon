//! Test if synths can be spawned polyphonically

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

mod audio_test_utils;
use audio_test_utils::{calculate_rms, find_dominant_frequency};

#[test]
fn test_saw_synth_produces_audio() {
    let input = "tempo: 1.0\nout: saw 110 * 0.3";

    let (_, statements) = parse_dsl(input).expect("Parse failed");
    let compiler = DslCompiler::new(44100.0);
    let mut graph = compiler.compile(statements);

    let buffer = graph.render(44100);
    let rms = calculate_rms(&buffer);
    let dominant_freq = find_dominant_frequency(&buffer, 44100.0);

    println!("\n=== Saw Synth Test ===");
    println!("RMS: {:.4}", rms);
    println!(
        "Dominant frequency: {:.1} Hz (expected ~110 Hz)",
        dominant_freq
    );

    if rms > 0.01 {
        println!("✅ Saw synth produces audio!");

        if (dominant_freq - 110.0).abs() < 10.0 {
            println!("✅ Frequency is correct");
        } else {
            println!("⚠️  Frequency is wrong");
        }
    } else {
        println!("❌ Saw synth is SILENT");
        println!("   Synths are continuous (not gated)");
        println!("   User is right: need pattern-triggered synths");
    }
}

#[test]
fn test_synth_with_note_pattern() {
    // What the user wants: pattern-triggered synth
    // Like Tidal: n "c a g e" # s "saw"

    println!("\n=== Pattern-Triggered Synth Test ===");
    println!("This is what the user wants:\n");
    println!("  tempo: 2.0");
    println!("  out: synth(\"c4 a3 g3 e4\", saw()) * 0.3\n");
    println!("Each note should:");
    println!("  1. Trigger a new synth voice");
    println!("  2. Set the pitch for that voice");
    println!("  3. Apply ADSR envelope (attack/decay)");
    println!("  4. Voice stops when envelope completes\n");
    println!("Current reality:");
    println!("  - saw 110 is continuous (plays forever)");
    println!("  - No pattern triggering for synths");
    println!("  - No polyphonic voice spawning");
    println!("  - No ADSR envelopes\n");
    println!("This is a FUNDAMENTAL MISSING FEATURE");
}

#[test]
fn test_samples_vs_synths_comparison() {
    println!("\n=== Samples vs Synths ===\n");

    // Samples work
    let input_sample = "tempo: 1.0\nout: s(\"bd sn bd sn\")";
    let (_, statements1) = parse_dsl(input_sample).expect("Parse failed");
    let compiler1 = DslCompiler::new(44100.0);
    let mut graph1 = compiler1.compile(statements1);
    let buffer_sample = graph1.render(88200);
    let rms_sample = calculate_rms(&buffer_sample);

    println!("Samples: s(\"bd sn bd sn\")");
    println!("  RMS: {:.4}", rms_sample);
    if rms_sample > 0.01 {
        println!("  ✅ Samples work - they trigger polyphonically");
    }

    // Synths are continuous
    let input_synth = "tempo: 1.0\nout: saw 110 * 0.3";
    let (_, statements2) = parse_dsl(input_synth).expect("Parse failed");
    let compiler2 = DslCompiler::new(44100.0);
    let mut graph2 = compiler2.compile(statements2);
    let buffer_synth = graph2.render(88200);
    let rms_synth = calculate_rms(&buffer_synth);

    println!("\nSynths: saw 110");
    println!("  RMS: {:.4}", rms_synth);
    if rms_synth > 0.01 {
        println!("  ⚠️  Synth is continuous (not pattern-triggered)");
    } else {
        println!("  ❌ Synth is silent");
    }

    println!("\nConclusion:");
    println!("  Samples: Pattern-triggered ✅ Polyphonic ✅");
    println!("  Synths: Continuous ⚠️  Not polyphonic ❌");
    println!("\nWhat's needed:");
    println!("  - Pattern-triggered synth voices");
    println!("  - Voice spawning (like samples)");
    println!("  - ADSR envelopes per voice");
    println!("  - Syntax: synth(\"c4 e4 g4\", saw())");
}
