# E2E Audio Test Expansion Plan

## Mission
Comprehensive end-to-end verification that Phonon renders correct audio with proper timing, frequencies, and effects processing. Every feature must be scientifically verified with FFT analysis and temporal checks.

## Test Strategy
- **FFT Analysis**: Verify specific frequencies, harmonics, filtering
- **Onset Detection**: Verify rhythm, timing, attack/decay
- **RMS Analysis**: Verify amplitude, mixing, envelopes
- **Temporal Stability**: Verify patterns repeat correctly over many cycles

## Priority 1: Pattern-Controlled Parameters ⭐⭐⭐
**Why:** This is Phonon's UNIQUE feature - patterns as continuous control signals (impossible in Tidal/Strudel)

### Tests to Implement
1. **Pattern Modulates Filter Cutoff**
   - LFO pattern sweeps filter cutoff
   - FFT verifies frequency sweep (spectral centroid changes over time)
   ```phonon
   ~lfo: sine 0.5
   out: saw 110 # lpf (~lfo * 1000 + 500) 0.8
   ```
   - Verify: Spectral centroid sweeps from ~500Hz to ~1500Hz

2. **Pattern Modulates Amplitude**
   - Pattern creates amplitude envelope
   - RMS analysis verifies envelope shape
   ```phonon
   ~env: sine 0.25
   out: sine 440 * ~env
   ```
   - Verify: RMS varies over time, following sine wave

3. **Pattern Arithmetic**
   - Complex math on patterns works
   ```phonon
   ~lfo1: sine 0.5
   ~lfo2: sine 0.3
   out: sine 440 * (~lfo1 * 0.5 + ~lfo2 * 0.5)
   ```
   - Verify: Combined modulation detectable in amplitude variation

4. **Pattern Controls Synthesis Frequency**
   - Pattern sweeps oscillator frequency
   ```phonon
   ~sweep: sine 0.5
   out: sine (~sweep * 220 + 440)
   ```
   - Verify: FFT shows frequency sweep from 220Hz to 660Hz

5. **Pattern Controls Resonance**
   - Pattern modulates filter Q
   ```phonon
   ~q_mod: sine 0.5
   out: saw 110 # lpf 500 (~q_mod * 5 + 1)
   ```
   - Verify: Resonant peak varies in FFT

## Priority 2: Effects Chains
**Why:** Verify DSP effects actually process audio correctly

### Tests to Implement
1. **Low-Pass Filter**
   - Removes high frequencies
   ```phonon
   out: saw 110 # lpf 500 0.8
   ```
   - Verify: FFT shows rolloff above 500Hz, no content above 1000Hz

2. **High-Pass Filter**
   - Removes low frequencies
   ```phonon
   out: saw 110 # hpf 300 0.8
   ```
   - Verify: FFT shows rolloff below 300Hz, no content below 100Hz

3. **Band-Pass Filter**
   - Only allows band to pass
   ```phonon
   out: noise # bpf 1000 0.5
   ```
   - Verify: FFT shows energy concentrated around 1000Hz

4. **Reverb Tail**
   - Extends decay time
   ```phonon
   out: s "cp" # reverb 0.5 0.8
   ```
   - Verify: Onset analysis shows extended tail (RMS decay time > 0.5s)

5. **Delay Echoes**
   - Creates distinct echoes
   ```phonon
   out: s "cp" # delay 0.25 0.5
   ```
   - Verify: Multiple onsets at ~0.25s intervals

6. **Distortion Harmonics**
   - Generates harmonics
   ```phonon
   out: sine 110 # distort 0.8
   ```
   - Verify: FFT shows harmonics at 220Hz, 330Hz, 440Hz

7. **Compressor Dynamics**
   - Reduces dynamic range
   ```phonon
   out: s "bd sn" # compress 0.3 3.0 0.01 0.1
   ```
   - Verify: Peak/RMS ratio reduced compared to uncompressed

8. **Effect Chain Order**
   - Verify order matters
   ```phonon
   # Filter then distort
   ~a: sine 110 # lpf 200 0.8 # distort 0.5
   # Distort then filter
   ~b: sine 110 # distort 0.5 # lpf 200 0.8
   ```
   - Verify: Different harmonic content in FFT

## Priority 3: Polyphony & Voice Management
**Why:** Verify 64-voice system handles overlapping samples correctly

### Tests to Implement
1. **Many Overlapping Voices**
   ```phonon
   out: s "bd*16"
   ```
   - Verify: All 16 triggers fire, no dropouts, RMS stable

2. **Voice Stealing**
   ```phonon
   out: s "bd*128"  # Exceeds 64 voice limit
   ```
   - Verify: Oldest voices stolen gracefully, no glitches

3. **Polyphonic Chords**
   ```phonon
   out: s "[bd, sn, hh, cp]"  # 4 simultaneous
   ```
   - Verify: All 4 samples present in spectrum

4. **Rapid Triggering**
   ```phonon
   out: s "hh*32" $ fast 4  # Very fast hi-hats
   ```
   - Verify: All triggers fire, stable timing

## Priority 4: Bus Routing & Mixing
**Why:** Verify multi-bus architecture works correctly

### Tests to Implement
1. **Two Bus Mix**
   ```phonon
   ~kick: s "bd"
   ~snare: s "sn"
   out: ~kick * 0.7 + ~snare * 0.3
   ```
   - Verify: Mix ratio approximately 70/30 in RMS

2. **Multiple Bus Arithmetic**
   ```phonon
   ~a: sine 440
   ~b: sine 880
   ~c: sine 1320
   out: (~a + ~b + ~c) * 0.33
   ```
   - Verify: All three frequencies present with equal magnitude

3. **Bus Through Effects**
   ```phonon
   ~dry: s "bd"
   ~wet: ~dry # reverb 0.5 0.8
   out: ~dry * 0.5 + ~wet * 0.5
   ```
   - Verify: Both dry and reverb tail present

4. **Complex Routing**
   ```phonon
   ~bass: saw 55
   ~filtered: ~bass # lpf 800 0.8
   ~distorted: ~filtered # distort 0.3
   out: ~distorted * 0.5
   ```
   - Verify: Signal path processes correctly

## Priority 5: Signal Chaining (#)
**Why:** Verify left-to-right signal flow works

### Tests to Implement
1. **Three-Stage Chain**
   ```phonon
   out: saw 110 # lpf 500 0.8 # reverb 0.3 0.5 # compress 0.5 2.0 0.01 0.1
   ```
   - Verify: All three effects applied in order

2. **Sample Through Effects**
   ```phonon
   out: s "bd" # lpf 1000 0.8 # distort 0.2
   ```
   - Verify: Kick filtered and distorted

3. **Synthesis Chain**
   ```phonon
   out: sine 220 # distort 0.5 # hpf 200 0.5
   ```
   - Verify: Harmonics generated, then filtered

## Priority 6: Mini-Notation Features
**Why:** Verify pattern mini-notation works correctly

### Tests to Implement
1. **Repetition (*)**
   ```phonon
   out: s "bd*4"
   ```
   - Verify: 4 onsets per cycle

2. **Division (/)**
   ```phonon
   out: s "bd/2"
   ```
   - Verify: 1 onset every 2 cycles

3. **Polyrhythm ([])**
   ```phonon
   out: s "[bd bd bd, sn sn]"
   ```
   - Verify: 3 kicks and 2 snares in same cycle

4. **Alternation (<>)**
   ```phonon
   out: s "<bd sn>"
   ```
   - Verify: Alternates each cycle

5. **Rests (~)**
   ```phonon
   out: s "bd ~ sn ~"
   ```
   - Verify: Only 2 onsets per cycle

6. **Nested Structures**
   ```phonon
   out: s "[bd*2, [sn hh]*2]"
   ```
   - Verify: Complex polyrhythm timing correct

## Priority 7: Stress Tests
**Why:** Verify stability under extreme conditions

### Tests to Implement
1. **Long Render (100 Cycles)**
   ```phonon
   out: s "bd sn hh cp"
   ```
   - Verify: No timing drift, memory stable

2. **64 Simultaneous Voices**
   ```phonon
   out: s "[bd,sn,hh,cp,bd,sn,hh,cp,bd,sn,hh,cp,bd,sn,hh,cp]*16"
   ```
   - Verify: All voices play, no corruption

3. **Complex Pattern Composition**
   ```phonon
   ~k: s "bd" $ euclid 5 8
   ~s: s "sn" $ euclid 3 8 $ fast 2
   ~h: s "hh*8" $ sometimes (fast 2)
   ~c: s "cp*4" $ every 3 (rev)
   out: ~k*0.4 + ~s*0.3 + ~h*0.2 + ~c*0.2
   ```
   - Verify: All patterns render correctly

4. **Memory Stability**
   - Render 1000 cycles
   - Verify: No memory leaks, consistent performance

## Priority 8: Edge Cases
**Why:** Verify graceful handling of unusual inputs

### Tests to Implement
1. **Empty Pattern**
   ```phonon
   out: s ""
   ```
   - Verify: Silence (RMS near 0)

2. **Zero Frequency**
   ```phonon
   out: sine 0
   ```
   - Verify: DC signal (RMS very low)

3. **Extreme Filter Q**
   ```phonon
   out: saw 110 # lpf 500 20.0
   ```
   - Verify: Doesn't explode, resonant peak visible

4. **Negative Parameters**
   ```phonon
   out: sine 440 * -0.5
   ```
   - Verify: Phase inversion works

5. **Invalid Sample Name**
   ```phonon
   out: s "nonexistent_sample"
   ```
   - Verify: Fails gracefully or silence

6. **Very High Tempo**
   ```phonon
   tempo: 20.0
   out: s "bd"
   ```
   - Verify: Extremely fast cycles work

7. **Very Low Tempo**
   ```phonon
   tempo: 0.1
   out: s "bd"
   ```
   - Verify: Very slow cycles work

## Implementation Order
1. ✅ Pattern-Controlled Parameters (MOST IMPORTANT)
2. ✅ Effects Chains (Core DSP verification)
3. ✅ Mini-Notation Features (Pattern syntax)
4. ✅ Polyphony & Voice Management
5. ✅ Bus Routing & Mixing
6. ✅ Signal Chaining
7. ✅ Stress Tests
8. ✅ Edge Cases

## Success Criteria
- All E2E tests pass with FFT and temporal verification
- 100% of core features covered
- Scientific proof that Phonon renders correct audio
- Comprehensive regression protection

## Timeline
Continue until complete - no stopping! Implement all 50+ tests systematically.
