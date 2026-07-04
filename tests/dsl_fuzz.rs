//! Generative fuzz harness for Phonon's TWO divergent DSL front-ends
//! (wave-3 task `wave3-dsl-fuzzing`, improvement-plan I4 / test-gap P1-B).
//!
//! Phonon parses source through two independent parsers:
//!   * `unified_graph_parser::parse_dsl`      (`src/unified_graph_parser.rs`)
//!   * `compositional_parser::parse_program`  (`src/compositional_parser.rs`)
//!
//! A live coder mistypes constantly, so a panic or a silently-dropped statement
//! mid-set is a show-stopper. `fix-parse-dsl` (d1d8a75) just found `parse_dsl`
//! SILENTLY dropping every statement after `struct "pat" $ src` — a fuzzer
//! catches that whole class trivially. This harness asserts the safety contract
//! that BOTH front-ends must uphold on ANY input (well-formed OR garbage):
//!
//!   1. NEVER panics / unwraps / indexes out of bounds.
//!   2. NEVER silently drops a trailing statement — a well-formed program is
//!      consumed in full (or the parser signals a loud error); it must not stop
//!      mid-way and discard the remainder without a trace.
//!   3. Terminates within a per-input budget (no infinite loop / runaway memory).
//!
//! Both parsers return `Ok((remaining, statements))` and only *stop early* by
//! leaving a non-empty `remaining` tail (which the fixed `parse_dsl` now also
//! warns about on stderr). So "silent drop" is detectable as: a well-formed
//! program parsed into fewer statements than it contains, and/or left a
//! non-empty tail.
//!
//! Determinism: the generative properties run under a fixed-seed `TestRng` with
//! on-disk failure persistence disabled, and honour `PROPTEST_CASES` so
//! `verify-wave3` / CI can run a bounded, reproducible smoke pass
//! (e.g. `PROPTEST_CASES=64 cargo test --test dsl_fuzz`).
//!
//! NOTE: a coverage-guided `cargo-fuzz` target was considered (task marks it
//! OPTIONAL) but deliberately NOT added: it requires a nightly toolchain +
//! libfuzzer and a workspace-excluded crate, so it cannot run under the normal
//! `cargo test` gate that `verify-wave3`/CI use. The proptest harness below
//! covers every required validation item without that toolchain dependency.

use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};

use phonon::compositional_parser::parse_program;
use phonon::unified_graph_parser::parse_dsl;

// ---------------------------------------------------------------------------
// Parser adapters: normalise both front-ends to a common summary.
// ---------------------------------------------------------------------------

/// Summary of a parse: `Ok((statement_count, trimmed_tail))` when the parser
/// returned normally, or `Err(())` when it returned a nom error (a *loud*
/// failure — acceptable under the contract, never a silent drop).
type Summary = Result<(usize, String), ()>;

fn dsl_summary(src: &str) -> Summary {
    match parse_dsl(src) {
        Ok((rem, stmts)) => Ok((stmts.len(), rem.trim().to_string())),
        Err(_) => Err(()),
    }
}

fn program_summary(src: &str) -> Summary {
    match parse_program(src) {
        Ok((rem, stmts)) => Ok((stmts.len(), rem.trim().to_string())),
        Err(_) => Err(()),
    }
}

// ---------------------------------------------------------------------------
// Deterministic proptest runner.
// ---------------------------------------------------------------------------

/// Number of generated cases per property. Honours `PROPTEST_CASES` (so CI /
/// `verify-wave3` can shrink it for a fast smoke pass) and otherwise defaults to
/// 1000 to satisfy the ">=1000 generated cases" validation criterion.
fn case_count() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000)
}

/// Build a `TestRunner` seeded from a fixed constant so the sequence of
/// generated cases is identical on every run (reproducible for CI and for
/// re-running a reported failure). On-disk failure persistence is disabled so
/// behaviour never depends on a `proptest-regressions/` file in the shared repo.
fn deterministic_runner() -> TestRunner {
    let config = Config {
        cases: case_count(),
        failure_persistence: None,
        ..Config::default()
    };
    // Fixed 32-byte seed for the ChaCha PRNG — a constant fill is all that is
    // needed for a reproducible case sequence.
    let seed: [u8; 32] = [0x42; 32];
    TestRunner::new_with_rng(config, TestRng::from_seed(RngAlgorithm::ChaCha, &seed))
}

// ---------------------------------------------------------------------------
// Grammar-aware generator: plausible, well-formed Phonon statements.
//
// Each generated statement is a SINGLE line that begins with `<ident> <sep>`
// (`$`, `#`, or `:`), so the multiline preprocessor in both parsers treats it as
// a new definition and never merges adjacent statements. That keeps statement
// boundaries unambiguous, which is exactly the property the silent-drop test
// depends on.
// ---------------------------------------------------------------------------

fn bus_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "~a", "~b", "~bass", "~lfo", "~drums", "~osc", "~kick", "out", "o1", "d1",
    ])
    .prop_map(String::from)
}

fn number() -> impl Strategy<Value = String> {
    prop_oneof![
        (0u32..4000).prop_map(|n| n.to_string()),
        (0u32..4000, 1u32..1000).prop_map(|(a, b)| format!("{a}.{b}")),
    ]
}

fn mini_token() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec!["bd", "sn", "hh", "cp", "~", "808", "c4", "e4", "1", "2"])
            .prop_map(String::from),
        (2u32..64).prop_map(|n| format!("bd*{n}")),
        (1u32..8u32, 1u32..16u32).prop_map(|(k, n)| format!("bd({k},{n})")), // euclid
        // one level of bracket / alternation nesting (bounded)
        (
            prop::sample::select(vec!["bd", "sn", "hh"]),
            prop::sample::select(vec!["bd", "sn", "hh"])
        )
            .prop_map(|(a, b)| format!("[{a} {b}]")),
    ]
}

/// A quoted mini-notation pattern: `"bd sn*2 hh(3,8)"`.
fn mini_pattern() -> impl Strategy<Value = String> {
    prop::collection::vec(mini_token(), 1..6).prop_map(|toks| format!("\"{}\"", toks.join(" ")))
}

/// A plain audio source expression (no inner `$` chaining).
fn source() -> impl Strategy<Value = String> {
    prop_oneof![
        number().prop_map(|n| format!("sine {n}")),
        number().prop_map(|n| format!("saw {n}")),
        number().prop_map(|n| format!("square {n}")),
        mini_pattern().prop_map(|p| format!("sine {p}")),
        mini_pattern().prop_map(|p| format!("s {p}")),
    ]
}

/// Tidal-style struct source injection — the exact fix-parse-dsl class:
/// `struct "pat" $ sine "N"`. A now-supported form in the `$` audio-bus path,
/// where it must never drop what follows.
fn struct_source() -> impl Strategy<Value = String> {
    (mini_pattern(), number()).prop_map(|(p, n)| format!("struct {p} $ sine \"{n}\""))
}

/// Sources valid on the `$` audio-bus path: plain sources plus struct injection.
fn audio_source() -> impl Strategy<Value = String> {
    prop_oneof![source(), struct_source()]
}

/// A `$`-chained pattern transform.
fn transform() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("rev".to_string()),
        (2u32..8).prop_map(|n| format!("fast {n}")),
        (2u32..8).prop_map(|n| format!("slow {n}")),
        (2u32..8).prop_map(|n| format!("every {n} rev")),
    ]
}

/// A `#`-chained filter / parameter modifier.
fn filter() -> impl Strategy<Value = String> {
    prop_oneof![
        number().prop_map(|c| format!("lpf {c} 0.8")),
        number().prop_map(|c| format!("hpf {c} 0.5")),
        number().prop_map(|g| format!("gain {g}")),
    ]
}

/// A single, plausible, well-formed statement (one line).
fn valid_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        // audio source with an optional filter (`#`) chain
        (bus_name(), audio_source(), prop::collection::vec(filter(), 0..3)).prop_map(
            |(bus, src, filters)| {
                let mut s = format!("{bus} $ {src}");
                for f in filters {
                    s.push_str(&format!(" # {f}"));
                }
                s
            }
        ),
        // audio source with an optional transform (`$`) chain
        (bus_name(), audio_source(), prop::collection::vec(transform(), 0..3)).prop_map(
            |(bus, src, xforms)| {
                let mut s = format!("{bus} $ {src}");
                for t in xforms {
                    s.push_str(&format!(" $ {t}"));
                }
                s
            }
        ),
        // modifier bus. NOTE: uses `source()` (no struct injection), NOT
        // `audio_source()`. `# struct "pat" $ src` is a KNOWN silent-drop BUG in
        // parse_dsl's modifier-bus path (tracked by follow-up task
        // `fix-parse-dsl-2`; reproduced by the #[ignore]d test below). Re-enable
        // struct injection here — swap `source()` -> `audio_source()` — once that
        // fix lands, so the silent-drop property covers the modifier path too.
        (bus_name(), source()).prop_map(|(bus, src)| format!("{bus} # {src}")),
        // scalar config
        number().prop_map(|n| format!("tempo: {n}")),
        number().prop_map(|n| format!("bpm: {n}")),
        number().prop_map(|n| format!("cps: {n}")),
    ]
}

fn valid_program() -> impl Strategy<Value = String> {
    prop::collection::vec(valid_statement(), 1..6).prop_map(|v| v.join("\n"))
}

// ---------------------------------------------------------------------------
// Adversarial / garbage generator: structured-random + mutated source.
// ---------------------------------------------------------------------------

/// Characters that stress the tokenizer: quotes, brackets, separators, digits,
/// whitespace, escapes, and a stray unicode letter.
const NASTY: &[char] = &[
    '"', '\'', '[', ']', '<', '>', '(', ')', '{', '}', '$', '#', '~', '*', ':', ';', ',', '\\',
    '.', '-', '\n', '\t', ' ', '0', '9', 'e', 'λ',
];

fn nasty_char() -> impl Strategy<Value = char> {
    prop::sample::select(NASTY.to_vec())
}

fn garbage_token() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "$",
        "#",
        "\"",
        "[",
        "]",
        "<>",
        "(",
        ")",
        "*999999999",
        "*99999999999999999999999999999",
        "~",
        "}}}",
        "\\x",
        "λ",
        "🎵",
        "--",
    ])
    .prop_map(String::from)
}

/// A well-formed program with random nasty characters inserted at random
/// positions (unbalances quotes/brackets, splices separators mid-token, etc.).
fn mutated_program() -> impl Strategy<Value = String> {
    (
        valid_program(),
        prop::collection::vec((any::<prop::sample::Index>(), nasty_char()), 0..12),
    )
        .prop_map(|(prog, inserts)| {
            let mut chars: Vec<char> = prog.chars().collect();
            for (idx, ch) in inserts {
                let pos = idx.index(chars.len() + 1);
                chars.insert(pos, ch);
            }
            chars.into_iter().collect()
        })
}

/// The full adversarial source strategy fed to the no-panic property.
fn garbage_source() -> impl Strategy<Value = String> {
    prop_oneof![
        // arbitrary unicode (control chars, emoji, RTL, everything), bounded
        prop::collection::vec(any::<char>(), 0..200).prop_map(|cs| cs.into_iter().collect()),
        // dense string of only structural / nasty characters
        prop::collection::vec(nasty_char(), 0..200).prop_map(|cs| cs.into_iter().collect()),
        // a clean well-formed program (baseline — must also never panic)
        valid_program(),
        // a well-formed program with characters spliced in
        mutated_program(),
        // valid statements interleaved with lone garbage tokens
        prop::collection::vec(
            prop_oneof![valid_statement(), garbage_token()],
            1..10
        )
        .prop_map(|v| v.join("\n")),
    ]
}

// ---------------------------------------------------------------------------
// Property 1: neither front-end ever panics on any input.
//
// The parser is called directly (no catch_unwind): a panic propagates and
// proptest automatically shrinks it to a minimal reproducer.
// ---------------------------------------------------------------------------

fn assert_no_panic(label: &str, parse: impl Fn(&str)) {
    let mut runner = deterministic_runner();
    let result = runner.run(&garbage_source(), |src| {
        parse(&src); // a panic here is caught + shrunk by proptest
        Ok(())
    });
    if let Err(e) = result {
        panic!("no-panic property failed ({label}): {e}");
    }
}

#[test]
fn test_dsl_fuzz_parse_dsl_never_panics() {
    assert_no_panic("parse_dsl", |s| {
        let _ = parse_dsl(s);
    });
}

#[test]
fn test_dsl_fuzz_parse_program_never_panics() {
    assert_no_panic("parse_program", |s| {
        let _ = parse_program(s);
    });
}

// ---------------------------------------------------------------------------
// Property 2: no silent statement drop (the fix-parse-dsl class, generalised).
//
// Strategy: generate a list of candidate statements; keep only those that each
// parse *in isolation* to exactly one statement with an empty tail (this is
// self-calibrating per front-end — the two grammars diverge, so each parser is
// only asked about the forms it actually accepts). The concatenation of N
// independently-valid statements MUST parse into N statements with an empty
// tail. Any shortfall — a lost statement or a leftover tail — is a silent drop,
// which is exactly the failure `fix-parse-dsl` repaired.
//
// This is false-positive-free: a statement a parser does not support is filtered
// out before composition, so the test only ever flags a genuine regression where
// individually-valid statements stop composing.
// ---------------------------------------------------------------------------

fn assert_no_silent_drop(label: &str, summary: impl Fn(&str) -> Summary) {
    let mut runner = deterministic_runner();
    let strat = prop::collection::vec(valid_statement(), 1..7);
    let result = runner.run(&strat, |candidates| {
        // Keep statements that individually parse cleanly (1 stmt, empty tail).
        let kept: Vec<String> = candidates
            .iter()
            .filter(|s| matches!(summary(s), Ok((1, ref tail)) if tail.is_empty()))
            .cloned()
            .collect();
        prop_assume!(!kept.is_empty());

        let program = kept.join("\n");
        match summary(&program) {
            Ok((count, tail)) => {
                prop_assert!(
                    tail.is_empty(),
                    "SILENT DROP ({}): non-empty tail {:?} after a well-formed program:\n---\n{}\n---",
                    label,
                    tail,
                    program
                );
                // A tail that begins with a chain operator means a statement was
                // cut mid-chain (the precise fix-parse-dsl signature).
                prop_assert!(
                    !tail.starts_with('$') && !tail.starts_with('#'),
                    "DANGLING CHAIN OP ({}): tail {:?} after:\n---\n{}\n---",
                    label,
                    tail,
                    program
                );
                prop_assert_eq!(
                    count,
                    kept.len(),
                    "STATEMENT DROPPED ({}): parsed {} of {} independently-valid statements:\n---\n{}\n---",
                    label,
                    count,
                    kept.len(),
                    program
                );
            }
            Err(()) => {
                // A loud parser error on a program built from independently-valid
                // statements is unexpected; surface it (it is not a silent drop,
                // but it signals a grammar composition regression worth knowing).
                prop_assert!(
                    false,
                    "well-formed program returned a parser error ({}):\n---\n{}\n---",
                    label,
                    program
                );
            }
        }
        Ok(())
    });
    if let Err(e) = result {
        panic!("silent-drop property failed ({label}): {e}");
    }
}

#[test]
fn test_dsl_fuzz_parse_dsl_no_silent_drop() {
    assert_no_silent_drop("parse_dsl", dsl_summary);
}

#[test]
fn test_dsl_fuzz_parse_program_no_silent_drop() {
    assert_no_silent_drop("parse_program", program_summary);
}

// ---------------------------------------------------------------------------
// Permanent regression SEED: the exact fix-parse-dsl reproducer.
//
// Pre-fix, `parse_dsl` consumed only `struct "t(3,8,1)"`, left `$ sine "66"...`
// as an unparsed tail, and SILENTLY dropped the trailing `out $ ...` statement
// (a struct-with-no-source => total silence). This test would have FAILED before
// fix-parse-dsl (d1d8a75) and passes now. It is the named regression case
// required by the task's validation.
// ---------------------------------------------------------------------------

#[test]
fn test_dsl_fuzz_no_silent_statement_drop() {
    // Two statements: a struct-chained source, then a trailing `out` that MUST
    // survive. Pre-fix this parsed to a single statement with a `$ sine ...` tail.
    let code = "~a $ struct \"t(3,8,1)\" $ sine \"66\"\nout $ ~a * 0.5";

    let (remaining, statements) = parse_dsl(code).expect("parse_dsl must not error");

    assert!(
        remaining.trim().is_empty(),
        "trailing statement was silently dropped; unparsed tail = {:?}",
        remaining
    );
    assert_eq!(
        statements.len(),
        2,
        "both statements (struct source + trailing `out`) must survive, got {}",
        statements.len()
    );
}

// ---------------------------------------------------------------------------
// KNOWN BUG reproducer (discovered by this harness), tracked by follow-up task
// `fix-parse-dsl-2`. `fix-parse-dsl` repaired `$`-source injection for the AUDIO
// bus, but the `#` MODIFIER bus path still SILENTLY DROPS the `$ src` tail (and
// every statement after it) for `# struct "pat" $ src` when it is not the first
// statement. This test asserts the FIXED behavior, so it FAILS on current `main`
// and is #[ignore]d until the follow-up lands. When fixing the parser, remove the
// `#[ignore]` (turning this into a live regression guard) and re-enable the
// `# struct` generator arm in `valid_statement`.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "known parser bug, tracked by fix-parse-dsl-2: # modifier-bus struct chaining drops following statements"]
fn test_dsl_fuzz_known_bug_modifier_bus_struct_chaining_drops() {
    // A struct-chained MODIFIER bus followed by a trailing `out` statement. The
    // trailing statement must survive once the modifier-bus path reaches parity
    // with the already-fixed audio-bus path.
    let code = "~a $ sine 0\n~b # struct \"bd\" $ sine \"0\"\nout $ ~a";

    let (remaining, statements) = parse_dsl(code).expect("parse_dsl must not error");

    assert!(
        remaining.trim().is_empty(),
        "modifier-bus struct chaining left an unparsed tail: {:?}",
        remaining
    );
    assert_eq!(
        statements.len(),
        3,
        "all three statements must survive (modifier-bus struct chaining), got {}",
        statements.len()
    );
}

// ---------------------------------------------------------------------------
// Malformed corpus: a fixed set of deliberately-broken inputs run through BOTH
// front-ends under catch_unwind. Every one must return an error/empty parse,
// never panic. Serves as a permanent, human-readable regression corpus.
// ---------------------------------------------------------------------------

#[test]
fn test_dsl_fuzz_malformed_corpus_no_panic() {
    let corpus: Vec<String> = vec![
        // empty / whitespace-only
        "".into(),
        " ".into(),
        "\n\n\t  \n".into(),
        "-- just a comment".into(),
        // unbalanced quotes
        "~a $ sine \"".into(),
        "~a $ s \"bd sn".into(),
        "out $ s \"bd\" sn\"".into(),
        // unbalanced brackets / alternation
        "~a $ s \"[bd sn\"".into(),
        "~a $ s \"bd]\"".into(),
        "~a $ s \"<bd sn\"".into(),
        "~a $ s \"bd>\"".into(),
        // missing / dangling operands
        "~a $ sine 440 # lpf".into(),
        "out $ struct \"t(3,8,1)\" $".into(),
        "~a $ ".into(),
        "$ $ $ $".into(),
        "# # # #".into(),
        // giant / overflowing repeat counts
        "~a $ s \"bd*999999999\"".into(),
        "~a $ s \"bd*99999999999999999999999999999999\"".into(),
        // scalar configs with junk
        "tempo:".into(),
        "bpm: ".into(),
        "cps: abc".into(),
        // unicode identifiers / pattern content
        "~🎵 $ sïne \"béd\"".into(),
        "λλλ $$$ ### ~~~".into(),
        // walls of structural characters
        "((((((((((".into(),
        "))))))))))".into(),
        "[[[[[[[[[[".into(),
        "]]]]]]]]]]".into(),
        "<<<<<<<<<<".into(),
        "\"\"\"\"\"\"\"".into(),
        // moderately deep (bounded) bracket nesting inside a pattern
        format!("~a $ s \"{}bd{}\"", "[".repeat(48), "]".repeat(48)),
        // the fix-parse-dsl reproducer, with a trailing statement
        "out $ struct \"t(3,8,1)\" $ sine \"66\"\nout $ sine 440".into(),
    ];

    for input in &corpus {
        let s1 = input.clone();
        let dsl_ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = parse_dsl(&s1);
        }))
        .is_ok();
        assert!(dsl_ok, "parse_dsl PANICKED on malformed input {:?}", input);

        let s2 = input.clone();
        let prog_ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = parse_program(&s2);
        }))
        .is_ok();
        assert!(
            prog_ok,
            "parse_program PANICKED on malformed input {:?}",
            input
        );
    }
}

// ---------------------------------------------------------------------------
// Termination: pathological (but bounded) inputs must parse within a generous
// wall-clock budget. The budget is deliberately huge (30s) relative to the
// microsecond-scale real parse time, so this catches a true infinite loop /
// runaway without false-failing under CPU oversubscription — it is a liveness
// safety net, NOT a performance gate.
// ---------------------------------------------------------------------------

fn assert_terminates(label: &str, f: impl FnOnce() + Send + 'static) {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let handle = std::thread::Builder::new()
        // Generous stack so bounded-depth mini-notation nesting recurses safely.
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            f();
            let _ = tx.send(());
        })
        .expect("failed to spawn watchdog thread");

    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
        Ok(()) => {
            let _ = handle.join();
        }
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            let preview: String = label.chars().take(80).collect();
            panic!(
                "parse did NOT terminate within 30s budget (input len {}): {:?}",
                label.len(),
                preview
            );
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            // Thread panicked before signalling; re-surface the panic.
            if let Err(p) = handle.join() {
                std::panic::resume_unwind(p);
            }
        }
    }
}

#[test]
fn test_dsl_fuzz_terminates_on_pathological_input() {
    let cases: Vec<String> = vec![
        // absurdly long repeat-count digit run (parsed, never expanded at parse time)
        format!("~a $ s \"bd*{}\"", "9".repeat(300)),
        // a very long flat mini-notation pattern
        format!("~a $ s \"{}\"", "bd ".repeat(5000)),
        // a very long `#` filter chain
        format!("~a $ sine 1 {}", "# lpf 1000 0.8 ".repeat(2000)),
        // a very long `$` transform chain
        format!("~a $ s \"bd\" {}", "$ fast 2 ".repeat(2000)),
        // many statements
        (0..2000)
            .map(|i| format!("~b{i} $ sine {i}"))
            .collect::<Vec<_>>()
            .join("\n"),
        // bounded deep bracket nesting inside a pattern (exercises recursion)
        format!("~a $ s \"{}bd{}\"", "[".repeat(64), "]".repeat(64)),
    ];

    for input in cases {
        let owned = input.clone();
        assert_terminates(&input, move || {
            let _ = parse_dsl(&owned);
            let _ = parse_program(&owned);
        });
    }
}
