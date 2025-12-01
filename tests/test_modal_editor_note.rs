use std::time::{Duration, Instant};

use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;
use phonon::modal_editor::test_harness::EditorTestHarness;

/// Test that note modifier works correctly in modal editor's real-time context
/// This test simulates the exact code path used by phonon edit
#[test]
fn test_note_modifier_with_wall_clock_timing() {
    println!("Testing note modifier with wall-clock timing (modal editor path)...");

    let code = r#"
tempo: 0.5
out $ s "bd*4" # note "c3"
"#;

    // Parse
    let (_, statements) = parse_program(code).expect("Parse failed");

    // Compile WITHOUT midi queue (headless)
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");

    // Enable wall-clock timing - THIS IS THE MODAL EDITOR DIFFERENCE
    graph.enable_wall_clock_timing();

    // Simulate real-time processing
    let mut buffer = [0.0f32; 512];
    let chunks_per_second = 44100 / 512;
    let total_chunks = chunks_per_second * 2; // 2 seconds

    let start = Instant::now();
    let timeout = Duration::from_secs(10);

    for i in 0..total_chunks {
        if start.elapsed() > timeout {
            panic!(
                "Timeout after {} chunks! Possible infinite loop in wall-clock mode.",
                i
            );
        }

        let chunk_start = Instant::now();
        graph.process_buffer(&mut buffer);
        let chunk_time = chunk_start.elapsed();

        // If any chunk takes > 100ms, something is very wrong
        if chunk_time.as_millis() > 100 {
            panic!(
                "Chunk {} took {:?} - exceeds real-time budget in wall-clock mode!",
                i, chunk_time
            );
        }
    }

    let total_time = start.elapsed();
    println!(
        "  Processed {} chunks (wall-clock mode) in {:?}",
        total_chunks, total_time
    );

    assert!(
        total_time < Duration::from_secs(5),
        "Processing with wall-clock timing took {:?}, should be faster than real-time",
        total_time
    );

    println!("✅ note modifier with wall-clock timing test passed");
}

/// Test the exact code from w.ph that allegedly hangs
#[test]
fn test_wph_code_with_wall_clock_timing() {
    println!("Testing w.ph code with wall-clock timing...");

    let code = r#"
tempo: 0.5
~drums $ s "bd(3,4) [cp, ~ ~synth]"
~synth $ sine 330
~vowel $ s "~synth(3,17,1)" # note "c3"
out $ ~drums + ~vowel
"#;

    // Parse
    let (rest, statements) = parse_program(code).expect("Parse failed");
    eprintln!("Parsed {} statements, rest: {:?}", statements.len(), rest);

    // Compile WITHOUT midi queue (headless)
    let mut graph = compile_program(statements, 44100.0, None).expect("Compile failed");

    // Enable wall-clock timing
    graph.enable_wall_clock_timing();

    // Simulate real-time processing
    let mut buffer = [0.0f32; 512];
    let chunks_per_second = 44100 / 512;
    let total_chunks = chunks_per_second * 2; // 2 seconds

    let start = Instant::now();
    let timeout = Duration::from_secs(10);

    for i in 0..total_chunks {
        if start.elapsed() > timeout {
            panic!(
                "w.ph code timeout after {} chunks! Possible infinite loop.",
                i
            );
        }

        let chunk_start = Instant::now();
        graph.process_buffer(&mut buffer);
        let chunk_time = chunk_start.elapsed();

        if chunk_time.as_millis() > 100 {
            panic!(
                "w.ph code chunk {} took {:?} - exceeds real-time budget!",
                i, chunk_time
            );
        }
    }

    let total_time = start.elapsed();
    println!(
        "  w.ph code processed {} chunks in {:?}",
        total_chunks, total_time
    );

    println!("✅ w.ph code with wall-clock timing test passed");
}

/// Test letter note names specifically
#[test]
fn test_letter_note_names_wall_clock() {
    println!("Testing various letter note names with wall-clock timing...");

    let test_cases = vec![
        (r#"tempo: 0.5
out $ s "bd" # note "c3""#, "c3"),
        (r#"tempo: 0.5
out $ s "bd" # note "d4""#, "d4"),
        (r#"tempo: 0.5
out $ s "bd" # note "e4 f4 g4""#, "e4 f4 g4"),
        (r#"tempo: 0.5
out $ s "bd*4" # note "c3 d3 e3 f3""#, "c3 d3 e3 f3"),
    ];

    for (code, note_name) in test_cases {
        println!("  Testing note '{}'", note_name);

        let (_, statements) = parse_program(code).expect("Parse failed");
        let mut graph =
            compile_program(statements, 44100.0, None).expect("Compile failed");

        graph.enable_wall_clock_timing();

        let mut buffer = [0.0f32; 512];
        let start = Instant::now();
        let timeout = Duration::from_secs(5);

        for i in 0..100 {
            // Process 100 chunks
            if start.elapsed() > timeout {
                panic!(
                    "Timeout processing note '{}' after {} chunks!",
                    note_name, i
                );
            }
            graph.process_buffer(&mut buffer);
        }

        println!("    ✅ note '{}' processed ok", note_name);
    }

    println!("✅ all letter note names test passed");
}

/// Test note modifier using the full editor test harness
/// This simulates the EXACT behavior of phonon edit with Ctrl+X evaluation
#[test]
fn test_note_modifier_via_editor_harness() {
    println!("Testing note modifier via full editor test harness...");

    let code = r#"tempo: 0.5
out $ s "bd*4" # note "c3""#;

    // Create headless editor
    let mut harness = EditorTestHarness::new().expect("Failed to create test harness");

    // Set content and evaluate with Ctrl+X (exact modal editor behavior)
    harness.set_content(code);
    harness.ctrl_x();

    // Verify graph was loaded
    assert!(
        harness.has_graph(),
        "Graph should be loaded after Ctrl+X evaluation"
    );

    // Enable wall-clock timing (like modal editor does)
    harness
        .enable_wall_clock_timing()
        .expect("Failed to enable wall-clock timing");

    // Process audio chunks (2 seconds worth = ~172 chunks at 512 samples/chunk)
    let chunks = (44100 / 512) * 2;
    let result = harness.process_audio_chunks(chunks, 10000); // 10 second timeout

    match result {
        Ok(processed) => {
            println!(
                "  ✅ Processed {} audio chunks via editor harness",
                processed
            );
        }
        Err(e) => {
            panic!(
                "note modifier via editor harness FAILED: {}\nThis indicates a hang issue!",
                e
            );
        }
    }

    println!("✅ note modifier via editor harness test passed");
}

/// Test w.ph style code via editor harness
#[test]
fn test_wph_style_via_editor_harness() {
    println!("Testing w.ph style code via editor harness...");

    let code = r#"tempo: 0.5
~drums $ s "bd(3,4) cp"
~synth $ sine 330
~vowel $ s "bd" # note "c3"
out $ ~drums + ~vowel"#;

    let mut harness = EditorTestHarness::new().expect("Failed to create test harness");
    harness.set_content(code);
    harness.ctrl_x();

    assert!(harness.has_graph(), "Graph should be loaded");

    harness
        .enable_wall_clock_timing()
        .expect("Failed to enable wall-clock timing");

    let chunks = (44100 / 512) * 2;
    let result = harness.process_audio_chunks(chunks, 10000);

    match result {
        Ok(processed) => {
            println!(
                "  ✅ w.ph style processed {} chunks via harness",
                processed
            );
        }
        Err(e) => {
            panic!("w.ph style via editor harness FAILED: {}", e);
        }
    }

    println!("✅ w.ph style via editor harness test passed");
}

/// Test various note modifiers via editor harness
#[test]
fn test_various_notes_via_editor_harness() {
    println!("Testing various note modifiers via editor harness...");

    let test_cases = vec![
        ("tempo: 0.5\nout $ s \"bd\" # note \"c3\"", "c3"),
        ("tempo: 0.5\nout $ s \"bd\" # note \"d4\"", "d4"),
        ("tempo: 0.5\nout $ s \"bd*4\" # note \"c3 d3 e3 f3\"", "c3 d3 e3 f3"),
        ("tempo: 0.5\nout $ s \"bd\" # note \"60\"", "60 (MIDI)"),
        ("tempo: 0.5\nout $ s \"bd\" # note \"-12\"", "-12 (semitones)"),
    ];

    for (code, description) in test_cases {
        println!("  Testing note '{}'", description);

        let mut harness = EditorTestHarness::new().expect("Failed to create test harness");
        harness.set_content(code);
        harness.ctrl_x();

        if !harness.has_graph() {
            panic!("Graph should be loaded for note '{}'", description);
        }

        harness
            .enable_wall_clock_timing()
            .expect("Failed to enable wall-clock timing");

        // Process 100 chunks (enough to catch any hang)
        let result = harness.process_audio_chunks(100, 5000);

        match result {
            Ok(_) => {
                println!("    ✅ note '{}' processed ok via harness", description);
            }
            Err(e) => {
                panic!("note '{}' via editor harness FAILED: {}", description, e);
            }
        }
    }

    println!("✅ various notes via editor harness test passed");
}
