-- FFT/Spectral Freeze Demo
-- Demonstrates real-time spectrum freezing using FFT/IFFT

-- Spectral freeze captures and holds the frequency spectrum
-- of an audio signal when triggered. This creates unique
-- sound design possibilities.

tempo: 2.0

-- Example 1: Basic spectral freeze with trigger pattern
-- The freeze happens on "x" events, holding the spectrum

~source: sine 440
~trigger: "x ~ ~ ~"     -- Freeze every 4 beats
~frozen: freeze ~source ~trigger
out: ~frozen * 0.4

-- Example 2: Spectral freeze on complex sources
-- Freezing harmonic content creates interesting textures

-- ~chord: saw "110 165 220"
-- ~trig: "x ~ x ~"     -- Freeze twice per cycle
-- ~freeze_chord: freeze ~chord ~trig * 0.3
-- out: ~freeze_chord

-- Example 3: Pattern-modulated freeze triggers
-- Using pattern transformations on the trigger

-- ~synth: sine "220 330 440 550"
-- ~trig_pattern: "x x ~ x" $ fast 2
-- ~spectral: freeze ~synth ~trig_pattern * 0.3
-- out: ~spectral

-- Example 4: Mix dry and frozen for hybrid sounds
-- Layer the original with the frozen spectrum

-- ~bass: saw 55
-- ~trig: "~ x ~ ~"
-- ~wet: freeze ~bass ~trig
-- ~mix: ~bass * 0.4 + ~wet * 0.6
-- out: ~mix

-- How it works:
-- 1. Input signal buffered in 2048-sample frames
-- 2. Apply Hann window for smooth transitions
-- 3. Perform FFT to convert to frequency domain
-- 4. On trigger: capture and store current spectrum
-- 5. Perform IFFT on frozen spectrum
-- 6. Overlap-add reconstruction (75% overlap)
--
-- Technical details:
-- - FFT size: 2048 samples (46ms @ 44.1kHz)
-- - Hop size: 512 samples (75% overlap)
-- - Window: Hann (minimizes spectral leakage)
-- - Phase: Preserved from capture moment
--
-- Sound Design Tips:
-- 1. Freeze works best on:
--    - Harmonic sounds (chords, pads)
--    - Rhythmic patterns (creates stutters)
--    - Evolving textures (captures moments)
--
-- 2. Trigger patterns:
--    - Slow triggers: smooth transitions
--    - Fast triggers: glitchy stutters
--    - Pattern-modulated: rhythmic variations
--
-- 3. Creative uses:
--    - Spectral holds: freeze pad chords
--    - Rhythmic freezes: drum stutters
--    - Hybrid sounds: mix dry + frozen
--    - Time stretching: freeze + slow trigger changes
--
-- Comparison with other effects:
-- - vs Delay: Freeze holds spectrum, not time-domain
-- - vs Reverb: Static hold, not decay
-- - vs Sample & Hold: Works on full spectrum
--
-- Future enhancements:
-- - Spectral gate (threshold-based)
-- - Spectral smear (blur across frequencies)
-- - Spectral morph (crossfade between captures)
-- - Variable FFT size parameter
