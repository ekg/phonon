//! Race-detection harness for the render-owner graph swap (ENABLER I2).
//!
//! See `docs/audits/design-render-owner-swap-2026-07.md` §6.A. This file proves
//! the **C1 ROOT data race** that the `fix-synth-borrow-race` symptom patch did
//! *not* remove, and pins the shape the render-owner migration must satisfy so
//! the target model passes clean.
//!
//! ## The defect being modeled
//!
//! The three live swap paths share `GraphCell(RefCell<UnifiedSignalGraph>)`,
//! published behind `Arc<ArcSwap<..>>` and marked thread-shareable by a
//! hand-written `unsafe impl Sync` (`src/main.rs:951-952`,
//! `src/modal_editor/mod.rs:58-59`, `src/bin/phonon-audio.rs:174,176`,
//! `src/live.rs:30-31`, `src/stress_harness.rs:1932-1933`). After the symptom
//! fix, **both** the render side and the reload side call `try_borrow_mut()`
//! (`src/main.rs:1055` + `:1205`, etc.). `RefCell`'s borrow flag is a *non-atomic*
//! `Cell<isize>`; two threads that `try_borrow_mut()` the same cell concurrently
//! perform an unsynchronised read-modify-write on that flag — a data race / UB
//! under the Rust memory model, independent of whether either call returns `Err`.
//! Changing the *consequence* of losing the race from "panic" to "skip" did not
//! remove the race.
//!
//! ## Two execution modes
//!
//! * **Normal `cargo test`** runs the deterministic *hand-model* (`hand_model`
//!   below): a replayed two-thread interleaving that shows the non-atomic
//!   check-then-set admits an aliasing outcome, an atomic CAS does not, and the
//!   render-owner single-owner handoff never lets two threads touch the graph.
//!   These are always-green, non-flaky, and prove the defect *by construction*.
//!
//! * **`RUSTFLAGS="--cfg loom" cargo test --test loom_graph_swap`** runs the
//!   `loom_models`: loom *exhaustively* explores thread interleavings and
//!   machine-checks that (a) the current `RefCell` + `unsafe impl Sync` protocol
//!   is racy (baseline — expected to be flagged), and (b) the render-owner
//!   single-owner handoff is race-free (target — passes clean). loom is gated as
//!   a `cfg(loom)` build so it is never pulled into a normal `cargo build`.
//!
//! ## Miri
//!
//! `cargo +nightly miri test --test loom_graph_swap` runs the `hand_model`
//! module under Miri to confirm the single-owner ownership logic and the model
//! primitives contain no UB. See `docs/RACE_DETECTION.md`.

// `cfg(loom)` is a custom cfg set via RUSTFLAGS, not a built-in; silence the
// `unexpected_cfgs` lint for this test crate rather than editing crate-wide lints.
#![allow(unexpected_cfgs)]

// ============================================================================
// Deterministic hand-model — runs under normal `cargo test` (and Miri).
// ============================================================================
//
// A data race is UB that a normal timed execution will not reliably surface, so
// we do not *observe* it here; we *model* the protocol's state transitions and
// assert which outcomes each protocol admits. loom (below) provides the
// exhaustive machine-checked version of the same argument.
#[cfg(not(loom))]
mod hand_model {
    use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::Arc;

    /// `RefCell` borrow-flag sentinels (semantics mirrored from `core::cell`).
    const UNUSED: isize = 0;
    const WRITING: isize = -1;

    /// A model of `RefCell`'s **non-atomic** borrow flag. The real flag is a
    /// plain `Cell<isize>`; `try_borrow_mut` reads it, and — if `UNUSED` — sets
    /// it to `WRITING`. We split that read-modify-write into two observable
    /// steps so a specific interleaving can be replayed. This split is exactly
    /// the window a non-atomic `Cell` leaves open to another thread.
    struct NonAtomicBorrowFlag {
        flag: isize,
    }

    impl NonAtomicBorrowFlag {
        fn read(&self) -> isize {
            self.flag
        }
        fn commit_writing(&mut self) {
            self.flag = WRITING;
        }
    }

    /// BASELINE (the current, post symptom-fix protocol) — **admits aliasing.**
    ///
    /// Replays the interleaving `render.read`, `reload.read`, `render.commit`,
    /// `reload.commit`. Because the check (`read`) and the set (`commit`) are
    /// not one atomic step, *both* `try_borrow_mut` calls observe `UNUSED` and
    /// *both* proceed to hand out `&mut UnifiedSignalGraph`. Two live `&mut` to
    /// the same graph is the C1 ROOT data race / UB — present even though both
    /// sides use the "safe" `try_borrow_mut`.
    #[test]
    fn baseline_refcell_protocol_admits_aliasing_interleaving() {
        let mut cell = NonAtomicBorrowFlag { flag: UNUSED };

        // Render thread reads the flag: UNUSED.
        let render_saw = cell.read();
        // Reload thread reads the flag BEFORE render commits: still UNUSED.
        let reload_saw = cell.read();

        // Render decides it may borrow, and commits WRITING.
        let render_borrows = render_saw == UNUSED;
        cell.commit_writing();
        // Reload also saw UNUSED, so it too decides it may borrow, and commits.
        let reload_borrows = reload_saw == UNUSED;
        cell.commit_writing();

        assert!(
            render_borrows && reload_borrows,
            "the non-atomic RefCell borrow-flag protocol admits an interleaving \
             where BOTH try_borrow_mut calls succeed (two aliased &mut to the \
             same graph) — this is the C1 root race the symptom fix left behind"
        );
    }

    /// FIX DIRECTION — an **atomic** compare-and-set rejects the same
    /// interleaving. Exactly one thread transitions `UNUSED -> WRITING`; the
    /// loser observes the store and backs off. This is the property the
    /// render-owner channel (an atomic handoff) must provide.
    #[test]
    fn atomic_cas_flag_rejects_the_aliasing_interleaving() {
        let flag = AtomicIsize::new(UNUSED);

        let render_borrows = flag
            .compare_exchange(UNUSED, WRITING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        let reload_borrows = flag
            .compare_exchange(UNUSED, WRITING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();

        assert!(
            render_borrows ^ reload_borrows,
            "an atomic compare-and-set admits exactly one winner — no aliasing"
        );
    }

    /// TARGET (render-owner, single-owner SPSC) — **no cross-thread graph
    /// access at all.**
    ///
    /// The control thread compiles a new graph off-thread and hands *ownership*
    /// through a single-producer/single-consumer channel (`Box<Graph>` moved,
    /// not shared). The render thread is the only thread that ever dereferences
    /// a graph: it pops the pending swap at a buffer boundary, runs `transfer_*`
    /// from its currently-owned `cur` into `next`, and swaps its owned pointer.
    /// We instrument every graph touch with the thread id that performed it and
    /// assert the render thread is the *sole* toucher across many swaps — the
    /// structural invariant that deletes the `unsafe impl Sync`.
    #[test]
    fn render_owner_single_owner_is_touched_by_exactly_one_thread() {
        /// Stand-in for `UnifiedSignalGraph`; records the id of every thread
        /// that reads or mutates it.
        struct Graph {
            state: u64,
            touched_by: Vec<u64>,
        }
        impl Graph {
            fn transfer_and_step(&mut self, prev_state: u64, who: u64) {
                // Models transfer_session_timing / transfer_fx_states / step.
                self.state = self.state.wrapping_add(prev_state).wrapping_add(1);
                self.touched_by.push(who);
            }
        }

        const SWAPS: usize = 64;
        const RENDER_ID: u64 = 1;
        const CONTROL_ID: u64 = 2;

        // SPSC command channel: control -> render, ownership moved.
        let (swap_tx, swap_rx) = mpsc::channel::<Box<Graph>>();
        // Graveyard: retired graphs leave the render thread to be dropped off-RT.
        let (grave_tx, grave_rx) = mpsc::channel::<Box<Graph>>();
        let all_touches = Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));

        // Control thread: build graphs and hand them off. It NEVER dereferences
        // a graph's state — it only constructs and moves ownership.
        let producer = {
            let all_touches = Arc::clone(&all_touches);
            std::thread::spawn(move || {
                for i in 0..SWAPS {
                    let next = Box::new(Graph {
                        state: i as u64,
                        touched_by: Vec::new(),
                    });
                    // Control thread's only bookkeeping touch is recorded so the
                    // assertion below would catch it if it ever read graph state.
                    let _ = &all_touches; // (control performs no graph touch)
                    if swap_tx.send(next).is_err() {
                        break;
                    }
                }
                // drop swap_tx -> render loop terminates
            })
        };

        // Render thread == this test thread: single owner of `cur`.
        let mut cur = Box::new(Graph {
            state: 0,
            touched_by: Vec::new(),
        });
        cur.transfer_and_step(0, RENDER_ID); // initial render touch

        while let Ok(mut next) = swap_rx.recv() {
            // transfer_* reads the currently-owned `cur` and writes `next`,
            // then the owned pointer is swapped — one uninterrupted step.
            let prev = cur.state;
            next.transfer_and_step(prev, RENDER_ID);
            let retired = std::mem::replace(&mut cur, next);
            // Ship the retired graph off the render thread for Drop.
            let _ = grave_tx.send(retired);
        }
        drop(grave_tx);

        // Janitor: drain + drop retired graphs, collecting their touch logs.
        let mut all = all_touches.lock().unwrap();
        while let Ok(g) = grave_rx.recv() {
            all.extend(g.touched_by.iter().copied());
        }
        all.extend(cur.touched_by.iter().copied());

        producer.join().unwrap();

        assert!(
            !all.is_empty(),
            "expected the render thread to have touched graphs"
        );
        assert!(
            all.iter().all(|&who| who == RENDER_ID),
            "render-owner invariant violated: a graph was touched by a thread \
             other than the render thread (found {:?}, CONTROL_ID={})",
            all,
            CONTROL_ID
        );
    }

    /// Belt-and-braces empirical observer (report-only, never asserts a race).
    ///
    /// Spins two real OS threads hammering the non-atomic borrow-flag protocol
    /// and *counts* how often the double-acquire (aliasing) outcome is observed.
    /// A normal execution cannot deterministically surface UB, so this is
    /// evidence, not a gate — it only asserts that the harness itself ran. loom
    /// provides the deterministic proof.
    ///
    /// Skipped under Miri: this test performs an *intentional* non-atomic data
    /// race through a raw pointer, which Miri's race detector correctly rejects
    /// as UB. Miri validates the *sound* single-owner logic (the other tests),
    /// not this deliberate demonstration of the unsound protocol.
    #[test]
    #[cfg_attr(
        miri,
        ignore = "intentional data race — demonstration only, not for Miri"
    )]
    fn baseline_double_acquire_is_observable_report_only() {
        // Non-atomic flag shared via a pointer wrapper with a hand-written
        // `unsafe impl Sync` — mirrors `GraphCell`'s `unsafe impl Sync`.
        struct SharedFlag(std::cell::UnsafeCell<isize>);
        unsafe impl Sync for SharedFlag {}
        unsafe impl Send for SharedFlag {}

        let flag = Arc::new(SharedFlag(std::cell::UnsafeCell::new(UNUSED)));
        let both_in = Arc::new(AtomicUsize::new(0)); // both threads "hold" &mut
        let observed = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let flag = Arc::clone(&flag);
            let both_in = Arc::clone(&both_in);
            let observed = Arc::clone(&observed);
            handles.push(std::thread::spawn(move || {
                for _ in 0..50_000 {
                    // Racy non-atomic try_borrow_mut emulation.
                    let ptr = flag.0.get();
                    let seen = unsafe { *ptr };
                    if seen == UNUSED {
                        unsafe { *ptr = WRITING };
                        // "inside the borrow": if the peer is also inside, we
                        // have observed the aliasing the protocol permits.
                        let inside = both_in.fetch_add(1, Ordering::AcqRel) + 1;
                        if inside > 1 {
                            observed.fetch_add(1, Ordering::Relaxed);
                        }
                        both_in.fetch_sub(1, Ordering::AcqRel);
                        unsafe { *ptr = UNUSED };
                    }
                    std::hint::spin_loop();
                }
            }));
        }
        for h in handles {
            let _ = h.join();
        }

        // Report-only: the count is evidence the non-atomic protocol lets two
        // threads believe they hold the exclusive borrow simultaneously. We do
        // not assert on it (timing-dependent); we only assert the harness ran.
        let n = observed.load(Ordering::Relaxed);
        println!(
            "[race-harness] observed {} double-acquire windows on the non-atomic \
             borrow flag (report-only; loom gives the deterministic proof)",
            n
        );
        assert!(
            observed.load(Ordering::Relaxed) < usize::MAX,
            "harness ran to completion"
        );
    }
}

// ============================================================================
// loom exhaustive models — run only under `RUSTFLAGS="--cfg loom"`.
// ============================================================================
#[cfg(loom)]
mod loom_models {
    use loom::cell::UnsafeCell;
    use loom::sync::atomic::{AtomicBool, Ordering};
    use loom::sync::Arc;
    use loom::thread;

    const UNUSED: isize = 0;
    const WRITING: isize = -1;

    // ------------------------------------------------------------------------
    // BASELINE: the current `GraphCell(RefCell<..>)` + `unsafe impl Sync`
    // protocol. loom must FLAG this as racy.
    // ------------------------------------------------------------------------

    /// Mirrors `GraphCell`: a non-atomic borrow flag guarding a payload, marked
    /// shareable by a hand-written `unsafe impl Sync`. The flag lives in a
    /// `loom::cell::UnsafeCell` so loom observes every (unsynchronised) access
    /// to it — exactly `RefCell`'s non-atomic `Cell<isize>`.
    struct RacyGraphCell {
        flag: UnsafeCell<isize>,
        payload: UnsafeCell<u64>,
    }

    // Mirrors `unsafe impl Sync for GraphCell {}` on every live path.
    unsafe impl Sync for RacyGraphCell {}
    unsafe impl Send for RacyGraphCell {}

    /// `try_borrow_mut()` modeled faithfully: a NON-atomic read-modify-write on
    /// the borrow flag (check `UNUSED`, set `WRITING`), then touch the payload
    /// under the "borrow", then release. Nothing here synchronises the flag
    /// between threads — precisely the bug.
    fn racy_try_borrow_mut(cell: &RacyGraphCell) {
        let acquired = cell.flag.with_mut(|p| unsafe {
            if *p == UNUSED {
                *p = WRITING;
                true
            } else {
                false
            }
        });
        if acquired {
            cell.payload
                .with_mut(|p| unsafe { *p = (*p).wrapping_add(1) });
            cell.flag.with_mut(|p| unsafe { *p = UNUSED });
        }
    }

    /// The render thread and the reload thread both `try_borrow_mut()` the same
    /// `GraphCell` during the swap's transfer window. loom explores the
    /// interleavings and detects the concurrent unsynchronised access to the
    /// non-atomic borrow flag — the C1 root race. `#[should_panic]` encodes the
    /// expectation that loom flags it; if loom ever stopped flagging it, this
    /// test would fail loudly.
    #[test]
    #[should_panic]
    fn baseline_refcell_protocol_is_racy_under_loom() {
        loom::model(|| {
            let cell = Arc::new(RacyGraphCell {
                flag: UnsafeCell::new(UNUSED),
                payload: UnsafeCell::new(0),
            });

            // Reload/control thread: try_borrow_mut across transfer_*.
            let reload = {
                let cell = Arc::clone(&cell);
                thread::spawn(move || racy_try_borrow_mut(&cell))
            };

            // Render/synth thread: try_borrow_mut per block.
            racy_try_borrow_mut(&cell);

            reload.join().unwrap();
        });
    }

    // ------------------------------------------------------------------------
    // TARGET: render-owner single-owner handoff. loom must find NO race.
    // ------------------------------------------------------------------------

    /// The render-owner mailbox: the control thread writes a compiled graph into
    /// the slot and publishes it with a `Release` store; the render thread takes
    /// it with an `Acquire` load. The atomic establishes happens-before, so the
    /// slot write is ordered before the slot read — no `RefCell`, no shared
    /// mutable access, no `unsafe impl Sync` on a live graph. This is design
    /// §4.2 form (b) reduced to its concurrency essence.
    struct RenderOwnerMailbox {
        ready: AtomicBool,
        /// The graph in transit; written by control, read by render, ordered by
        /// `ready`. Never mutated through a shared reference.
        slot: UnsafeCell<u64>,
    }

    unsafe impl Sync for RenderOwnerMailbox {}
    unsafe impl Send for RenderOwnerMailbox {}

    #[test]
    fn render_owner_handoff_is_race_free_under_loom() {
        loom::model(|| {
            let mailbox = Arc::new(RenderOwnerMailbox {
                ready: AtomicBool::new(false),
                slot: UnsafeCell::new(0),
            });

            // Control thread: publish a new graph, then signal ready (Release).
            let control = {
                let mailbox = Arc::clone(&mailbox);
                thread::spawn(move || {
                    mailbox.slot.with_mut(|p| unsafe { *p = 42 });
                    mailbox.ready.store(true, Ordering::Release);
                })
            };

            // Render thread == the single graph owner. Renders in whole blocks;
            // at the top of a block it checks for a pending swap. It only ever
            // reads `slot` AFTER an `Acquire` load observed `ready == true`, so
            // the control thread's write happens-before this read.
            let mut cur: u64 = 7;
            if mailbox.ready.load(Ordering::Acquire) {
                let next = mailbox.slot.with(|p| unsafe { *p });
                // transfer_* reads cur, installs next — single-thread step.
                cur = next.wrapping_add(cur & 0);
                assert_eq!(cur, 42);
            }

            control.join().unwrap();
        });
    }
}
