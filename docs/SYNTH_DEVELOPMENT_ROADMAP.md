# Synth Development Roadmap

## Current Status (2025-10-19)

### ✅ COMPLETE: Audio-Rate Pattern Modulation
**Status:** PROVEN with comprehensive tests
**Achievement:** TRUE audio-rate pattern evaluation (44,100 Hz)

**What works:**
- LFO modulating synthesis parameters at audio rate
- Pattern-as-control-signal for continuous modulation
- Audio-rate FM synthesis (oscillator → oscillator)
- Complex modulation networks with feedback
- Meta-modulation (patterns modulating pattern parameters)
- High-frequency control signals (100 Hz LFO proven)

**Test coverage:**
- 7 dedicated audio-rate modulation proof tests (all passing)
- 131+ test files covering synthesis
- Signal analysis verification (RMS, amplitude, spectral)

**Documentation:** `/tmp/AUDIO_RATE_PROOF.md` + `tests/test_audio_rate_modulation_proof.rs`

---

## Gap Analysis

### ✅ What's Available in Phonon DSL Today

**Basic building blocks:**
```phonon
# Oscillators
sine 440, saw 220, square 110, tri 330

# Filters
lpf 1000 0.8, hpf 500 0.5, bpf 800 2.0

# Effects
reverb 0.7 0.5 0.3
distortion 2.0 0.3
delay 0.25 0.5 0.4
chorus 2.0 0.5 0.3
bitcrush 4.0 8000.0

# Samples
s "bd sn hh cp"

# Patterns & Transforms
"bd sn" $ fast 2 $ rev
```

### ❌ What's Missing

**1. Synth Library Not Exposed**
- `src/superdirt_synths.rs` contains 11 professional synths
- **NOT accessible from Phonon DSL**
- Users can't call `supersaw()`, `superkick()`, etc.

**Available but hidden synths:**
- Drums: `superkick`, `supersnare`, `superhat`, `superclap`
- Melodic: `supersaw`, `superpwm`, `supersquare`, `superchip`, `superfm`
- Bass: `superbass`, `superreese`

**2. No Noise Source**
- Can't build percussion without noise oscillator
- Missing: `noise()` function

**3. Limited Envelope Support**
- Sample nodes have `attack`/`release` parameters
- Synth nodes (oscillators) have NO envelope
- Missing: `adsr()`, `env()`, envelope generators

**4. No User-Defined Functions**
- Can't abstract synth patches
- Can't create reusable `supersaw(freq, detune, voices)`
- Everything must be inline

---

## Development Phases

### PHASE 1: Quick Wins (1-2 days) ⚡

**Goal:** Professional synth sounds immediately available

#### 1.1 Add Noise Oscillator
**Effort:** 2 hours
**Files:** `src/unified_graph.rs`, `src/compositional_compiler.rs`

**Implementation:**
```rust
// Add to Waveform enum
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,  // NEW
}

// Add to compositional_compiler.rs
"noise" => compile_oscillator(ctx, Waveform::Noise, args),
```

**Test:**
```phonon
tempo: 2.0
~hh: noise # hpf 8000 2.0 * 0.3
out: ~hh
```

**Value:** Enables building percussion synths

---

#### 1.2 Expose SynthLibrary to DSL
**Effort:** 4-6 hours
**Files:** `src/compositional_compiler.rs`, create `src/synth_compiler.rs`

**Implementation approach:**
```rust
// In compositional_compiler.rs:
"superkick" => compile_superkick(ctx, args),
"supersaw" => compile_supersaw(ctx, args),
"superfm" => compile_superfm(ctx, args),
// ... etc

// Helper functions in new synth_compiler.rs
fn compile_superkick(
    ctx: &mut CompilerContext,
    args: Vec<Expr>,
) -> Result<NodeId, String> {
    let freq = if args.is_empty() {
        Signal::Value(50.0)  // Default kick freq
    } else {
        let freq_node = compile_expr(ctx, args[0].clone())?;
        Signal::Node(freq_node)
    };

    // Use existing SynthLibrary implementation
    let synth_lib = SynthLibrary;
    let node_id = synth_lib.build_superkick(&mut ctx.graph, freq, None, None);
    Ok(node_id)
}
```

**User syntax:**
```phonon
tempo: 2.0
~kick: superkick 50
~saw: supersaw 220 0.3 7  # freq, detune, voices
~fm: superfm 110 2.0 5.0  # carrier, mod_freq, mod_amount

out: ~kick + ~saw * 0.3 + ~fm * 0.2
```

**Value:** Professional synth sounds instantly available

---

#### 1.3 Add Envelope Support to Oscillators
**Effort:** 4-6 hours
**Files:** `src/unified_graph.rs`, `src/compositional_compiler.rs`

**Current state:**
- Sample nodes have `attack`/`release`
- Oscillator nodes have NO envelope

**Implementation:**
```rust
// Add envelope parameters to Oscillator node
pub enum SignalNode {
    Oscillator {
        freq: Signal,
        waveform: Waveform,
        phase: f32,
        attack: Signal,   // NEW
        release: Signal,  // NEW
    },
    // ...
}
```

**Syntax options:**
```phonon
# Option A: Inline parameters (like samples)
~kick: sine 50 attack:0.01 release:0.3

# Option B: Separate envelope node
~env: adsr 0.01 0.1 0.7 0.3
~kick: sine 50 * ~env

# Start with Option B (simpler, more compositional)
```

**Value:** Enables proper synth envelopes

---

### PHASE 2: Compositional Synth Building (current state, documented)

**Goal:** Document and improve current capability

**What users CAN do today:**
```phonon
tempo: 2.0

# Build a supersaw by hand (7 detuned oscillators)
~freq: 220
~osc1: saw (~freq * 0.97)
~osc2: saw (~freq * 0.985)
~osc3: saw (~freq * 1.0)
~osc4: saw (~freq * 1.015)
~osc5: saw (~freq * 1.03)
~osc6: saw (~freq * 0.96)
~osc7: saw (~freq * 1.04)
~supersaw: (~osc1 + ~osc2 + ~osc3 + ~osc4 + ~osc5 + ~osc6 + ~osc7) * 0.14

# Add filter modulation
~lfo: sine 0.5
~cutoff: ~lfo * 1500 + 1500
~filtered: ~supersaw # lpf ~cutoff 0.7

# Effects
out: ~filtered # reverb 0.5 0.4 0.2
```

**Document in:**
- Quickstart guide
- Example library
- Tutorial series

---

### PHASE 3: User-Defined Functions (2-4 weeks) 🚀

**Goal:** True compositionality - define synths in DSL

**New syntax proposal:**
```phonon
# Define a function
def supersaw(freq, detune, voices):
    ~oscs: []
    for i in range(voices):
        ~offset: (i / (voices - 1) - 0.5) * 2.0
        ~detune_factor: 1.0 + (~offset * detune * 0.1)
        ~detuned_freq: freq * ~detune_factor
        ~oscs.push(saw ~detuned_freq)
    return mix(~oscs, 1.0 / voices * 0.7)

# Use it
tempo: 2.0
~bass: supersaw(55, 0.3, 7) # lpf 400 0.8
out: ~bass * 0.4
```

**Implementation requirements:**
1. **Parser extensions:**
   - Function definition syntax: `def name(args): body`
   - Return statements
   - Local scope handling

2. **Compiler extensions:**
   - Function symbol table
   - Parameter binding
   - Closure support (capture external buses)

3. **Control flow:**
   - Loops: `for`, `while`
   - Conditionals: `if`/`else`
   - List operations

4. **Standard library:**
   - `mix(signals, gain)` - mix multiple signals
   - `range(n)` - generate range
   - `map(pattern, fn)` - transform pattern values

**Challenges:**
- Scope management (local vs global buses)
- Function calls in pattern context
- Performance (inline vs runtime dispatch)

**Value:**
- Users can build synth libraries in pure Phonon
- Shareable patches
- Community synth collections
- True live-codeable instruments

---

## Implementation Priority

### Week 1: Phase 1 Quick Wins
- **Day 1:** Noise oscillator + tests
- **Day 2:** Expose SynthLibrary + tests
- **Day 3-4:** Envelope support + comprehensive tests
- **Day 5:** Documentation + examples

**Deliverable:** Professional synth sounds accessible from DSL

### Week 2-3: Phase 2 Documentation
- Tutorial: Building synths compositionally
- Example library: 10+ synth patches
- Video demonstrations
- Performance tips

**Deliverable:** Users know how to build complex synths

### Month 2-3: Phase 3 User-Defined Functions
- Week 1: Parser extensions (def, return)
- Week 2: Compiler extensions (function table)
- Week 3: Control flow (for, if)
- Week 4: Standard library + polish
- Week 5-6: Testing + documentation

**Deliverable:** Full compositionality

---

## Success Metrics

### Phase 1 Success:
- [ ] `noise()` oscillator works
- [ ] All 11 superdirt synths callable from DSL
- [ ] Oscillators have envelope support
- [ ] 20+ passing tests for new features
- [ ] Example patches demonstrating each synth

### Phase 2 Success:
- [ ] Tutorial published
- [ ] 10+ example synth patches
- [ ] Performance benchmarks documented
- [ ] Community feedback incorporated

### Phase 3 Success:
- [ ] User-defined functions work
- [ ] Standard library complete
- [ ] 50+ passing tests
- [ ] Community synth library started
- [ ] Example: user recreates all superdirt synths in pure Phonon

---

## Technical Notes

### Why This Architecture?

**Phase 1 (Expose Rust synths):**
- ✅ Fast time-to-value (1-2 days)
- ✅ Professional quality immediately
- ✅ Low risk (leverages existing code)
- ⚠️ Not live-codeable (hard-coded in Rust)

**Phase 3 (User functions):**
- ✅ True compositionality
- ✅ Live-codeable
- ✅ Community extensible
- ⚠️ Higher complexity
- ⚠️ Longer timeline

**Doing both:**
1. Phase 1 gives immediate value
2. Phase 3 enables long-term ecosystem
3. Users can choose: use built-ins OR build their own
4. Best of both worlds

### Alternatives Considered

**A. Only expose Rust synths (no Phase 3)**
- Pros: Simple, fast
- Cons: Not extensible, not live-codeable
- Verdict: ❌ Limits vision

**B. Only implement user functions (skip Phase 1)**
- Pros: Pure vision
- Cons: Slow to deliver value, high risk
- Verdict: ❌ Too risky

**C. Current hybrid approach (Phase 1 → 3)**
- Pros: Fast value + long-term vision
- Cons: More total work
- Verdict: ✅ BEST APPROACH

---

## Related Documents

- `/tmp/AUDIO_RATE_PROOF.md` - Proof of audio-rate modulation
- `tests/test_audio_rate_modulation_proof.rs` - Comprehensive tests
- `/tmp/phonon_analysis.md` - Cost/time analysis
- `ROADMAP.md` - Overall project roadmap
- `src/superdirt_synths.rs` - Existing synth implementations

---

## Questions & Decisions

### Q: Should we implement ADSR or simple attack/release first?
**A:** Start with attack/release (matches sample nodes), add ADSR in Phase 2

### Q: Should noise() take a frequency parameter?
**A:** No - white noise is frequency-independent. Filter it for colored noise.

### Q: Expose all 11 synths or start with subset?
**A:** Expose ALL - they're already implemented, just need wiring

### Q: Make synth parameters pattern-controllable?
**A:** YES - this is the killer feature. All synth params should accept patterns.

Example:
```phonon
~detune: "0.1 0.3 0.5"  # Pattern of detune values
~saw: supersaw 220 ~detune 7  # Detune modulated by pattern!
```

---

**Status:** Ready to implement
**Next step:** Phase 1.1 - Add noise oscillator
**Timeline:** Phase 1 complete by end of week
