//! Regression test for rt-safety audit F-7 (task `parser-arena-no-leak`).
//!
//! `parse_program` / `parse_program_with_macros` used to call `Box::leak` on the
//! preprocessed and macro-expanded source strings, leaking one (or two) copies of
//! the source on *every* parse. A live-coding session issues one parse per edit /
//! reload, so hundreds of edits grew resident memory without bound.
//!
//! This test installs a counting global allocator and asserts that repeatedly
//! parsing the same program does NOT grow net-live heap proportionally to the
//! number of parses. Before the fix, each parse permanently leaks ~`source.len()`
//! bytes, so net-live grows by `N * source.len()`; after the fix it stays flat.
//!
//! NOTE: the counter is process-global, so the measurement is only meaningful
//! when nothing else is allocating concurrently. Everything therefore lives in a
//! SINGLE `#[test]` (libtest runs it on one worker thread while the main thread
//! blocks), and the two sub-measurements run sequentially, never overlapping.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

use phonon::compositional_parser::{parse_program, parse_program_with_macros};

/// A pass-through allocator that tracks total bytes allocated and freed so a test
/// can measure *net-live* heap (allocated - freed) around a region of work.
struct CountingAlloc;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static FREED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        FREED.fetch_add(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Net bytes currently live on the heap (allocated minus freed).
fn net_live_bytes() -> i64 {
    ALLOCATED.load(Ordering::Relaxed) as i64 - FREED.load(Ordering::Relaxed) as i64
}

/// A realistic multi-statement program with several buses and transforms,
/// including quoted mini-notation strings.
fn sample_program() -> String {
    r#"
cps: 2.0
~drums $ s "bd sn hh*4 cp" $ fast 2
~bass $ saw "55 82.5 110 82.5" # lpf 1200 0.8
~lead $ sine "220 330 440 550" # lpf 2000 0.7 # delay 0.25 0.4
~lfo # sine 0.25
~filtered $ saw 110 # lpf (~lfo * 1500 + 500) 0.8
out $ ~drums * 0.4 + ~bass * 0.3 + ~lead * 0.2 + ~filtered * 0.1
"#
    .to_string()
}

/// A program that also exercises macro expansion (the second `Box::leak` site).
fn sample_program_with_macros() -> String {
    r#"
cps: 2.0
for i in 1..5:
    ~osc[i] $ sine (110 * i) # lpf 1500 0.7
out $ sum(~osc[1..5]) * 0.15
"#
    .to_string()
}

/// Runs `f` `n` times, returning net-live heap growth (bytes) across the run.
/// Warms up first so lazy statics / one-time caches don't pollute the measurement.
fn heap_growth_over<F: Fn()>(n: usize, f: F) -> i64 {
    // Warm up: initialize any lazy statics, thread-locals, allocator arenas.
    for _ in 0..40 {
        f();
    }
    let before = net_live_bytes();
    for _ in 0..n {
        f();
    }
    let after = net_live_bytes();
    after - before
}

#[test]
fn parser_does_not_leak_across_many_parses() {
    const N: usize = 2_000;

    // --- Sanity: the fix must not change parse results. ---
    let program = sample_program();
    let (rest, statements) = parse_program(&program).expect("sample should parse");
    assert!(rest.trim().is_empty(), "expected full parse, leftover: {rest:?}");
    // cps, ~drums, ~bass, ~lead, ~lfo, ~filtered, out => 7 statements.
    assert_eq!(statements.len(), 7, "sample program should parse to 7 statements");

    // --- parse_program: first Box::leak site (preprocessed source). ---
    let growth = heap_growth_over(N, || {
        let (rest, statements) = parse_program(black_box(&program)).expect("program should parse");
        // Fully consume + drop the parse result so only *leaked* memory persists.
        assert!(rest.trim().is_empty());
        black_box(statements.len());
    });
    let per_parse = growth / N as i64;
    // Before the fix: per_parse >= program.len() (a leaked boxed str per parse).
    // After the fix:  per_parse ~= 0. Threshold sits far below the leak signal
    // and far above steady-state allocator noise.
    let threshold = (program.len() / 8) as i64;
    assert!(
        per_parse < threshold,
        "parse_program leaked ~{per_parse} bytes/parse over {N} parses \
         (program is {} bytes, threshold {threshold}); total growth {growth} bytes. \
         A Box::leak site is still present.",
        program.len()
    );

    // --- parse_program_with_macros: second Box::leak site (expanded source), ---
    // --- run sequentially so the global counter is uncontended. ---
    let mprogram = sample_program_with_macros();
    let mgrowth = heap_growth_over(N, || {
        let (_rest, statements) =
            parse_program_with_macros(black_box(&mprogram)).expect("macro program should parse");
        black_box(statements.len());
    });
    let mper_parse = mgrowth / N as i64;
    // `parse_program_with_macros` used to leak BOTH the expanded string and (via
    // its call to parse_program) the preprocessed string — two leaks per parse.
    let mthreshold = (mprogram.len() / 8) as i64;
    assert!(
        mper_parse < mthreshold,
        "parse_program_with_macros leaked ~{mper_parse} bytes/parse over {N} parses \
         (program is {} bytes, threshold {mthreshold}); total growth {mgrowth} bytes. \
         A Box::leak site is still present.",
        mprogram.len()
    );
}
