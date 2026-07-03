# Design: Render-Thread-Owned Graph Swap

**Date:** 2026-07-03
**Task:** `design-render-owner-swap`
**Type:** Design only — **no engine code changed by this task**. Deliverables are this
document plus a set of follow-on `wg` task stubs (§7).
**Inputs:** `docs/audits/improvement-plan-2026-07.md` §5 (the deferred "render-owner graph
swap", biggest architectural win); `docs/audits/live-transition-2026-07.md` §8 Rank 5 (R1–R3
elimination) and §6 (the race analysis); `docs/audits/rt-safety-2026-07.md` F-1/B7/F-10 (the
C1 borrow race and its `unsafe impl Sync` root); `docs/audits/feature-gap-2026-07.md` (wave-2
context).

> **Scope discipline.** Every claim about *current behavior* below cites a `file:line` from
> the tree as of this commit. Line numbers were re-verified against the **current** source,
> not the audits' — several wave-1 fixes have landed since those audits were written
> (`fix-synth-borrow-race`, `unify-live-clock`, `fix-swap-boundary` render-continuity,
> `sanitize-node-state`, `migrate-modal-editor`), so the current tree is **past** the state
> the audits describe. Where the current tree differs from the audit, this document reflects
> the current tree.

---

## 1. Executive Summary

The interactive hot-swap ("re-evaluate the running patch") is implemented four times over
(`phonon live` in `src/main.rs`, `phonon-audio` in `src/bin/phonon-audio.rs`, the modal
editor in `src/modal_editor/mod.rs`, and the unreachable `LiveSession` in `src/live.rs`),
each a near-identical ~30-line block that:

1. compiles + preloads a new graph on a **non-render** thread, then
2. **reaches across threads** to `try_borrow_mut()` the *old* graph and run
   `transfer_session_timing` / `transfer_fx_states` / `transfer_voice_manager`, then
3. `ArcSwap::store`s the new graph so the render thread picks it up.

The shared cell is `GraphCell(RefCell<UnifiedSignalGraph>)`, published via
`Arc<ArcSwap<Option<GraphCell>>>`, and marked thread-shareable by a hand-written
`unsafe impl Sync for GraphCell` (`src/main.rs:951-952`, `src/modal_editor/mod.rs:58-59`,
`src/bin/phonon-audio.rs:174,176`, `src/live.rs:30-31`, `src/stress_harness.rs:1932-1933`).

The recent `fix-synth-borrow-race` work made **both** the render side and the reload side use
`try_borrow_mut()` (render side: `src/main.rs:1055`, `src/bin/phonon-audio.rs:310`,
`src/modal_editor/mod.rs:300`, `src/live.rs:128`). That removed the **panic symptom** (C1 /
rt-safety F-1): a double mutable borrow no longer *panics* the synth thread. **It did not
remove the root defect.** `RefCell`'s borrow flag is a plain non-atomic `Cell<isize>`; two
threads that call `try_borrow_mut()` on the *same* `RefCell` concurrently perform an
unsynchronised read-modify-write on that flag. Under the Rust/C++ memory model that is a
**data race → undefined behavior**, independent of whether either call happens to observe the
other and return `Err`. The `unsafe impl Sync` is what makes the compiler accept this; it is
**unsound** (`rt-safety-2026-07.md` §3 note on `unsafe impl Send/Sync`; F-1's "*Also* the
underlying data race on `RefCell`'s non-atomic borrow flag" per `improvement-plan-2026-07.md`
C1 row). No test can currently observe this: there is **no loom, TSan, or Miri** anywhere in
the tree (verified — grep for `loom`/`miri`/`sanitizer` returns nothing), and a data race is
precisely the class of bug a normal execution will not reliably surface.

**The fix is structural, not another patch:** make the graph **single-owner on the render
thread**. The control thread compiles + preloads off-thread and hands the finished graph to
the render thread through a **bounded lock-free SPSC command channel**; the render thread —
the only thread that ever touches the graph — pops the pending swap **at a buffer boundary**,
runs `transfer_*` from its currently-owned graph into the new one, swaps its owned pointer,
and ships the retired graph to a janitor thread for off-RT `Drop`. There is then no
`RefCell`, no `unsafe impl Sync`, and no cross-thread borrow.

This one change removes:

| Defect | Source | Why it disappears under render-owner |
|---|---|---|
| **C1 root data race** | `improvement-plan` C1; rt F-1/B7; `unsafe impl Sync` at the sites above | Graph is touched by exactly one thread → no shared-mutable state → no data race → the `unsafe impl Sync` is deleted. |
| **R1** beat jump on give-up | `live-transition` §6 R1; retry ceiling `src/main.rs:1204`, `src/bin/phonon-audio.rs:569`, `src/modal_editor/mod.rs:758` | The synth thread *always* applies the swap on its own next boundary. There is no 25 ms borrow-contention retry loop and no "could not transfer state" branch (`src/main.rs:1222-1224`). |
| **R2** synth starvation during transfer | `live-transition` §6 R2; synth skip arms `src/main.rs:1055` (Err), `src/bin/phonon-audio.rs:310`, `src/modal_editor/mod.rs:300` | The synth never blocks on, or yields to, the control thread; the transfer is in-thread and bounded by the transfer cost, not by another thread holding a borrow. |
| **R3** voiceless-old-graph window | `live-transition` §6 R3; `take_voice_manager` `src/unified_graph.rs:6678`, store deferred to `src/main.rs:1232` / `:603` / `:826` | `take_voice_manager` + `transfer_voice_manager` + pointer swap execute as one uninterrupted render-thread step; there is no interval where the *published* graph has been emptied of voices but is still being rendered. |
| **3-way path divergence** | four duplicated swap blocks (§3) | All frontends share one render-owner swap primitive; hush / panic / tempo / reload become channel commands. |

It is deliberately **deferred until after wave-1** and gated behind two test enablers (a race
detector and a live-path conformance suite) so the rewrite is *provable*, per
`improvement-plan-2026-07.md` §5 ("Pair with I2 … and I5 … to prove it").

---

## 2. Current swap data-flow (with citations)

### 2.1 Shared-state shape (identical across the four surfaces)

```
 control thread (UI / file-watch / IPC)          render/synth thread              device callback
 ───────────────────────────────────────         ────────────────────             ───────────────
 compile new_graph (off-thread)
 new_graph.enable_wall_clock_timing()
 old = graph.load()                       ┐
 for _ in 0..50 {                         │ 25 ms
   old.0.try_borrow_mut() ── transfer_* ──┼──►  graph.load() snapshot
 }                                        ┘      cell.0.try_borrow_mut():
 new_graph.preload_samples()                       Ok  → process_buffer → ring.push_slice ─► pop_slice → device
 graph.store(Some(new_graph))  ──────────────►     Err → skip (write nothing)
```

- `GraphCell(RefCell<UnifiedSignalGraph>)` — defined **five** times, once per surface:
  `src/main.rs:950`, `src/modal_editor/mod.rs:57`, `src/bin/phonon-audio.rs:172`,
  `src/live.rs:29`, `src/stress_harness.rs:1929`.
- Published via `graph: Arc<ArcSwap<Option<GraphCell>>>` (`src/modal_editor/mod.rs:76`,
  `src/live.rs:36`, `src/stress_harness.rs:2046`; the `main.rs`/`phonon-audio.rs` local
  streams build the same type inline).
- Marked thread-shareable by hand: `unsafe impl Send for GraphCell {}` / `unsafe impl Sync
  for GraphCell {}` at `src/main.rs:951-952`, `src/modal_editor/mod.rs:58-59`,
  `src/bin/phonon-audio.rs:174,176`, `src/live.rs:30-31`, `src/stress_harness.rs:1932-1933`;
  and `unsafe impl Send/Sync for UnifiedSignalGraph` at `src/unified_graph.rs:5327-5328`.

### 2.2 The render side (what reads the cell)

| Surface | Render borrow | Skip-on-contention |
|---|---|---|
| `phonon live` | `src/main.rs:1055` `graph_cell.0.try_borrow_mut()` | `Err` arm skips the block |
| `phonon-audio` | `src/bin/phonon-audio.rs:310` `graph_cell.0.try_borrow_mut()` → `:315` `process_buffer_at` | `Err` arm skips |
| modal editor | `src/modal_editor/mod.rs:300` `graph_cell.0.try_borrow_mut()` | `Err` arm skips |
| `LiveSession` (latent) | `src/live.rs:128` `graph_cell.0.try_borrow_mut()` | `Err` arm skips |

All four render loops now use `try_borrow_mut` + skip — the `fix-synth-borrow-race`
outcome. The panic is gone; the shared `RefCell` remains.

### 2.3 The reload side (what writes the cell)

The reload block is duplicated verbatim across the three reachable surfaces:

| Surface | Retry loop (50 × 500 µs = 25 ms) | Transfers | Preload | Store |
|---|---|---|---|---|
| `phonon live` | `src/main.rs:1204` `for _attempt in 0..50` | `:1207-1211` timing / fx / voice | `:1229` `preload_samples()` | `:1232` `graph.store(...)` |
| `phonon-audio` | `src/bin/phonon-audio.rs:569` | `:572-574` | `:591` | `:603` |
| modal editor | `src/modal_editor/mod.rs:758` `for attempt in 0..50` | `:773-787` | `:821` | `:826` |
| `LiveSession` (latent) | **no retry — raw borrow** `src/live.rs:341,343,345` `old_graph_cell.0.borrow()/.borrow_mut()` | inline | `src/live.rs` | `:355` |

`src/live.rs:341-345` still uses the *panicking* `borrow()`/`borrow_mut()` on the old graph
(rt-safety **F-10 / C4**, latent because no CLI command constructs `LiveSession` — verified:
the only references to `LiveSession` are within `src/live.rs`). It is a copy-paste hazard and
must be retired or migrated by this effort so no raw-borrow swap path survives.

The transfer functions themselves (all `src/unified_graph.rs`, single-writer under the
caller's borrow):

- `preload_samples()` `:6341` — disk I/O for uncached samples; **must stay on a non-render
  thread**.
- `take_voice_manager()` `:6678` — `mem::replace`s the **old** graph's `VoiceManager` with a
  fresh empty one and returns the live one. This is what opens the R3 window: the old graph,
  still published in `ArcSwap`, is now voiceless until `store`.
- `transfer_voice_manager()` `:6686` — installs the taken manager into the new graph **after
  `release_synthesis_voices()` + `release_sample_voices()`** (`:6688-6690`); i.e. voices are
  *faded*, not preserved (D1 / B8 / G7 — out of this task's scope but noted so the migration
  preserves the current fade-on-swap semantics unchanged).
- `transfer_session_timing()` `:6709` — carries the wall-clock reference and, at `:6815`,
  calls `transfer_render_continuity()` `:6835`, which seeds the new graph's
  `prev_buffer_tail` (`:5295`) so the Phase-4d boundary crossfade (`:9082-9093`) smooths the
  seam (the landed `fix-swap-boundary` / D3 fix).
- `transfer_fx_states()` `:9354`.
- `enable_wall_clock_timing()` `:9837`.

### 2.3.1 Timing today (post `unify-live-clock`)

`phonon live` already owns a persistent `LiveClock` on the synth thread: it seeds once
(`src/main.rs:1062`), follows tempo with rebasing (`:1071`), re-seeds a swapped-in graph's
node position from the live clock (`:1080` `graph.set_cycle_position(c.position())`), and
advances by exactly one block (`:1088` `advance_buffer`). So in `phonon live` the beat no
longer teleports even when `transfer_session_timing` is skipped — the clock is the source of
truth. This is exactly the ownership pattern render-owner generalises to **all** surfaces and
to **all** state (not just the clock): the render thread owns the graph, so a swap can never
race or reset it.

### 2.4 Where the race lives — precisely

The data race is the concurrent, unsynchronised access to **one** `RefCell`'s borrow flag by
**two** threads:

- Render thread: `cell.0.try_borrow_mut()` at `src/main.rs:1055` / `src/bin/phonon-audio.rs:310`
  / `src/modal_editor/mod.rs:300`.
- Reload thread: `old.0.try_borrow_mut()` at `src/main.rs:1205` / `src/bin/phonon-audio.rs:570`
  / `src/modal_editor/mod.rs:759` (or the raw `borrow_mut()` at `src/live.rs:345`).

Both operate on the same `GraphCell` for the duration of the reload's transfer window,
because `graph.store` (`src/main.rs:1232` etc.) does not happen until *after* the transfers
complete — the pointer still resolves to the old cell throughout. `try_borrow_mut` internally
does `flag = self.borrow.get(); if flag == UNUSED { self.borrow.set(WRITING); ... }` on a
non-atomic `Cell`. Two threads doing that RMW on the same address, with no `Acquire`/`Release`
or atomic, is a textbook data race. The `try_borrow_mut` patch changed the *consequence* of
losing the race from "panic" to "return `Err` and skip", but the race — the UB — is still
there on every overlapping swap. **This is the root C1 that the symptom fix did not touch,**
and it is why `improvement-plan-2026-07.md` lists "▷ render-owner model (root)" separately
from "★ `fix-synth-borrow-race` (symptom)" in the C1 row.

---

## 3. Why there are three (four) live paths, and what "unify" means

The same swap protocol is duplicated in `src/main.rs:1204-1234`, `src/bin/phonon-audio.rs:569-603`,
`src/modal_editor/mod.rs:758-826`, and (latent, divergent) `src/live.rs:341-355`. They differ
only in incidental wiring:

- **Trigger**: file-watch (`phonon live`), Ctrl-x keystroke (modal), IPC message
  (`phonon-audio`).
- **Clock**: `phonon live` and modal use wall-clock/`LiveClock`; `phonon-audio` renders on an
  **external** clock via `process_buffer_at` (`src/bin/phonon-audio.rs:315`).
- **Hush/panic**: modal stores `None` directly (`src/modal_editor/mod.rs:2339,2348`);
  `phonon-audio` receives IPC Hush/Panic and stores `None` (`:618,623`).

"Unify" = one **render-owner swap primitive** (a command channel + a render-thread
`apply_pending_commands` step) that every frontend feeds. The frontends keep their own
trigger + clock source, but the *mechanism* that moves a compiled graph onto the render
thread — and the concurrency reasoning that makes it sound — exists **once**. `src/stress_harness.rs`
already had to hand-replicate this protocol a fifth time (`:2118-2168`) precisely because
there was no shared primitive to test against; the conformance suite (§6) will target the
shared primitive instead.

---

## 4. The render-owned swap design

### 4.1 Ownership model

**Invariant:** the live `UnifiedSignalGraph` is owned by, and only ever touched by, the
render (synth) thread. No `Arc`, no `RefCell`, no `ArcSwap<GraphCell>`, no `unsafe impl Sync`
on the graph.

```
 control thread (UI / file-watch / IPC)                 render thread (single owner)
 ─────────────────────────────────────                 ──────────────────────────────
 on trigger:                                            loop {
   new = compile(code)                                    // buffer boundary — before render
   new.enable_wall_clock_timing()                         while let Some(cmd) = swap_rx.pop() {
   new.preload_samples()   // disk I/O off-RT               match cmd {
   swap_tx.push(Cmd::Swap(Box::new(new)))  ───────────►       Swap(mut next) => {
                                                                next.transfer_session_timing(&cur);
                                                                next.transfer_fx_states(&cur);
                                                                next.transfer_voice_manager(cur.take_voice_manager());
                                                                let retired = mem::replace(&mut cur, *next);
                                                                grave_tx.push(retired); // drop off-RT
                                                              }
                                                              Hush  => cur.silence_all(),
                                                              Panic => cur.hard_reset(),
                                                              SetTempo(c) => cur.set_cps(c),
   janitor thread:                                          }
   while let Some(g) = grave_rx.pop() { drop(g) } ◄─────── }
                                                          cur.process_buffer(&mut block); // or _at for phonon-audio
                                                          ring.push_slice(&block)
                                                        }
```

- **`cur: UnifiedSignalGraph`** is a plain local on the render thread — single owner, zero
  synchronisation on the graph itself.
- **`transfer_*` runs on the render thread**, reading the still-owned `cur` and writing
  `next`. No borrow crosses a thread boundary → the `try_borrow_mut` retry loop
  (`src/main.rs:1204` etc.) and its 25 ms ceiling are deleted (R1/R2 gone). `take_voice_manager`
  + install + swap are one uninterrupted step, so the published graph is never rendered
  voiceless (R3 gone).
- **The old graph is dropped off the render thread.** Dropping a `UnifiedSignalGraph`
  frees voice buffers, sample `Arc`s, FX delay lines, and parser-leaked strings; doing that
  on the render thread would be an unbounded free on the hot path. The retired graph is
  pushed to a **graveyard SPSC ring** drained by a low-priority janitor thread. (This also
  fixes, for free, the current situation where the *old* `Arc<GraphCell>` is dropped by
  whichever thread releases the last reference — today that can be the render thread.)

### 4.2 The channel discipline (what replaces `ArcSwap<RefCell>`)

Use a **bounded lock-free SPSC ring of commands**, not `ArcSwap`. Rationale:

- **Move semantics.** The render thread must *take ownership* of the incoming graph (so it
  becomes the sole owner). `ArcSwap` publishes a *shared* `Arc` — multiple readers, no move —
  which is the wrong primitive for single-ownership handoff. An SPSC ring of
  `Box<UnifiedSignalGraph>` (inside a `Cmd` enum) gives exactly one consumer that pops and
  owns.
- **Reuse what's already in the tree.** `ringbuf::HeapRb` is already the audio ring
  (`src/stress_harness.rs:2050`; the frontends' `HeapRb::<f32>`). A second `HeapRb<Cmd>`
  (capacity ~8) is the command channel; a third `HeapRb<Box<UnifiedSignalGraph>>` is the
  graveyard. No new dependency.
- **Unifies control messages.** `Cmd::{Swap, Hush, Panic, SetTempo, SetCycle, …}` replaces
  the ad-hoc `store(None)` hush (`src/modal_editor/mod.rs:2339`) and the IPC dispatch
  (`src/bin/phonon-audio.rs:618,623`) with one ordered stream, so ordering (Hush-then-Swap,
  Panic-precedence) is well-defined and testable in one place.

**Where does the "double-buffer / ArcSwap discipline" the brief mentions fit?** Two forms are
acceptable; the design mandates form (a) and documents (b) as the fallback:

- **(a) SPSC command ring (recommended).** Genuine move + ordered control. Double-buffering is
  *inherent*: at the seam the render thread holds `cur` **and** `next` simultaneously
  (renders `cur`'s tail into `prev_buffer_tail`, then swaps to `next`), then retires `cur`.
- **(b) ArcSwap *mailbox* (fallback, if a ring proves awkward for a frontend).** Keep
  `ArcSwap`, but change the discipline so it is a **single-slot mailbox of an immutable,
  `Send` payload** that **only the render thread consumes**: `ArcSwap<Option<Arc<PendingSwap>>>`
  where the control thread `store`s and the render thread does `swap(None)` to *take* the
  slot. Crucially the payload is **not** a `RefCell` and is **never mutated through the
  `Arc`** — the render thread `Arc::try_unwrap`s (or clones out) into an owned graph before
  touching it. This keeps `ArcSwap`'s lock-free publish while removing the shared-mutable
  `RefCell`. Form (a) is preferred because move + ordering + graveyard fall out naturally.

Either way, **the defining discipline is: exactly one thread mutates a graph, and ownership
is transferred, never shared.** That is what deletes the `unsafe impl Sync`.

### 4.3 Buffer-boundary timing

- The render thread already renders in whole blocks (`process_buffer` / `process_buffer_at`,
  `src/main.rs:1089`, `src/bin/phonon-audio.rs:315`). The command drain happens **at the top of
  the block loop, before the render call** — so a swap can only ever take effect *between*
  buffers, never mid-buffer. This is the natural, allocation-free boundary.
- Seam continuity is preserved by the **already-landed** `transfer_render_continuity`
  (`src/unified_graph.rs:6835`, called from `transfer_session_timing:6815`): `next` inherits
  `cur.prev_buffer_tail` (`:5295`), so the Phase-4d crossfade (`:9082-9093`) fires on `next`'s
  first block exactly as it does today. The render-owner move does **not** change the audio
  math of the seam; it only changes *which thread* and *when* the swap is applied. The D3 seam
  behavior is therefore unchanged (and a conformance assertion pins that).
- **Timing continuity is strengthened, not just preserved.** Because the render thread owns
  the clock too (generalising `unify-live-clock`, `src/main.rs:1062-1088`), a swap seeds
  `next`'s node position from the render-owned clock (`set_cycle_position`, cf.
  `src/main.rs:1080`) with no wall-clock re-read and no possibility of a reset — R1's beat jump
  cannot occur even in principle.

### 4.4 What stays on the control thread (must, for RT-safety)

- `compile_program` / `parse_program` (allocates, leaks strings via
  `src/compositional_parser.rs` — the P2 leak is orthogonal and tracked separately).
- `enable_wall_clock_timing()` (`src/unified_graph.rs:9837`).
- `preload_samples()` (`src/unified_graph.rs:6341`) — **disk I/O; must not move to the render
  thread.** The control thread finishes preloading before `swap_tx.push`, so the render thread
  only ever runs in-memory `transfer_*`.

### 4.5 Non-goals (explicitly out of scope for the render-owner change)

- **Voice preservation on swap (D1 / B8 / G7).** The migration keeps the current
  fade-on-swap (`transfer_voice_manager` → `release_*`, `src/unified_graph.rs:6688-6690`)
  **byte-for-byte**. Voice→node identity is a separate deferred design
  (`improvement-plan-2026-07.md` §5, `voice-preservation-on-swap`).
- **FX-state completeness (D2).** Already handled by the in-flight `complete-fx-state`; the
  migration calls the same `transfer_fx_states` (`src/unified_graph.rs:9354`).
- **Stale-ring latency / clear-and-crossfade (D4).** Independent ring-boundary question
  (`live-transition` §8 Rank 4); not entangled with graph ownership.
- **Per-buffer DAG allocations (P1).** `dag-scratch-arena` (already landed / wave-2) is
  orthogonal.

Keeping these fixed-in-place makes the render-owner change a **behavior-preserving refactor of
the concurrency model** — the only intended observable difference is "no more race, no more
R1/R2/R3 windows", which is exactly what the test plan (§6) asserts.

---

## 5. Migration strategy (per-frontend, golden-rule safe)

The four frontends live in four distinct files, so their migrations are **parallel-safe**
(golden rule: same file ⇒ sequential, distinct files ⇒ parallel). The shared primitive lives
in a **new** module so it is an independent root that does not serialise against the
frontends.

| Component | File(s) | Shares a file with any parallel task? |
|---|---|---|
| Swap primitive (`Cmd` enum, SPSC channel wrappers, `apply_pending_commands`, graveyard) | **new** `src/render_swap.rs` + one `mod render_swap;` line in `src/lib.rs` | No (new file); `lib.rs` line owned by the core task only |
| `transfer_*` reachable from render context | `src/unified_graph.rs` (already `&mut self`, single-writer — no change needed beyond visibility) | Serial after core if touched |
| `phonon live` migration | `src/main.rs` | No |
| `phonon-audio` migration | `src/bin/phonon-audio.rs` | No |
| modal editor migration | `src/modal_editor/mod.rs` | No |
| `LiveSession` retire/migrate | `src/live.rs` | No |

Sequencing: **core primitive → (parallel) four frontend migrations → verify**. The two test
enablers (§6) are independent roots that can be built in parallel with the core primitive and
must land before verify.

---

## 6. Test plan

Two categories, both required by the validation criteria: **(A) race proof** and **(B) live-
path conformance**. Neither exists today.

### 6.A Race detection (loom / TSan / Miri) — proves the C1 root is gone

There is **no** loom / TSan / Miri harness in the tree today (verified: no `loom`,
`cfg(loom)`, `miri`, or `sanitizer` references in `Cargo.toml`, `src/`, or `tests/`). The
existing concurrent harness `run_concurrent_session_mode` (`src/stress_harness.rs:2017`) spins
a real synth thread + `ArcSwap` + `RefCell` + `HeapRb` ring and can flip between
`SynthBorrow::Unconditional` (`:2091`, the pre-fix panic) and `SynthBorrow::TryBorrowSkip`
(`:2066`, the current code). But it can only observe the **panic symptom** (thread death /
permanent silence); it **cannot** prove or disprove the underlying **data race**, because a
data race is UB that a normal timed execution will not deterministically surface.

The race proof must therefore use tools that model or instrument memory ordering:

1. **loom model** (`cfg(loom)` test target, **new** `tests/loom_graph_swap.rs`). Model the
   minimal two-thread interaction: one "render" thread and one "reload" thread contending on
   the swap primitive. Under loom, exhaustively explore interleavings and assert **no data
   race and no lost/torn swap**.
   - *Baseline (must FAIL / be flagged):* a model of the **current** `RefCell` + `unsafe impl
     Sync` protocol — loom (or a hand-modeled non-atomic flag) exhibits the racy access.
   - *Target (must PASS):* the render-owner SPSC-channel model — the graph is single-owner;
     loom explores all channel interleavings and finds no UB and no dropped command.
2. **ThreadSanitizer lane** (CI job: `RUSTFLAGS="-Zsanitizer=thread" cargo +nightly test
   --target x86_64-unknown-linux-gnu concurrent_swap`). Run the concurrent harness
   (`src/stress_harness.rs:2017`) under TSan.
   - *Baseline:* TSan reports a race on the `RefCell` borrow flag in `SynthBorrow::TryBorrowSkip`
     mode (proving the symptom fix did not fix the race).
   - *Target:* post-migration, TSan is clean across a many-swap concurrent session.
3. **Miri** (`cargo +nightly miri test render_swap`) on the **primitive's** own unit tests
   (channel push/pop, graveyard hand-off, `apply_pending_commands`) to catch UB in the new
   unsafe-free code path and confirm no `unsafe impl Sync` remains. Miri does not run threads
   preemptively, so it validates the single-threaded ownership logic and any remaining
   `unsafe`, complementing loom/TSan for the cross-thread part.

Because ThreadSanitizer/Miri need nightly and are slow, the CI lane is a **separate,
non-blocking-for-PR-but-required-for-merge** job (mirrors how the budget harness is gated in
`src/stress_harness.rs`); loom runs in normal `cargo test --cfg loom` and is cheap enough to
gate.

### 6.B Live-path conformance suite — proves all three paths behave identically (I5)

A **new** `tests/live_path_conformance.rs` (plus a shared driver, reusing
`src/stress_harness.rs`'s program pool and detectors) that runs the **same scenario matrix**
against **each** frontend's swap primitive and asserts **identical** invariants. This is the
I5 "live-path unification conformance suite" (`improvement-plan-2026-07.md` I5, test-gap
P2-A), which the plan says "lands with the render-owner model".

Paths under test (the three reachable surfaces + the retired/migrated fourth):

- `phonon live` swap path (`src/main.rs`),
- `phonon-audio` swap path (`src/bin/phonon-audio.rs`, external-clock `process_buffer_at`),
- modal editor swap path (`src/modal_editor/mod.rs`),
- (`src/live.rs` only if it is migrated rather than deleted).

Invariants asserted for every path, driven through the real concurrent harness
(`run_concurrent_session_mode`, `src/stress_harness.rs:2017`, extended to target the shared
primitive):

| Invariant | Assertion | Guards |
|---|---|---|
| No permanent silence | `report.permanent_silence == false`, `synth_thread_alive == true` (`src/stress_harness.rs:2182-2188`) | C1 (no thread death / stuck ring) |
| No beat jump on load | cycle position after swap == expected continuation within ε; **zero** "could not transfer state" notes (contrast today's `src/stress_harness.rs:2155-2159`) | R1 |
| No underrun attributable to swap | ring occupancy never hits zero across a swap under nominal load; underrun counter unchanged by swaps | R2 |
| No voiceless window | voice-count trajectory has no swap-induced drop-to-zero-then-refill spike beyond the intended fade | R3 |
| Seam continuity unchanged | swap-boundary delta stays within the existing D3 crossfade envelope (no *new* click vs. current behavior) — `boundary_delta` (`src/stress_harness.rs:282`) | regression guard on the landed D3 fix |
| Cross-path equivalence | the invariant vector is **identical** (within tolerance) across all paths for the same seed | I5 unification |

The suite must run in both **pre-migration mode** (baseline: R1/R2/R3 notes present, TSan
dirty) and **post-migration mode** (all invariants green, TSan clean), so it doubles as the
regression gate. Wire it into `tests/smoke/manifest.toml` under the verify task's `owners`
so future regressions are caught by the smoke gate.

### 6.C Existing coverage reused (not rebuilt)

- Deterministic seeded driver `run_random_session` (`src/stress_harness.rs:998`) — glitch /
  budget / NaN gates; unchanged, run post-migration for regression.
- Concurrent harness `run_concurrent_session_mode` (`src/stress_harness.rs:2017`) — extended
  (not replaced) to drive the shared primitive and to run under TSan.
- Budget/deadline detectors (`Thresholds`, `evaluate_budget`, `src/stress_harness.rs`) —
  confirm the render-thread transfer stays within the block budget.

---

## 7. Phased implement → verify plan (filed as follow-on `wg` task stubs)

Filed by this task via `wg add`, rooted at `design-render-owner-swap`, with the golden-rule
dependency structure from §5. (Task IDs are the `wg`-assigned slugs; see `wg show
design-render-owner-swap` children.)

```
 design-render-owner-swap (this doc)
   ├─► render-owner-race-harness        (ENABLER I2: new tests/loom_graph_swap.rs + TSan CI lane; baseline dirty, target clean)
   ├─► render-owner-conformance-suite   (ENABLER I5: new tests/live_path_conformance.rs against the shared primitive)
   └─► render-owner-core-channel        (NEW src/render_swap.rs + lib.rs mod: SPSC Cmd ring + graveyard + apply_pending_commands)
          └─► render-owner-transfer-boundary  (render-thread transfer_* + boundary swap; render_swap.rs + unified_graph.rs)
                 ├─► migrate-phonon-live-render-owner   (src/main.rs only)            ┐
                 ├─► migrate-phonon-audio-render-owner  (src/bin/phonon-audio.rs only)├─ parallel (distinct files)
                 ├─► migrate-modal-editor-render-owner  (src/modal_editor/mod.rs only)│
                 └─► retire-or-migrate-livesession      (src/live.rs only)            ┘
                        │
 verify-render-owner-swap ◄── after all four migrations + both enablers
   (full cargo test + glitch_stress seeded clean + loom green + TSan/Miri clean + conformance green on every path)
```

Phase gating rationale:

- **Enablers first-or-parallel.** The race harness and conformance suite must exist (and show
  the *baseline* failing / TSan-dirty) before the migration lands, so the migration's success
  is demonstrable rather than asserted (`improvement-plan-2026-07.md` §5: "after the harness
  can actually exercise concurrency").
- **Core primitive before frontends.** `render-owner-core-channel` and
  `render-owner-transfer-boundary` build the shared mechanism; both touch `src/render_swap.rs`
  (and the latter `src/unified_graph.rs`) so they are a **serial** two-step chain.
- **Frontend migrations parallel.** Four distinct files ⇒ four parallel tasks, each "implement
  directly — do not decompose further".
- **Single verify join.** `verify-render-owner-swap` depends on all four migrations **and**
  both enablers, and only passes when the full suite (unit + glitch_stress + loom + TSan/Miri
  + conformance-on-every-path) is green.

Each stub carries a `## Validation` section with concrete acceptance criteria and its file
scope, and (for user-visible interactive behavior) requires the conformance suite / concurrent
harness rather than a CLI-only substitute, per the WG live-human-flow rule.

---

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Render thread now *owns* `transfer_*`; if a transfer is slow it eats block budget (new R2-like pressure) | Transfers are in-memory only (`preload_samples` stays on control thread, §4.4); the budget harness (`src/stress_harness.rs` `evaluate_budget`) gates transfer cost; a transfer that would overrun can be amortised over the crossfade region if measured to be a problem. |
| Dropping the retired graph on the render thread would free unboundedly | Graveyard SPSC ring → janitor thread (§4.1); assert (Miri/TSan) that no `Drop` of a graph runs on the render thread. |
| `phonon-audio`'s external clock (`process_buffer_at`) differs from wall-clock frontends | The primitive is clock-agnostic: the render loop keeps its own `process_buffer` vs `process_buffer_at` call; only the swap application is shared. Conformance suite runs the `phonon-audio` path with the external-clock render call. |
| Migrating four frontends is large | Behavior-preserving by construction (§4.5 non-goals keep voice/FX/ring semantics identical); the conformance suite pins equivalence to current behavior for everything except the race/R1/R2/R3 windows. |
| `LiveSession` deletion could break an external caller | It is unreachable from any CLI command (verified, §2.3); default to deletion, fall back to migration if any consumer is found. Either way the raw `borrow()` at `src/live.rs:341-345` must not survive. |

---

## 9. Validation of this design task

- [x] `docs/audits/design-render-owner-swap-2026-07.md` exists with **(a)** the current swap
      data-flow and where the race lives, every claim cited to `file:line` (§2, §2.4);
      **(b)** the render-owned swap design — ownership transfer (§4.1), channel /
      double-buffer / ArcSwap discipline (§4.2), buffer-boundary timing (§4.3) — plus what
      must stay off-RT (§4.4) and explicit non-goals (§4.5); **(c)** a phased implement→verify
      plan filed as follow-on `wg` task stubs (§7); **(d)** a test plan covering
      loom/TSan/Miri for the race (§6.A) and a live-path conformance suite across all three
      paths (§6.B).
- [x] Every claim about current behavior cites a `file:line`, re-verified against the current
      tree (not the source audits, which predate several landed wave-1 fixes).
- [x] No engine code modified — this task produces the document + follow-on `wg` task stubs
      only.
