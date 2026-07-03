//! Render-thread-owned graph swap primitive.
//!
//! See `docs/audits/design-render-owner-swap-2026-07.md` §4.1 (ownership model),
//! §4.2 (channel discipline) and §5 (migration strategy).
//!
//! ## Why this exists
//!
//! The interactive hot-swap ("re-evaluate the running patch") used to be
//! implemented by publishing an `Arc<ArcSwap<Option<GraphCell>>>` where
//! `GraphCell(RefCell<UnifiedSignalGraph>)` was marked shareable by a
//! hand-written `unsafe impl Sync`. Two threads (render + reload) then both
//! called `try_borrow_mut()` on the *same* `RefCell`, performing an
//! unsynchronised read-modify-write on its **non-atomic** borrow flag — a data
//! race / UB, regardless of whether either call observed the other. The
//! `try_borrow_mut` patch only turned the *panic* into a silent skip; the race
//! remained (design §2.4).
//!
//! This module removes the race **structurally** by making the graph
//! **single-owner on the render thread**:
//!
//! * The control thread compiles + preloads a new graph off the render thread,
//!   then hands the finished, owned graph to the render thread through a
//!   **bounded lock-free SPSC command ring** ([`CommandSender`] →
//!   [`RenderSwap`]).
//! * The render thread — the only thread that ever touches the graph — pops the
//!   pending [`Cmd`] **at a buffer boundary** ([`RenderSwap::apply_pending_commands`]),
//!   runs the in-memory `transfer_*` from its currently-owned graph into the
//!   incoming one (via [`RenderGraph::absorb_state`]), swaps its owned pointer,
//!   and ships the retired graph to a **graveyard SPSC ring** ([`Graveyard`])
//!   drained by a low-priority janitor thread for off-RT `Drop`.
//!
//! There is then **no `RefCell`, no `ArcSwap<GraphCell>`, no cross-thread
//! borrow, and no `unsafe impl Sync`** on the graph. Ownership is *transferred*,
//! never shared — which is exactly the discipline that deletes the unsound
//! `unsafe impl Sync` (design §4.2).
//!
//! ## Scope
//!
//! This is the CORE channel primitive (task `render-owner-core-channel`). It is
//! generic over the graph type via the [`RenderGraph`] trait so the channel
//! mechanics are engine-independent and testable/`miri`-checkable without the
//! full audio engine. The follow-on task `render-owner-transfer-boundary`
//! implements `RenderGraph for UnifiedSignalGraph` (wiring the real
//! `transfer_session_timing` / `transfer_fx_states` / `transfer_voice_manager`
//! into [`RenderGraph::absorb_state`]) and touches `src/unified_graph.rs`.

use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

/// Default capacity of the command ring. Commands are human-paced (a keystroke
/// or file-save per swap), so a small ring is ample; a full ring signals the
/// render thread has stalled and the control thread should back off, not block.
pub const DEFAULT_CMD_CAPACITY: usize = 8;

/// Default capacity of the graveyard ring. Each retired graph corresponds to one
/// swap; the janitor drains in microseconds while swaps are seconds apart, so
/// this essentially never fills. If it ever does, [`RenderSwap`] stashes the
/// retired graph rather than dropping it on the render thread (see
/// [`RenderSwap::apply_pending_commands`]).
pub const DEFAULT_GRAVE_CAPACITY: usize = 16;

/// The render-owned operations a [`Cmd`] applies. Implemented for the real
/// `UnifiedSignalGraph` by the `render-owner-transfer-boundary` task; this crate
/// keeps the channel core generic and testable against a mock.
///
/// All methods run **on the render thread**, at a buffer boundary. They must be
/// bounded and allocation-free on the hot path — anything that touches disk
/// (e.g. `preload_samples`) must happen on the control thread *before* the
/// [`Cmd::Swap`] is enqueued (design §4.4).
///
/// Every method has a default no-op body so the channel core can be exercised
/// independently of any real graph; the transfer-boundary task overrides them.
pub trait RenderGraph {
    /// Absorb live state — session timing, FX states, voice manager — from the
    /// outgoing graph `prev` into `self` (the incoming graph), at the swap
    /// boundary. `self` is the freshly-compiled graph that is about to become
    /// the render thread's owned `cur`; `prev` is the graph being retired.
    ///
    /// `prev` is `&mut` because installing the live voice manager requires
    /// *taking* it out of the old graph (`take_voice_manager`), mirroring the
    /// existing `next.transfer_voice_manager(cur.take_voice_manager())` flow
    /// (design §2.3 / §4.1).
    fn absorb_state(&mut self, prev: &mut Self) {
        let _ = prev;
    }

    /// `Cmd::Hush` — silence all currently sounding voices without changing the
    /// graph structure.
    fn hush(&mut self) {}

    /// `Cmd::Panic` — hard reset: silence everything and clear transient state.
    fn panic(&mut self) {}

    /// `Cmd::SetTempo(cps)` — set cycles-per-second.
    fn set_tempo(&mut self, cps: f64) {
        let _ = cps;
    }

    /// `Cmd::SetCycle(cycle)` — set the absolute cycle position.
    fn set_cycle(&mut self, cycle: f64) {
        let _ = cycle;
    }
}

/// A render-thread command.
///
/// [`Cmd::Swap`] carries the incoming graph by **owned `Box`** so the render
/// thread *takes ownership* (the single-owner model) rather than sharing an
/// `Arc`. Boxing also keeps every command pointer-sized: the ring stores a thin
/// pointer, and applying a swap is a pointer swap — no large-struct memcpy and
/// no allocation on the render thread.
pub enum Cmd<G> {
    /// Replace the render-owned graph with this freshly-compiled, preloaded one.
    Swap(Box<G>),
    /// Silence all sounding voices (see [`RenderGraph::hush`]).
    Hush,
    /// Hard reset (see [`RenderGraph::panic`]).
    Panic,
    /// Set tempo in cycles-per-second (see [`RenderGraph::set_tempo`]).
    SetTempo(f64),
    /// Set the absolute cycle position (see [`RenderGraph::set_cycle`]).
    SetCycle(f64),
}

impl<G> Cmd<G> {
    /// A short, allocation-free label for the command variant (for logging /
    /// tests). Does not touch the boxed graph.
    pub fn kind(&self) -> &'static str {
        match self {
            Cmd::Swap(_) => "swap",
            Cmd::Hush => "hush",
            Cmd::Panic => "panic",
            Cmd::SetTempo(_) => "set_tempo",
            Cmd::SetCycle(_) => "set_cycle",
        }
    }
}

/// Control-thread handle for enqueuing commands to the render thread.
///
/// This is the *single producer* of the SPSC command ring. All sends are
/// non-blocking: if the ring is full they return `Err(cmd)` (backpressure) and
/// the caller decides whether to retry, coalesce, or drop — the control thread
/// never blocks the render thread.
pub struct CommandSender<G> {
    tx: HeapProd<Cmd<G>>,
}

impl<G> CommandSender<G> {
    /// Enqueue a command. Returns `Err(cmd)` (handing the command back) if the
    /// ring is full. Never blocks.
    pub fn send(&mut self, cmd: Cmd<G>) -> Result<(), Cmd<G>> {
        self.tx.try_push(cmd)
    }

    /// Convenience for the common case: enqueue a compiled graph for swap.
    /// Returns `Err(Cmd::Swap(graph))` if the ring is full, so the caller keeps
    /// ownership of the graph and can retry.
    pub fn swap(&mut self, graph: Box<G>) -> Result<(), Cmd<G>> {
        self.send(Cmd::Swap(graph))
    }

    /// `true` if the command ring is full (the render thread is behind).
    pub fn is_full(&self) -> bool {
        self.tx.is_full()
    }

    /// Number of command slots currently free.
    pub fn vacant_len(&self) -> usize {
        self.tx.vacant_len()
    }

    /// Number of commands currently queued but not yet applied.
    pub fn occupied_len(&self) -> usize {
        self.tx.occupied_len()
    }
}

/// Render-thread endpoint: the *single consumer* of the command ring and the
/// *single producer* of the graveyard ring.
///
/// It owns nothing but the two ring ends plus a small overflow stash of retired
/// graphs (used only if the graveyard is momentarily full). The render-owned
/// graph itself is a plain `Box<G>` local passed in by `&mut` — this type never
/// owns or shares it.
pub struct RenderSwap<G> {
    cmd_rx: HeapCons<Cmd<G>>,
    grave_tx: HeapProd<Box<G>>,
    /// Retired graphs that could not be pushed to the graveyard because it was
    /// momentarily full. Held here — never `Drop`ped on the render thread — and
    /// flushed on the next `apply_pending_commands` call. Under normal operation
    /// this stays empty (the janitor drains far faster than swaps arrive).
    stash: Vec<Box<G>>,
}

impl<G: RenderGraph> RenderSwap<G> {
    /// Drain and apply **all** pending commands to the render-owned graph `cur`,
    /// at a buffer boundary (call this at the top of the render block loop,
    /// before rendering — so a swap can only take effect *between* buffers).
    ///
    /// For [`Cmd::Swap`]: the incoming graph absorbs live state from `cur`
    /// ([`RenderGraph::absorb_state`]), then `cur` is swapped to the incoming
    /// graph by a single pointer swap, and the retired graph is shipped to the
    /// graveyard for off-RT `Drop`. `take_voice_manager` + install + swap are
    /// one uninterrupted step, so the graph is never rendered voiceless
    /// (design §4.1, R3).
    ///
    /// Returns the number of commands applied this call.
    ///
    /// ## RT-safety invariant
    ///
    /// A retired graph is **never `Drop`ped on the render thread**. Dropping a
    /// graph frees voice buffers, sample `Arc`s and FX delay lines — an
    /// unbounded free unfit for the hot path (design §4.1, §8). If the graveyard
    /// is momentarily full, the retired graph is moved to an internal stash and
    /// flushed on the next call; the only heap work then is a `Vec` push, never
    /// a graph `Drop`.
    pub fn apply_pending_commands(&mut self, cur: &mut Box<G>) -> usize {
        // Flush any previously-stashed retired graphs first, so the stash is
        // normally empty and retirements below go straight to the graveyard.
        self.flush_stash();

        let mut applied = 0;
        while let Some(cmd) = self.cmd_rx.try_pop() {
            match cmd {
                Cmd::Swap(mut next) => {
                    // Incoming graph takes live state from the outgoing one.
                    next.absorb_state(cur);
                    // Single-owner handoff: pointer swap, no big memcpy, no alloc.
                    let retired = std::mem::replace(cur, next);
                    self.retire(retired);
                }
                Cmd::Hush => cur.hush(),
                Cmd::Panic => cur.panic(),
                Cmd::SetTempo(cps) => cur.set_tempo(cps),
                Cmd::SetCycle(c) => cur.set_cycle(c),
            }
            applied += 1;
        }
        applied
    }

    /// Ship a retired graph to the graveyard, or stash it if the graveyard is
    /// full. Never drops the graph on the current (render) thread.
    fn retire(&mut self, retired: Box<G>) {
        if let Err(retired) = self.grave_tx.try_push(retired) {
            self.stash.push(retired);
        }
    }

    /// Move as many stashed retired graphs into the graveyard as it will accept.
    fn flush_stash(&mut self) {
        while let Some(g) = self.stash.pop() {
            if let Err(g) = self.grave_tx.try_push(g) {
                // Graveyard still full — put it back and stop; retry next call.
                self.stash.push(g);
                break;
            }
        }
    }

    /// Number of commands currently queued but not yet applied.
    pub fn pending_commands(&self) -> usize {
        self.cmd_rx.occupied_len()
    }

    /// Number of retired graphs held in the overflow stash (normally 0). Nonzero
    /// only when the janitor has fallen behind and the graveyard filled up.
    pub fn stashed_retired(&self) -> usize {
        self.stash.len()
    }
}

/// Graveyard consumer, drained by a low-priority janitor thread that `Drop`s
/// retired graphs off the render thread.
pub struct Graveyard<G> {
    rx: HeapCons<Box<G>>,
}

impl<G> Graveyard<G> {
    /// Drain and `Drop` **all** retired graphs currently in the graveyard.
    /// Returns the number dropped this call. Call this in a loop on the janitor
    /// thread (with a park/backoff between empty polls).
    pub fn collect(&mut self) -> usize {
        let mut n = 0;
        while let Some(g) = self.rx.try_pop() {
            drop(g);
            n += 1;
        }
        n
    }

    /// Take a single retired graph, if any, without dropping it — useful for a
    /// janitor that wants to bound work per wake-up, or for tests that inspect
    /// the retired graph before dropping it.
    pub fn try_pop(&mut self) -> Option<Box<G>> {
        self.rx.try_pop()
    }

    /// `true` if there is nothing to collect.
    pub fn is_empty(&self) -> bool {
        self.rx.is_empty()
    }

    /// Number of retired graphs currently awaiting collection.
    pub fn len(&self) -> usize {
        self.rx.occupied_len()
    }
}

/// Build a render-owner swap channel: a bounded SPSC command ring plus a bounded
/// SPSC graveyard ring.
///
/// Returns `(sender, render, graveyard)`:
/// * `sender` ([`CommandSender`]) lives on the control thread and enqueues
///   [`Cmd`]s.
/// * `render` ([`RenderSwap`]) lives on the render thread and applies commands
///   to its owned graph via [`RenderSwap::apply_pending_commands`].
/// * `graveyard` ([`Graveyard`]) lives on the janitor thread and `Drop`s retired
///   graphs off the render thread via [`Graveyard::collect`].
///
/// `cmd_capacity` and `grave_capacity` must be non-zero (ring buffers require
/// capacity ≥ 1). See [`DEFAULT_CMD_CAPACITY`] / [`DEFAULT_GRAVE_CAPACITY`].
pub fn render_swap_channel<G>(
    cmd_capacity: usize,
    grave_capacity: usize,
) -> (CommandSender<G>, RenderSwap<G>, Graveyard<G>) {
    assert!(cmd_capacity > 0, "command ring capacity must be non-zero");
    assert!(grave_capacity > 0, "graveyard ring capacity must be non-zero");

    let (cmd_tx, cmd_rx) = HeapRb::<Cmd<G>>::new(cmd_capacity).split();
    let (grave_tx, grave_rx) = HeapRb::<Box<G>>::new(grave_capacity).split();

    (
        CommandSender { tx: cmd_tx },
        RenderSwap {
            cmd_rx,
            grave_tx,
            stash: Vec::new(),
        },
        Graveyard { rx: grave_rx },
    )
}

/// Build a render-owner swap channel with the default capacities
/// ([`DEFAULT_CMD_CAPACITY`], [`DEFAULT_GRAVE_CAPACITY`]).
pub fn render_swap_channel_default<G>() -> (CommandSender<G>, RenderSwap<G>, Graveyard<G>) {
    render_swap_channel(DEFAULT_CMD_CAPACITY, DEFAULT_GRAVE_CAPACITY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Minimal stand-in for `UnifiedSignalGraph`: deliberately NOT `Clone`, never
    /// wrapped in a `RefCell`/`Arc`, and carrying a unique heap allocation whose
    /// address is this graph's identity — so tests can prove the *same* allocation
    /// is moved through the channel (ownership transfer, not sharing).
    struct MockGraph {
        id: u64,
        tag: Box<u64>,
        absorbed_from: Option<u64>,
        hushed: bool,
        panicked: bool,
        tempo: f64,
        cycle: f64,
        drops: Arc<AtomicUsize>,
    }

    impl MockGraph {
        fn new(id: u64, drops: Arc<AtomicUsize>) -> Self {
            MockGraph {
                id,
                tag: Box::new(id),
                absorbed_from: None,
                hushed: false,
                panicked: false,
                tempo: 0.0,
                cycle: 0.0,
                drops,
            }
        }
        fn tag_addr(&self) -> usize {
            self.tag.as_ref() as *const u64 as usize
        }
    }

    impl Drop for MockGraph {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl RenderGraph for MockGraph {
        fn absorb_state(&mut self, prev: &mut Self) {
            self.absorbed_from = Some(prev.id);
        }
        fn hush(&mut self) {
            self.hushed = true;
        }
        fn panic(&mut self) {
            self.panicked = true;
        }
        fn set_tempo(&mut self, cps: f64) {
            self.tempo = cps;
        }
        fn set_cycle(&mut self, c: f64) {
            self.cycle = c;
        }
    }

    fn boxed(id: u64, drops: &Arc<AtomicUsize>) -> Box<MockGraph> {
        Box::new(MockGraph::new(id, drops.clone()))
    }

    /// The primary TDD test: ownership is *moved* through the channel (not shared
    /// via a `RefCell`), and the retired graph is dropped by the janitor, never
    /// on the render thread.
    #[test]
    fn test_render_swap_moves_ownership_no_shared_refcell() {
        let drops = Arc::new(AtomicUsize::new(0));
        let (mut tx, mut rsw, mut grave) = render_swap_channel::<MockGraph>(8, 8);

        // Render thread owns graph 0.
        let mut cur = boxed(0, &drops);

        // Control thread compiles graph 1 and hands it off by MOVE.
        let incoming = boxed(1, &drops);
        let incoming_addr = incoming.tag_addr();
        assert!(tx.swap(incoming).is_ok(), "command ring has space");
        // `incoming` moved into the ring — compile-time proof it is not shared.

        // Buffer boundary: render thread applies pending commands.
        let applied = rsw.apply_pending_commands(&mut cur);
        assert_eq!(applied, 1);

        // Render thread now owns the EXACT allocation the control thread built.
        assert_eq!(cur.id, 1);
        assert_eq!(
            cur.tag_addr(),
            incoming_addr,
            "same heap allocation moved through channel (move, not copy/share)"
        );
        assert_eq!(cur.absorbed_from, Some(0));

        // Retired graph 0 must NOT be dropped on the render thread.
        assert_eq!(
            drops.load(Ordering::SeqCst),
            0,
            "retired graph must not Drop on render thread"
        );

        // Janitor drains the graveyard off the render thread and drops it.
        assert_eq!(grave.collect(), 1);
        assert_eq!(
            drops.load(Ordering::SeqCst),
            1,
            "retired graph dropped by janitor"
        );
    }

    /// Push/pop move semantics for every command variant, applied FIFO.
    #[test]
    fn test_command_push_pop_move_semantics() {
        let drops = Arc::new(AtomicUsize::new(0));
        let (mut tx, mut rsw, mut grave) = render_swap_channel_default::<MockGraph>();
        let mut cur = boxed(0, &drops);

        assert!(tx.send(Cmd::SetTempo(2.5)).is_ok());
        assert!(tx.send(Cmd::SetCycle(4.0)).is_ok());
        assert!(tx.send(Cmd::Panic).is_ok());
        assert_eq!(tx.occupied_len(), 3);

        let applied = rsw.apply_pending_commands(&mut cur);
        assert_eq!(applied, 3);
        assert_eq!(cur.tempo, 2.5);
        assert_eq!(cur.cycle, 4.0);
        assert!(cur.panicked);
        assert_eq!(tx.occupied_len(), 0);

        // No swaps issued → nothing retired.
        assert!(grave.is_empty());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
    }

    /// Commands are applied in the exact order enqueued: Hush-then-Swap means the
    /// Hush lands on the OLD graph *before* it is retired, and the swap installs
    /// the new graph afterwards.
    #[test]
    fn test_hush_then_swap_ordered() {
        let drops = Arc::new(AtomicUsize::new(0));
        let (mut tx, mut rsw, mut grave) = render_swap_channel_default::<MockGraph>();
        let mut cur = boxed(0, &drops);

        // Enqueue Hush, THEN Swap — order must be preserved.
        assert!(tx.send(Cmd::Hush).is_ok());
        assert!(tx.swap(boxed(1, &drops)).is_ok());

        assert_eq!(rsw.apply_pending_commands(&mut cur), 2);

        // New graph is now current and was NOT hushed (Hush preceded the swap).
        assert_eq!(cur.id, 1);
        assert!(!cur.hushed);
        assert_eq!(cur.absorbed_from, Some(0));

        // Inspect the retired graph without dropping it: the Hush hit graph 0
        // *before* it was retired, proving Hush-then-Swap ordering.
        let retired = grave.try_pop().expect("graph 0 retired");
        assert_eq!(retired.id, 0);
        assert!(retired.hushed, "Hush applied to old graph before swap");
        drop(retired);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    /// Multiple swaps in one drain: each retirement reaches the graveyard, in
    /// order, and none is dropped on the render thread until the janitor runs.
    #[test]
    fn test_graveyard_handoff_multiple_swaps() {
        let drops = Arc::new(AtomicUsize::new(0));
        let (mut tx, mut rsw, mut grave) = render_swap_channel_default::<MockGraph>();
        let mut cur = boxed(0, &drops);

        assert!(tx.swap(boxed(1, &drops)).is_ok());
        assert!(tx.swap(boxed(2, &drops)).is_ok());
        assert!(tx.swap(boxed(3, &drops)).is_ok());

        assert_eq!(rsw.apply_pending_commands(&mut cur), 3);
        assert_eq!(cur.id, 3);

        // Graphs 0,1,2 all retired to the graveyard; none dropped on render thread.
        assert_eq!(grave.len(), 3);
        assert_eq!(drops.load(Ordering::SeqCst), 0);

        // Graveyard preserves retirement order: 0, then 1, then 2.
        assert_eq!(grave.try_pop().unwrap().id, 0);
        assert_eq!(grave.try_pop().unwrap().id, 1);
        assert_eq!(grave.try_pop().unwrap().id, 2);
        assert_eq!(drops.load(Ordering::SeqCst), 3);
        assert!(grave.is_empty());
    }

    /// Command-ring capacity backpressure: once the ring is full, `send` returns
    /// `Err(cmd)` handing the command back — the control thread is never blocked
    /// and never loses the graph.
    #[test]
    fn test_command_ring_capacity_backpressure() {
        let drops = Arc::new(AtomicUsize::new(0));
        let (mut tx, mut rsw, _grave) = render_swap_channel::<MockGraph>(2, 8);
        let mut cur = boxed(0, &drops);

        assert!(tx.swap(boxed(1, &drops)).is_ok());
        assert!(tx.swap(boxed(2, &drops)).is_ok());
        assert!(tx.is_full());

        // Third send is refused; we get the command (and its graph) back.
        let rejected = boxed(3, &drops);
        let rejected_addr = rejected.tag_addr();
        match tx.swap(rejected) {
            Err(Cmd::Swap(g)) => {
                assert_eq!(g.id, 3);
                assert_eq!(g.tag_addr(), rejected_addr, "rejected graph handed back intact");
                // Drop it here explicitly (control thread) — no leak, no panic.
                drop(g);
            }
            _ => panic!("expected the full ring to reject and return the command"),
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1, "only the rejected graph dropped so far");

        // Draining frees a slot, and the queued swaps still apply in order.
        assert_eq!(rsw.apply_pending_commands(&mut cur), 2);
        assert_eq!(cur.id, 2);
        assert!(tx.swap(boxed(4, &drops)).is_ok(), "slot freed after drain");
    }

    /// Graveyard-ring backpressure: if the janitor stalls and the graveyard
    /// fills, retired graphs are stashed on the render side — NEVER dropped on
    /// the render thread — and flushed once the janitor drains.
    #[test]
    fn test_graveyard_backpressure_never_drops_on_render_thread() {
        let drops = Arc::new(AtomicUsize::new(0));
        // Tiny graveyard (capacity 1) to force overflow; roomy command ring.
        let (mut tx, mut rsw, mut grave) = render_swap_channel::<MockGraph>(8, 1);
        let mut cur = boxed(0, &drops);

        // First swap: graph 0 retired → graveyard (now full).
        assert!(tx.swap(boxed(1, &drops)).is_ok());
        assert_eq!(rsw.apply_pending_commands(&mut cur), 1);
        assert_eq!(grave.len(), 1);
        assert_eq!(rsw.stashed_retired(), 0);

        // Second swap: graph 1 retired but graveyard is full → goes to the stash,
        // NOT dropped on the render thread.
        assert!(tx.swap(boxed(2, &drops)).is_ok());
        assert_eq!(rsw.apply_pending_commands(&mut cur), 1);
        assert_eq!(cur.id, 2);
        assert_eq!(rsw.stashed_retired(), 1, "retired graph stashed, not dropped");
        assert_eq!(
            drops.load(Ordering::SeqCst),
            0,
            "NOTHING dropped on the render thread even under graveyard backpressure"
        );

        // Janitor drains graph 0, freeing a graveyard slot.
        assert_eq!(grave.collect(), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        // Next apply (no new commands) flushes the stash into the graveyard.
        assert_eq!(rsw.apply_pending_commands(&mut cur), 0);
        assert_eq!(rsw.stashed_retired(), 0, "stash flushed to graveyard");
        assert_eq!(grave.len(), 1);

        // Janitor drains graph 1.
        assert_eq!(grave.collect(), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    /// End-to-end across real threads: control thread produces swaps, render
    /// thread consumes and applies them, janitor drops the retired graphs — all
    /// by move, proving the primitive is `Send`-correct with no shared state.
    /// Skipped under Miri (which validates the single-threaded ownership logic in
    /// the other tests; see design §6.A).
    #[cfg(not(miri))]
    #[test]
    fn test_cross_thread_spsc_handoff() {
        use std::sync::mpsc;
        use std::thread;

        const N: u64 = 200;
        let drops = Arc::new(AtomicUsize::new(0));
        // Command ring is small (exercises producer backpressure/spin); the
        // graveyard is sized above N so retirements never overflow into the
        // render-side stash — that keeps the collected-count deterministic
        // (a stashed graph would be dropped on the render thread at exit, off
        // the janitor's tally).
        let (mut tx, mut rsw, mut grave) = render_swap_channel::<MockGraph>(4, 256);

        // Render thread: own graph 0, apply swaps until it has become graph N,
        // then hand the final graph back for inspection.
        let (done_tx, done_rx) = mpsc::channel::<Box<MockGraph>>();
        let render = thread::spawn(move || {
            let mut cur = Box::new(MockGraph::new(0, Arc::new(AtomicUsize::new(0))));
            while cur.id != N {
                rsw.apply_pending_commands(&mut cur);
                std::hint::spin_loop();
            }
            done_tx.send(cur).unwrap();
        });

        // Janitor thread: drain the graveyard until the render thread is done.
        let grave_drops = drops.clone();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let janitor = thread::spawn(move || {
            let mut collected = 0;
            loop {
                collected += grave.collect();
                if stop_rx.try_recv().is_ok() {
                    // Final sweep after being told to stop.
                    collected += grave.collect();
                    break;
                }
                std::hint::spin_loop();
            }
            (collected, grave_drops.load(Ordering::SeqCst))
        });

        // Control thread: push N swaps by move, spinning on a full ring (never
        // blocks the render thread).
        for id in 1..=N {
            let mut g = boxed(id, &drops);
            loop {
                match tx.swap(g) {
                    Ok(()) => break,
                    Err(Cmd::Swap(back)) => {
                        g = back;
                        std::hint::spin_loop();
                    }
                    Err(_) => unreachable!(),
                }
            }
        }

        let final_graph = done_rx.recv().unwrap();
        assert_eq!(final_graph.id, N, "render thread applied every swap in order");
        // The final graph is still owned here; drop it (its drop counter is the
        // separate one created inside the render thread, so ignore for the tally).
        drop(final_graph);

        stop_tx.send(()).unwrap();
        let (collected, _) = janitor.join().unwrap();
        render.join().unwrap();

        // Every retired graph (0..=N-1 = N graphs) was collected by the janitor.
        assert_eq!(collected as u64, N, "janitor collected every retired graph");
    }
}
