//! Test timing continuity across code reloads
//!
//! CRITICAL REQUIREMENT: When reloading code (Ctrl-X in editor, file change in live mode),
//! the cycle position must continue smoothly without jumps or resets.
//!
//! This test verifies that time is immutable from the user's perspective.

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::UnifiedSignalGraph;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Test that cycle position advances continuously when swapping graphs
#[test]
fn test_cycle_position_continuity_on_graph_swap() {
    let sample_rate = 44100.0;
    let cps = 2.0; // 2 cycles per second

    // Parse and compile initial code
    let code_v1 = r#"
tempo: 2.0
out: s "bd sn hh cp"
"#;

    let (_, statements) = parse_program(code_v1).expect("Failed to parse v1");
    let mut graph_v1 = compile_program(statements, sample_rate).expect("Failed to compile v1");

    // Enable wall-clock timing (like live mode)
    graph_v1.enable_wall_clock_timing();

    // Process some audio to advance time
    let block_size = 512;
    let mut buffer = vec![0.0f32; block_size];

    // Process 1 second of audio (should advance by 2 cycles at cps=2)
    let blocks_per_second = (sample_rate as usize) / block_size;
    for _ in 0..blocks_per_second {
        graph_v1.process_buffer(&mut buffer);
    }

    // Record cycle position before swap
    let position_before_swap = graph_v1.get_cycle_position();
    println!("Cycle position before swap: {:.6}", position_before_swap);

    // Parse and compile new code (user made an edit) - happens INSTANTLY in real editor
    let code_v2 = r#"
tempo: 2.0
out: s "bd*2 sn hh*3 cp"
"#;

    let (_, statements) = parse_program(code_v2).expect("Failed to parse v2");
    let mut graph_v2 = compile_program(statements, sample_rate).expect("Failed to compile v2");

    // CRITICAL: Transfer timing state from old graph to new graph
    graph_v2.enable_wall_clock_timing();
    graph_v2.transfer_session_timing(&graph_v1);

    // Record cycle position after swap
    let position_after_swap = graph_v2.get_cycle_position();
    println!("Cycle position after swap: {:.6}", position_after_swap);

    // The cycle position should be THE SAME (or very close) because transfer happens instantly
    let actual_advance = position_after_swap - position_before_swap;

    println!("Cycle advance during swap: {:.6} cycles", actual_advance);

    // Verify timing continuity - position should NOT jump
    let max_jump = 0.01; // Allow tiny difference due to timing precision
    assert!(
        actual_advance.abs() < max_jump,
        "Timing jump detected during swap! Position changed by {:.6} cycles",
        actual_advance
    );

    // Verify we didn't reset to 0 - position should be whatever it was before swap
    assert!(
        position_after_swap == position_before_swap || (position_after_swap - position_before_swap).abs() < 0.001,
        "Cycle position changed during swap! Before: {:.6}, After: {:.6}",
        position_before_swap,
        position_after_swap
    );

    println!("✅ Timing preserved: cycle {:.6} -> {:.6} (no jump)", position_before_swap, position_after_swap);
}

/// Test that multiple rapid reloads don't cause timing drift
#[test]
fn test_no_timing_drift_on_rapid_reloads() {
    let sample_rate = 44100.0;
    let cps = 4.0; // Faster tempo to make drift more obvious

    let code = r#"
tempo: 4.0
out: s "bd sn"
"#;

    // Create initial graph
    let (_, statements) = parse_program(code).expect("Failed to parse");
    let mut graph = compile_program(statements, sample_rate).expect("Failed to compile");
    graph.enable_wall_clock_timing();

    // Process some audio to get started
    let mut buffer = vec![0.0f32; 512];
    for _ in 0..10 {
        graph.process_buffer(&mut buffer);
    }

    let start_position = graph.get_cycle_position();
    let start_time = Instant::now();

    // Simulate 10 rapid code reloads
    for i in 0..10 {
        // Process some audio before reloading (simulates audio callback running)
        for _ in 0..5 {
            graph.process_buffer(&mut buffer);
        }

        std::thread::sleep(Duration::from_millis(50)); // 50ms between reloads

        // Create new graph
        let (_, statements) = parse_program(code).expect("Failed to parse");
        let mut new_graph = compile_program(statements, sample_rate).expect("Failed to compile");
        new_graph.enable_wall_clock_timing();

        // Transfer timing
        new_graph.transfer_session_timing(&graph);

        // Verify continuity
        let old_pos = graph.get_cycle_position();
        let new_pos = new_graph.get_cycle_position();

        println!(
            "Reload {}: old_pos={:.6}, new_pos={:.6}, diff={:.6}",
            i,
            old_pos,
            new_pos,
            new_pos - old_pos
        );

        assert!(
            (new_pos - old_pos).abs() < 0.5,
            "Large timing jump detected on reload {}!",
            i
        );

        graph = new_graph;
    }

    let end_position = graph.get_cycle_position();
    let elapsed = start_time.elapsed().as_secs_f64();

    // Expected advance = elapsed_seconds * cps
    let expected_cycles = elapsed * cps as f64;
    let actual_cycles = end_position - start_position;

    println!("\nAfter 10 reloads:");
    println!("  Elapsed time: {:.3}s", elapsed);
    println!("  Expected cycles: {:.3}", expected_cycles);
    println!("  Actual cycles: {:.3}", actual_cycles);
    println!("  Drift: {:.6} cycles", actual_cycles - expected_cycles);

    // Allow 100ms total drift across all reloads
    let max_drift = 0.1 * cps as f64;
    assert!(
        (actual_cycles - expected_cycles).abs() < max_drift,
        "Accumulated timing drift too large: {:.3} cycles (max: {:.3})",
        actual_cycles - expected_cycles,
        max_drift
    );
}

/// Test that cycle position doesn't reset when changing tempo
#[test]
fn test_tempo_change_preserves_cycle_position() {
    let sample_rate = 44100.0;

    // Start at 2 cps
    let code_v1 = r#"
tempo: 2.0
out: s "bd sn"
"#;

    let (_, statements) = parse_program(code_v1).expect("Failed to parse v1");
    let mut graph_v1 = compile_program(statements, sample_rate).expect("Failed to compile v1");
    graph_v1.enable_wall_clock_timing();

    // Advance to cycle 5
    let mut buffer = vec![0.0f32; 512];
    while graph_v1.get_cycle_position() < 5.0 {
        graph_v1.process_buffer(&mut buffer);
    }

    let position_at_tempo_change = graph_v1.get_cycle_position();
    println!(
        "Cycle position before tempo change: {:.6}",
        position_at_tempo_change
    );

    std::thread::sleep(Duration::from_millis(50));

    // Change tempo to 4 cps
    let code_v2 = r#"
tempo: 4.0
out: s "bd sn"
"#;

    let (_, statements) = parse_program(code_v2).expect("Failed to parse v2");
    let mut graph_v2 = compile_program(statements, sample_rate).expect("Failed to compile v2");
    graph_v2.enable_wall_clock_timing();
    graph_v2.transfer_session_timing(&graph_v1);

    let position_after_tempo_change = graph_v2.get_cycle_position();
    println!(
        "Cycle position after tempo change: {:.6}",
        position_after_tempo_change
    );

    // Position should be approximately the same (plus the 50ms that elapsed)
    let expected_advance = 0.05 * 2.0; // 50ms at old tempo (2 cps)
    let actual_advance = position_after_tempo_change - position_at_tempo_change;

    println!("Expected advance: ~{:.3} cycles", expected_advance);
    println!("Actual advance: {:.3} cycles", actual_advance);

    // The cycle NUMBER should be preserved, even if tempo changes
    assert!(
        position_after_tempo_change >= position_at_tempo_change,
        "Cycle position went backwards! {} -> {}",
        position_at_tempo_change,
        position_after_tempo_change
    );

    // Should still be around cycle 5, not reset to 0
    assert!(
        position_after_tempo_change > 4.5,
        "Cycle position reset instead of continuing! Got: {}",
        position_after_tempo_change
    );
}

/// Test concurrent audio processing and graph swapping (simulates live mode)
#[test]
fn test_concurrent_processing_and_swap() {
    let sample_rate = 44100.0;

    let code_v1 = r#"
tempo: 2.0
out: s "bd sn hh cp"
"#;

    let (_, statements) = parse_program(code_v1).expect("Failed to parse");
    let graph = compile_program(statements, sample_rate).expect("Failed to compile");

    // Wrap in Arc for thread-safe sharing
    let graph_arc = Arc::new(Mutex::new(graph));
    let graph_clone = graph_arc.clone();

    // Simulate audio thread processing
    let audio_thread = std::thread::spawn(move || {
        let mut positions = Vec::new();
        let mut buffer = vec![0.0f32; 512];

        for i in 0..100 {
            {
                let mut g = graph_clone.lock().unwrap();
                if i == 0 {
                    g.enable_wall_clock_timing();
                }
                g.process_buffer(&mut buffer);
                positions.push(g.get_cycle_position());
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        positions
    });

    // Simulate main thread swapping graph (like Ctrl-X reload)
    std::thread::sleep(Duration::from_millis(250)); // Let audio start

    let code_v2 = r#"
tempo: 2.0
out: s "bd*2 sn*2 hh*2 cp*2"
"#;

    let (_, statements) = parse_program(code_v2).expect("Failed to parse v2");
    let mut new_graph = compile_program(statements, sample_rate).expect("Failed to compile v2");

    // Get old graph state
    let old_graph = {
        let g = graph_arc.lock().unwrap();
        g.get_cycle_position()
    };

    // Transfer and swap
    {
        let old = graph_arc.lock().unwrap();
        new_graph.enable_wall_clock_timing();
        new_graph.transfer_session_timing(&*old);
    }

    let new_pos = new_graph.get_cycle_position();
    println!(
        "Graph swap: old={:.3}, new={:.3}, diff={:.6}",
        old_graph,
        new_pos,
        new_pos - old_graph
    );

    *graph_arc.lock().unwrap() = new_graph;

    // Wait for audio thread to finish
    let positions = audio_thread.join().expect("Audio thread panicked");

    // Verify positions always increase (no resets)
    for i in 1..positions.len() {
        assert!(
            positions[i] >= positions[i - 1],
            "Cycle position went backwards at sample {}: {:.6} -> {:.6}",
            i,
            positions[i - 1],
            positions[i]
        );
    }

    println!("✅ All {} position samples continuously increased", positions.len());
}
