//! Regression tests for parse_dsl chaining a source into `struct` via `$`.
//!
//! Bug (fix-parse-dsl): the unified_graph_parser front-end `parse_dsl` did NOT
//! support `struct "pat" $ src`. It consumed only `struct "pat"` and returned
//! `$ src ...` as an UNPARSED tail. Because parse_dsl silently discarded the
//! remainder, every subsequent statement was also dropped, yielding a
//! struct-with-no-source => total silence.

use phonon::unified_graph_parser::{parse_dsl, DslCompiler};

fn find_peak(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

/// Core case: `out $ struct "pat" $ sine "66"` must parse fully (no tail) and
/// render audio.
#[test]
fn test_struct_source_chaining_parses_and_renders() {
    let code = r#"out $ struct "t(3,8,1)" $ sine "66""#;

    let (remaining, statements) = parse_dsl(code).expect("parse_dsl should succeed");

    // No statement may be silently dropped: the whole input must be consumed.
    assert!(
        remaining.trim().is_empty(),
        "parse_dsl left an unparsed tail: {:?}",
        remaining
    );
    assert_eq!(statements.len(), 1, "expected exactly one statement");

    // Rendering the struct-gated source must produce audio.
    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let audio = graph.render(44100); // 1 second
    let peak = find_peak(&audio);
    assert!(
        peak > 0.0,
        "struct-gated sine should produce audio, got peak={peak}"
    );
}

/// The silent-drop cascade: a struct-chain statement must NOT swallow the
/// statements that follow it.
#[test]
fn test_struct_chaining_does_not_drop_following_statements() {
    let code = r#"
        ~a $ struct "t(3,8,1)" $ sine "66"
        ~b $ sine 440
        out $ ~a * 0.3 + ~b * 0.3
    "#;

    let (remaining, statements) = parse_dsl(code).expect("parse_dsl should succeed");

    assert!(
        remaining.trim().is_empty(),
        "parse_dsl left an unparsed tail: {:?}",
        remaining
    );
    assert_eq!(
        statements.len(),
        3,
        "all three statements must be parsed, got {}: {:#?}",
        statements.len(),
        statements
    );

    let mut graph = DslCompiler::new(44100.0).compile(statements);
    let audio = graph.render(44100);
    assert!(
        find_peak(&audio) > 0.0,
        "combined graph should produce audio"
    );
}

/// Existing `$` transform chaining must keep working (no regression).
#[test]
fn test_transform_chaining_still_works() {
    let code = r#"out $ s "bd sn" $ fast 2 $ rev"#;

    let (remaining, statements) = parse_dsl(code).expect("parse_dsl should succeed");

    assert!(
        remaining.trim().is_empty(),
        "parse_dsl left an unparsed tail: {:?}",
        remaining
    );
    assert_eq!(statements.len(), 1);
}
