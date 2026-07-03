//! Render-thread lock hardening tests.
//!
//! Covers audit findings F-2 (VST3 plugin path) and F-3 (per-node / clock mutexes)
//! from docs/audits/rt-safety-2026-07.md.  The render thread must never:
//!   * hold per-node scalar state behind a `Mutex<f32>` (locked 4x/sample), or
//!   * call `.lock().unwrap()` (a poisoned lock would panic and kill audio).
//!
//! These are enforced structurally (the source must not contain the banned
//! constructs) and behaviourally (a poisoned render-path mutex degrades to
//! silence instead of panicking).

use std::fs;

fn unified_graph_src() -> String {
    // Integration tests run with CWD = crate root.
    fs::read_to_string("src/unified_graph.rs").expect("read src/unified_graph.rs")
}

/// F-3: per-node scalar state (the audio->pattern sample&hold) must be lock-free.
/// It used two `Arc<Mutex<f32>>` cells locked four times every sample with
/// `.unwrap()` on the hot path — a poisoned lock would panic the render thread,
/// and a scalar cell does not need a mutex at all.
#[test]
fn test_per_node_scalar_state_is_lock_free() {
    let src = unified_graph_src();
    assert!(
        !src.contains("Mutex<f32>"),
        "per-node scalar state must be lock-free: no Mutex<f32> may remain in unified_graph.rs"
    );
    // The replacement stores the f32 bit pattern in an atomic cell.
    assert!(
        src.contains("AtomicU32"),
        "SignalAsPattern sample&hold state should use a lock-free AtomicU32 cell"
    );
}

/// F-2/F-3: no `.lock().unwrap()` may remain on the render path.  A panic anywhere
/// poisons the mutex; `.unwrap()` on a poisoned lock then panics the render thread,
/// draining the ring and stopping audio permanently.  Every render-path lock must
/// use `try_lock` / `if let Ok(..)` with a graceful (silence) fallback.
#[test]
fn test_no_lock_unwrap_on_render_path() {
    let src = unified_graph_src();
    let count = src.matches(".lock().unwrap()").count();
    assert_eq!(
        count, 0,
        "render path must not use .lock().unwrap() ({count} occurrence(s) remain)"
    );
}

/// F-2: the VST3 real-plugin render branch must not allocate a fresh
/// `Vec<(String, f32)>` (with a cloned param name per entry) every sample. Parameter
/// values are evaluated into a reused, thread-local scratch buffer via
/// `PluginParamScratch`, which retains capacity across samples and clones no names.
#[cfg(feature = "vst3")]
#[test]
fn test_vst3_param_path_uses_reused_scratch() {
    let src = unified_graph_src();
    assert!(
        src.contains("PluginParamScratch::acquire()"),
        "the VST3 render branch must use the reused param scratch, not a per-sample Vec alloc"
    );
    // The real-plugin apply loop must zip params against the scratch values rather
    // than collecting a fresh Vec of (name.clone(), value) each sample.
    assert!(
        src.contains("params.iter().zip(param_values.iter())"),
        "the VST3 param apply loop should zip the reused scratch against params"
    );
}

/// F-2: a poisoned `real_plugins` mutex must not crash the render thread.  Before
/// the fix the plugin branch did `self.real_plugins.lock().unwrap()`, so a panic in
/// any lock holder poisoned the mutex and the next render tick panicked.  After the
/// fix the branch uses `try_lock` and degrades to silence.
#[cfg(feature = "vst3")]
#[test]
fn test_poisoned_real_plugins_lock_degrades_to_silence() {
    use phonon::unified_graph::{SignalNode, UnifiedSignalGraph};
    use std::cell::{Cell, RefCell};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    let mut graph = UnifiedSignalGraph::new(44100.0);

    // A plugin instrument referencing a plugin that is neither a mock nor a real
    // installed plugin, so eval reaches the real-VST3 branch (which locks
    // `real_plugins`) rather than the mock branch.
    let note_pattern = phonon::pattern::Pattern::pure(69.0);
    let plugin_node = SignalNode::PluginInstance {
        plugin_id: "definitely-not-a-real-plugin-xyz".to_string(),
        audio_inputs: vec![],
        params: HashMap::new(),
        note_pattern: Some(note_pattern),
        note_pattern_str: Some("69".to_string()),
        last_note_cycle: Cell::new(-1),
        triggered_notes: RefCell::new(HashSet::new()),
        cached_note_events: RefCell::new(Vec::new()),
        instance: RefCell::new(None),
        last_processed_end: Cell::new(-1.0),
    };
    let node_id = graph.add_node(plugin_node);
    graph.set_output(node_id);

    // Poison the shared render-path mutex: a thread panics while holding the lock.
    let poison_target = Arc::clone(&graph.real_plugins);
    let joined = std::thread::spawn(move || {
        let _guard = poison_target.lock().unwrap();
        panic!("intentionally poison the real_plugins mutex");
    })
    .join();
    assert!(joined.is_err(), "helper thread should have panicked");
    assert!(
        graph.real_plugins.lock().is_err(),
        "real_plugins mutex should be poisoned by the panicking thread"
    );

    // The render path must NOT panic on the poisoned lock; it must degrade to silence.
    let mut output = vec![0.0f32; 1024];
    graph.process_buffer_dag(&mut output, 0.0, 1.0 / 44100.0);

    let rms: f32 = (output.iter().map(|s| s * s).sum::<f32>() / output.len() as f32).sqrt();
    assert!(
        rms < 1e-6,
        "poisoned-lock render path must degrade to silence, got RMS={rms}"
    );
}

/// The audio->pattern sample&hold node must still round-trip its sampled value
/// through its (now lock-free) state cell without panicking during a render.
/// `fast ~lfo` uses an audio bus as a pattern-transform parameter, which compiles
/// to a `SignalAsPattern` node whose scalar state cell is read/written each sample.
#[test]
fn test_signal_as_pattern_renders_without_panic() {
    use phonon::compositional_compiler::compile_program;
    use phonon::compositional_parser::parse_program;

    let dsl = "tempo: 0.5\n~lfo $ sine 2\nout $ s \"bd sn hh cp\" $ fast ~lfo\n";
    let (_remaining, statements) = parse_program(dsl).expect("parse SignalAsPattern DSL");
    let mut graph =
        compile_program(statements, 44100.0, None).expect("compile SignalAsPattern DSL");

    // Render several buffers so the sample&hold crosses a cycle boundary and both
    // the write (node eval) and read (pattern closure) paths run.  Must not panic.
    let mut output = vec![0.0f32; 2048];
    for _ in 0..8 {
        graph.process_buffer_dag(&mut output, 0.0, 1.0 / 44100.0);
    }
}
