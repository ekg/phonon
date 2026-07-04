# Design: Network Tempo Sync (Ableton Link) for Ensemble Live Coding

**Task:** `wave3-design-ableton-link` · **Date:** 2026-07-04 · **Status:** design-first (no engine code in this task)
**Feature gap:** #11 — *Ableton Link / network tempo sync* (`docs/audits/feature-gap-2026-07.md:74,152`)
**Precedent:** mirrors `design-render-owner-swap` → implement sub-graph → verify (`docs/audits/design-render-owner-swap-2026-07.md`).

---

## 1. Executive summary

Phonon has MIDI in/out (`src/midi_input.rs`, `src/midi_output.rs`) and OSC
(`src/osc_control.rs`, `src/osc_live_server.rs`), but **no shared clock** for
playing in time with other Phonon instances or with other apps (Ableton Live,
SuperCollider, TidalCycles, Traktor, iOS). Verified absent: `ls src/*link*` →
**none** (no `link` module anywhere under `src/`).

This is the one genuinely-missing *performer* capability. It is HIGH effort
because it needs (a) an external crate with a native C++ dependency, and (b) a
change to how the **already-unified live clock** (T1) is driven — it must now be
able to *follow* an external tempo+phase without violating the two hard timing
invariants that waves 1–2 established:

- **T1 — sample-advancing clock, no per-buffer wall-clock re-anchor.** The live
  clock advances by *samples emitted*, never by wall-clock at render time; wall
  time is consulted *only to rebase* at startup / resync / post-underrun
  (`src/unified_graph.rs:4878-4886,4939-4941,4980-4981`). A network clock must
  fold in the *same way* — a controlled rebase/varispeed, **not** a per-buffer
  teleport.
- **T2 — f64 trigger timekeeping.** `last_trigger_time` is an f64 absolute
  cycle position (`src/unified_graph.rs:1080`); the Link↔cycle mapping must stay
  f64 end-to-end so it inherits the no-drift guarantee rather than reintroducing
  an f32 precision cliff.
- **C1 — the graph is `Send`-only, not `Sync`.** `unsafe impl Send for
  UnifiedSignalGraph` with a deliberately-absent `Sync`
  (`src/unified_graph.rs:5412,5420`) is what closed the render-owner data race.
  The Link session must **not** re-share the graph across threads.

**Recommendation (short version):** add a source-agnostic clock adapter
`src/link_clock.rs` (pure library, no native dep) that maps a shared *tempo+beat
phase* onto Phonon's `cps`/`cycle_position`, with a `rusty_link` backend behind
an **off-by-default `link` Cargo feature** and a zero-dep internal OSC
leader/follower as the always-available fallback. Feed the *existing* single-owner
clock (`LiveClock` / `GlobalClock`) through the *existing* lock-free snapshot /
render-owner-command paths. No new cross-thread sharing of the graph; no
per-buffer teleport.

---

## 2. Current clock survey (with citations)

Phonon already **unified** its live clock in wave 1 (`unify-live-clock`, T1). There
are two render frontends and they share one clock *model*, instantiated twice.

### 2.1 `GlobalClock` — the `phonon-audio` external-clock authority

`src/bin/phonon-audio.rs:105-167`. A `#[derive(Clone, Copy)]` struct holding
`base_time: Instant`, `base_cycle_position: f64`, `cps: f32`, `sample_rate: f32`
(`:105-114`).

- Position is derived from wall-clock: `get_position() = base_cycle_position +
  elapsed * cps` (`:129-132`).
- `set_cps()` **re-anchors before changing tempo** so a tempo change never
  teleports the cycle position: it saves `get_position()` into
  `base_cycle_position`, resets `base_time`, then sets `cps`
  (`:142-151`).
- `get_buffer_timing()` returns `(position, increment, cps)` atomically for one
  buffer (`:162-166`).
- **RT-safety (audit F-3):** published as a lock-free `ArcSwap<GlobalClock>`
  double-buffered snapshot, **not** a `Mutex` (`:98-102,273`). The render thread
  reads it once per buffer with a lock-free `.load()` (`:341-344`); the IPC
  thread is the single writer (load → copy → `set_cps` → store, `:666-668`).

### 2.2 `LiveClock` — the `phonon-live` / `main.rs` sample-advancing clock (T1)

`src/unified_graph.rs:4891-4988`. This is the T1 fix (pt-F1/pt-F2). Fields:
`cycle_position: f64` (accumulated from samples, **not** wall-clock),
`cps: f32`, `sample_rate: f32`, plus a wall-clock `anchor_time`/`anchor_position`
used *only* for rebasing (`:4893-4904`).

- `advance_buffer(n)` returns `(start_cycle, increment, cps)` and advances by
  **exactly `n` samples** (`:4943-4948`) — the steady-state render entry point
  that *never reads the wall clock*.
- `set_cps(new_cps)` re-anchors at the current preserved position, so tempo
  changes are continuous — "only the per-sample increment differs going forward"
  (`:4957-4965`).
- `set_position(pos)` is the explicit jump (setCycle / seek / resync)
  (`:4968-4972`).
- `rebase_to_wall_clock()` is the **only** wall-clock read, and its doc-comment
  is explicit that it is *not* called on the steady-state path, "that would
  reintroduce the pt-F1 clustering" (`:4974-4987`).

**The T1 invariant, stated for this design:** the clock is driven by
*samples emitted*; any external time source (wall clock **or Link**) may only
enter through a deliberate, bounded rebase — never a per-buffer snap.

### 2.3 How the two render loops advance cycles

- **`main.rs` (phonon-live)** `src/main.rs:1073-1152`. The render thread owns a
  single `Option<LiveClock>` — "THE single source of timing truth" (`:1073`).
  First real graph seeds the clock from the compiled position (`:1113-1117`);
  each buffer follows the graph tempo via `c.set_cps(cur.get_cps())` (`:1122`);
  on a swap it seeds the incoming graph from the live clock with
  `cur.set_cycle_position(c.position())` (`:1129`); then
  `c.advance_buffer(frames)` → `cur.process_buffer_at(...)` (`:1137-1138`).
- **`phonon-audio`** `src/bin/phonon-audio.rs:322-361`. Each buffer loads the
  `GlobalClock` snapshot lock-free, `get_buffer_timing()` → `process_buffer_at`
  (`:341-361`). Tempo changes arrive as `IpcMessage::SetTempo` and update the
  `ArcSwap<GlobalClock>` **and** route `Cmd::SetTempo` through the render-owner
  ring (`:660-675`).

### 2.4 The render-owner model the Link session must respect (C1)

Both frontends are **single-owner** (`src/render_swap.rs`): the render thread
solely owns the live `Box<UnifiedSignalGraph>` (`src/main.rs:1077`,
`src/bin/phonon-audio.rs:320`); the control thread hands new graphs and control
commands across a bounded lock-free SPSC command ring
(`Cmd::{Swap,Hush,Panic,SetTempo,SetCycle}`, `src/render_swap.rs:112-121`), each
applied at a buffer boundary by `apply_pending_commands`
(`src/render_swap.rs:218`, `src/main.rs:1094`, `src/bin/phonon-audio.rs:329`).

`UnifiedSignalGraph` is now `Send`-**only**: `unsafe impl Send`
(`src/unified_graph.rs:5420`) with `Sync` **deliberately absent** — the comment
at `:5410-5419` states re-adding `Sync` "would reintroduce the ability to alias a
`&UnifiedSignalGraph` across threads and mutate its `RefCell`s concurrently — the
exact data race the render-owner migration eliminated," locked in by the
`render_owner_graph_is_send_but_not_sync` regression test.

---

## 3. Sync-source options

| Option | Interop | Native dep / license | Threading | Jitter / phase | Verdict |
|---|---|---|---|---|---|
| **`rusty_link` (Ableton Link)** | Excellent — Live, SuperCollider (`LinkClock`), Tidal (`hs-abletonlink`), Traktor, Reason, many iOS apps | Native C++ (Link built via `cc`/submodule); Link core is **GPLv2+ or commercial** (rusty_link bindings are MIT, but the linked Link library sets the effective license) | Link runs its own realtime-safe network/timer threads; audio thread uses lock-free `capture_audio_session_state` / `commit_audio_session_state` | Sub-ms on LAN, drift-corrected phase-lock — purpose-built | **Recommended** primary, behind an off-by-default feature |
| **OSC / MIDI-clock fallback** | MIDI clock talks to any DAW/hardware (tempo only); OSC is Phonon-bespoke | **Zero** new dep — reuses `src/midi_output.rs`/`src/midi_input.rs` (24 ppqn `0xF8` + start/stop/continue) and `src/osc_control.rs` | Existing MIDI/OSC threads | MIDI clock is **tempo-only** (no absolute bar phase; phase must be inferred from `start`), higher jitter, **no peer discovery**, no drift correction | **Recommended** fallback / stopgap |
| **Internal-only leader/follower** | Phonon↔Phonon only | Zero dep — one instance broadcasts `cps`+`cycle_position` over UDP/OSC | Existing OSC threads | Must reinvent Link's latency/drift compensation (the hard part) | Subsumed by the OSC fallback backend |

**Trade-off analysis.**

- **Native dep + license is the decisive constraint.** Ableton Link's core is
  GPLv2+ unless a commercial license is obtained; statically linking it into the
  default Phonon binary would impose GPL on the whole binary. Mitigation: gate
  `rusty_link` behind a **`link` Cargo feature that is OFF by default**, so the
  stock build has no native toolchain requirement and no GPL entanglement. Only
  a user who opts into `--features link` pulls it in.
- **Jitter/quality:** Link is the only option that delivers ensemble-grade
  *phase* lock (shared bar position, drift-corrected). MIDI clock gives tempo but
  not absolute phase; an internal protocol would have to re-solve drift
  compensation, which is exactly Link's value-add.
- **Threading:** Link's own threads are fine — the risk is *how Phonon reads
  Link*, not Link itself. See §5.
- **Reuse:** the OSC/MIDI fallback reuses shipped modules, keeps a zero-dep
  build fully functional, and covers the Phonon↔Phonon ensemble case without any
  license question.

**Decision:** build a **source-agnostic adapter** so tempo source is pluggable:

```
trait TempoSource {                         // src/link_clock.rs (no native dep)
    /// Latest shared tempo, in BPM.
    fn tempo_bpm(&self) -> f64;
    /// Beat position on the shared timeline at wall-clock instant `at`
    /// (Link "beat"; monotonic f64 beats since session epoch).
    fn beat_at(&self, at: Instant) -> f64;
    /// Bar length in beats (Link "quantum"); maps to one Phonon cycle.
    fn quantum(&self) -> f64;
    fn is_playing(&self) -> bool;
}
```

- `rusty_link` backend (`src/link_backend_rusty.rs`, `#[cfg(feature="link")]`)
  implements `TempoSource` over `AblLink` / `SessionState`.
- OSC leader/follower backend implements the same trait with zero native dep.
- Tests use an in-process mock `TempoSource` — no network, deterministic.

---

## 4. Mapping a shared tempo+phase onto Phonon cps/cycle (no teleport)

**Link model:** shared tempo (BPM), a continuous f64 **beat** timeline, a
**quantum** (bar length in beats), and **phase = beat mod quantum**.
**Phonon model:** `cps` (cycles/sec) and continuous f64 `cycle_position`.

### 4.1 The constants and conversions (pure, f64, in `link_clock.rs`)

Choose `beats_per_cycle` (default **4**, configurable) so **one Phonon cycle ==
one Link bar** of `quantum = beats_per_cycle` beats:

```
cps            = tempo_bpm / 60.0 / beats_per_cycle
target_cycle   = link_beat / beats_per_cycle           // absolute, f64
```

Because Link beats are f64 and Phonon's trigger timekeeping is already f64
(`last_trigger_time: f64`, `src/unified_graph.rs:1080`, T2), this mapping keeps
full precision on long sets — no f32 cliff (respects **T2**).

### 4.2 Tempo (the easy half)

When the source reports a new tempo, convert to `cps` and route it through the
**existing** no-teleport tempo path:

- `main.rs`: `LiveClock::set_cps(new_cps)` (`src/unified_graph.rs:4957-4965`),
  reached today via `cur.get_cps()` → `c.set_cps` (`src/main.rs:1122`).
- `phonon-audio`: `GlobalClock::set_cps` (`src/bin/phonon-audio.rs:142-151`),
  reached today via the `SetTempo` IPC path (`:666-668`) and/or
  `Cmd::SetTempo` (`src/render_swap.rs:120`).

Both re-anchor before changing tempo, so a Link tempo change is continuous by
construction (**pt-F2 / T1** preserved for free).

### 4.3 Phase (the subtle half) — controlled convergence, never a per-buffer snap

Link supplies **absolute** phase; Phonon **accumulates** position from samples.
Snapping `set_position` to Link's phase every buffer would be *exactly the T1
anti-pattern* — re-anchoring to an external clock every buffer reintroduces the
onset clustering / jitter LiveClock was built to eliminate
(`src/unified_graph.rs:4980-4981`). Instead, three regimes:

1. **Join / explicit resync (allowed rebase).** On first lock or an explicit
   `link resync`, compute `target_cycle = beat_at(now)/beats_per_cycle` for the
   buffer's presentation time and call `LiveClock::set_position` **once**
   (`src/unified_graph.rs:4968-4972`) — a deliberate seek, semantically identical
   to `setCycle`. On the `phonon-audio` path this is `Cmd::SetCycle`
   (`Cmd::SetCycle` `src/render_swap.rs:122`, `RenderGraph::set_cycle` `:99-102`)
   — **no new command variant required**.

2. **Steady state (bounded varispeed correction).** Keep advancing by samples
   (`advance_buffer`) and, once per buffer, measure
   `err = target_cycle − live.position()` and nudge tempo by a tiny proportional
   term `cps *= (1 + k*err)`, clamped so the effective rate change is inaudible
   (≤ ~0.5 %). Phase error converges over a few seconds with no discontinuity:
   position stays accumulated and monotonic (T1 intact), yet the clock tracks the
   network. This is the same "soft" phase-adjust that SuperCollider's `LinkClock`
   uses instead of a hard jump. The correction is applied by folding `err` into
   the value passed to the existing `set_cps`, so no new render-path mutator is
   introduced.

3. **Large-error fallback (dropout / new peer / post-underrun).** If
   `|err|` exceeds a large threshold (e.g. > half a cycle), the soft correction
   would take too long / be audible as a long slew — fall back to a single hard
   `set_position` reseek (regime 1). This is the *same* controlled-rebase
   category as `rebase_to_wall_clock` after an underrun
   (`src/unified_graph.rs:4974-4987`): rare, deliberate, not per-buffer.

All of the `err`/nudge/threshold math lives in `src/link_clock.rs` as pure,
unit-testable functions; the frontends only call `set_cps` / `set_position`
(main) or enqueue `Cmd::SetTempo` / `Cmd::SetCycle` (phonon-audio).

---

## 5. Thread-safety vs the render-owner model (must not reopen C1)

**Where the Link session lives:** on the **control side**, never attached to the
graph. `AblLink` owns its own network/timer threads internally — that is fine and
independent of `UnifiedSignalGraph`. The hard rule: **do not give the render
thread an `&`-shared reader of the graph, and do not attach any Link handle to
the graph** — the graph is `Send`-only, not `Sync`, precisely to forbid
cross-thread aliasing (`src/unified_graph.rs:5412,5420`). Re-adding `Sync` to
carry a Link reader would reintroduce the C1 race
(`src/unified_graph.rs:5414-5419`).

The clock — not the graph — is the thing Link updates, and both frontends already
have a **single-writer, lock-free** channel to their clock. Reuse them verbatim:

- **`phonon-audio`.** A control-side **Link reader thread** captures Link's
  `SessionState` at a cadence, computes `(cps, target_cycle)` via `link_clock`,
  and (a) updates the `ArcSwap<GlobalClock>` exactly like the current `SetTempo`
  handler (load → copy → `set_cps` → store, `src/bin/phonon-audio.rs:666-668`)
  and (b) enqueues a `Cmd::SetCycle` for a hard reseek when needed
  (`src/render_swap.rs:122`). The render thread keeps reading the `GlobalClock`
  snapshot lock-free once per buffer (`:341-344`). **No new sharing, no lock on
  the render path** (RT-safety F-3, `:98-102`).

- **`main.rs`.** The `LiveClock` lives **on the render thread**
  (`src/main.rs:1073`), so we cannot let another thread mutate it. Mirror
  `phonon-audio`'s discipline: the Link reader thread is the single writer to a
  lock-free `ArcSwap<LinkSnapshot>` (`LinkSnapshot { cps, target_cycle, epoch,
  playing }`); the render loop `.load()`s it once per buffer (like the
  `GlobalClock` load) and folds it into its existing
  `set_cps` / `advance_buffer` / optional-`set_position` step
  (`src/main.rs:1119-1138`). The render thread stays the sole mutator of
  `LiveClock`; no `&mut` crosses a thread; the graph stays `Send`-only.

**Non-negotiables** (all inherited from the render-owner + F-3 audits):

- No `Arc<Mutex<…>>` / `.lock()` on the render path — use `ArcSwap` snapshots
  (`src/bin/phonon-audio.rs:98-102,339`).
- No `unsafe impl Sync for UnifiedSignalGraph` (assert
  `render_owner_graph_is_send_but_not_sync` still passes).
- The Link native dep is optional (`#[cfg(feature="link")]`) so the default
  build's Send-only, lock-free, zero-native-dep invariants are unchanged.

---

## 6. Test plan

- **Unit (core, no audio/native):** BPM↔cps and beat↔cycle round-trips;
  phase-error → clamped `cps` nudge (monotone, bounded ≤0.5 %); large-error →
  hard-reseek decision boundary; quantum≠beats_per_cycle handling. All against a
  mock `TempoSource`.
- **Convergence / no-teleport (the marquee test):** drive a `LiveClock` from a
  mock source with a fixed phase offset; assert (a) `position()` is **monotonic
  non-decreasing** every buffer (no teleport in steady state), (b) phase error
  decays to ~0 within N buffers, (c) `set_position` is called **only** at join /
  large-error, never per steady buffer. This is the T1 guard for the Link path.
- **Feature build:** `cargo build --features link` compiles the `rusty_link`
  backend; default `cargo build` still has zero native dep.
- **Regression:** `render_owner_graph_is_send_but_not_sync` still passes (C1
  stays closed); full `cargo test` green.
- **Interop (manual / gated):** two Phonon instances (OSC backend) and, with
  `--features link`, one Phonon + Ableton Live agree on bar phase.

---

## 7. Phased implement → verify plan (filed as follow-on `wg` task stubs)

Filed by this task via `wg add`, rooted at `wave3-design-ableton-link`, with
golden-rule file scoping. New logic is isolated in **new files**
(`src/link_clock.rs`, `src/link_backend_rusty.rs`) so the risky shared files are
touched minimally and serially.

```
 wave3-design-ableton-link (this doc)
   └─► link-clock-core            (NEW src/link_clock.rs + lib.rs mod line)
          ├─► link-backend-rusty  (Cargo.toml `link` feature + NEW src/link_backend_rusty.rs + lib.rs cfg-mod)
          │        --after link-clock-core, wave3-dsl-fuzzing   ← both edit Cargo.toml (golden rule)
          ├─► link-frontend-main            (src/main.rs only)            ┐ parallel (distinct files)
          └─► link-frontend-phonon-audio    (src/bin/phonon-audio.rs only)┘  each --after link-clock-core
                 │
 verify-link-clock ◄── after link-backend-rusty + both frontends
   (full cargo test + `--features link` build + convergence/no-teleport test + C1 Send-not-Sync assert)
```

**File-scope matrix (golden rule — no two concurrent tasks edit the same file):**

| Task | New files | Edited shared files | Depends on |
|---|---|---|---|
| `link-clock-core` | `src/link_clock.rs` | `src/lib.rs` (add `mod`) | this doc |
| `link-backend-rusty` | `src/link_backend_rusty.rs` | `Cargo.toml`, `src/lib.rs` (cfg-mod, serial after core) | `link-clock-core`, `wave3-dsl-fuzzing` |
| `link-frontend-main` | — | `src/main.rs` | `link-clock-core` |
| `link-frontend-phonon-audio` | — | `src/bin/phonon-audio.rs` | `link-clock-core` |
| `verify-link-clock` | `tests/link_clock_convergence.rs` | tests only | the three leaves |

Notes:
- `src/render_swap.rs` and `src/unified_graph.rs` are **not** edited — phase
  reseek reuses the existing `Cmd::SetCycle` / `RenderGraph::set_cycle`
  (`src/render_swap.rs:99-102,122`) and `LiveClock::set_position` already exists
  (`src/unified_graph.rs:4968-4972`). This keeps the two race-critical files out
  of the Link work entirely.
- `Cargo.toml` is a golden-rule serialization point: `wave3-dsl-fuzzing` also
  edits it (dev-dep, `docs/audits/wave3-scope-2026-07.md` §4a), so
  `link-backend-rusty` is chained `--after wave3-dsl-fuzzing` — the two Cargo.toml
  editors never run concurrently.
- `src/lib.rs` is edited by both `link-clock-core` and `link-backend-rusty`, but
  serially (`link-backend-rusty --after link-clock-core`), so the golden rule
  holds.
- The frontends depend on the *trait* (core) and use a mock `TempoSource`, so
  they can land and be tested before the real backend; `verify-link-clock`
  depends on the backend too for the `--features link` integration build.

---

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| GPL entanglement from linking Ableton Link | `link` feature OFF by default; stock build never links it; OSC fallback keeps ensemble play working license-free. |
| Re-introducing the C1 race by sharing a clock/Link handle into the graph | Link updates flow to the *clock* via the existing single-writer `ArcSwap` / render-owner command paths; graph stays `Send`-only; regression test asserts no `Sync`. |
| Per-buffer phase snap re-creates pt-F1 onset clustering | Steady-state uses bounded varispeed folded into `set_cps`; `set_position` only at join / large-error (§4.3); convergence test asserts monotonic position + no per-buffer `set_position`. |
| Native build breaks CI runners without a C++ toolchain | Feature is off by default; a dedicated `--features link` CI lane covers it; core + frontends are testable with the mock source and no native dep. |
| Link reader cadence adds jitter | Reader runs on the control side, not the render path; render thread only does a lock-free `.load()` once per buffer, identical to today's `GlobalClock` read. |
| Cargo.toml merge collision with `wave3-dsl-fuzzing` | `link-backend-rusty --after wave3-dsl-fuzzing` serializes the two editors (golden rule). |

---

## 9. Validation of this design task

- [x] Doc exists with the clock survey (§2), sync-source options + recommendation
      (§3), cps/phase mapping without teleport (§4), thread-safety vs render-owner
      (§5), test plan (§6), and a phased implement→verify plan (§7).
- [x] Every code claim cites a real `file:line` verified against HEAD:
      `GlobalClock` (`src/bin/phonon-audio.rs:105-167,341-344,660-675`),
      `LiveClock`/T1 (`src/unified_graph.rs:4878-4988`), the two render loops
      (`src/main.rs:1073-1152`, `src/bin/phonon-audio.rs:322-361`), T2 f64 triggers
      (`src/unified_graph.rs:1080`), render-owner / C1 Send-not-Sync
      (`src/unified_graph.rs:5410-5420`, `src/render_swap.rs:99-121,218`), absence
      of a link module (`ls src/*link*` → none), and existing MIDI/OSC modules.
- [x] The implement→verify sub-graph is FILED as `wg` tasks (§7) with golden-rule
      deps and a `## Validation` section each, so `verify-wave3` can confirm the
      follow-on graph exists.
- [x] No engine code modified — this task produces the document + follow-on `wg`
      task stubs only.
