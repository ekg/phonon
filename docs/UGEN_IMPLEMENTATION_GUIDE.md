# UGen Implementation Guide
## How to Add a New Synthesis Unit to Phonon

This guide shows you **exactly** how to add a new UGen, step by step.

---

## Example: Adding ADSR Envelope

We'll implement an ADSR (Attack, Decay, Sustain, Release) envelope as a complete example.

### Step 1: Define the SignalNode (5 minutes)

**File**: `src/unified_graph.rs`

Find the `SignalNode` enum (around line 500) and add:

```rust
/// ADSR Envelope (Attack, Decay, Sustain, Release)
ADSR {
    trigger: Signal,      // Pattern that triggers envelope
    attack: Signal,       // Attack time in seconds
    decay: Signal,        // Decay time in seconds
    sustain: Signal,      // Sustain level (0.0 to 1.0)
    release: Signal,      // Release time in seconds
    state: ADSRState,
},
```

### Step 2: Define the State (5 minutes)

Still in `unified_graph.rs`, add near the other state structs (around line 750):

```rust
/// ADSR envelope state
#[derive(Debug, Clone)]
pub struct ADSRState {
    phase: ADSRPhase,
    level: f32,
    last_trigger: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ADSRPhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl ADSRState {
    pub fn new() -> Self {
        Self {
            phase: ADSRPhase::Idle,
            level: 0.0,
            last_trigger: 0.0,
        }
    }
}

impl Default for ADSRState {
    fn default() -> Self {
        Self::new()
    }
}
```

### Step 3: Implement Evaluation Logic (30 minutes)

In `unified_graph.rs`, find the `eval_node()` function (around line 1200).

Add this case to the match statement:

```rust
SignalNode::ADSR {
    trigger,
    attack,
    decay,
    sustain,
    release,
    state,
} => {
    // Evaluate parameters
    let trigger_val = self.eval_signal(trigger, cycle_position, references)?;
    let attack_time = self.eval_signal(attack, cycle_position, references)?.max(0.001);
    let decay_time = self.eval_signal(decay, cycle_position, references)?.max(0.001);
    let sustain_level = self.eval_signal(sustain, cycle_position, references)?.clamp(0.0, 1.0);
    let release_time = self.eval_signal(release, cycle_position, references)?.max(0.001);

    let sample_rate = self.sample_rate;
    let dt = 1.0 / sample_rate;

    // Detect trigger (rising edge above 0.5)
    let triggered = trigger_val > 0.5 && state.last_trigger <= 0.5;
    state.last_trigger = trigger_val;

    if triggered {
        state.phase = ADSRPhase::Attack;
    }

    // Update envelope based on phase
    match state.phase {
        ADSRPhase::Idle => {
            state.level = 0.0;
        }
        ADSRPhase::Attack => {
            // Linear attack
            state.level += dt / attack_time;
            if state.level >= 1.0 {
                state.level = 1.0;
                state.phase = ADSRPhase::Decay;
            }
        }
        ADSRPhase::Decay => {
            // Exponential decay to sustain
            let target = sustain_level;
            state.level += (target - state.level) * (dt / decay_time) * 5.0;
            if (state.level - target).abs() < 0.01 {
                state.level = target;
                state.phase = ADSRPhase::Sustain;
            }
        }
        ADSRPhase::Sustain => {
            // Hold at sustain level until trigger goes low
            state.level = sustain_level;
            if trigger_val <= 0.5 {
                state.phase = ADSRPhase::Release;
            }
        }
        ADSRPhase::Release => {
            // Exponential release to zero
            state.level += (0.0 - state.level) * (dt / release_time) * 5.0;
            if state.level < 0.001 {
                state.level = 0.0;
                state.phase = ADSRPhase::Idle;
            }
        }
    }

    Ok(state.level)
}
```

### Step 4: Add Compiler Function (20 minutes)

**File**: `src/compositional_compiler.rs`

Add this function (around line 1000, after other compile functions):

```rust
/// Compile ADSR envelope
fn compile_adsr(ctx: &mut CompilerContext, args: Vec<Expr>) -> Result<NodeId, String> {
    // Extract input/trigger (handles both standalone and chained forms)
    let (trigger_signal, params) = extract_chain_input(ctx, &args)?;

    if params.len() != 4 {
        return Err(format!(
            "adsr requires 4 parameters (attack, decay, sustain, release), got {}",
            params.len()
        ));
    }

    // Compile parameters
    let attack_node = compile_expr(ctx, params[0].clone())?;
    let decay_node = compile_expr(ctx, params[1].clone())?;
    let sustain_node = compile_expr(ctx, params[2].clone())?;
    let release_node = compile_expr(ctx, params[3].clone())?;

    use crate::unified_graph::ADSRState;

    let node = SignalNode::ADSR {
        trigger: trigger_signal,
        attack: Signal::Node(attack_node),
        decay: Signal::Node(decay_node),
        sustain: Signal::Node(sustain_node),
        release: Signal::Node(release_node),
        state: ADSRState::default(),
    };

    Ok(ctx.graph.add_node(node))
}
```

### Step 5: Register in Function Table (2 minutes)

In `compositional_compiler.rs`, find the `compile_function()` match statement (around line 400).

Add this line:

```rust
// ========== Envelopes ==========
"adsr" => compile_adsr(ctx, args),
"env" | "envelope" => compile_envelope(ctx, args),
```

### Step 6: Write Tests (30 minutes)

**File**: `tests/test_adsr_envelope.rs` (create new file)

```rust
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod pattern_verification_utils;
use pattern_verification_utils::*;

/// LEVEL 1: Pattern query - verify trigger pattern
#[test]
fn test_adsr_level1_trigger_pattern() {
    use phonon::mini_notation_v3::parse_mini_notation;
    use phonon::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    // Pattern triggers envelope
    let pattern = parse_mini_notation("x ~ x ~");

    let mut triggers = Vec::new();
    for cycle in 0..2 {
        let state = State {
            span: TimeSpan::new(
                Fraction::from_float(cycle as f64),
                Fraction::from_float((cycle + 1) as f64),
            ),
            controls: HashMap::new(),
        };

        let events = pattern.query(&state);
        triggers.push(events.len());
    }

    // Should have 2 triggers per cycle (4 total over 2 cycles)
    assert_eq!(triggers.iter().sum::<usize>(), 4);
}

/// LEVEL 2: Onset detection - verify envelope shapes audio
#[test]
fn test_adsr_level2_onset_detection() {
    let dsl = r#"
tempo: 2.0
~trigger: "x ~ x ~"
~env: ~trigger # adsr 0.01 0.1 0.5 0.2
~tone: sine 440 * ~env
out: ~tone
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0).unwrap();

    // Render 1 second (2 cycles at tempo 2.0)
    let audio = graph.render(44100);

    // Detect onsets (should match trigger pattern)
    let onsets = detect_audio_events(&audio, 44100.0, 0.01);

    // Should detect 4 envelope attacks
    assert!(
        onsets.len() >= 3 && onsets.len() <= 5,
        "Expected 4 onsets, got {}",
        onsets.len()
    );
}

/// LEVEL 3: Audio characteristics - verify envelope shape
#[test]
fn test_adsr_level3_envelope_shape() {
    let dsl = r#"
tempo: 1.0
~trigger: "x"
~env: ~trigger # adsr 0.1 0.2 0.5 0.3
~tone: sine 440 * ~env
out: ~tone
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0).unwrap();

    // Render 2 seconds (enough for full envelope)
    let audio = graph.render(88200);

    // Find peak (should occur around attack time)
    let peak_sample = audio
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
        .unwrap()
        .0;

    let peak_time = peak_sample as f32 / 44100.0;

    // Peak should be near attack time (0.1s) + some decay
    assert!(
        peak_time > 0.05 && peak_time < 0.2,
        "Peak at {:.3}s, expected around 0.1s",
        peak_time
    );

    // End should be quiet (release completed)
    let end_rms = calculate_rms(&audio[audio.len() - 4410..]);
    assert!(
        end_rms < 0.01,
        "End RMS {:.6}, should be near zero after release",
        end_rms
    );
}

/// Test with pattern-controlled parameters
#[test]
fn test_adsr_pattern_params() {
    let dsl = r#"
tempo: 2.0
~trigger: "x x x x"
~attack_pattern: "0.01 0.05 0.1 0.2"
~env: ~trigger # adsr ~attack_pattern 0.1 0.7 0.2
~tone: sine 440 * ~env
out: ~tone
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0).unwrap();

    let audio = graph.render(22050); // 0.5 seconds

    // Should produce audio
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "RMS {:.6}, should have signal", rms);
}
```

### Step 7: Run Tests (2 minutes)

```bash
cargo test test_adsr
```

All tests should pass!

### Step 8: Create Example (10 minutes)

**File**: `docs/examples/adsr_demo.ph`

```phonon
-- ADSR Envelope Demonstration
tempo: 2.0

-- Simple trigger pattern
~trigger: "x ~ x ~"

-- ADSR with different settings
~env_short: ~trigger # adsr 0.001 0.1 0.0 0.1   -- Percussive
~env_pad: ~trigger # adsr 0.5 0.3 0.7 1.0        -- Pad sound

-- Apply to oscillators
~perc: sine 440 * ~env_short
~pad: saw 220 * ~env_pad

-- Pattern-controlled attack
~melody: "x ~ x ~ x x ~ x"
~attack_var: "0.01 0.05 0.1 0.2 0.3 0.2 0.1 0.05"
~env_dynamic: ~melody # adsr ~attack_var 0.2 0.6 0.3
~synth: saw 330 * ~env_dynamic

-- Mix
out: (~perc * 0.3 + ~pad * 0.2 + ~synth * 0.3) * 0.8
```

Test it:
```bash
cargo run --bin phonon -- render docs/examples/adsr_demo.ph output.wav --duration 4
```

### Step 9: Document (10 minutes)

Add to `docs/DSL_REFERENCE.md`:

```markdown
## adsr - ADSR Envelope Generator

Generates an Attack-Decay-Sustain-Release envelope triggered by a pattern.

### Syntax
```phonon
pattern # adsr attack decay sustain release
```

### Parameters
- `attack`: Attack time in seconds (time to reach peak)
- `decay`: Decay time in seconds (time to reach sustain)
- `sustain`: Sustain level (0.0 to 1.0)
- `release`: Release time in seconds (time to fade to zero)

### Examples

**Basic envelope**:
```phonon
~trigger: "x ~ x ~"
~env: ~trigger # adsr 0.01 0.1 0.7 0.2
~synth: sine 440 * ~env
out: ~synth
```

**Pattern-controlled parameters**:
```phonon
~trigger: "x x x x"
~attack_times: "0.001 0.01 0.1 0.5"
~env: ~trigger # adsr ~attack_times 0.2 0.6 0.3
```

**Musical use - bass pluck**:
```phonon
~bassline: "bd ~ bd ~"
~env: ~bassline # adsr 0.001 0.2 0.0 0.1
~bass: saw 55 * ~env # lpf 400 0.9
out: ~bass * 0.7
```
```

### Step 10: Update Status (2 minutes)

Update `docs/SYNTHESIS_PARITY_PLAN.md`:

```markdown
### Tier 1: Essential (Complete First - 3 months)

1. ✅ **Envelopes** (ADSR, AD, Line) - COMPLETED 2025-10-25
   - ✅ ADSR implemented
   - ⏳ AD (attack-decay)
   - ⏳ Line (linear ramp)
```

---

## Total Time: ~2 hours

Breaking it down:
- Define nodes: 10 min
- Implement logic: 30 min
- Compiler integration: 20 min
- Write tests: 30 min
- Create examples: 10 min
- Documentation: 10 min
- Testing/debugging: 10 min

**First UGen takes 2 hours. After 5 UGens, you'll be down to 1 hour each.**

---

## Template for Next UGens

Save this as `scripts/create_ugen.sh`:

```bash
#!/bin/bash
# Quick UGen template generator

UGEN_NAME=$1
PARAMS=$2

cat > "tests/test_${UGEN_NAME}.rs" <<EOF
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

mod pattern_verification_utils;
use pattern_verification_utils::*;

#[test]
fn test_${UGEN_NAME}_basic() {
    let dsl = r#"
tempo: 2.0
~sig: ${UGEN_NAME} 440  -- TODO: add params
out: ~sig
"#;

    let (_, statements) = parse_program(dsl).unwrap();
    let mut graph = compile_program(statements, 44100.0).unwrap();
    let audio = graph.render(44100);

    let rms = calculate_rms(&audio);
    assert!(rms > 0.001, "Should produce audio");
}
EOF

echo "✅ Created tests/test_${UGEN_NAME}.rs"
echo ""
echo "Next steps:"
echo "1. Add SignalNode::${UGEN_NAME} to src/unified_graph.rs"
echo "2. Implement eval logic in eval_node()"
echo "3. Add compile_${UGEN_NAME}() to src/compositional_compiler.rs"
echo "4. Register in compile_function() match"
echo "5. Run: cargo test test_${UGEN_NAME}"
```

---

## Next UGens to Implement

Following the priority plan:

### 1. FM Oscillator (Next - Week 1)
**Complexity**: Medium
**Impact**: High - enables huge range of sounds
**Files to modify**: Same pattern as ADSR

```rust
SignalNode::FM {
    carrier_freq: Signal,
    mod_freq: Signal,
    mod_index: Signal,
    carrier_phase: f32,
    mod_phase: f32,
}
```

### 2. White Noise (Week 2)
**Complexity**: Easy
**Impact**: High - essential for drums/percussion

```rust
SignalNode::WhiteNoise {
    seed: u64,
}

// Eval: use rand::RngCore
```

### 3. Pulse/PWM (Week 3)
**Complexity**: Easy
**Impact**: Medium - analog synth sounds

```rust
SignalNode::Pulse {
    freq: Signal,
    width: Signal,  // 0.0 to 1.0
    phase: f32,
}
```

### 4. Pan2 (Week 4)
**Complexity**: Medium (requires multi-channel)
**Impact**: High - need stereo!

This requires architectural changes - see Phase 5 of main plan.

---

## Tips for Success

### 1. Study Before Implementing
- Read Julius O. Smith's books (free online)
- Look at SuperCollider source
- Check if `fundsp` already has it

### 2. Test As You Go
- Write tests FIRST (TDD)
- Test each phase of envelope/oscillator
- Use three-level methodology

### 3. Make Musical Examples
- Don't just verify it works
- Verify it sounds GOOD
- Share examples for feedback

### 4. Ask for Help
- Rust audio Discord: https://discord.gg/QPdhk2u
- Post questions with code snippets
- Many friendly experts

### 5. Commit Often
```bash
git add .
git commit -m "Implement ADSR envelope with tests"
git push
```

Small commits = easy to revert if stuck

---

## Common Pitfalls

### 1. Forgetting to Initialize State
```rust
// ❌ Bad - phase not initialized
SignalNode::FM { phase: 0.0, ... }  // Created once

// ✅ Good - state persists across eval() calls
state: FMState::default()
```

### 2. Not Handling Sample Rate
```rust
// ❌ Bad - hardcoded
phase += freq * 0.00002267;

// ✅ Good - use sample_rate
phase += freq / self.sample_rate;
```

### 3. Missing Edge Cases
```rust
// ❌ Bad - division by zero
let period = 1.0 / freq;

// ✅ Good - guard against zero
let period = 1.0 / freq.max(0.001);
```

### 4. Not Testing Pattern Modulation
```rust
// Test with constant AND pattern
~env: adsr 0.1 0.2 0.5 0.3           -- ✓ Test
~env: adsr ~attack "0.2" 0.5 0.3     -- ✓ Test both!
```

---

## Getting Unstuck

**If implementation is hard**:
1. Look at existing UGens (Reverb, Distortion)
2. Copy structure, modify algorithm
3. Start simple, add complexity later

**If tests are failing**:
1. Test each component separately
2. Print intermediate values
3. Visualize waveform (write to WAV, open in Audacity)

**If unsure about algorithm**:
1. Google "[effect name] algorithm"
2. Check SuperCollider source
3. Read Julius O. Smith's books
4. Ask in Discord

---

## Celebrate Progress!

After each UGen:
- ✅ Run ALL tests (make sure nothing broke)
- ✅ Make a demo track using it
- ✅ Update SYNTHESIS_PARITY_PLAN.md
- ✅ Commit and push
- ✅ Share in Discord/forum

**You're building something amazing. Every UGen makes Phonon more powerful!**

---

*Ready to start? Pick one UGen and follow this guide exactly. In 2 hours, you'll have added your first synthesis unit to Phonon!*
