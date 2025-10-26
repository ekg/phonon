# Phonon Synthesis Parity Plan
## Achieving CSound/SuperCollider Feature Parity

**Vision**: Make Phonon a complete synthesis system with all capabilities of CSound and SuperCollider, while maintaining its unique live-coding ergonomics and pattern-first philosophy.

**Status**: 2025-10-25
**Target**: Full parity within 18-24 months

---

## Executive Summary

**Strategy**: We don't need to rewrite 30+ years of work. We will:
1. **Study** existing implementations (CSound, SuperCollider, Rust DSP libraries)
2. **Port** algorithms where possible (with proper attribution)
3. **Integrate** existing Rust crates for complex DSP
4. **Test** everything with our three-level methodology
5. **Document** with musical examples

**Why This Works**: Phonon's architecture is already solid. We just need to add more `SignalNode` variants and compile them properly.

---

## Phase 1: UGen Inventory & Categorization

### A. Oscillators & Signal Generators (20 UGens)

**Currently Have** (4):
- ✅ Sine
- ✅ Saw
- ✅ Square
- ✅ Triangle

**Need to Add** (16):
1. **Pulse** (variable pulse width)
2. **Noise** (white, pink, brown)
3. **Wavetable** (arbitrary waveforms)
4. **FM Oscillator** (frequency modulation)
5. **PM Oscillator** (phase modulation)
6. **Formant Oscillator** (vowel sounds)
7. **Impulse** (single impulse)
8. **Blip** (band-limited impulse train)
9. **VCO** (voltage-controlled oscillator model)
10. **SuperSaw** (detuned saw stack)
11. **Karplus-Strong** (plucked string)
12. **Waveguide** (physical modeling)
13. **Grain** (granular synthesis)
14. **FOF** (formant synthesis)
15. **Additive** (harmonic series)
16. **Vector** (2D morphing between waveforms)

**Porting Strategy**:
- **Rust crates**: `fundsp`, `dasp`, `dsp-chain`
- **Learn from**: SuperCollider's `LFSaw.cpp`, CSound's `oscils.c`
- **Reference**: Julius O. Smith's work (CCRMA)

---

### B. Filters (15 UGens)

**Currently Have** (3):
- ✅ LPF (low-pass)
- ✅ HPF (high-pass)
- ✅ BPF (band-pass)

**Need to Add** (12):
1. **Notch** (band-reject)
2. **Comb** (feedback delay)
3. **Allpass** (phase manipulation)
4. **Formant** (vowel filters)
5. **Moog Ladder** (classic analog model)
6. **SVF** (state variable filter)
7. **Biquad** (generic 2nd order)
8. **Resonz** (resonant bandpass)
9. **RLPF/RHPF** (resonant versions)
10. **Median** (median filter)
11. **Slew** (slew rate limiter)
12. **Lag** (exponential lag)

**Porting Strategy**:
- **Rust crate**: `biquad` (already well-implemented)
- **Learn from**: Moog filter papers, Will Pirkle's books
- **Port from**: SuperCollider's `Filter.cpp`

---

### C. Envelopes (8 UGens)

**Currently Have**: ⚠️ Partial (env_trig exists but incomplete)

**Need to Add** (8):
1. **ADSR** (attack, decay, sustain, release)
2. **AD** (attack, decay only)
3. **ASR** (attack, sustain, release)
4. **Exponential** (shaped envelopes)
5. **Line** (linear ramp)
6. **XLine** (exponential ramp)
7. **Env** (arbitrary breakpoint)
8. **EnvGen** (trigger-based envelope)

**Critical Decision**: Phonon uses **continuous patterns** not discrete triggers. We need:
```phonon
-- Pattern triggers envelope
~kick: s "bd" # adsr 0.001 0.1 0.0 0.2

-- Or pattern AS envelope
~env: envelope "0.0 1.0 0.5 0.0" 0.5  -- values over duration
~synth: saw 220 * ~env
```

**Porting Strategy**:
- **Learn from**: SuperCollider's `EnvGen.cpp`
- **Design challenge**: Adapt trigger-based to pattern-based

---

### D. Effects (25 UGens)

**Currently Have** (6):
- ✅ Reverb
- ✅ Delay
- ✅ Distortion
- ✅ Chorus
- ✅ Compressor
- ✅ Bitcrush

**Need to Add** (19):
1. **Convolution Reverb** (IR-based)
2. **Plate Reverb** (Dattorro algorithm)
3. **Spring Reverb**
4. **Flanger**
5. **Phaser**
6. **Tremolo**
7. **Vibrato**
8. **Ring Modulator**
9. **Frequency Shifter**
10. **Pitch Shifter**
11. **Time Stretcher**
12. **Vocoder**
13. **Limiter**
14. **Gate/Expander**
15. **Multiband Compressor**
16. **EQ** (parametric)
17. **Graphic EQ**
18. **Stereo Width**
19. **Saturation/Waveshaping**

**Porting Strategy**:
- **Rust crates**: `rubato` (resampling), `realfft` (FFT operations)
- **Learn from**: Freeverb, Dattorro reverb papers
- **Port from**: SuperCollider's effects, CSound opcodes

---

### E. Analysis & Control (12 UGens)

**Currently Have**: None

**Need to Add** (12):
1. **Amplitude Follower**
2. **Pitch Tracker**
3. **FFT** (spectral analysis)
4. **PV_** (phase vocoder operations)
5. **Onset Detector**
6. **Beat Tracker**
7. **Peak Follower**
8. **RMS**
9. **Schmidt Trigger**
10. **Trig** (trigger detector)
11. **Timer**
12. **Latch** (sample and hold)

**Use Case**:
```phonon
~input: audioin 1
~pitch: pitchtrack ~input
~synth: saw ~pitch  -- follow input pitch
```

---

### F. Spatial & Routing (10 UGens)

**Currently Have**: Basic mixing only

**Need to Add** (10):
1. **Pan2** (stereo panning)
2. **Pan4** (quad panning)
3. **Rotate2** (stereo rotation)
4. **Binaural** (HRTF)
5. **Ambisonics** (spatial encoding)
6. **Splay** (spread signals)
7. **XFade** (crossfade)
8. **Select** (signal routing)
9. **Mix** (sum multiple signals)
10. **NumChannels** (adapt channel count)

**Architecture Change Needed**:
```rust
// Currently: mono only
pub struct UnifiedSignalGraph { ... }

// Future: multi-channel
pub enum ChannelConfig {
    Mono,
    Stereo,
    Quad,
    Surround5_1,
    Surround7_1,
    Ambisonic(u8),
}
```

---

## Phase 2: Porting Strategy & Techniques

### A. Direct Rust Crate Integration

**Existing Rust Audio Ecosystem**:

1. **fundsp** - Complete DSP framework
   ```rust
   // They already have: oscillators, filters, effects
   // Strategy: Wrap their UGens in our SignalNode enum
   ```

2. **dasp** - Sample processing
   ```rust
   // Use for: sample rate conversion, interpolation
   ```

3. **biquad** - Filter implementations
   ```rust
   // Already production-ready biquad filters
   ```

4. **rubato** - Resampling
   ```rust
   // For pitch shifting, time stretching
   ```

5. **realfft** - FFT operations
   ```rust
   // For spectral processing
   ```

**Integration Pattern**:
```rust
// In unified_graph.rs
SignalNode::FunDSPOscillator {
    oscillator: Box<dyn fundsp::AudioUnit>,
    state: FunDSPState,
}

// In compositional_compiler.rs
"supersaw" => {
    let osc = fundsp::supersaw(freq);
    compile_fundsp_unit(ctx, osc, args)
}
```

**Advantages**:
- Battle-tested implementations
- Maintained by community
- Proper licensing (MIT/Apache-2.0)

**Challenges**:
- Need to wrap their API in ours
- State management differences
- Performance overhead?

---

### B. Port from SuperCollider (C++)

**SuperCollider Source Code**: https://github.com/supercollider/supercollider

**What to Port**:
- Individual UGen algorithms (`.cpp` files in `server/plugins/`)
- Filter designs (Moog ladder, SVF, etc.)
- Effect algorithms (Freeverb, etc.)

**Porting Process**:

1. **Identify Algorithm**:
   ```cpp
   // From SC: LFSaw.cpp
   float LFSaw_next(LFSaw *unit, int inNumSamples) {
       float freq = IN0(0);
       float phase = unit->m_phase;
       // ... algorithm ...
   }
   ```

2. **Translate to Rust**:
   ```rust
   // In unified_graph.rs
   SignalNode::LFSaw {
       freq: Signal,
       phase: f32
   }

   // Evaluation logic
   match node {
       SignalNode::LFSaw { freq, phase } => {
           let freq_val = self.eval_signal(freq, ...);
           let output = 2.0 * (*phase) - 1.0;  // Sawtooth
           *phase = (*phase + freq_val / sample_rate).fract();
           output
       }
   }
   ```

3. **Test with Three-Level Methodology**:
   ```rust
   #[test]
   fn test_lfsaw_frequency() {
       // LEVEL 1: Pattern query
       // LEVEL 2: Onset detection
       // LEVEL 3: Spectral analysis (verify fundamental freq)
   }
   ```

**Legal/Ethical**:
- SuperCollider is GPL
- We must: Credit original, maintain GPL for ported code
- OR: Study algorithm, reimplement cleanly (clean-room)

---

### C. Learn from CSound

**CSound Source**: https://github.com/csound/csound

**What to Learn**:
- Opcode designs (`Opcodes/` directory)
- FM synthesis implementations
- Physical modeling algorithms

**Approach**:
- **Don't copy code** (LGPL licensing)
- **Study algorithms**, implement from scratch in Rust
- **Reference papers** cited in CSound docs

**Example - FM Synthesis**:
```c
// CSound foscil.c (study only)
// Chowning FM algorithm
carrier = sin(2π * fc * t + I * sin(2π * fm * t))
```

```rust
// Our clean implementation in Rust
SignalNode::FM {
    carrier_freq: Signal,
    mod_freq: Signal,
    mod_index: Signal,
    carrier_phase: f32,
    mod_phase: f32,
}

// Evaluate
let carrier_freq = eval_signal(carrier_freq);
let mod_freq = eval_signal(mod_freq);
let mod_index = eval_signal(mod_index);

let modulator = (2.0 * PI * mod_phase).sin();
let carrier = (2.0 * PI * carrier_phase + mod_index * modulator).sin();

carrier_phase = (carrier_phase + carrier_freq / sample_rate).fract();
mod_phase = (mod_phase + mod_freq / sample_rate).fract();

carrier
```

---

### D. Academic Papers & Books

**Essential Resources**:

1. **Julius O. Smith III** (Stanford CCRMA)
   - "Physical Audio Signal Processing"
   - "Spectral Audio Signal Processing"
   - Free online, reference implementations
   - Use: Waveguide synthesis, filter design

2. **Will Pirkle**
   - "Designing Audio Effect Plugins in C++"
   - Detailed filter/effect algorithms
   - Code examples (translate to Rust)

3. **Udo Zölzer**
   - "DAFX: Digital Audio Effects"
   - Academic rigor, tested algorithms

4. **Curtis Roads**
   - "The Computer Music Tutorial"
   - "Microsound" (granular synthesis)

**Strategy**:
- Implement algorithms from papers
- Cite in code comments
- Test against reference implementations

---

## Phase 3: Systematic Implementation Process

### Step-by-Step UGen Addition

**Template for Adding Any UGen**:

```bash
# 1. Define in unified_graph.rs
# Add to SignalNode enum:
SignalNode::NewUGen {
    input: Signal,
    param1: Signal,
    param2: Signal,
    state: NewUGenState,
}

# 2. Add state struct if needed
#[derive(Debug, Clone)]
pub struct NewUGenState {
    phase: f32,
    buffer: Vec<f32>,
}

# 3. Implement evaluation in eval_node()
SignalNode::NewUGen { input, param1, param2, state } => {
    // DSP algorithm here
}

# 4. Add compiler function in compositional_compiler.rs
fn compile_newugen(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    let (input_signal, params) = extract_chain_input(ctx, &args)?;
    // Compile parameters
    // Create node
    // Return node ID
}

# 5. Register in compile_function()
"newugen" => compile_newugen(ctx, args),

# 6. Write tests (three-level methodology)
#[test]
fn test_newugen_basic() { ... }

#[test]
fn test_newugen_pattern_modulation() { ... }

#[test]
fn test_newugen_audio_quality() { ... }

# 7. Document in DSL examples
# Create docs/examples/newugen_demo.ph
```

**Time Estimate per UGen**: 4-8 hours
- 1-2 hours: Research algorithm
- 1-2 hours: Implementation
- 1-2 hours: Testing
- 1-2 hours: Documentation

**With 90 UGens to add**: 360-720 hours = 9-18 months at 20 hrs/week

---

## Phase 4: Prioritized Implementation Order

### Tier 1: Essential (Complete First - 3 months)

**These enable 80% of synthesis use cases**:

1. **Envelopes** (ADSR, AD, Line) - Week 1-2
2. **FM Oscillator** - Week 3
3. **Noise Generators** (white, pink) - Week 4
4. **Pan2** (stereo panning) - Week 5
5. **Limiter** - Week 6
6. **EQ** (parametric) - Week 7-8
7. **Moog Ladder Filter** - Week 9
8. **Pulse Oscillator** (PWM) - Week 10
9. **Ring Modulator** - Week 11
10. **Flanger** - Week 12

**Deliverable**: "Phonon can now make professional-sounding tracks"

---

### Tier 2: Advanced Synthesis (6 months)

**Complex algorithms, special techniques**:

11. **Wavetable Oscillator** - Weeks 13-14
12. **Granular Synthesis** - Weeks 15-17
13. **Karplus-Strong** - Week 18
14. **Physical Modeling** (waveguide) - Weeks 19-21
15. **Formant Synthesis** - Weeks 22-23
16. **Additive Synthesis** - Weeks 24-25
17. **Vocoder** - Weeks 26-28
18. **Pitch Shifter** - Weeks 29-30
19. **FFT/Spectral** - Weeks 31-34
20. **Convolution Reverb** - Weeks 35-36

**Deliverable**: "Phonon rivals dedicated synths"

---

### Tier 3: Specialized (6 months)

**Nice-to-have, niche applications**:

21. **Binaural/HRTF** - Weeks 37-40
22. **Ambisonics** - Weeks 41-44
23. **Advanced PV operations** - Weeks 45-48
24. **Waveshaping** (complex) - Weeks 49-50
25. **Modal synthesis** - Weeks 51-52

**Deliverable**: "Phonon does things SC/CSound can't easily do"

---

### Tier 4: Analysis & Control (3 months)

**Real-time analysis, interactive control**:

26. **Pitch Tracker** - Weeks 53-54
27. **Beat Tracker** - Weeks 55-56
28. **Onset Detector** - Week 57
29. **Amplitude Follower** - Week 58
30. **FFT Analysis** - Weeks 59-60
31. **Control Rate Signals** - Weeks 61-64

**Deliverable**: "Phonon can respond to audio input"

---

## Phase 5: Integration Architecture

### Multi-Channel Support

**Current**: Mono only
**Target**: Stereo, Quad, Surround, Ambisonics

**Architecture Change**:

```rust
// unified_graph.rs
pub enum Signal {
    Constant(f32),
    Node(NodeId),
    MultiChannel(Vec<NodeId>),  // NEW
}

pub struct UnifiedSignalGraph {
    // Current: single sample rate
    sample_rate: f32,

    // NEW: channel configuration
    channels: ChannelConfig,

    // NEW: multi-channel output
    output: Vec<Option<NodeId>>,
}

impl UnifiedSignalGraph {
    // Current: mono render
    pub fn render(&mut self, num_samples: usize) -> Vec<f32>

    // NEW: multi-channel render
    pub fn render_multi(&mut self, num_samples: usize) -> Vec<Vec<f32>>
}
```

**Migration Strategy**:
1. Keep mono path for backward compatibility
2. Add multi-channel as opt-in
3. Gradually migrate examples

---

### Plugin Architecture (Future)

**Why**: Don't reimplement everything, use existing VST/LV2 plugins

```rust
// plugin.rs (NEW)
pub enum PluginNode {
    VST(vst::Plugin),
    LV2(lv2::Plugin),
}

// In SignalNode
SignalNode::Plugin {
    input: Signal,
    plugin: PluginNode,
    parameters: HashMap<String, Signal>,
}
```

**DSL Syntax**:
```phonon
-- Load VST plugin
~reverb_send: vst "Valhalla Room" {
    size: 0.8,
    damping: 0.6,
    mix: 0.3
}

~synth: saw 220 # ~reverb_send
out: ~synth
```

**Rust Crates**:
- `vst` - VST 2.4 host
- `lv2` - LV2 plugin support

---

## Phase 6: Testing Strategy

### Three-Level Methodology (Mandatory for ALL)

**LEVEL 1: Pattern Query**
```rust
#[test]
fn test_fm_pattern_query() {
    let pattern = parse_mini_notation("110 220 440");
    let fm = pattern.fm(2.0, 5.0);  // mod ratio, index

    // Verify events
    let events = fm.query(&state);
    assert_eq!(events.len(), 3);
}
```

**LEVEL 2: Onset/Event Detection**
```rust
#[test]
fn test_fm_audio_events() {
    let code = "out: fm \"110 220\" 2.0 5.0";
    let audio = render_dsl(code, 1.0);

    let onsets = detect_audio_events(&audio);
    assert_eq!(onsets.len(), 2);  // Two notes
}
```

**LEVEL 3: Audio Characteristics**
```rust
#[test]
fn test_fm_spectral_content() {
    let code = "out: fm 440 1.5 3.0";  // C:M ratio 1.5
    let audio = render_dsl(code, 1.0);

    let spectrum = fft_analyze(&audio);

    // Verify sidebands at 440 ± (1.5 * 440)
    assert_peak_near(&spectrum, 440.0);     // Carrier
    assert_peak_near(&spectrum, 1100.0);    // Upper sideband
    // etc.
}
```

### Regression Testing

**Every UGen Addition**:
1. Must not break existing tests (all 340+ tests still pass)
2. Must add minimum 3 new tests (one per level)
3. Must include musical example in docs/examples/

---

## Phase 7: Documentation & Examples

### For Each UGen

**1. Reference Documentation**
```markdown
## fm - Frequency Modulation Oscillator

### Syntax
```phonon
fm carrier_freq mod_freq mod_index
```

### Parameters
- `carrier_freq`: Base frequency (Hz or pattern)
- `mod_freq`: Modulator frequency (Hz or pattern)
- `mod_index`: Modulation depth (0-10+)

### Examples
```phonon
-- Classic FM bell
~bell: fm 440 2.0 5.0 # adsr 0.001 0.5 0.0 0.3

-- Pattern-controlled
~melody: "220 330 440 550"
~fm_synth: fm ~melody 1.5 3.0

-- Dynamic modulation
~lfo: sine 0.1
~fm_evolving: fm 110 2.0 (~lfo * 5.0 + 2.0)
```
```

**2. Musical Examples**
- Create `docs/examples/fm_synthesis.ph`
- Show practical musical use
- Demonstrate pattern integration

**3. Video Tutorials** (Future)
- Screen recordings of live coding
- Explaining synthesis concepts
- Building tracks from scratch

---

## Phase 8: Community & Ecosystem

### Contribution Guidelines

**Make it Easy to Add UGens**:

1. **Template Generator**:
   ```bash
   cargo run --bin create-ugen -- --name supersaw --params freq,detune,voices
   # Generates: SignalNode variant, compiler function, test template
   ```

2. **UGen Library** (Separate Crate):
   ```
   phonon-ugens/
   ├── oscillators/
   ├── filters/
   ├── effects/
   ├── envelopes/
   └── analysis/
   ```

3. **Contribution Workflow**:
   - Pick UGen from TODO list
   - Implement using template
   - Submit PR with tests + examples
   - Review focuses on audio quality, not just code

### Preset Library

**User-Contributed Synths**:
```phonon
-- community/presets/tb303.ph
-- Roland TB-303 style bass
fn tb303(note, cutoff, resonance) {
    ~osc: saw note # pulse note 0.5
    ~filtered: ~osc # moog_ladder cutoff resonance
    ~env: adsr 0.001 0.2 0.0 0.1
    return ~filtered * ~env
}
```

**Package Manager** (Future):
```bash
phonon install tb303
phonon install dub-techno-pack
```

---

## Phase 9: Tooling & Developer Experience

### A. Real-Time Feedback

**Problem**: Current workflow requires restart
**Solution**: Hot-reload everything

```rust
// Watch for changes
let watcher = notify::watcher(tx, Duration::from_millis(50))?;
watcher.watch("*.ph", RecursiveMode::Recursive)?;

// Reload on change
match rx.recv() {
    Ok(DebouncedEvent::Write(path)) => {
        graph = recompile(path)?;  // Instant update
    }
}
```

### B. Visual Feedback

**Oscilloscope Mode**:
```bash
phonon edit my_synth.ph --visualize
```
Shows:
- Waveform display
- Spectrum analyzer
- Envelope visualization
- Pattern timeline

**Technology**: `ratatui` for terminal UI

### C. REPL/Interactive Shell

```bash
$ phonon repl
phonon> let bass = saw 55
phonon> bass # lpf 300 0.8
[plays audio]
phonon> bass $ fast 2
[plays faster]
```

---

## Phase 10: Performance & Optimization

### A. Benchmarking Suite

**For Every UGen**:
```rust
#[bench]
fn bench_fm_synthesis(b: &mut Bencher) {
    let mut graph = create_fm_graph();
    b.iter(|| {
        graph.render(44100)  // 1 second
    });
}

// Target: <10ms for 1 second of audio @ 44.1kHz
```

### B. SIMD Optimization

**Use Rust's Portable SIMD**:
```rust
use std::simd::*;

// Process 4 samples at once
let freq_vec = f32x4::from_array([freq; 4]);
let phase_vec = f32x4::from_array([phase, phase+dt, phase+2*dt, phase+3*dt]);
let output = (phase_vec * TAU).sin() * amp_vec;
```

**Expected Speedup**: 2-4x for oscillators/filters

### C. Multi-Threading

**Pattern Evaluation**: Already parallel
**Graph Evaluation**: Can parallelize independent buses

```rust
// Future: parallel bus evaluation
buses.par_iter_mut().for_each(|(name, node_id)| {
    evaluate_subgraph(node_id);
});
```

---

## Phase 11: Licensing Strategy

### Code Provenance

**Three Categories**:

1. **Original Phonon Code** (MIT)
   - All current code
   - New implementations

2. **Ported from SuperCollider** (GPL)
   - Must keep GPL
   - Separate module: `phonon-gpl/`
   - Users opt-in

3. **Third-Party Crates** (Various)
   - `fundsp` (MIT/Apache-2.0) ✅
   - `biquad` (MIT/Apache-2.0) ✅
   - Track all licenses in `CREDITS.md`

### Attribution

**Every Ported Algorithm**:
```rust
/// FM Synthesis implementation
///
/// Algorithm based on:
/// - John Chowning (1973) "The Synthesis of Complex Audio Spectra"
/// - SuperCollider's FM7.cpp (GPL)
///
/// This implementation is a clean-room reimplementation
/// studying the original papers and SC source.
///
/// License: MIT (clean-room) or GPL (if ported)
SignalNode::FM { ... }
```

---

## Phase 12: Milestone Roadmap

### Year 1 (Months 1-12)

**Q1 (Months 1-3): Foundation**
- ✅ All Tier 1 UGens (10 essential)
- ✅ Multi-channel architecture
- ✅ ADSR envelopes working
- ✅ Stereo panning
- **Deliverable**: Can make professional tracks

**Q2 (Months 4-6): Synthesis**
- FM, wavetable, granular
- Karplus-Strong, waveguide
- Physical modeling basics
- **Deliverable**: Rival dedicated synths

**Q3 (Months 7-9): Effects**
- Convolution reverb
- Vocoder
- Pitch shifter
- Spectral processing
- **Deliverable**: Studio-quality effects

**Q4 (Months 10-12): Polish**
- All Tier 2 UGens complete
- Performance optimization
- Documentation complete
- 10+ example tracks
- **Deliverable**: Beta release

### Year 2 (Months 13-24)

**Q1 (Months 13-15): Advanced**
- Tier 3 UGens
- Plugin hosting (VST/LV2)
- MIDI support
- **Deliverable**: Feature-complete

**Q2 (Months 16-18): Analysis**
- Real-time pitch tracking
- Beat detection
- Onset detection
- **Deliverable**: Interactive instruments

**Q3 (Months 19-21): Spatial**
- Binaural/HRTF
- Ambisonics
- Surround sound
- **Deliverable**: 3D audio

**Q4 (Months 22-24): Community**
- Preset library (100+ synths)
- Tutorial series
- Package manager
- **Deliverable**: 1.0 release

---

## Resources & References

### Rust Audio Crates to Study

1. **fundsp** - https://github.com/SamiPerttu/fundsp
   - Complete DSP framework
   - Many UGens already implemented
   - Great API design to learn from

2. **dasp** - https://github.com/RustAudio/dasp
   - Sample/frame abstractions
   - Interpolation

3. **biquad** - https://github.com/korken89/biquad-rs
   - Production-ready filters

4. **cpal** - https://github.com/RustAudio/cpal
   - We already use this for audio I/O

5. **hound** - WAV file I/O (already using)

### SuperCollider Source Code

**Most Useful Files**:
- `server/plugins/` - All UGen implementations
- `server/plugins/LFOscillators.cpp` - Oscillator templates
- `server/plugins/FilterUGens.cpp` - Filter designs
- `server/plugins/DelayUGens.cpp` - Delay-based effects
- `include/plugin_interface/SC_PlugIn.h` - UGen API

**Study, Don't Copy**:
- Read for algorithm understanding
- Implement cleanly in Rust
- Cite in comments

### CSound Opcodes

**Documentation**: http://www.csounds.com/manual/html/
**Source**: https://github.com/csound/csound/tree/master/Opcodes

**Most Useful**:
- `foscil.c` - FM synthesis
- `grain.c` - Granular
- `phisem.c` - Physical modeling
- `pvs*.c` - Phase vocoder

### Academic Papers

1. **Julius O. Smith III** - https://ccrma.stanford.edu/~jos/
   - All books free online
   - Reference C code included
   - Gold standard for digital audio

2. **Miller Puckette** - Theory and Practice of Computer Music
   - Fundamentals of DSP
   - Pure Data creator

3. **Curtis Roads** - Microsound
   - Granular synthesis bible
   - Time-domain techniques

### Books to Buy

1. **"Designing Audio Effect Plugins in C++"** - Will Pirkle
   - Every effect algorithm explained
   - C++ code that ports easily to Rust

2. **"DAFX: Digital Audio Effects"** - Udo Zölzer
   - Academic rigor
   - MATLAB code (translate to Rust)

3. **"The Audio Programming Book"** - Boulanger & Lazzarini
   - CSound-focused
   - Comprehensive opcode guide

---

## Success Metrics

### Technical Metrics

**Coverage**:
- ☐ 90+ UGens implemented
- ☐ All major synthesis methods (FM, granular, physical modeling, etc.)
- ☐ All standard effects (reverb, delay, modulation, dynamics)
- ☐ Multi-channel support (stereo, quad, surround)
- ☐ 500+ tests passing (three-level methodology)

**Performance**:
- ☐ Real-time on consumer hardware
- ☐ <10ms render time for 1 second @ 44.1kHz
- ☐ Support 64+ simultaneous voices
- ☐ <50MB memory usage typical session

### Musical Metrics

**Can We Make**:
- ☐ Professional techno/house tracks
- ☐ Ambient soundscapes
- ☐ Realistic instruments (piano, strings, brass)
- ☐ Experimental/avant-garde sounds
- ☐ Film/game audio

**Artist Adoption**:
- ☐ 10+ artists using in production
- ☐ 100+ tracks released using Phonon
- ☐ Used in live performance

### Community Metrics

**Ecosystem**:
- ☐ 50+ contributors
- ☐ 100+ community presets
- ☐ 10+ tutorial videos
- ☐ Active Discord/forum
- ☐ Package repository

---

## Getting Started TODAY

### Week 1 Action Items

**Monday-Tuesday**: Set up porting infrastructure
```bash
# Clone references
git clone https://github.com/supercollider/supercollider sc-reference
git clone https://github.com/SamiPerttu/fundsp

# Create UGen tracking
mkdir phonon-ugens
touch UGEN_STATUS.md  # Track what's implemented
```

**Wednesday-Thursday**: Implement ADSR envelope
- Most requested feature
- Enables 80% of synth patches
- Good template for other UGens

**Friday**: Test & document
- Three-level tests
- Musical examples
- Update this plan

### This Month Goal

**Deliverable**: 5 new UGens
1. ADSR envelope
2. FM oscillator
3. White noise
4. Pulse (PWM)
5. Pan2 (stereo)

**Result**: "Phonon can now make analog-style synths"

---

## Call to Action

This plan is ambitious but **achievable**. Here's why:

1. **We're not starting from scratch** - Rust ecosystem + 50 years of research
2. **Architecture is solid** - Just adding nodes, not redesigning
3. **Testing methodology works** - Three-level catches bugs early
4. **Community exists** - Rust audio community is active

**What's needed**:
- Consistent effort (20 hours/week for 18-24 months)
- Systematic approach (follow this plan)
- Quality over speed (test everything)
- Community building (make it easy to contribute)

**Let's build the synthesis system that Rust deserves.**

---

*Last Updated: 2025-10-25*
*Status: Planning Complete - Ready to Execute*
*Next Review: 2026-01-01 (quarterly)*
