-- Formant Synthesis Demo
-- Creates vowel sounds by filtering a source through three resonant bandpass filters
-- Each vowel is characterized by specific formant frequencies (F1, F2, F3)

tempo: 2.0

-- SYNTAX: formant source f1 f2 f3 bw1 bw2 bw3
-- source: input signal to filter (sawtooth, pulse, or noise work well)
-- f1, f2, f3: formant center frequencies (Hz)
-- bw1, bw2, bw3: formant bandwidths (Hz) - narrower = more resonant

-- ========== VOWEL PRESETS (Male Voice) ==========

-- /a/ vowel (as in "father")
~source_a: saw 110
~vowel_a: formant ~source_a 730 1090 2440 80 90 120

-- /e/ vowel (as in "bet")
~source_e: saw 110
~vowel_e: formant ~source_e 530 1840 2480 80 90 120

-- /i/ vowel (as in "beet")
~source_i: saw 110
~vowel_i: formant ~source_i 270 2290 3010 60 90 150

-- /o/ vowel (as in "boat")
~source_o: saw 110
~vowel_o: formant ~source_o 570 840 2410 80 90 120

-- /u/ vowel (as in "boot")
~source_u: saw 110
~vowel_u: formant ~source_u 300 870 2240 60 70 100

-- ========== VOWEL MORPHING ==========

-- Morph between /a/ and /i/
~morph_source: saw 110
~f1_morph: "730 270"      -- /a/ to /i/ F1
~f2_morph: "1090 2290"    -- /a/ to /i/ F2
~f3_morph: "2440 3010"    -- /a/ to /i/ F3
~vowel_morph: formant ~morph_source ~f1_morph ~f2_morph ~f3_morph 80 90 120

-- ========== MELODIC VOWELS ==========

-- Singing melody with /e/ vowel
~melody: "110 165 220 165"
~melody_source: saw ~melody
~singing_e: formant ~melody_source 530 1840 2480 80 90 120

-- Bass line with /o/ vowel
~bass_notes: "55 55 82.5 110"
~bass_source: saw ~bass_notes
~singing_bass: formant ~bass_source 570 840 2410 80 90 120

-- High melody with /i/ vowel
~high_melody: "220 330 440 330"
~high_source: saw ~high_melody
~singing_i: formant ~high_source 270 2290 3010 60 90 150

-- ========== DIFFERENT SOURCES ==========

-- Pulse wave source (more buzzy, like vocal cords)
~pulse_source: square 110
~vowel_a_pulse: formant ~pulse_source 730 1090 2440 80 90 120

-- Noise source (whispered vowel)
~noise_source: noise
~whispered_a: formant ~noise_source 730 1090 2440 80 90 120

-- Triangle wave source (mellower)
~tri_source: tri 110
~vowel_a_tri: formant ~tri_source 730 1090 2440 80 90 120

-- ========== BANDWIDTH VARIATION ==========

-- Narrow bandwidth (more resonant, nasal)
~nasal_a: formant (saw 110) 730 1090 2440 30 40 50

-- Wide bandwidth (breathy, less defined)
~breathy_a: formant (saw 110) 730 1090 2440 150 200 250

-- ========== PATTERN MODULATION ==========

-- Modulate F1 for dynamic timbre
~dynamic_f1: formant (saw 220) "500 700 900 700" 1840 2480 80 90 120

-- Modulate bandwidth for articulation
~dynamic_bw: formant (saw 220) 730 1090 2440 "50 150 100" 90 120

-- Vowel sequence (/a/ /e/ /i/ /o/)
~vowel_seq_source: saw 110
~f1_seq: "730 530 270 570"      -- F1 for /a/ /e/ /i/ /o/
~f2_seq: "1090 1840 2290 840"   -- F2
~f3_seq: "2440 2480 3010 2410"  -- F3
~vowel_sequence: formant ~vowel_seq_source ~f1_seq ~f2_seq ~f3_seq 80 90 120

-- ========== CHOIR EFFECT ==========

-- Multiple voices with slightly different pitches
~choir_source1: saw 108
~choir_source2: saw 110
~choir_source3: saw 112
~choir1: formant ~choir_source1 730 1090 2440 80 90 120
~choir2: formant ~choir_source2 730 1090 2440 80 90 120
~choir3: formant ~choir_source3 730 1090 2440 80 90 120
~choir: (~choir1 + ~choir2 + ~choir3) * 0.3

-- ========== WITH EFFECTS ==========

-- Formant through reverb (cathedral vowel)
~cathedral: formant (saw 110) 530 1840 2480 80 90 120 # reverb 0.7 0.9 0.4

-- Formant through delay (echoing vowel)
~echoing: formant (saw 220) 270 2290 3010 60 90 150 # delay 0.25 0.4

-- Formant through chorus (thick vowel)
~thick: formant (saw 165) 570 840 2410 80 90 120 # chorus 3 0.8 0.2 0.5

-- ========== RHYTHMIC PATTERNS ==========

-- Pulsing vowel
~pulsing_source: saw "110 ~ 110 ~"
~pulsing_vowel: formant ~pulsing_source 730 1090 2440 80 90 120

-- Fast vowel sequence
~fast_vowel: formant (saw "110 165 220 165") "730 530 270 530" "1090 1840 2290 1840" "2440 2480 3010 2480" 80 90 120 $ fast 2

-- ========== OUTPUT ==========

-- Choose your sound!
out: ~vowel_a * 0.3

-- Try these variations:
-- out: ~vowel_a * 0.3                              -- /a/ vowel
-- out: ~vowel_e * 0.3                              -- /e/ vowel
-- out: ~vowel_i * 0.3                              -- /i/ vowel
-- out: ~vowel_o * 0.3                              -- /o/ vowel
-- out: ~vowel_u * 0.3                              -- /u/ vowel
-- out: ~vowel_morph * 0.3                          -- Morph /a/ to /i/
-- out: ~singing_e * 0.3                            -- Singing /e/
-- out: ~singing_bass * 0.3                         -- Bass /o/
-- out: ~singing_i * 0.3                            -- High /i/
-- out: ~vowel_a_pulse * 0.3                        -- Pulse source
-- out: ~whispered_a * 0.15                         -- Whispered
-- out: ~vowel_a_tri * 0.3                          -- Triangle source
-- out: ~nasal_a * 0.3                              -- Nasal (narrow)
-- out: ~breathy_a * 0.3                            -- Breathy (wide)
-- out: ~dynamic_f1 * 0.3                           -- Dynamic F1
-- out: ~dynamic_bw * 0.3                           -- Dynamic bandwidth
-- out: ~vowel_sequence * 0.3                       -- Vowel sequence
-- out: ~choir * 0.4                                -- Choir effect
-- out: ~cathedral * 0.2                            -- With reverb
-- out: ~echoing * 0.3                              -- With delay
-- out: ~thick * 0.3                                -- With chorus
-- out: ~pulsing_vowel * 0.3                        -- Pulsing
-- out: ~fast_vowel * 0.3                           -- Fast sequence

-- ========== CREATIVE TIPS ==========

-- FORMANT FREQUENCIES (F1, F2, F3):
--   - F1: Jaw opening (lower freq = more closed)
--   - F2: Tongue position (varies most between vowels)
--   - F3: Lip rounding (higher for /i/, lower for /u/)
--   - Try values between 200-3000 Hz for each formant
--   - Spacing between formants affects vowel quality

-- BANDWIDTH (BW1, BW2, BW3):
--   - Narrow (20-50 Hz): Resonant, nasal, clear vowel
--   - Medium (60-100 Hz): Natural, realistic vowel
--   - Wide (150-300 Hz): Breathy, whispered, fuzzy vowel
--   - Higher formants typically have wider bandwidths

-- SOURCE SIGNAL:
--   - Sawtooth: Rich harmonics, bright vocal tone
--   - Pulse/Square: Buzzy, nasal, vocal cord-like
--   - Triangle: Mellower, less harmonic content
--   - Noise: Whispered, breathysp consonants
--   - Lower pitch sources (55-220 Hz) sound more natural

-- VOWEL CHART (Male Voice, Hz):
--   /i/ (beet):   F1=270,  F2=2290, F3=3010
--   /ɪ/ (bit):    F1=390,  F2=1990, F3=2550
--   /e/ (bet):    F1=530,  F2=1840, F3=2480
--   /æ/ (bat):    F1=660,  F2=1720, F3=2410
--   /ɑ/ (father): F1=730,  F2=1090, F3=2440
--   /ɔ/ (bought): F1=570,  F2=840,  F3=2410
--   /o/ (boat):   F1=450,  F2=880,  F3=2240
--   /u/ (boot):   F1=300,  F2=870,  F3=2240
--   /ʌ/ (but):    F1=640,  F2=1190, F3=2390

-- FEMALE VOICE (multiply F1, F2, F3 by ~1.2):
--   /a/ (father): F1=850,  F2=1220, F3=2810

-- ADVANCED TECHNIQUES:
--   - Morph between vowels with pattern modulation
--   - Create diphthongs (two vowels in sequence)
--   - Layer multiple formant filters for complex timbres
--   - Modulate F1 for vibrato-like effects
--   - Combine with envelopes for speech-like articulation
--   - Use rhythmic patterns for vocal percussion
--   - Add reverb/delay for spatial vocal effects
--   - Create alien/robotic voices with unusual formant ratios
