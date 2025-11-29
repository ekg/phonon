/// Live test for tempo doubling bug during C-x in phonon-edit
///
/// This test actually runs the modal editor with real audio and detects
/// tempo changes by checking CPS values after each C-x press.

use phonon::modal_editor::test_harness::EditorTestHarness;
use std::time::Duration;

#[test]
#[ignore] // Requires audio device
fn test_tempo_stability_100_presses() {
    eprintln!("\n🧪 Testing tempo stability with 100 C-x presses...\n");

    let code = r#"out $ s "bd(3,4) cp""#;

    let mut harness = EditorTestHarness::new().expect("Failed to create editor");
    harness.set_content(code);

    let mut cps_values = Vec::new();
    let mut errors = Vec::new();

    for i in 0..100 {
        // Trigger C-x to evaluate
        harness.ctrl_x();

        // Wait a bit for processing
        std::thread::sleep(Duration::from_millis(30));

        // Record CPS
        if let Some(cps) = harness.get_cps() {
            cps_values.push(cps);
            if (cps - 0.5).abs() > 0.01 {
                errors.push((i, cps));
            }
        }

        // Vary the timing between presses to catch edge cases
        let delay_ms = match i % 10 {
            0 => 5,    // Very fast
            1 => 10,
            2 => 20,
            3 => 50,
            4 => 100,
            5 => 150,
            6 => 200,
            7 => 250,
            8 => 300,
            _ => 500,  // Slow
        };
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    eprintln!("📊 Collected {} CPS values", cps_values.len());

    if !errors.is_empty() {
        eprintln!("\n❌ ERRORS DETECTED:");
        for (i, cps) in &errors {
            let ratio = *cps as f64 / 0.5;
            eprintln!("   Press {}: CPS={:.3} (ratio={:.2}x)", i, cps, ratio);
        }
        panic!("TEMPO INSTABILITY: {} errors out of {} presses", errors.len(), cps_values.len());
    }

    eprintln!("\n✅ All {} CPS values stable at 0.5", cps_values.len());
}

#[test]
#[ignore] // Requires audio device
fn test_rapid_fire_c_x_200() {
    eprintln!("\n🧪 Testing rapid-fire C-x (200 presses, every 10ms)...\n");

    let code = r#"out $ s "bd(3,4) cp""#;

    let mut harness = EditorTestHarness::new().expect("Failed to create editor");
    harness.set_content(code);

    let mut errors = Vec::new();
    let mut doubled_count = 0;

    for i in 0..200 {
        harness.ctrl_x();

        // Check CPS immediately after
        if let Some(cps) = harness.get_cps() {
            if (cps - 0.5).abs() > 0.01 {
                let ratio = cps as f64 / 0.5;
                errors.push((i, cps, ratio));
                if ratio > 1.5 {
                    doubled_count += 1;
                }
            }
        }

        // Very short delay - 10ms
        std::thread::sleep(Duration::from_millis(10));
    }

    eprintln!("📊 200 rapid-fire C-x presses completed");
    eprintln!("📊 Errors detected: {}", errors.len());
    eprintln!("📊 Tempo doublings (>1.5x): {}", doubled_count);

    if !errors.is_empty() {
        eprintln!("\n❌ ERRORS:");
        for (i, cps, ratio) in &errors {
            eprintln!("   Press {}: CPS={:.3} (ratio={:.2}x)", i, cps, ratio);
        }
        panic!("TEMPO ERRORS: {} detected, {} doublings", errors.len(), doubled_count);
    }

    eprintln!("\n✅ All 200 presses maintained CPS=0.5");
}

#[test]
#[ignore] // Requires audio device
fn test_wall_clock_stays_enabled() {
    eprintln!("\n🧪 Testing wall-clock timing stays enabled...\n");

    let code = r#"out $ s "bd(3,4) cp""#;

    let mut harness = EditorTestHarness::new().expect("Failed to create editor");
    harness.set_content(code);

    // Initial evaluation
    harness.ctrl_x();
    std::thread::sleep(Duration::from_millis(200)); // Longer wait for initial load

    // Check initial state - allow Some(None) which means try_borrow failed
    // The critical check is that CPS is stable, not the wall_clock flag directly
    let initial_wall_clock = harness.is_wall_clock_enabled();
    eprintln!("📊 Initial wall-clock: {:?}", initial_wall_clock);

    // If we got a value, it must be true
    if let Some(enabled) = initial_wall_clock {
        assert!(enabled, "Wall-clock should be enabled after first C-x");
    }

    // Do many more C-x presses
    let mut wall_clock_checks = 0;
    let mut cps_checks = 0;

    for i in 0..50 {
        harness.ctrl_x();
        std::thread::sleep(Duration::from_millis(20));

        // Check wall-clock is still enabled (when accessible)
        if let Some(wall_clock) = harness.is_wall_clock_enabled() {
            wall_clock_checks += 1;
            if !wall_clock {
                panic!("Wall-clock got disabled at press {}!", i);
            }
        }

        // CPS is the critical check - must stay at 0.5
        if let Some(cps) = harness.get_cps() {
            cps_checks += 1;
            if (cps - 0.5).abs() > 0.01 {
                panic!("CPS changed to {} at press {}!", cps, i);
            }
        }
    }

    eprintln!("📊 Wall-clock checks: {}, CPS checks: {}", wall_clock_checks, cps_checks);

    // At least some checks should have succeeded
    assert!(cps_checks > 10, "Should have successfully checked CPS at least 10 times, got {}", cps_checks);

    eprintln!("\n✅ Wall-clock remained enabled through 50 presses");
}

#[test]
#[ignore] // Requires audio device
fn test_cycle_position_advances_correctly() {
    eprintln!("\n🧪 Testing cycle position advancement rate...\n");

    let code = r#"tempo: 0.5
out $ s "bd sn""#;

    let mut harness = EditorTestHarness::new().expect("Failed to create editor");
    harness.set_content(code);

    // Initial evaluation
    harness.ctrl_x();
    std::thread::sleep(Duration::from_millis(200));

    // Get initial position
    let pos1 = harness.get_cycle_position().expect("No cycle position");
    let start = std::time::Instant::now();

    // Wait 1 second
    std::thread::sleep(Duration::from_secs(1));

    // Get final position
    let pos2 = harness.get_cycle_position().expect("No cycle position");
    let elapsed = start.elapsed().as_secs_f64();

    let cycles_advanced = pos2 - pos1;
    let measured_cps = cycles_advanced / elapsed;

    eprintln!("📊 Elapsed: {:.3}s", elapsed);
    eprintln!("📊 Cycles advanced: {:.4}", cycles_advanced);
    eprintln!("📊 Measured CPS: {:.4}", measured_cps);
    eprintln!("📊 Expected CPS: 0.5");

    let ratio = measured_cps / 0.5;
    eprintln!("📊 Ratio: {:.3}", ratio);

    // Allow 20% tolerance for timing overhead
    assert!(
        (ratio - 1.0).abs() < 0.2,
        "Cycle position rate wrong: measured={:.4}, expected=0.5, ratio={:.2}x",
        measured_cps, ratio
    );

    eprintln!("\n✅ Cycle position advances at correct rate");
}

#[test]
#[ignore] // Requires audio device
fn test_note_chord_plus_offset_syntax() {
    eprintln!("\n🧪 Testing note chord + offset syntax in editor...\n");

    let code = r#"~synth $ saw 55
o2 $ s "~synth" # note "c3'maj" + "0 3 7" # gain 0.3"#;

    eprintln!("Code:\n{}\n", code);

    // First test: does the parser handle this?
    use phonon::compositional_parser::parse_program;
    use phonon::compositional_compiler::compile_program;

    let (rest, stmts) = match parse_program(code) {
        Ok((rest, stmts)) => {
            eprintln!("✅ Parser succeeded: {} statements", stmts.len());
            eprintln!("   Remaining: {:?}", rest.trim());
            if !rest.trim().is_empty() {
                eprintln!("❌ Parser did not consume all input!");
            }
            (rest, stmts)
        }
        Err(e) => {
            eprintln!("❌ Parse error: {}", e);
            panic!("Parse failed: {}", e);
        }
    };

    // Second test: does it compile?
    eprintln!("Compiling...");
    match compile_program(stmts, 44100.0, None) {
        Ok(graph) => {
            eprintln!("✅ Compiled successfully!");
            eprintln!("   CPS: {}", graph.get_cps());
        }
        Err(e) => {
            eprintln!("❌ Compile error: {}", e);
            panic!("Compile failed: {}", e);
        }
    }

    eprintln!("Creating editor harness...");
    let harness_result = EditorTestHarness::new();
    if let Err(ref e) = harness_result {
        eprintln!("❌ Failed to create editor: {}", e);
    }
    let mut harness = harness_result.expect("Failed to create editor");
    eprintln!("Setting content...");
    harness.set_content(code);

    // Trigger C-x to evaluate
    eprintln!("Triggering C-x...");
    harness.ctrl_x();
    std::thread::sleep(Duration::from_millis(500));

    // Check if graph loaded
    let has_graph = harness.has_graph();
    eprintln!("📊 Graph loaded: {}", has_graph);

    if let Some(cps) = harness.get_cps() {
        eprintln!("📊 CPS: {}", cps);
    } else {
        eprintln!("❌ Could not get CPS - graph may not have loaded");
    }

    assert!(has_graph, "note chord + offset syntax should parse and create graph");
    eprintln!("\n✅ Syntax parsed successfully!");
}

#[test]
#[ignore] // Requires audio device
fn test_stress_500_varied_timing() {
    eprintln!("\n🧪 Stress test: 500 C-x with varied timing...\n");

    let code = r#"out $ s "bd(3,4) cp""#;

    let mut harness = EditorTestHarness::new().expect("Failed to create editor");
    harness.set_content(code);

    let mut errors = Vec::new();

    for i in 0..500 {
        harness.ctrl_x();

        if let Some(cps) = harness.get_cps() {
            if (cps - 0.5).abs() > 0.01 {
                errors.push((i, cps));
            }
        }

        // Very varied timing - catch any edge case
        let delay_ms = match i % 20 {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 5,
            4 => 7,
            5 => 10,
            6 => 15,
            7 => 20,
            8 => 25,
            9 => 30,
            10 => 40,
            11 => 50,
            12 => 75,
            13 => 100,
            14 => 125,
            15 => 150,
            16 => 200,
            17 => 250,
            18 => 300,
            _ => 500,
        };
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    eprintln!("📊 500 varied-timing presses completed");
    eprintln!("📊 Errors: {}", errors.len());

    if !errors.is_empty() {
        for (i, cps) in errors.iter().take(20) {
            eprintln!("   ❌ Press {}: CPS={:.3} (ratio={:.2}x)", i, cps, *cps as f64 / 0.5);
        }
        if errors.len() > 20 {
            eprintln!("   ... and {} more", errors.len() - 20);
        }
        panic!("STRESS TEST FAILED: {} errors", errors.len());
    }

    eprintln!("\n✅ All 500 presses stable");
}
