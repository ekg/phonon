# Race detection: the render-owner graph-swap harness (ENABLER I2)

This harness proves the **C1 ROOT data race** on the shared graph cell, before
and after the render-owner migration. Background and the exact `file:line`
citations are in `docs/audits/design-render-owner-swap-2026-07.md` §6.A.

## What the defect is

The three live swap paths (`phonon live` in `src/main.rs`, `phonon-audio` in
`src/bin/phonon-audio.rs`, the modal editor in `src/modal_editor/mod.rs`; plus
the latent `src/live.rs`) share

```rust
struct GraphCell(RefCell<UnifiedSignalGraph>);
unsafe impl Send for GraphCell {}
unsafe impl Sync for GraphCell {}   // hand-written — UNSOUND
```

published behind `Arc<ArcSwap<Option<GraphCell>>>`. The `fix-synth-borrow-race`
work made **both** the render side and the reload side call `try_borrow_mut()`,
which removed the *panic symptom*. It did **not** remove the root defect:
`RefCell`'s borrow flag is a non-atomic `Cell<isize>`, and two threads calling
`try_borrow_mut()` on the same cell concurrently perform an unsynchronised
read-modify-write on that flag — a data race / UB under the Rust memory model,
regardless of whether either call returns `Err`. The `unsafe impl Sync` is what
lets the compiler accept this; it is unsound.

The fix is structural (render-thread-owned graph, single owner, SPSC handoff),
so the graph is touched by exactly one thread and the `unsafe impl Sync`
disappears. This harness is the *proof* gate for that change: **baseline dirty,
target clean.**

## Three tools, three layers

| Tool | What it checks | Runs where |
|---|---|---|
| **loom** | Exhaustive interleaving model: baseline racy, target clean | `cfg(loom)` lane |
| **ThreadSanitizer** | The *real* `RefCell` borrow flag racing under two threads | nightly lane |
| **Miri** | UB in the single-owner ownership logic / model primitives | nightly lane |

### 1. loom (`tests/loom_graph_swap.rs`)

loom is gated behind `--cfg loom` (see `[target.'cfg(loom)'.dev-dependencies]`
in `Cargo.toml`) so it is **never** pulled into a normal `cargo build`.

```bash
RUSTFLAGS="--cfg loom" cargo test --test loom_graph_swap --release
```

* `loom_models::baseline_refcell_protocol_is_racy_under_loom` — models the
  current `GraphCell(RefCell)` + `unsafe impl Sync` protocol. loom detects the
  concurrent unsynchronised access to the non-atomic borrow flag and flags it.
  The test is `#[should_panic]`: it **passes because loom correctly flags the
  race** (if loom ever stopped flagging it, the test would fail loudly).
* `loom_models::render_owner_handoff_is_race_free_under_loom` — models the
  render-owner mailbox handoff (atomic `Release`/`Acquire` around the slot,
  single graph owner). loom explores every interleaving and finds **no** race;
  the test passes clean. This is the shape the migration must satisfy.

Under a normal `cargo test` the file instead runs the deterministic
`hand_model` module (no loom dependency): a replayed interleaving showing the
non-atomic check-then-set admits an aliasing outcome, an atomic CAS does not,
and the single-owner SPSC handoff never lets two threads touch the graph. These
are always-green and non-flaky; loom is the machine-checked version of the same
argument.

### 2. ThreadSanitizer (`src/stress_harness.rs`)

`run_borrow_flag_race_probe(iterations, mode)` isolates the borrow-flag race
into a dependency-light two-thread probe so TSan can instrument it quickly. The
lane targets the `#[ignore]`d tests whose names contain `concurrent_swap`:

```bash
# nightly toolchain + rust-src for -Zbuild-std
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

RUSTFLAGS="-Zsanitizer=thread" \
  cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu \
  --lib concurrent_swap -- --ignored --nocapture
```

* **Baseline (current tree):** TSan reports a data race on `RefCell`'s borrow
  flag in `SynthBorrow::TryBorrowSkip` mode — proving the symptom fix did not
  fix the race.
* **Target (post-migration):** the render-owner single-owner path is run instead
  and TSan is clean across a many-swap concurrent session.

The heavier, end-to-end alternative is the existing concurrent harness
`run_concurrent_session_mode` (`src/stress_harness.rs`), also runnable under
TSan; the probe is preferred for the lane because it is fast under instrumentation.

### 3. Miri

```bash
rustup component add miri --toolchain nightly
cargo +nightly miri test --test loom_graph_swap
```

Miri runs the `hand_model` module (the aliasing model, the atomic-CAS fix
direction, and the single-owner SPSC handoff) to confirm no UB in the
render-owner path, complementing loom/TSan for the cross-thread part. The
report-only `baseline_double_acquire_is_observable_report_only` test is
`#[cfg_attr(miri, ignore)]`d — it performs an *intentional* non-atomic data
race to demonstrate the unsound protocol, which Miri correctly rejects; Miri is
there to validate the *sound* logic. Miri does not run threads preemptively, so
it validates the single-thread ownership logic, not the interleavings.

## CI

The GitHub Actions `race-detection` job (`.github/workflows/ci.yml`) runs the
loom lane (cheap, gating) plus the TSan and Miri lanes (nightly, slower). Per the
design, the sanitizer lanes are **required for merge but not PR-blocking**; loom
is cheap enough to gate on every PR.

## The contract this harness pins

* **Before the migration:** loom baseline flags the race; TSan is dirty on the
  borrow flag; the hand-model shows the aliasing interleaving. ✅ demonstrated.
* **After the migration:** loom target is clean; TSan is clean; no `RefCell` and
  no `unsafe impl Sync` remain on a live graph; Miri finds no UB. The
  `verify-render-owner-swap` task re-runs all three against the migrated code.
