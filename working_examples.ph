-- Working Examples for Phonon Render
-- Test different synthesis techniques
-- Usage: phonon render working_examples.phonon output.wav --duration 4

-- ==================================================
-- WORKING EXAMPLES - Uncomment one line at a time
-- ==================================================

-- --- MELODIC SOUNDS ---

-- Simple melody with sine waves
out sine "440 550 660 550" * 0.2

-- Bass line with saw wave
-- out saw "55 82.5 110 55" # lpf 1000 3 * 0.25

-- Square wave chiptune-style melody
-- out square "220 ~ 330 ~ 440 330 220 ~" # lpf 2000 2 * 0.15

-- --- BASS SOUNDS ---

-- Sub bass drone
-- out sine 27.5 * 0.4

-- Wobble bass (fast filter modulation)
-- out saw 55 # lpf("200 2000", 5) * 0.3

-- Acid bass (high resonance filter sweep)
-- out saw 55 # lpf("100 200 400 800 1600 800 400 200", 8) * 0.2

-- --- PERCUSSIVE SOUNDS ---

-- Kick drum (low-passed noise burst)
-- out noise # lpf("100 50", 20) * 0.3

-- Hi-hat (high-passed noise)
-- out noise # hpf 8000 10 * 0.15

-- Snare (band-passed noise)
-- out noise # hpf 200 5 # lpf 5000 5 * 0.2

-- --- SPECIAL EFFECTS ---

-- Filter sweep on saw wave
-- out saw 110 # lpf("100 500 1000 2000 4000 8000 4000 2000 1000 500", 3) * 0.2

-- Ring mod effect (using very high frequency patterns)
-- out sine "55 55 82.5 82.5" # lpf("500 1000 2000 1000", 4) * 0.25

-- White noise texture with rhythmic filtering
-- out noise # lpf("100 ~ 5000 ~ 100 ~ 2000 ~", 10) * 0.15

-- --- HARMONIC SOUNDS ---

-- Fifth harmony
-- out sine "220 330" * 0.2

-- Octave jumping
-- out saw "55 110 55 110" # lpf 2000 2 * 0.2

-- Detuned unison (sounds like one note but richer)
-- Note: You'd need multiple oscillators for true detuning
-- This approximates it with fast alternation
-- out saw "110 110.5 110 109.5" # lpf 1500 2 * 0.2