# SIMD Integration Plan for VoiceManager

**Date**: 2025-11-15
**Status**: Prototyping complete, integration in progress
**Goal**: Integrate 3× SIMD speedup into production voice processing

## Progress Summary

### ✅ Completed (Phase 1 Days 1-5)

1. **SIMD Prototype Created** (`src/voice_simd.rs`)
   - Sample interpolation (AVX2)
   - Equal-power panning (AVX2)
   - Envelope calculation (partial)
   - Runtime CPU detection

2. **Benchmarking Complete** (`benches/voice_simd_bench.rs`)
   - Interpolation: **3.0× speedup** (11.4ns → 3.8ns)
   - Panning: **3.3× speedup** (22.5ns → 6.9ns)
   - Results documented in `SIMD_BENCHMARK_RESULTS.md`

3. **Integration Point Identified**
   - Target: `VoiceManager::process_buffer_per_node()` (line 1021)
   - Current bottleneck: `voice.process_stereo()` called 512× per voice per buffer

### 🔄 In Progress (Phase 1 Days 6-7)

**SIMD Integration into VoiceManager**

Current challenge: The `process_stereo()` function (lines 369-483) contains complex logic:
- Envelope processing with state machine
- Auto-release timing
- Loop handling (forward/reverse)
- Boundary checking
- Position advancement

**Two integration approaches:**

## Approach A: Full SIMD Voice Processing (Complex, High Gain)

Vectorize the entire `process_stereo()` pipeline to process 8 voices simultaneously.

### Implementation Steps

1. **Extract voice state into arrays** (Data-Oriented Design)
```rust
struct VoiceArrays {
    positions: [f32; 8],
    speeds: [f32; 8],
    gains: [f32; 8],
    pans: [f32; 8],
    env_values: [f32; 8],
    states: [VoiceState; 8],
    sample_lens: [f32; 8],
    // ... other fields
}
```

2. **Vectorize envelope processing**
```rust
unsafe fn process_envelopes_simd_x8(
    env_states: &mut [EnvelopeState; 8],
    env_levels: &mut [f32; 8],
    // ... envelope parameters
) -> [f32; 8] {
    // Use _mm256_* instructions to process 8 envelopes at once
    process_voices_envelope_simd_x8(...)
}
```

3. **Vectorize sample interpolation**
```rust
// Extract positions and sample data for 8 voices
let positions = extract_positions(&voices);
let samples_curr = extract_current_samples(&voices);
let samples_next = extract_next_samples(&voices);

// SIMD interpolation
let interpolated = interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);
```

4. **Vectorize panning**
```rust
let pans = extract_pans(&voices);
let (left_batch, right_batch) = apply_panning_simd_x8(&interpolated, &pans);
```

5. **Update voice positions (vectorized)**
```rust
// positions += speeds (using AVX2)
let new_positions = _mm256_add_ps(positions_vec, speeds_vec);
```

### Challenges

- **Branching**: Voice states cause divergent code paths (Free, Playing, Releasing)
- **Complex conditionals**: Looping, boundary checks, reverse playback
- **Memory layout**: Current AoS (Array of Structs) → need SoA (Struct of Arrays) for SIMD
- **Edge cases**: Handling remainder voices (non-multiple of 8)

### Expected Outcome

- **Performance**: 3-4× speedup on full voice processing
- **Complexity**: High (2-3 weeks implementation + testing)
- **Risk**: Medium (complex refactoring, potential for bugs)

## Approach B: Selective SIMD for Hot Operations (Simple, Moderate Gain)

Keep current architecture, vectorize only the innermost hot loops (interpolation + panning).

### Implementation Steps

1. **Add SIMD fast path to `process_stereo()`**

```rust
impl Voice {
    pub fn process_stereo(&mut self) -> (f32, f32) {
        // ... existing envelope processing (scalar)

        // SIMD path for interpolation + panning (if conditions met)
        #[cfg(target_arch = "x86_64")]
        if can_use_simd_path(self) {
            return self.process_stereo_simd();
        }

        // Existing scalar path (fallback)
        // ... rest of current implementation
    }

    #[cfg(target_arch = "x86_64")]
    fn process_stereo_simd(&mut self) -> (f32, f32) {
        // Use SIMD only for interpolation + panning
        // Keep envelope, state management in scalar
        unsafe {
            // Broadcast single voice data to SIMD lanes
            let positions = [self.position; 8];
            let samples_curr = [self.get_current_sample(); 8];
            let samples_next = [self.get_next_sample(); 8];

            let interpolated = interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);
            let sample_value = interpolated[0]; // Extract first lane

            // ... apply gain and envelope (scalar)

            let pans = [self.pan; 8];
            let samples = [output_value; 8];
            let (left_batch, right_batch) = apply_panning_simd_x8(&samples, &pans);

            (left_batch[0], right_batch[0])
        }
    }
}
```

### Challenges

- **Limited benefit**: Broadcasting to SIMD only to extract first element wastes SIMD potential
- **Overhead**: SIMD function call overhead might negate small per-voice benefit
- **Complexity**: Adds SIMD code path without full performance benefit

### Expected Outcome

- **Performance**: 1.5-2× speedup (limited by Amdahl's Law)
- **Complexity**: Low (1-2 days implementation + testing)
- **Risk**: Low (minimal changes to existing code)

## Approach C: Batch SIMD in Buffer Processing (Pragmatic, Good Balance)

Process 8 voices simultaneously at the buffer level, keep individual voice logic scalar.

### Implementation Steps

1. **Modify `process_buffer_per_node()` to batch voices**

```rust
pub fn process_buffer_per_node(&mut self, buffer_size: usize) -> Vec<HashMap<usize, f32>> {
    let mut output: Vec<HashMap<usize, f32>> = vec![HashMap::new(); buffer_size];

    // Check if SIMD is available
    #[cfg(target_arch = "x86_64")]
    let use_simd = is_avx2_supported() && self.voices.len() >= 8;

    #[cfg(target_arch = "x86_64")]
    if use_simd {
        // Process voices in batches of 8
        let num_full_batches = self.voices.len() / 8;

        for batch_idx in 0..num_full_batches {
            let start = batch_idx * 8;
            let end = start + 8;
            let voice_batch = &mut self.voices[start..end];

            // Process this batch with SIMD for the entire buffer
            process_voice_batch_simd(voice_batch, &mut output, buffer_size);
        }

        // Handle remainder voices (scalar)
        for voice in &mut self.voices[(num_full_batches * 8)..] {
            // ... scalar processing
        }

        return output;
    }

    // Fallback: Existing scalar/parallel implementation
    // ... current code
}

#[cfg(target_arch = "x86_64")]
fn process_voice_batch_simd(
    voices: &mut [Voice],  // Exactly 8 voices
    output: &mut Vec<HashMap<usize, f32>>,
    buffer_size: usize,
) {
    assert_eq!(voices.len(), 8);

    for sample_idx in 0..buffer_size {
        // Extract state from 8 voices
        let mut positions = [0.0f32; 8];
        let mut samples_curr = [0.0f32; 8];
        let mut samples_next = [0.0f32; 8];
        let mut pans = [0.0f32; 8];
        let mut gains_envs = [0.0f32; 8];
        let mut source_nodes = [0usize; 8];

        for (i, voice) in voices.iter_mut().enumerate() {
            // Process envelope (scalar per voice)
            let env_value = voice.envelope.process();

            // Extract data for SIMD
            if let Some(ref samples) = voice.sample_data {
                let pos_floor = voice.position.floor() as usize;
                if pos_floor + 1 < samples.len() {
                    positions[i] = voice.position;
                    samples_curr[i] = samples[pos_floor];
                    samples_next[i] = samples[pos_floor + 1];
                    pans[i] = voice.pan;
                    gains_envs[i] = voice.gain * env_value;
                    source_nodes[i] = voice.source_node;
                }
            }
        }

        // SIMD operations on 8 voices
        unsafe {
            // Interpolate all 8 samples simultaneously
            let interpolated = interpolate_samples_simd_x8(&positions, &samples_curr, &samples_next);

            // Apply gains/envelopes
            let mut gained = [0.0f32; 8];
            for i in 0..8 {
                gained[i] = interpolated[i] * gains_envs[i];
            }

            // Pan all 8 voices simultaneously
            let (left_batch, right_batch) = apply_panning_simd_x8(&gained, &pans);

            // Accumulate to output
            for i in 0..8 {
                let mono = (left_batch[i] + right_batch[i]) / std::f32::consts::SQRT_2;
                output[sample_idx]
                    .entry(source_nodes[i])
                    .and_modify(|v| *v += mono)
                    .or_insert(mono);

                // Advance voice positions (scalar)
                voices[i].position += voices[i].speed;
                voices[i].age += 1;
            }
        }
    }
}
```

### Advantages

- **Moderate complexity**: Don't need to refactor entire Voice structure
- **Good performance**: Get 3× benefit on hottest operations
- **Incremental**: Can optimize further later
- **Safe**: Keep complex logic (envelopes, looping) in proven scalar code

### Challenges

- **Extracting state**: Need to gather data from 8 voices each sample
- **Scattering results**: Need to update 8 voices after SIMD operations
- **Load balancing**: Batches may have mix of active/free voices

### Expected Outcome

- **Performance**: 2-2.5× speedup (conservative estimate)
- **Complexity**: Medium (3-5 days implementation + testing)
- **Risk**: Low-Medium (isolated changes, fallback to scalar)

## Recommended Approach: **C (Batch SIMD)**

**Rationale**:
- **Pragmatic**: Balances performance gain vs implementation complexity
- **Incremental**: Delivers speedup now, can optimize further later
- **Safe**: Preserves proven scalar logic for complex edge cases
- **Testable**: Easy to verify (compare SIMD vs scalar output)

## Implementation Timeline

### Week 1 (Current)
- [x] Days 1-2: SIMD prototype and benchmarking
- [x] Days 3-5: Benchmarking and validation
- [ ] Days 6-7: Implement Approach C integration

### Week 2
- [ ] Days 1-2: Testing and correctness validation
- [ ] Days 3-4: Profile real workload (q.ph pattern)
- [ ] Day 5: Optimize and tune

### Success Criteria

1. **Audio correctness**: SIMD output matches scalar (or <-120dB difference)
2. **Performance**: 2-2.5× speedup on q.ph pattern (P95 latency: 8.66ms → ~3.5ms)
3. **Voice capacity**: 280 voices → 650+ voices @ <11.6ms budget
4. **Stability**: No crashes, no audio glitches, handles edge cases

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_simd_batch_processing_matches_scalar() {
    let mut vm_scalar = VoiceManager::new();
    let mut vm_simd = VoiceManager::new();

    // ... trigger same samples

    // Render buffers
    let scalar_output = vm_scalar.process_buffer_per_node(512);
    let simd_output = vm_simd.process_buffer_per_node(512);

    // Compare (should be bit-exact or very close)
    assert_buffers_match(&scalar_output, &simd_output, -120.0);  // -120dB tolerance
}
```

### Integration Tests

- Render q.ph pattern with SIMD enabled vs disabled
- Compare WAV outputs (should be identical)
- Profile P95 latency improvement

### Stress Tests

- 64 voices (full polyphony)
- Mix of forward/reverse playback
- Various speeds (0.5×, 1.0×, 2.0×)
- Edge cases (looping, sample boundaries)

## Next Immediate Steps

1. **Implement `process_voice_batch_simd()` helper** (Approach C)
2. **Integrate into `process_buffer_per_node()`**
3. **Add correctness tests** (scalar vs SIMD comparison)
4. **Profile with real workload** (q.ph pattern)
5. **Measure actual speedup** (expecting 2-2.5×)

## Future Optimizations (Post-Phase 1)

Once Approach C is working and validated:

1. **Complete envelope SIMD**: Finish vectorizing ADSR state machine
2. **Optimize data extraction**: Reduce gather/scatter overhead
3. **Explore AVX-512**: 16 voices at once (on supported CPUs)
4. **Consider Approach A**: Full SoA refactor for 4× speedup

---

**Current Status**: Benchmarking complete, ready to implement Approach C
**Expected Delivery**: End of Week 1 (SIMD integration working)
**Expected Performance**: 2-2.5× speedup (650+ voice capacity)
