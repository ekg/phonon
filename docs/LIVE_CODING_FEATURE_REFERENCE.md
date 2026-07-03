# Phonon — Live-Coding Feature Reference ("What Actually Works")

**Status date:** 2026-07-03
**Scope:** The single, code-verified reference for performers. Supersedes the stale
"missing / next-priority" lists in `UGEN_STATUS.md` (2025-11-13), `CLAUDE.md`
"Next Priority Features", `docs/LIVECODE_COMPATIBILITY_TODO.md`, and
`docs/DSL_GAPS_DISCOVERED.md`.

**Verification method:** Every "works" claim below is backed by either a `file:line`
code reference **or** a copy-pasteable example that was actually rendered with
`phonon render` on this branch (2026-07-03). The examples in this file are literally the
ones that were run — they render non-silent audio unless noted. Where a feature is a
**known gap**, it is listed in [§10](#10-known-gaps) with a link to the filed wave-2 task.

> **Read this first — three rules that trip up newcomers**
>
> 1. **You must route to output.** A bare `s "bd sn"` on its own line is *not* auto-played;
>    it errors and renders silence. Always assign to `out` (or `out1`/`out2`), e.g.
>    `out $ s "bd sn"`. (`out:` colon form also works.)
> 2. **`$` applies pattern transforms, `#` applies signal modifiers/effects.** When you use
>    both on one source, put **`$` transforms first, then `#` modifiers**:
>    `s "hh*8" $ degrade # gain 0.4` ✅ — the reverse (`# gain 0.4 $ degrade`) can fail to parse.
> 3. **Space-separated calls only.** `lpf 1000 0.8`, never `lpf(1000, 0.8)`.

---

## Quick smoke test

```phonon
out $ s "bd sn hh*4 cp"
```

```bash
phonon render smoke.phonon smoke.wav -d 2      # renders ~2 s, prints Peak/RMS
phonon play   smoke.phonon                     # render + auto-play
phonon edit   smoke.phonon                     # modal live-coding editor (C-x eval, C-h hush)
phonon live   smoke.phonon                     # file-watch live-reload
```

---

## 1. Sample playback (`s`)

Voice-based polyphonic sample playback (64 voices). Samples resolve from `./samples/`,
`~/phonon/samples/`, then `~/phonon/dirt-samples/` (`src/sample_loader.rs:261-296`).

| Feature | Example (renders) |
|---|---|
| Basic pattern | `out $ s "bd sn hh*4 cp"` |
| Sample-bank select `:N` | `out $ s "bd:0 bd:1 bd:2"` |
| Euclidean rhythm | `out $ s "bd(3,8) hh*4"` |
| Rests / subdivision | `out $ s "bd ~ sn ~"` |

```phonon
-- Copy-paste: a bank-selected, Euclidean drum line
out $ s "bd:0(3,8) hh*8 sn:2"
```

Mini-notation (`src/mini_notation_v3.rs`) supports `*` (repeat), `~` (rest), `[]`
(subdivision), `<>` (alternation per cycle), `(n,k)` / `(n,k,r)` (Euclid), `:N` (bank).

---

## 2. Synthesis — oscillators & sources

Dispatched in `src/compositional_compiler.rs:2934-2996`. Every frequency/param accepts a
**pattern** (`"55 110 220"`) or a bus reference, per the core architectural rule.

| UGen | Example (renders) | Ref |
|---|---|---|
| `sine` / `saw` / `square` / `tri` | `out $ saw "55 110 220" # lpf 1000 0.8 * 0.3` | `:2934-2937` |
| `fm` (FM oscillator) | `out $ fm 110 220 2 * 0.3` | `:2938` |
| `pulse` / PWM | `out $ pulse 110 0.3 * 0.3` | `:2958` |
| `noise` (white) / `pink` | `out $ noise # lpf 2000 0.7 * 0.2` | `:2975-2976` |
| `moog_ladder` (needs explicit input) | `out $ moog_ladder (saw 110) 1000 0.7 * 0.3` | `:3003` |
| `pluck` (Karplus–Strong) | `out $ pluck "220 330" 0.5 * 0.3` | `:2944` |
| SuperDirt synths | `out $ supersaw "55 110" * 0.2` · `out $ superkick "1 ~ 1 ~"` | `:2988-2995` |

Also compiled and available (see the dispatch block for the full list): `pm`, `blip`,
`vco`, `wavetable`, `granular`, `waveguide`, `formant`, `vowel`, `additive`,
`white_noise`/`pink_noise`/`brown_noise`, `impulse`, `superpwm`, `superchip`, `superfm`,
`supersnare`, `superhat`, plus the fundsp variants `saw_hz`/`square_hz`/`triangle_hz`/
`organ_hz`/`moog_hz`.

```phonon
-- Copy-paste: FM bell into a lowpass
out $ fm "110 165" 440 3 # lpf 3000 0.6 * 0.3
```

---

## 3. Filters (incl. resonant)

`compile_filter` + dedicated resonant nodes. **Resonant filters are implemented and
render** (real RBJ-biquad DSP in `src/unified_graph.rs:13224+`) — see the correction in
[§9](#9-corrections-to-earlier-status-docs).

| Filter | Example (renders) | Ref |
|---|---|---|
| `lpf` / `hpf` / `bpf` / `notch` | `out $ saw 110 # lpf 1000 0.8 * 0.3` | `compiler:2998-3001` |
| `rlpf` (resonant lowpass) | `out $ saw 110 # rlpf 800 8 * 0.3` | `compiler:6696`, `unified_graph:13300` |
| `rhpf` (resonant highpass) | `out $ saw 220 # rhpf 400 8 * 0.3` | `compiler:6724` |
| `resonz` (resonant bandpass) | `out $ saw 110 # resonz 900 20 * 0.3` | `compiler:6668`, `unified_graph:13224` |
| `allpass` | `out $ saw 110 # allpass 500 * 0.3` | `compiler:6530` |
| `svf_lp/hp/bp/notch` (state-variable) | `out $ saw 110 # svf_lp 800 5 * 0.3` | `compiler:6556` |
| `bq_lp/hp/bp/notch` (biquad) | `out $ saw 110 # bq_lp 900 0.7 * 0.3` | `compiler:6612` |
| `comb` | `out $ noise # comb 200 0.8 * 0.2` | `compiler:3002` |
| `parametric_eq` / `eq` (3-band, explicit input) | `out $ eq (s "bd*4") 200 3 0.7 1000 -2 0.7 4000 2 0.7` | `compiler:3004` |

```phonon
-- Copy-paste: resonant sweep (the classic acid line)
~env $ sine 0.25
out $ saw "55 55 82.5 55" # rlpf (~env * 1800 + 400) 9 * 0.3
```

> **Gotcha:** `moog_ladder` and `parametric_eq`/`eq` take their input as the **first
> positional argument** and do **not** support `#` chaining — write `moog_ladder (saw 110)
> 1000 0.7`, not `saw 110 # moog_ladder 1000 0.7` (the latter errors with an internal
> `ChainInput` marker). Every other filter above chains normally with `#`.

---

## 4. Effects

Dispatched in `src/compositional_compiler.rs:3007-3045`. Chain with `#`.

| Effect | Example (renders) | Args |
|---|---|---|
| `delay` | `out $ s "bd sn" # delay 0.25 0.4 0.5` | time feedback mix |
| `reverb` | `out $ s "bd sn" # reverb 0.3 0.5` | room mix |
| `distortion` / `dist` | `out $ saw 110 # distortion 5 * 0.2` | drive |
| `bitcrush` | `out $ saw 110 # bitcrush 4 8000 * 0.2` | bits rate |
| `chorus` | `out $ saw 220 # chorus 0.5 0.3 * 0.2` | rate depth |
| `compressor` / `comp` | `out $ s "bd*4" # compressor 0.3 4 0.01 0.1 2.0` | thresh ratio atk rel makeup |
| `expander` / `expand` | `out $ s "bd*4" # expander 0.1 2 0.01 0.1` | thresh ratio atk rel |

Also compiled: `tapedelay`/`tape`, `multitap`, `pingpong`, `plate`, `lush`, `flanger`,
`sidechain_compressor`, `coarse`, `djf`, `ring`, `tremolo`/`trem`, `vibrato`/`vib`,
`phaser`/`ph`, `freeze`, `convolve`. Envelopes: `adsr`, `ad`, `env`/`env_trig`, `line`,
`curve`, `segments`.

```phonon
-- Copy-paste: dub-delayed snare with an ADSR-shaped synth stab
out $ saw "220 330" # adsr 0.01 0.1 0.7 0.2 # delay 0.375 0.5 0.4 * 0.3
```

> **Signature note:** each effect validates its arg count and errors clearly if wrong
> (e.g. `compressor requires 5 parameters ...`). When in doubt, render — the error names the
> expected params.

---

## 5. Pattern transforms (`$`)

The `Transform` enum carries ~90 variants. Simple transforms are parsed in
`parse_transform_from_call` (`src/compositional_compiler.rs:30-118`); the full known-name
table (incl. higher-order combinators) is at `:638-642`. All examples below render.

| Transform | Example (renders) |
|---|---|
| `fast` / `slow` | `out $ s "bd sn" $ fast 2` |
| `rev` / `palindrome` | `out $ s "bd sn hh cp" $ rev` |
| `every n f` | `out $ s "bd sn hh cp" $ every 2 rev` |
| `jux f` (stereo split) | `out $ s "bd sn hh cp" $ jux rev` *(add `--stereo`)* |
| `degrade` / `degradeBy` | `out $ s "hh*8" $ degrade` |
| `sometimes` / `sometimesBy` | `out $ s "hh*8" $ sometimesBy 0.3 (# speed 2)` |
| `chop n` / `striate n` | `out $ s "bd" $ chop 4` |
| `hurry n` | `out $ s "bd sn" $ hurry 2` |
| `ply n` | `out $ s "bd sn" $ ply 2` |
| `iter n` | `out $ s "bd sn hh cp" $ iter 4` |
| `chunk n f` | `out $ s "bd sn hh cp" $ chunk 4 rev` |
| `within b e f` | `out $ s "bd sn hh cp" $ within 0 0.5 rev` |
| `stut n t d` | `out $ s "bd" $ stut 4 0.1 0.5` |
| `off t f` | `out $ s "bd sn" $ off 0.125 rev` |
| chained transforms | `out $ s "bd sn hh cp" $ fast 2 $ rev` |

Also available: `rotL`/`rotR`, `early`/`late`, `squeeze`, `fastGap`, `shuffle`/`scramble`,
`loopAt`, `slice`, `swing`, `groove`, `compress`, `zoom`, `struct`, `mask`, `sew`, `bite`,
`superimpose`/`layer`, `often`, `foldEvery`.

```phonon
-- Copy-paste: evolving break — reversed every 4th cycle, occasionally sped up
out $ s "breaks165:0 breaks165:1 breaks165:2 breaks165:3" $ every 4 rev $ sometimesBy 0.25 (fast 2)
```

---

## 6. Per-event DSP params (`#`)

Sample/voice parameters — every one accepts a **pattern**
(`src/compositional_compiler.rs:3060-3072`).

| Param | Example (renders) |
|---|---|
| `gain` | `out $ s "bd sn" # gain 0.9` |
| `pan` (pattern) | `out $ s "bd sn hh cp" # pan "0 0.3 0.7 1"` *(add `--stereo`)* |
| `speed` (pattern) | `out $ s "bd*4" # speed "1 1.5 0.5 2"` |
| `n` (sample index, numeric) | `out $ s "arpy" # n "0 2 4 7"` |
| `note` (numeric) | `out $ s "arpy" # note "0 3 7"` |
| `cut` / `attack` / `release` / `ar` | `out $ s "bd*4" # cut 1 # release 0.1` |
| `begin` / `end` / `loop` / `unit` | `out $ s "breaks165" # begin 0.25 # end 0.75` |

> `n`/`note` currently take **numbers only**. Note *names* (`"c e g"`), scale quantization,
> and chords are **not yet wired** — see [§10](#10-known-gaps).

---

## 7. Modulation — patterns & buses as control signals

Phonon's headline capability: a bus can be an audio source (`$`) or a modulation source
(`#`), and either can drive **any** parameter.

```phonon
-- Copy-paste: LFO-swept filter (verified)
~lfo $ sine 2
out $ saw 110 # lpf (~lfo * 2000 + 500) 0.8 * 0.3
```

```phonon
-- Copy-paste: layered track with bus mixing (verified)
~drums $ s "bd ~ sn ~"
~hats  $ s "hh*8" $ degrade
~bass  $ saw "55 55 82.5 55" # lpf 800 0.8
out $ ~drums * 0.9 + ~hats * 0.4 + ~bass * 0.3
```

> **Timing caveat (audible):** continuous signal-pattern modulation is currently sampled
> **once per audio block** (~86 Hz), not per-sample, so fast LFOs on a parameter can produce
> a "zipper" stairstep. Fix is filed as `promote-t3-continuous-patterns` ([§10](#10-known-gaps)).

---

## 8. Performance commands & I/O

### 8.1 Multi-output — `out1` / `out2`

Parsed as `Statement::OutputChannel` (`src/compositional_parser.rs:59,874-878`) and
compiled (`src/compositional_compiler.rs:774`). Renders to distinct channels.

```phonon
-- Copy-paste (verified)
out1 $ s "bd*4" * 0.4
out2 $ s "hh*8" # gain 0.6 * 0.3
```

### 8.2 `hush` / `unhush` / `panic` (live/TUI)

These are **live-session commands** — parsed as `Statement::Hush/Unhush/Panic`
(`src/compositional_parser.rs:970-987`), compiled at
`src/compositional_compiler.rs:844-858`, and bound in the modal editor to **`C-h` (Hush)**
(`src/modal_editor/mod.rs:1026-1028`). They silence/restore/kill audio during a live set;
they are not meant to be mixed with `out` in an offline render file.

- `hush` / `hush N` — silence all outputs (or channel *N*)
- `unhush` / `unhush N` — restore
- `panic` — stop all audio immediately

### 8.3 MIDI

- **MIDI out (CLI):** `phonon midi --pattern "c4 e4 g4" --device "IAC" --tempo 120 --channel 0`
  (`phonon midi --list` to enumerate devices; `src/main.rs:135,1281`, `src/midi_output.rs`).
- **MIDI in (DSL bus):** `~midi` (all channels) or `~midi1`..`~midi16` (per channel)
  (`src/compositional_compiler.rs:941-951`, `src/midi_input.rs`). Requires a connected
  device — with none, compilation errors `MIDI input not available - no MIDI device connected`.

```phonon
-- MIDI-controlled synth (needs a connected MIDI device)
~m $ ~midi1
out $ saw (~m + 110) # lpf 2000 0.8 * 0.3
```

### 8.4 Tempo

`cps:` (cycles/sec) or `bpm:` set the clock (both verified):

```phonon
cps: 2.0
out $ s "bd*4"
```

---

## 9. Corrections to earlier status docs

The task that spawned this reference (`docs/audits/feature-gap-2026-07.md`) documented that
`UGEN_STATUS.md`, `CLAUDE.md` "Next Priority", `LIVECODE_COMPATIBILITY_TODO.md`, and
`DSL_GAPS_DISCOVERED.md` all list **finished** work as "missing". Confirmed here by direct
render/code checks:

**`feature-gap-2026-07.md` §1 — CONFIRMED corrected (these are DONE, verified this pass):**
`gain`/`pan`/`speed` as patterns (§6), `hurry`/`chop`/`striate`/`loopAt` (§5),
multi-output `out1`/`out2` (§8.1), `hush`/`panic`/`unhush` (§8.2), FM/noise/pulse-PWM (§2),
limiter, parametric EQ (§3/§4). The `CLAUDE.md` "Next Priority Features" and "Essential
UGens (Next Priority)" lists are **complete** — treat them as historical, not a TODO.

**`feature-gap-2026-07.md` §1a — ADDITIONAL correction (the audit itself was stale here):**
§1a lists **"Resonant filters RLPF / RHPF / Resonz / SVF / Allpass / Biquad — MISSING"**,
citing `UGEN_STATUS.md:59-67`. This is **incorrect against the code**: `rlpf`, `rhpf`,
`resonz`, `allpass`, `svf_lp/hp/bp/notch`, and `bq_lp/hp/bp/notch` are all dispatched
(`compiler:6530-6760`) with full RBJ-biquad implementations (`unified_graph.rs:13224+`), and
all **render** (§3). These existed at the audit commit `b07833a` (added in `c9394d9`,
"Implement RLPF … 20 tests passing"). **Consequence:** the filed task
`feat-resonant-filters` is largely **already satisfied**; it should be re-scoped to
*verification / any missing variant* rather than fresh implementation.

**Additional finding (not in the original gap list):** the **OSC** modules
(`src/osc_control.rs`, `src/osc_live_server.rs`, addresses `/eval` `/hush` `/panic`) exist
and have integration tests, but are **not wired to any CLI command or flag** — there is no
`phonon osc` and no `--osc` on `live`/`edit`/`repl`. OSC is therefore **not user-reachable**
today despite being described as a working ergonomic in the audit's Dimension 3. Flagged
here for wave-3 planning (see [§10](#10-known-gaps)).

---

## 10. Known gaps

Genuinely absent today, each with its filed wave-2 task. (Verified absent by render:
`scale` → *"Unknown function: scale"*; `splice`/`stitch` → compile error; note-names in
`n`/`note` → numbers only.)

| Gap | Symptom | Filed task |
|---|---|---|
| Scale quantization in DSL (`# scale "minor"`) | `Unknown function: scale` | `feat-scale-quantization` |
| Note names in mini-notation (`note "c e g"`) | numbers only; names not pitched | `feat-scale-quantization` |
| Chords (`n "c'maj"`, `chord`) | no chord parsing | `feat-chord-support` |
| `splice` (speed-to-fit slicing) | compile error | `feat-splice-stitch` |
| `stitch` (boolean interleave) | keyword only, no compiler | `feat-splice-stitch` |
| Continuous patterns at sample rate (T3) | ~86 Hz zipper on modulated params | `promote-t3-continuous-patterns` |
| `f32` trigger timekeeping (T2) | onset drift on multi-hour sets | `promote-t2-trigger-f64` |
| Voice preservation across graph swap (G7) | amplitude notch on every `C-x` | `feat-voice-preservation-swap` |
| Render-owner graph swap (architecture) | design pass only | `design-render-owner-swap` |
| OSC live control not CLI-wired | `/eval` `/hush` `/panic` exist but unreachable | *see below* |

All feature tasks above converge on the verification task **`verify-feature-wave2`**
(full test suite + three-level audio + stress).

Deferred to wave-3 (per `feature-gap-2026-07.md` §5): Gate/Expander/Stereo-Width polish,
Ableton Link / network tempo sync, heavy Tier-2 DSP (pitch/time-stretch, FFT/PV, convolution
reverb), and long-session polish items (T4/T5/T6).

> **Graph note:** two corrections surfaced while verifying this reference — (a) resonant
> filters already work (re-scope `feat-resonant-filters` to verify-only), and (b) OSC is
> unreachable from the CLI. Both are flagged to the coordinator; the OSC wiring should be
> filed as its own wave-3 task if a performer needs remote/networked control.

---

## Appendix — how these examples were verified

Rendered on branch `wg/agent-162/doc-refresh-livecoder-reference` (2026-07-03):

```bash
# Each example was written to a .phonon file and rendered; non-silent Peak confirms audio.
phonon render example.phonon out.wav -d 2          # mono
phonon render example.phonon out.wav -d 2 --stereo # for jux / pan examples
```

`phonon render` prints RMS/Peak/DC on completion; a `Peak level: 0.000` means the graph
produced silence (usually a missing `out $`). Code references use the paths and line numbers
current on this branch; if a line has since moved, `grep` the cited symbol
(e.g. `compile_rlpf`, `Statement::Hush`).
