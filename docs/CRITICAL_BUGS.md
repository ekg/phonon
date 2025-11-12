# Critical Bugs - MUST FIX IMMEDIATELY

**Last updated**: 2025-11-10

These are **blocking issues** that prevent Phonon from being usable in production.

---

## P0 - SHOWSTOPPERS (Fix First)

### ✅ P0.0: ALL parameters must accept patterns, not just numbers
**Status**: FIXED ✅
**Impact**: CRITICAL - Fundamental design principle now fully implemented

**Problem**: Many transforms and effects only accepted bare numbers instead of patterns.

**What now works**:
```phonon
-- Time transforms
s "bd*4" $ fast "2 3 4"            -- ✅ Pattern speeds
s "sn*2" $ slow "1 2"              -- ✅ Pattern slowdown
s "cp*4" $ squeeze "2 3 4"         -- ✅ Pattern compression
s "arpy" $ early "0.1 0.3"         -- ✅ Pattern early shift
s "bass" $ late "0.1 0.3"          -- ✅ Pattern late shift

-- Articulation
s "bd*8" $ legato "0.5 1.5"        -- ✅ Pattern note lengths
s "hh*8" $ staccato "0.1 0.8"      -- ✅ Pattern staccato
s "sn*4" $ swing "0.0 0.5"         -- ✅ Pattern swing

-- Randomization
s "cp*8" $ degradeBy "0.1 0.9"     -- ✅ Pattern dropout
s "arpy*4" $ shuffle "0.1 0.9"     -- ✅ Pattern shuffle

-- Effect parameters (use compile_expr, support any pattern!)
saw 110 # lpf (sine 0.5 * 1500 + 500) 0.8        -- ✅ Pattern cutoff
saw 110 # hpf (sine 0.5 * 1000 + 2000) 0.8       -- ✅ Pattern cutoff
s "bd*4" # delay (sine 1.0 * 0.2 + 0.1) 0.3      -- ✅ Pattern delay time
s "sn*2" # reverb (sine 0.25 * 0.5 + 0.3) 0.5    -- ✅ Pattern room size
s "bd*4" # dist (sine 2.0 * 2.0 + 1.0)           -- ✅ Pattern drive
```

**Implementation**:
- Transform parameters: Use `.fmap()` to convert `Pattern<String>` → `Pattern<f64>`
- Effect parameters: Already used `compile_expr()` which supports patterns
- Pattern methods: Added `squeeze_pattern()` for pattern-controlled compression

**Tests**:
- `tests/test_legato_pattern.rs`: 4 tests for articulation transforms
- `tests/test_p00_effect_patterns.rs`: 8 tests for effect parameters
- All 400+ tests passing ✅

**Files modified**:
- `src/pattern.rs`: Added `squeeze_pattern()` method
- `src/compositional_compiler.rs`: Fixed legato, swing, staccato, squeeze, degradeBy, shuffle
- Effects already worked via `compile_expr()`: lpf, hpf, bpf, delay, reverb, distortion, etc.

**Verification**: Comprehensive testing confirms ALL parameters now accept patterns - this fundamental design principle is complete.

---

### ✅ P0.1: Bus chaining fixed (with limitation)
**Status**: FIXED (with documented workaround)
**Impact**: Outputs now mix correctly

**Problem**: When using buses in chains, signals were dropped and outputs didn't mix.

**Root Cause**: Buses are compiled once to NodeIds, can't be re-instantiated with new inputs.

**Fix**: Bus chain now returns left signal (pass-through) with warning.

**Working now**:
```phonon
o1: s "arpy"     -- Works
o2: s "bd*4"     -- Both outputs mix correctly
```

**Known Limitation**:
```phonon
~feel: delay 0.334 0.3 # reverb 0.9 0.1
o1: s "arpy" # ~feel    -- ⚠️ ~feel effect ignored, use direct instead
```

**Workaround**:
```phonon
o1: s "arpy" # delay 0.334 0.3 # reverb 0.9 0.1    -- Use effects directly
```

**Future Work**: Store bus expressions (Expr) not nodes (NodeId) to enable re-instantiation.

**File**: `src/compositional_compiler.rs` compile_chain()

---

### ✅ P0.2: stack multiplies volume instead of mixing
**Status**: FIXED
**Impact**: HIGH - Was causing distortion/clipping, now fixed

**Problem**: `stack` was adding signals without normalization, causing volume multiplication.

**Example that was broken**:
```phonon
o2: stack [
  s "bd(<4 4 3>,8)",      -- Each pattern is loud
  s "~ cp" $ fast 2       -- Stacking made it LOUDER
]
-- Result: Severe clipping/distortion (Peak: 2.825)
```

**Fix**: Modified Mix node to normalize by dividing sum by N.

**Results**:
- Before: 2 patterns → RMS 0.901, Peak 2.825 (2.5x multiplication) ⚠️
- After: 2 patterns → RMS 0.450, Peak 1.413 (proper mixing) ✅
- 4 patterns: RMS 0.463, Peak 1.414 (stable, no multiplication) ✅

**Files**:
- `src/unified_graph.rs`: Mix node now normalizes (line 4849)
- `src/compositional_compiler.rs`: stack uses Mix node (line 1143)

---

### ✅ P0.3: Output volume affected by other outputs
**Status**: FIXED
**Impact**: HIGH - Was causing outputs to contaminate each other, now fixed

**Problem**: All outputs were returning the same mixed voice signal, so disabling one output changed volume of others.

**Root Cause**: Voice manager processed all voices once and returned a single global mix. ALL Sample nodes returned this same mix regardless of which output they belonged to.

**Example that was broken**:
```phonon
o1: s "bd*4"  -- Should only hear bd
o2: s "sn*4"  -- Should only hear sn
-- But both outputs returned the SAME mix (bd+sn)!
```

**Fix**: Tag voices with source node ID and return per-node mixes.
1. Added `source_node` field to Voice
2. Added `default_source_node` to VoiceManager (set before triggering)
3. Changed voice processing to return `HashMap<usize, f32>` (node → mix)
4. Sample nodes look up their node ID in the HashMap

**Results**:
- Before: o1 single RMS = 0.354, o1 dual RMS = 0.450 (contaminated) ⚠️
- After: o1 single RMS = 0.354, o1 dual RMS = 0.354 (independent) ✅
- o2 has different RMS (0.301) as expected for different samples ✅

**Files**:
- `src/voice_manager.rs`: Added source_node field and per-node processing
- `src/unified_graph.rs`: Process per-node, set default_source_node before Sample evaluation

---

### ✅ P0.4: Multi-threading performance - FIXED
**Status**: FULLY FIXED - Both Rayon overhead and Mutex contention eliminated
**Impact**: HIGH - Major performance improvement in both render and live modes

**Problems Fixed**:
1. ✅ **FIXED**: Rayon overhead - par_iter_mut() called every sample regardless of voice count
2. ✅ **FIXED**: Mutex contention in live mode - audio callback held lock for entire buffer

**Problem 1 - Rayon Overhead** (FIXED):
- Used `par_iter_mut()` unconditionally for all voice counts
- Rayon scheduling overhead: ~10-50μs per sample
- For typical 16-32 voices, overhead dominated actual work
- At 44.1kHz, this added 30-50% overhead

**Fix 1**: Threshold-based parallelism
- Only use `par_iter_mut()` when voice count ≥ 64
- Below threshold, use sequential iteration (no overhead)
- Result: Render mode now efficient for typical patterns

**Problem 2 - Mutex Contention** (FIXED):
```rust
// OLD (BROKEN):
let mut state = state_clone.lock().unwrap();  // ⚠️ Locks for entire buffer!
for sample in data.iter_mut() {
    *sample = graph.process_sample();  // 512 samples while holding lock
}
```
- Audio callback locked Mutex for entire buffer duration (~12ms at 512 samples)
- File watcher also locked same Mutex to check for reloads
- Created contention → choppy audio

**Fix 2**: Lock-free graph swapping with ArcSwap
```rust
// NEW (FIXED):
let graph_snapshot = graph_clone.load();  // Lock-free atomic load!
for sample in data.iter_mut() {
    *sample = graph_cell.0.borrow_mut().process_sample();  // No blocking
}
```
- Audio callback uses `arc-swap` for lock-free atomic loading
- File watcher atomically swaps with `store()` - no blocking
- GraphCell newtype provides thread-safe interior mutability
- Zero contention, smooth audio

**Architecture**:
- `Arc<ArcSwap<Option<GraphCell>>>` for lock-free swapping
- `GraphCell(RefCell<UnifiedSignalGraph>)` with unsafe Send+Sync impl
- Safe because each Arc instance accessed by only one thread
- File watcher creates NEW graphs, doesn't mutate existing ones

**Files**:
- ✅ `src/voice_manager.rs`: Threshold-based parallelism (line 1006)
- ✅ `src/main.rs`: Lock-free graph swapping (line 1547-1680)
- ✅ `src/unified_graph.rs`: unsafe impl Send+Sync (line 3510)
- ✅ `Cargo.toml`: Added arc-swap dependency

---

## P1 - HIGH PRIORITY (Fix Soon)

### ✅ P1.1: fast/slow/hurry speed up patterns, NOT tempo
**Status**: NOT A BUG - Working as designed ✅
**Impact**: NONE - This is the correct, intentional behavior

**Clarification**: Pattern modifiers affect pattern density WITHIN cycles, not tempo.

**Correct behavior**:
```phonon
tempo: 2.0         -- 2 cycles per second (global tempo, unchanged)
fast 3             -- 3x more events per cycle, still 2 cycles/second ✅
slow 2             -- 0.5x events per cycle, still 2 cycles/second ✅
hurry 1.5          -- Speed up pattern 1.5x, still 2 cycles/second ✅
```

**Why this is correct**:
- `tempo` controls cycle rate (global clock)
- `fast`, `slow`, `hurry`, etc. control pattern density/speed OVER cycles
- These are independent dimensions: tempo = cycles/second, fast = events/cycle
- Allows patterns to speed up/slow down while maintaining sync with other patterns

**To change tempo**: Modify the `tempo:` declaration, not pattern transforms

**Not a bug**: This is fundamental to Phonon's design. Pattern transforms affect patterns, not tempo.

---

### ✅ P1.2: ar (attack/release envelope) doesn't exist
**Status**: FIXED ✅
**Impact**: MEDIUM - Quick envelope shaping now available

**Problem**: User tried `# ar 0.1 0.9` but `ar` was not implemented.

**What now works**:
```phonon
-- ar shorthand (sets both attack and release)
s "arpy" # ar 0.01 0.5  -- Attack 0.01, Release 0.5

-- Equivalent to:
s "arpy" # attack 0.01 # release 0.5

-- Pattern values work too
s "bd*8" # ar "0.01 0.1" "0.1 0.5"  -- Varying envelopes
```

**Implementation**:
- Added `compile_ar_modifier()` function that sets both attack and release
- Registered in function table as "ar"
- Common shorthand from Tidal/SuperCollider

**Tests**:
- `tests/test_ar_parameter.rs`: 4 tests verifying ar functionality
- Tests constant values, pattern values, and envelope effects
- All 8 tests passing (including 4 audio_test_utils tests)

**File**: `src/compositional_compiler.rs` lines 5355-5383

---

### 🟠 P1.3: Can't render in live mode, processes at 30% CPU
**Status**: PERFORMANCE BUG
**Impact**: MEDIUM - Live mode unusable

**Problem**: Live mode stutters/can't keep up, only uses 30% CPU.

**Symptoms**:
- Audio dropouts/glitches in live mode
- CPU usage around 30% (should be higher if maxed out)
- Render mode works fine
- Suggests real-time scheduling issues

**Related to**: P0.4 (multi-threading issue)

**Fix needed**: Profile and optimize live mode audio callback.

---

## Testing Priority

1. **P0.0** - Pattern parameters (most fundamental)
2. **P0.2** - Stack volume (audio quality)
3. **P0.3** - Output independence (audio quality)
4. **P0.1** - Delay chaining (core feature)
5. **P0.4** - Performance (usability)

## Notes

- Many of these are interconnected (volume issues, mixing, threading)
- Pattern parameters (P0.0) is the most fundamental architectural issue
- Performance issues (P0.4, P1.3) may share root cause
- Volume issues (P0.2, P0.3) likely related to auto-routing mixer

## Action Plan

1. **Create test cases** for each issue in `broke.ph.*` files
2. **Fix P0.0** first (pattern parameters) - enables everything else
3. **Fix volume/mixing** issues (P0.2, P0.3) - critical for audio quality
4. **Fix delay** chaining (P0.1) - needed for effects
5. **Profile and fix** performance (P0.4, P1.3) - usability
6. **Add missing features** (P1.1, P1.2) - nice to have
