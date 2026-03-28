-- Rhythm Test - Pattern-based rhythmic sounds
-- Render with: phonon render rhythm_test.phonon rhythm_test.wav --duration 4

-- Rhythmic bass with rests (~)
rhythmic_bass = saw "55 ~ 55 55 ~ 82.5 55 ~" # lpf("500 1000 2000 1000", 4)

-- Percussive blips using square wave
blips = square "880 ~ ~ 880 ~ 1320 ~ 880" # lpf 3000 5

-- Filtered noise bursts for texture
texture = noise # lpf("100 ~ 5000 ~ 100 ~ 2000 ~", 10)

-- Sine wave sub hits
sub_hits = sine "27.5 ~ ~ ~ 27.5 ~ ~ ~"

-- Output - try each line separately
out rhythmic_bass * 0.25
-- out blips * 0.1
-- out texture * 0.15
-- out sub_hits * 0.3