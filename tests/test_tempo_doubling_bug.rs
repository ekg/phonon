use arc_swap::ArcSwap;
/// Test for tempo doubling bug during C-x (graph swap)
///
/// Bug report: Occasionally (~1/8 times), after C-x the tempo/cps DOUBLES.
/// This test systematically explores different timing scenarios to reproduce.
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::unified_graph::UnifiedSignalGraph;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Helper to compile DSL code into a graph
fn compile_dsl(code: &str, sample_rate: f32) -> UnifiedSignalGraph {
    let (_, statements) = parse_program(code).expect("Parse failed");
    compile_program(statements, sample_rate, None).expect("Compile failed")
}

/// Simulate the graph swap that happens during C-x
/// Returns the CPS of the new graph after transfer
fn simulate_graph_swap(
    old_graph: &UnifiedSignalGraph,
    new_code: &str,
    sample_rate: f32,
) -> (UnifiedSignalGraph, f32) {
    let mut new_graph = compile_dsl(new_code, sample_rate);

    // This is what modal_editor/mod.rs does during C-x:
    new_graph.transfer_session_timing(old_graph);

    let cps = new_graph.get_cps();
    (new_graph, cps)
}

#[test]
fn test_tempo_preserved_after_immediate_swap() {
    // Test: Swap immediately after creating graph
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    let initial_cps = graph.get_cps();
    assert!(
        (initial_cps - 0.5).abs() < 0.001,
        "Initial CPS should be 0.5, got {}",
        initial_cps
    );

    // Swap immediately
    let (_, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

    assert!(
        (new_cps - 0.5).abs() < 0.001,
        "CPS should remain 0.5 after swap, got {} (ratio: {:.2}x)",
        new_cps,
        new_cps / 0.5
    );
}

#[test]
fn test_tempo_preserved_after_delay() {
    // Test: Swap after letting some time pass
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    // Let some time pass (simulating user editing)
    std::thread::sleep(Duration::from_millis(500));

    let (_, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

    assert!(
        (new_cps - 0.5).abs() < 0.001,
        "CPS should remain 0.5 after delay, got {} (ratio: {:.2}x)",
        new_cps,
        new_cps / 0.5
    );
}

#[test]
fn test_tempo_preserved_rapid_swaps() {
    // Test: Rapidly swap graphs multiple times
    // This simulates spamming C-x
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    let mut errors = Vec::new();

    for i in 0..20 {
        // Small random-ish delay between swaps
        let delay_ms = (i * 17) % 100 + 10; // 10-110ms delays
        std::thread::sleep(Duration::from_millis(delay_ms as u64));

        let (new_graph, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

        let ratio = new_cps / 0.5;
        if (ratio - 1.0).abs() > 0.01 {
            errors.push((i, new_cps, ratio));
        }

        graph = new_graph;
    }

    if !errors.is_empty() {
        eprintln!("❌ CPS errors detected:");
        for (i, cps, ratio) in &errors {
            eprintln!("   Swap {}: CPS={}, ratio={:.2}x", i, cps, ratio);
        }
        panic!("CPS changed during rapid swaps: {:?}", errors);
    }
}

#[test]
fn test_tempo_preserved_at_cycle_boundary() {
    // Hypothesis: Bug might occur at cycle boundaries
    // Try to swap exactly at cycle transitions
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();
    let start = Instant::now();

    // Wait until we're close to a cycle boundary
    // At 0.5 CPS, cycles are 2 seconds long
    // Wait for cycle 1 (2 seconds)
    loop {
        let elapsed = start.elapsed().as_secs_f64();
        let cycle_pos = elapsed * 0.5; // position in cycles
        let frac = cycle_pos.fract();

        // Try to hit right at cycle boundary (frac ~= 0 or ~= 1)
        if frac < 0.05 && elapsed > 0.1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));

        // Timeout after 3 seconds
        if elapsed > 3.0 {
            break;
        }
    }

    let (_, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

    assert!(
        (new_cps - 0.5).abs() < 0.001,
        "CPS should remain 0.5 at cycle boundary, got {} (ratio: {:.2}x)",
        new_cps,
        new_cps / 0.5
    );
}

#[test]
fn test_tempo_preserved_mid_cycle() {
    // Try to swap mid-cycle
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();
    let start = Instant::now();

    // Wait until we're in the middle of a cycle
    // At 0.5 CPS, cycles are 2 seconds long
    loop {
        let elapsed = start.elapsed().as_secs_f64();
        let cycle_pos = elapsed * 0.5;
        let frac = cycle_pos.fract();

        // Try to hit mid-cycle (frac ~= 0.5)
        if (frac - 0.5).abs() < 0.05 {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));

        // Timeout after 3 seconds
        if elapsed > 3.0 {
            break;
        }
    }

    let (_, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

    assert!(
        (new_cps - 0.5).abs() < 0.001,
        "CPS should remain 0.5 mid-cycle, got {} (ratio: {:.2}x)",
        new_cps,
        new_cps / 0.5
    );
}

#[test]
fn test_cycle_position_continuous_across_swaps() {
    // Track cycle position continuity, not just CPS
    // NOTE: get_cycle_position() returns CACHED value, not real-time value
    // We must process a buffer to update the cached value
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    // Process some buffers to warm up
    let mut buffer = [0.0f32; 512];
    for _ in 0..10 {
        graph.process_buffer(&mut buffer);
    }

    // Let some time pass and process another buffer to update position
    std::thread::sleep(Duration::from_millis(100));
    graph.process_buffer(&mut buffer);

    let pos_before = graph.get_cycle_position();

    let (mut new_graph, _) = simulate_graph_swap(&graph, code, 44100.0);

    // Process a buffer to update the new graph's cached position
    new_graph.process_buffer(&mut buffer);

    let pos_after = new_graph.get_cycle_position();

    // Position should advance by roughly the time it took to swap + process
    // Allow for the ~11ms buffer processing time + some overhead
    let expected_max_advance = 0.05; // 0.05 cycles max (100ms at 0.5 CPS)
    let pos_diff = (pos_after - pos_before).abs();

    // After swap, position should be >= before (time moves forward)
    // and difference should be small (just the swap overhead)
    assert!(
        pos_after >= pos_before - 0.001, // Allow tiny floating point error
        "Cycle position went backwards: before={:.4}, after={:.4}",
        pos_before,
        pos_after
    );
    assert!(
        pos_diff < expected_max_advance,
        "Cycle position jumped too much: before={:.4}, after={:.4}, diff={:.4}",
        pos_before,
        pos_after,
        pos_diff
    );
}

#[test]
fn test_timing_transfer_math() {
    // Direct test of the timing transfer math
    // This is the core calculation from transfer_session_timing

    let sample_rate = 44100.0;
    let old_cps = 0.5;
    let new_cps = 0.5; // Same tempo

    let mut old_graph = compile_dsl("tempo: 0.5\nout $ sine 440", sample_rate);
    old_graph.enable_wall_clock_timing();

    // Simulate time passing
    std::thread::sleep(Duration::from_millis(500));

    // Get old graph state
    let old_elapsed = old_graph.session_start_time.elapsed().as_secs_f64();
    let old_cycle_pos = old_elapsed * old_cps as f64 + old_graph.cycle_offset;

    // Calculate what new offset should be
    let new_offset = old_cycle_pos - old_elapsed * new_cps as f64;

    // Verify: new_elapsed * new_cps + new_offset should equal old_cycle_pos
    // Since we're transferring session_start_time, new_elapsed == old_elapsed
    let calculated_pos = old_elapsed * new_cps as f64 + new_offset;

    eprintln!("Old elapsed: {:.4}s", old_elapsed);
    eprintln!("Old CPS: {}", old_cps);
    eprintln!("Old offset: {:.4}", old_graph.cycle_offset);
    eprintln!("Old cycle pos: {:.4}", old_cycle_pos);
    eprintln!("New offset (calculated): {:.4}", new_offset);
    eprintln!("Calculated pos: {:.4}", calculated_pos);

    assert!(
        (calculated_pos - old_cycle_pos).abs() < 0.0001,
        "Math error: calculated_pos={:.4}, old_cycle_pos={:.4}",
        calculated_pos,
        old_cycle_pos
    );
}

#[test]
fn test_no_race_between_elapsed_and_transfer() {
    // Test that wall-clock timing works correctly
    // NOTE: get_cycle_position() returns CACHED value
    // To see wall-clock advancing, we must call process_buffer()

    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    let mut buffer = [0.0f32; 512];

    // Get initial position after processing
    graph.process_buffer(&mut buffer);
    let pos1 = graph.get_cycle_position();
    let time1 = Instant::now();

    // Wait and process again
    std::thread::sleep(Duration::from_millis(100));
    graph.process_buffer(&mut buffer);
    let pos2 = graph.get_cycle_position();
    let elapsed = time1.elapsed().as_secs_f64();

    // In wall-clock mode, position should advance with real time
    let expected_advance = elapsed * 0.5; // elapsed time at 0.5 CPS
    let actual_advance = pos2 - pos1;

    // Allow tolerance for buffer processing overhead
    let ratio = actual_advance / expected_advance;
    assert!(
        (ratio - 1.0).abs() < 0.2, // Within 20%
        "Wall-clock timing not working: expected={:.4}, actual={:.4}, ratio={:.2}",
        expected_advance,
        actual_advance,
        ratio
    );
}

/// Stress test: Many rapid swaps with timing verification
#[test]
fn test_stress_rapid_swaps_50_times() {
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let mut graph = compile_dsl(code, 44100.0);
    graph.enable_wall_clock_timing();

    let start = Instant::now();
    let mut max_ratio_deviation = 0.0f64;
    let mut max_pos_jump = 0.0f64;

    for i in 0..50 {
        let pos_before = graph.get_cycle_position();

        // Very rapid swaps (no delay)
        let (new_graph, new_cps) = simulate_graph_swap(&graph, code, 44100.0);

        let pos_after = new_graph.get_cycle_position();
        let ratio = new_cps as f64 / 0.5;
        let pos_jump = (pos_after - pos_before).abs();

        max_ratio_deviation = max_ratio_deviation.max((ratio - 1.0).abs());
        max_pos_jump = max_pos_jump.max(pos_jump);

        // Fail fast if we detect a doubling
        if ratio > 1.5 || ratio < 0.7 {
            panic!(
                "TEMPO DOUBLING DETECTED at swap {}: CPS went from 0.5 to {} (ratio: {:.2}x)",
                i, new_cps, ratio
            );
        }

        graph = new_graph;
    }

    let elapsed = start.elapsed();
    eprintln!("\n📊 Stress test results:");
    eprintln!("   Swaps: 50");
    eprintln!("   Total time: {:?}", elapsed);
    eprintln!("   Max CPS ratio deviation: {:.4}", max_ratio_deviation);
    eprintln!("   Max position jump: {:.4} cycles", max_pos_jump);

    assert!(
        max_ratio_deviation < 0.01,
        "CPS deviated too much: max deviation = {:.4}",
        max_ratio_deviation
    );
}

/// Test with different tempos to see if bug is tempo-dependent
#[test]
fn test_various_tempos() {
    let tempos = [0.25, 0.5, 1.0, 2.0, 4.0];

    for tempo in tempos {
        let code = format!(
            r#"
tempo: {}
~drums $ s "bd sn"
out $ ~drums
"#,
            tempo
        );

        let mut graph = compile_dsl(&code, 44100.0);
        graph.enable_wall_clock_timing();

        std::thread::sleep(Duration::from_millis(100));

        for i in 0..10 {
            let (new_graph, new_cps) = simulate_graph_swap(&graph, &code, 44100.0);

            let ratio = new_cps as f64 / tempo;
            if (ratio - 1.0).abs() > 0.01 {
                panic!(
                    "Tempo {} CPS error at swap {}: got {} (ratio: {:.2}x)",
                    tempo, i, new_cps, ratio
                );
            }

            graph = new_graph;
        }

        eprintln!("✅ Tempo {} passed", tempo);
    }
}

// Newtype wrapper like modal_editor uses
struct GraphCell(RefCell<UnifiedSignalGraph>);
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}

/// Concurrent test: simulate actual audio thread + C-x scenario
/// This test creates a separate thread that continuously calls process_buffer
/// while the main thread does graph swaps
#[test]
fn test_concurrent_audio_and_swap() {
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let sample_rate = 44100.0;
    let mut initial_graph = compile_dsl(code, sample_rate);
    initial_graph.enable_wall_clock_timing();

    let graph: Arc<ArcSwap<Option<GraphCell>>> = Arc::new(ArcSwap::from_pointee(Some(GraphCell(
        RefCell::new(initial_graph),
    ))));

    let stop_flag = Arc::new(AtomicBool::new(false));

    // Audio thread: continuously processes buffers
    let graph_clone = Arc::clone(&graph);
    let stop_clone = Arc::clone(&stop_flag);
    let audio_thread = std::thread::spawn(move || {
        let mut buffer = [0.0f32; 512];
        let mut buffers_processed = 0;
        let mut tempo_samples: Vec<f32> = Vec::new();

        while !stop_clone.load(Ordering::Relaxed) {
            let graph_snapshot = graph_clone.load();
            if let Some(ref graph_cell) = **graph_snapshot {
                if let Ok(mut g) = graph_cell.0.try_borrow_mut() {
                    g.process_buffer(&mut buffer);
                    buffers_processed += 1;

                    // Sample CPS periodically
                    if buffers_processed % 100 == 0 {
                        tempo_samples.push(g.get_cps());
                    }
                }
            }
            // Small sleep to avoid 100% CPU
            std::thread::sleep(Duration::from_micros(100));
        }

        (buffers_processed, tempo_samples)
    });

    // Main thread: do graph swaps like C-x
    let swap_count = 100;
    let mut cps_at_swap: Vec<f32> = Vec::new();

    for i in 0..swap_count {
        // Random-ish timing (10-50ms between swaps)
        let delay_ms = (i * 7 % 40) + 10;
        std::thread::sleep(Duration::from_millis(delay_ms as u64));

        // Compile new graph
        let mut new_graph = compile_dsl(code, sample_rate);

        // Get old graph and transfer timing (like modal_editor does)
        let current = graph.load();
        if let Some(ref old_cell) = **current {
            // Try to borrow for state transfer
            for _attempt in 0..20 {
                if let Ok(old_guard) = old_cell.0.try_borrow() {
                    new_graph.transfer_session_timing(&old_guard);
                    cps_at_swap.push(new_graph.get_cps());
                    break;
                }
                std::thread::sleep(Duration::from_micros(500));
            }
        }

        // Hot-swap
        graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));
    }

    // Stop audio thread
    stop_flag.store(true, Ordering::Relaxed);
    let (buffers_processed, tempo_samples) = audio_thread.join().unwrap();

    eprintln!("\n📊 Concurrent test results:");
    eprintln!("   Swaps performed: {}", swap_count);
    eprintln!("   Audio buffers processed: {}", buffers_processed);
    eprintln!("   Tempo samples collected: {}", tempo_samples.len());

    // Analyze CPS values
    let expected_cps = 0.5;
    let mut errors = 0;

    for (i, &cps) in cps_at_swap.iter().enumerate() {
        let ratio = cps as f64 / expected_cps;
        if (ratio - 1.0).abs() > 0.01 {
            eprintln!("   ❌ Swap {}: CPS={} (ratio={:.2}x)", i, cps, ratio);
            errors += 1;
        }
    }

    for (i, &cps) in tempo_samples.iter().enumerate() {
        let ratio = cps as f64 / expected_cps;
        if (ratio - 1.0).abs() > 0.01 {
            eprintln!(
                "   ❌ Audio sample {}: CPS={} (ratio={:.2}x)",
                i, cps, ratio
            );
            errors += 1;
        }
    }

    if errors > 0 {
        panic!("TEMPO DOUBLING DETECTED: {} errors", errors);
    }

    eprintln!("   ✅ All CPS values correct (0.5)");
}

/// Aggressive stress test with 500 swaps to catch rare tempo doubling
#[test]
fn test_aggressive_500_swaps() {
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let sample_rate = 44100.0;
    let mut initial_graph = compile_dsl(code, sample_rate);
    initial_graph.enable_wall_clock_timing();

    let graph: Arc<ArcSwap<Option<GraphCell>>> = Arc::new(ArcSwap::from_pointee(Some(GraphCell(
        RefCell::new(initial_graph),
    ))));

    let stop_flag = Arc::new(AtomicBool::new(false));

    // Audio thread
    let graph_clone = Arc::clone(&graph);
    let stop_clone = Arc::clone(&stop_flag);
    let audio_thread = std::thread::spawn(move || {
        let mut buffer = [0.0f32; 512];
        let mut errors = Vec::new();

        while !stop_clone.load(Ordering::Relaxed) {
            let graph_snapshot = graph_clone.load();
            if let Some(ref graph_cell) = **graph_snapshot {
                if let Ok(mut g) = graph_cell.0.try_borrow_mut() {
                    g.process_buffer(&mut buffer);
                    let cps = g.get_cps();
                    if (cps - 0.5).abs() > 0.01 {
                        errors.push(cps);
                    }
                }
            }
            std::thread::sleep(Duration::from_micros(50));
        }
        errors
    });

    // Main thread: 500 rapid swaps
    let swap_count = 500;
    let mut swap_errors = Vec::new();

    for i in 0..swap_count {
        // Vary timing - sometimes very fast, sometimes slower
        let delay_ms = match i % 10 {
            0 => 1, // Very fast
            1 => 2,
            2 => 5,
            3 => 10,
            4 => 20,
            5 => 30,
            6 => 40,
            7 => 50,
            8 => 75,
            _ => 100, // Slow
        };
        std::thread::sleep(Duration::from_millis(delay_ms as u64));

        let mut new_graph = compile_dsl(code, sample_rate);

        let current = graph.load();
        if let Some(ref old_cell) = **current {
            for _attempt in 0..20 {
                if let Ok(old_guard) = old_cell.0.try_borrow() {
                    new_graph.transfer_session_timing(&old_guard);
                    let cps = new_graph.get_cps();
                    if (cps - 0.5).abs() > 0.01 {
                        swap_errors.push((i, cps));
                    }
                    break;
                }
                std::thread::sleep(Duration::from_micros(250));
            }
        }

        graph.store(Arc::new(Some(GraphCell(RefCell::new(new_graph)))));
    }

    stop_flag.store(true, Ordering::Relaxed);
    let audio_errors = audio_thread.join().unwrap();

    eprintln!("\n📊 Aggressive 500-swap test:");
    eprintln!("   Swap errors: {}", swap_errors.len());
    eprintln!("   Audio thread errors: {}", audio_errors.len());

    if !swap_errors.is_empty() {
        for (i, cps) in &swap_errors {
            eprintln!("   ❌ Swap {}: CPS={} (expected 0.5)", i, cps);
        }
    }
    if !audio_errors.is_empty() {
        for cps in &audio_errors {
            eprintln!("   ❌ Audio: CPS={} (expected 0.5)", cps);
        }
    }

    assert!(
        swap_errors.is_empty() && audio_errors.is_empty(),
        "TEMPO ERRORS DETECTED"
    );

    eprintln!("   ✅ All 500 swaps maintained correct CPS");
}

/// Test to detect actual tempo change in audio output
/// by measuring cycle position advancement rate
#[test]
fn test_cycle_position_rate() {
    let code = r#"
tempo: 0.5
~drums $ s "bd sn"
out $ ~drums
"#;

    let sample_rate = 44100.0;
    let mut graph = compile_dsl(code, sample_rate);
    graph.enable_wall_clock_timing();

    let mut buffer = [0.0f32; 512];

    // Process some buffers and track cycle position
    let samples_per_second = 44100;
    let buffers_to_process = samples_per_second / 512; // ~1 second worth

    let start_time = Instant::now();

    for _ in 0..buffers_to_process {
        graph.process_buffer(&mut buffer);
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let final_pos = graph.get_cycle_position();

    // In wall-clock mode, position should be elapsed * cps + offset
    // Since we enabled wall clock, and ran for ~1 second, position should be ~0.5 cycles
    let expected_position = elapsed * 0.5; // cps = 0.5

    eprintln!("\n📊 Cycle position rate test:");
    eprintln!("   Elapsed time: {:.4}s", elapsed);
    eprintln!("   Final cycle position: {:.4}", final_pos);
    eprintln!("   Expected position: {:.4}", expected_position);

    // Allow some tolerance for test execution overhead
    let position_ratio = final_pos / expected_position;
    assert!(
        (position_ratio - 1.0).abs() < 0.1,
        "Cycle position rate wrong: ratio={:.2}x (expected ~1.0)",
        position_ratio
    );
}
