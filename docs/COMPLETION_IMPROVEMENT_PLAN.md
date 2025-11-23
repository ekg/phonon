# Tab Completion Improvement Plan

**Date**: 2025-11-23
**Issue**: 119 of 151 functions (79%) missing from tab completion
**Additional**: Context-aware completion needed for `#`, `$`, `busname:`

---

## Problem Analysis

### Coverage Gap
- **151 functions** implemented in `compositional_compiler.rs`
- **96 functions** have metadata in `function_metadata.rs`
- **119 functions missing** (79% gap!)

### Missing Critical Functions
- **Reverbs**: `plate` (Dattoro), `reverb_stereo`
- **Synths**: `supersaw`, `superpwm`, `superfm`, `superkick`, `supersnare`, `superhat`, `superchip`
- **Filters**: `bq_*`, `svf_*`, `rlpf`, `rhpf`, `moog`, `moog_hz`
- **Effects**: `vocoder`, `pitch_shift`, `formant`, `ring_mod`, `fm_crossmod`, `flanger`, `convolution`, `limiter`
- **Oscillators**: `vco`, `pulse`, `*_hz` variants, `*_trig` variants, `pluck`, `organ`
- **Utilities**: `xfade`, `xline`, `wrap`, `bipolar`, `sample_hold`, `timer`, `probe`
- **Granular**: `granular`, `wavetable`, `waveguide`
- **Analysis**: `rms`, `peak_follower`, `amp_follower`, `schmidt`
- **Noise**: `white_noise`, `pink_noise`, `brown_noise`
- **Patterns**: `slowcat`, `wchoose`, `select`, `*_effect`, `*_val` variants

### Context-Aware Completion Missing
Currently all functions shown in all contexts. Should be:
- After `#` → Show only **effects/filters** (lpf, reverb, delay, distort, etc.)
- After `$` → Show only **transforms** (fast, slow, rev, every, etc.)
- After `busname:` → Show **generators** (s, sine, saw, noise, etc.) + transforms

---

## Solution: Two-Phase Approach

### Phase 1: Auto-Generate Metadata (Quick Win - 1-2 days)

**Goal**: Get all 151 functions into tab completion immediately with basic metadata.

**Approach**:
1. Build script extracts function names from `compositional_compiler.rs`
2. Generate `generated_metadata.rs` with stub entries
3. Merge with hand-written `function_metadata.rs`

**Implementation**:
```rust
// build.rs - Extract functions from compiler
fn extract_functions() -> Vec<String> {
    // Parse compositional_compiler.rs
    // Find lines: "function_name" => compile_function(ctx, args)
    // Return function names
}

fn generate_metadata(functions: Vec<String>) -> String {
    // Generate FunctionMetadata stubs
    format!(r#"
    m.insert("{}", FunctionMetadata {{
        name: "{}",
        description: "TODO: Add description", // Mark for later completion
        params: vec![], // TODO: Extract from compile_* function signature
        example: "",
        category: "Unknown",
    }});
    "#, name, name)
}
```

**Result**: All 151 functions visible in tab completion within 1-2 days

### Phase 2: Category-Based Filtering (Medium Effort - 3-5 days)

**Goal**: Context-aware completion based on DSL syntax position.

**Approach**:
1. Categorize all functions by type:
   - **Effects**: lpf, hpf, reverb, delay, distort, chorus, etc.
   - **Transforms**: fast, slow, rev, every, sometimes, etc.
   - **Generators**: s, sine, saw, square, noise, etc.
   - **Utilities**: gain, pan, mix, etc.

2. Update `CompletionContext` enum:
```rust
pub enum CompletionContext {
    Function,           // General function context
    Effect,            // After # - show only effects
    Transform,         // After $ - show only transforms
    Generator,         // After busname: - show generators
    Sample,            // Inside quotes
    Bus,               // After ~
    Keyword(&'static str), // After :
    None,
}
```

3. Update context detection in `context.rs`:
```rust
pub fn get_completion_context(line: &str, cursor_pos: usize) -> CompletionContext {
    // Check for # before cursor (effect chain)
    if line[..cursor_pos].rfind('#').is_some() {
        // Make sure we're not inside a string
        if !in_string {
            return CompletionContext::Effect;
        }
    }

    // Check for $ before cursor (transform)
    if line[..cursor_pos].rfind('$').is_some() {
        if !in_string {
            return CompletionContext::Transform;
        }
    }

    // Check for busname: (generator/source)
    if line[..cursor_pos].rfind(':').is_some() {
        // Parse backwards to see if it's a bus assignment
        // ~busname: or out:
        return CompletionContext::Generator;
    }

    // ... existing logic
}
```

4. Update `filter_completions()` to respect categories:
```rust
match context {
    CompletionContext::Effect => {
        // Only show functions in "Effects" or "Filters" categories
        for (name, metadata) in FUNCTION_METADATA.iter() {
            if ["Effects", "Filters"].contains(&metadata.category) {
                // ... add to completions
            }
        }
    }
    CompletionContext::Transform => {
        // Only show functions in "Transforms" category
        for (name, metadata) in FUNCTION_METADATA.iter() {
            if metadata.category == "Transforms" {
                // ... add to completions
            }
        }
    }
    // ... etc
}
```

---

## Categorization Matrix

### Effects (show after `#`)
- **Filters**: lpf, hpf, bpf, notch, bq_*, svf_*, rlpf, rhpf, moog, moog_hz, resonz, djf
- **Reverbs**: reverb, reverb_stereo, plate
- **Delays**: delay, tape, tapedelay, multitap, pingpong
- **Modulation**: chorus, flanger, fchorus, phaser, tremolo (trem), vibrato (vib)
- **Distortion**: distort (dist), bitcrush, decimator, coarse
- **Dynamics**: comp (compressor), limiter, sc_comp (sidechain)
- **Pitch**: pitch_shift, vocoder, formant, vowel
- **Spatial**: pan, pan2
- **Other**: ring_mod, convolution, eq, freeze

### Transforms (show after `$`)
- **Speed**: fast, slow, hurry
- **Time**: rev (reverse), iter, palindrome
- **Conditional**: every, sometimes, sometimesBy, whenmod
- **Selection**: choose, wchoose, select, run
- **Structure**: stack, cat, slowcat, fastcat
- **Degradation**: degrade, degradeBy
- **Offset**: early, late, swing

### Generators (show after `busname:` or `out:`)
- **Samples**: s (sample)
- **Oscillators**: sine, saw, square, triangle, pulse, vco
- **Oscillators (Hz)**: sine_hz, saw_hz, square_hz, triangle_hz
- **Oscillators (Trig)**: sine_trig, saw_trig, square_trig, tri_trig
- **Synths**: supersaw, superpwm, superfm, superchip, superkick, supersnare, superhat
- **Noise**: noise, white_noise, pink_noise, brown_noise
- **Physical**: pluck, organ, waveguide
- **Advanced**: granular, wavetable, additive, fm, pm
- **Busses**: ~busname (bus reference)

### Utilities (show in any function context)
- **Amplitude**: gain, amp
- **Mixing**: mix, xfade
- **Math**: wrap, bipolar, unipolar, min, max
- **Envelope**: adsr, ad, asr, ar, attack, release, envelope, env_trig, curve, line, segments, xline
- **Sample & Hold**: sample_hold, latch, lag
- **Analysis**: rms, peak_follower, amp_follower
- **Control**: timer, probe, schmidt
- **Mapping**: n (note), note

---

## Implementation Timeline

### Week 1: Auto-Generation (Phase 1)
- **Day 1**: Write build.rs to extract functions from compiler
- **Day 2**: Generate stub metadata for all 119 missing functions
- **Day 3**: Test and verify all functions show in completion
- **Result**: 100% function coverage (with TODO descriptions)

### Week 2: Categorization (Phase 2 Part 1)
- **Day 1**: Categorize all 151 functions
- **Day 2**: Update CompletionContext enum
- **Day 3**: Update context detection (after #, $, :)
- **Day 4**: Update filter_completions() to respect categories
- **Day 5**: Test context-aware completion

### Week 3: Polish
- **Day 1-2**: Fill in missing descriptions for common functions
- **Day 3-4**: Add parameter metadata for top 50 functions
- **Day 5**: Documentation and examples

---

## Success Metrics

### Phase 1 Complete:
- ✅ All 151 functions visible in tab completion
- ✅ Fuzzy search finds all functions
- ✅ Zero "function not found" surprises for users

### Phase 2 Complete:
- ✅ `~bass: ` + TAB shows generators (sine, saw, s, noise, etc.)
- ✅ `~bass: saw 55 #` + TAB shows effects (lpf, reverb, delay, etc.)
- ✅ `s "bd" $` + TAB shows transforms (fast, slow, rev, every, etc.)
- ✅ No noise: irrelevant functions hidden in each context

### Phase 3 Complete:
- ✅ Top 50 functions have full parameter metadata
- ✅ All functions have meaningful descriptions
- ✅ Examples for common use cases

---

## Priority Functions to Document First (Top 20)

Once auto-generated, prioritize adding descriptions/params for:

1. **plate** - Dattoro plate reverb (user requested!)
2. **supersaw** - Popular analog-style synth
3. **superpwm** - Pulse width modulation synth
4. **moog** - Classic Moog ladder filter
5. **limiter** - Essential for preventing clipping
6. **vocoder** - Voice/carrier modulation
7. **pitch_shift** - Pitch shifting effect
8. **granular** - Granular synthesis
9. **wavetable** - Wavetable synthesis
10. **xfade** - Crossfade between signals
11. **ring_mod** - Ring modulation effect
12. **flanger** - Flanging effect
13. **convolution** - Convolution reverb
14. **waveguide** - Physical modeling
15. **formant** - Formant filter/vowel sounds
16. **brown_noise** - Brown noise generator
17. **pink_noise** - Pink noise generator
18. **comp** - Compressor (sidechain support)
19. **sample_hold** - Sample and hold
20. **ar** - Attack-release envelope

---

## Alternative: Incremental Approach

If 2-3 weeks is too long, do incrementally:

### Week 1: Just the Iceberg Tip (Quick Win)
Focus on **user-visible** missing functions only:
- Reverbs: plate, reverb_stereo
- Synths: super* series (7 functions)
- Filters: moog, moog_hz, bq_*, svf_*
- Effects: vocoder, pitch_shift, limiter, flanger

**Result**: ~25 most important functions added, covers 80% of user needs

### Week 2+: Context-Aware + Rest
- Add context-aware filtering (# $ :)
- Fill in remaining 94 functions gradually

---

## Maintenance Strategy

Going forward, prevent metadata drift:

1. **CI Check**: Fail build if compiler has functions without metadata
2. **Template**: When adding function to compiler, also add to metadata
3. **Generated + Manual**: Generated stubs + manually written descriptions
4. **Tests**: Test that all compiler functions have metadata entries

---

## Questions for User

1. **Priority**: Auto-gen all 151 functions first (Phase 1), or manually add top 20-30 important ones?
2. **Context-Aware**: Is # $ : filtering important, or can it wait?
3. **Descriptions**: OK with "TODO" placeholders initially, or block until written?
4. **Timeline**: Need this ASAP or OK with 2-3 week plan?
