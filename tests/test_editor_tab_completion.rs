//! Integration tests for editor tab completion
//!
//! Tests the full tab completion flow in the modal editor using the test harness.
//! Verifies that:
//! 1. Tab completion triggers on function names
//! 2. Kwargs completion works with space (e.g. "gain <tab>")
//! 3. Kwargs completion works with colon (e.g. "gain :<tab>")
//! 4. Partial matching works (e.g. "gain :a<tab>")
//! 5. Navigation through completions works
//! 6. Accepting completions inserts correct text

use phonon::modal_editor::test_harness::EditorTestHarness;

#[test]
fn test_harness_basic() {
    // First verify the harness itself works
    let mut harness = EditorTestHarness::new().unwrap();
    harness.type_text("hello");

    assert_eq!(harness.content(), "hello");
    assert_eq!(harness.cursor_pos(), 5);
}

#[test]
fn test_tab_key_detection() {
    // Test that tab key is being processed
    let mut harness = EditorTestHarness::new().unwrap();
    harness.type_text("ga");  // Partial function name

    eprintln!("Before tab - Content: {:?}", harness.content());
    eprintln!("Before tab - Cursor: {}", harness.cursor_pos());
    eprintln!("Before tab - Completion visible: {}", harness.is_completion_shown());

    harness.tab();

    eprintln!("After tab - Content: {:?}", harness.content());
    eprintln!("After tab - Cursor: {}", harness.cursor_pos());
    eprintln!("After tab - Completion visible: {}", harness.is_completion_shown());
    eprintln!("After tab - Completion options: {:?}", harness.completion_options());

    // Just check if completion was triggered at all
    if !harness.is_completion_shown() {
        panic!("Tab key didn't trigger completion!");
    }
}

#[test]
fn test_function_completion_basic() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gai" and press tab - should show "gain" as option
    harness.type_text("gai")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains("gain");
}

#[test]
fn test_kwargs_completion_with_space() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain " (with space) and press tab
    // Should show kwargs with colon prefix: ":amount"
    harness.type_text("gain ")
        .debug_state()
        .tab()
        .debug_state()
        .assert_completion_shown()
        .assert_completion_contains(":amount");
}

#[test]
fn test_kwargs_completion_with_colon_only() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain :" and press tab
    // Should show kwargs with colon prefix: ":amount"
    harness.type_text("gain :")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":amount");
}

#[test]
fn test_kwargs_completion_with_partial() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain :am" and press tab
    // Should show "amount" (without colon since we already typed it)
    harness.type_text("gain :am")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains("amount");
}

#[test]
fn test_kwargs_accept_completion() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain ", tab to show completions, then tab again to accept first
    harness.type_text("gain ")
        .tab()
        .assert_completion_shown()
        .tab(); // Accept first completion

    // Should have inserted ":amount"
    let line = harness.current_line();
    assert!(
        line.contains(":amount"),
        "Expected line to contain ':amount', got: {:?}",
        line
    );
}

#[test]
fn test_lpf_kwargs_multiple_params() {
    let mut harness = EditorTestHarness::new().unwrap();

    // LPF has two kwargs: :cutoff and :q
    harness.type_text("lpf ")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":cutoff")
        .assert_completion_contains(":q");
}

#[test]
fn test_reverb_kwargs() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Reverb has positional args but also :mix kwarg
    harness.type_text("reverb 0.8 0.5 :")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":mix");
}

#[test]
fn test_plate_kwargs_many_params() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Plate has many kwargs
    harness.type_text("plate ")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":pre_delay")
        .assert_completion_contains(":decay")
        .assert_completion_contains(":diffusion")
        .assert_completion_contains(":damping")
        .assert_completion_contains(":mix");
}

#[test]
fn test_completion_in_chain() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Test kwargs completion works in chain context
    // "saw 55 # lpf :" should show lpf kwargs, not saw
    harness.type_text("saw 55 # lpf ")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":cutoff")
        .assert_completion_contains(":q");
}

#[test]
fn test_no_double_colon_bug() {
    let mut harness = EditorTestHarness::new().unwrap();

    // The bug was: "gain :a<tab>" -> "gain ::amount"
    // Should be: "gain :a<tab>" -> "gain :amount"
    harness.type_text("gain :a")
        .tab() // Show completions
        .tab(); // Accept first

    let line = harness.current_line();

    // Should NOT have double colon
    assert!(
        !line.contains("::"),
        "Should not have double colon! Got: {:?}",
        line
    );

    // Should have single colon with amount
    assert!(
        line.contains(":amount"),
        "Should contain ':amount'. Got: {:?}",
        line
    );
}

#[test]
fn test_completion_navigation_with_arrows() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Show completions for plate (has multiple kwargs)
    harness.type_text("plate ")
        .tab()
        .assert_completion_shown();

    // Navigate with up/down shouldn't crash
    harness.send_key(crossterm::event::KeyCode::Down)
        .send_key(crossterm::event::KeyCode::Down)
        .send_key(crossterm::event::KeyCode::Up);

    // Should still be showing completions
    harness.assert_completion_shown();
}

#[test]
fn test_completion_cancel_with_esc() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Show completions
    harness.type_text("gain ")
        .tab()
        .assert_completion_shown();

    // Press Esc to cancel
    harness.send_key(crossterm::event::KeyCode::Esc)
        .assert_completion_hidden();
}

#[test]
fn test_completion_filter_as_typing() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Show completions for plate
    harness.type_text("plate ")
        .tab()
        .assert_completion_shown();

    // Start typing to filter
    harness.type_text(":d");

    // Should still show completions but filtered
    // Plate has :decay, :diffusion, :damping that match "d"
    harness.assert_completion_shown();
}

#[test]
fn test_sample_completion() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type 's "' to trigger sample completion context
    harness.type_text("s \"")
        .tab();

    // If dirt-samples are available, should show completions
    // (This might not show anything if samples aren't installed, but shouldn't crash)
    // Just verify it doesn't crash
}

#[test]
fn test_bus_completion() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Define a bus first
    harness.type_text("~bass: sine 110")
        .enter();

    // Now try to reference it
    harness.type_text("out: ~")
        .tab();

    // Should show completions including ~bass
    // (Might be empty if bus extraction isn't working yet)
}

#[test]
fn test_multiline_completion() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type on first line
    harness.type_text("tempo: 2.0")
        .enter();

    // Type on second line
    harness.type_text("gain ")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":amount");
}

#[test]
fn test_completion_with_existing_code() {
    let mut harness = EditorTestHarness::with_content(
        "tempo: 2.0\n~drums: s \"bd sn\"\nout: ~drums"
    ).unwrap();

    // Move to end and add new line
    harness.send_key(crossterm::event::KeyCode::End)
        .enter()
        .type_text("gain ")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":amount");
}

#[test]
fn test_kwargs_after_positional_args() {
    let mut harness = EditorTestHarness::new().unwrap();

    // LPF can take: lpf <input> <cutoff> <q>
    // Or with kwargs: lpf 800 :q 2.0
    harness.type_text("lpf 800 :")
        .tab()
        .assert_completion_shown()
        .assert_completion_contains(":cutoff")
        .assert_completion_contains(":q");
}

#[test]
fn test_completion_preserves_cursor_position() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type and show completion
    harness.type_text("gain ")
        .tab();

    let cursor_before_accept = harness.cursor_pos();

    // Accept completion
    harness.tab();

    let cursor_after = harness.cursor_pos();

    // Cursor should have moved forward (text was inserted)
    assert!(
        cursor_after > cursor_before_accept,
        "Cursor should advance after accepting completion"
    );
}

// ==================== CTRL+SPACE KWARGS EXPANSION TESTS ====================

#[test]
fn test_ctrl_space_expands_gain_kwargs() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain" and press Ctrl+Space
    harness.type_text("gain");
    harness.ctrl_space();

    let line = harness.current_line();

    // Should have expanded kwargs with default value
    assert!(
        line.contains(":amount"),
        "Should contain ':amount' parameter. Got: {:?}",
        line
    );

    // Should contain default value
    assert!(
        line.contains("1.0") || line.contains("1"),
        "Should contain default value. Got: {:?}",
        line
    );
}

#[test]
fn test_ctrl_space_expands_lpf_kwargs() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "lpf" and press Ctrl+Space
    harness.type_text("lpf");
    harness.ctrl_space();

    let line = harness.current_line();

    // LPF has :cutoff and :q parameters
    assert!(
        line.contains(":cutoff") || line.contains(":q"),
        "Should contain LPF kwargs. Got: {:?}",
        line
    );
}

#[test]
fn test_ctrl_space_expands_plate_kwargs() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "plate" and press Ctrl+Space
    harness.type_text("plate");
    harness.ctrl_space();

    let line = harness.current_line();

    // Plate has many kwargs: pre_delay, decay, diffusion, damping, mix
    let has_kwargs = line.contains(":pre_delay")
        || line.contains(":decay")
        || line.contains(":diffusion")
        || line.contains(":damping")
        || line.contains(":mix");

    assert!(has_kwargs, "Should contain plate kwargs. Got: {:?}", line);
}

#[test]
fn test_ctrl_space_with_space_after_function() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "gain " (with space) and press Ctrl+Space
    harness.type_text("gain ");
    harness.ctrl_space();

    let line = harness.current_line();

    // Should still expand kwargs
    assert!(
        line.contains(":amount"),
        "Should expand kwargs even with trailing space. Got: {:?}",
        line
    );
}

#[test]
fn test_ctrl_space_after_partial_args() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type "lpf 800 " (partial args) and press Ctrl+Space
    harness.type_text("lpf 800 ");
    harness.ctrl_space();

    let line = harness.current_line();

    // Should expand kwargs after existing args
    assert!(
        line.contains("800") && (line.contains(":cutoff") || line.contains(":q")),
        "Should keep existing args and add kwargs. Got: {:?}",
        line
    );
}

#[test]
fn test_ctrl_space_no_function() {
    let mut harness = EditorTestHarness::new().unwrap();

    // Type something that's not a function
    harness.type_text("hello world");
    harness.ctrl_space();

    let line = harness.current_line();

    // Should not modify content if no function found
    assert_eq!(
        line, "hello world",
        "Should not modify non-function text"
    );
}
