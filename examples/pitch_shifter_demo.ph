-- Pitch Shifter Demo
-- Changes pitch without changing duration using granular synthesis
-- Classic use: Harmonizers, vocal correction, creative sound design

tempo: 0.5

-- SYNTAX: pitch_shift input semitones
-- input: signal to pitch shift
-- semitones: pitch shift amount (+12 = octave up, -12 = octave down)

-- ========== BASIC PITCH SHIFTING ==========

-- No shift (0 semitones = original pitch)
~original: saw 220
~no_shift: pitch_shift ~original 0

-- Octave up (+12 semitones)
~octave_up: pitch_shift (saw 220) 12

-- Octave down (-12 semitones)
~octave_down: pitch_shift (saw 220) -12

-- Perfect fifth up (+7 semitones)
~fifth_up: pitch_shift (saw 220) 7

-- Perfect fourth up (+5 semitones)
~fourth_up: pitch_shift (saw 220) 5

-- Minor third up (+3 semitones)
~third_up: pitch_shift (saw 220) 3

-- ========== HARMONIZER EFFECTS ==========

-- Simple harmonizer (mix original + shifted)
~voice: saw 220
~harmony: pitch_shift ~voice 7
~harmonizer: (~voice + ~harmony) * 0.5

-- Thick harmonizer (octave + fifth)
~thick1: saw 220
~thick2: pitch_shift ~thick1 7   -- Fifth
~thick3: pitch_shift ~thick1 12  -- Octave
~thick_harmony: (~thick1 + ~thick2 + ~thick3) * 0.33

-- Shimmer (octave up + reverb-like)
~shimmer_in: saw 220
~shimmer_shift: pitch_shift ~shimmer_in 12
~shimmer: ~shimmer_shift * 0.7

-- ========== CHORD GENERATION ==========

-- Major chord from single note (root, major third, perfect fifth)
~chord_root: saw 220
~chord_third: pitch_shift ~chord_root 4   -- Major third
~chord_fifth: pitch_shift ~chord_root 7   -- Perfect fifth
~major_chord: (~chord_root + ~chord_third + ~chord_fifth) * 0.33

-- Minor chord (root, minor third, perfect fifth)
~minor_root: saw 220
~minor_third: pitch_shift ~minor_root 3   -- Minor third
~minor_fifth: pitch_shift ~minor_root 7   -- Perfect fifth
~minor_chord: (~minor_root + ~minor_third + ~minor_fifth) * 0.33

-- 7th chord (root, third, fifth, seventh)
~seventh_root: saw 220
~seventh_third: pitch_shift ~seventh_root 4
~seventh_fifth: pitch_shift ~seventh_root 7
~seventh_seventh: pitch_shift ~seventh_root 11
~seventh_chord: (~seventh_root + ~seventh_third + ~seventh_fifth + ~seventh_seventh) * 0.25

-- ========== PATTERN-MODULATED PITCH SHIFTING ==========

-- Arpeggiator (pitch shifts over time)
~arp_source: saw 220
~arp_pattern: "0 7 12 7"
~arpeggiator: pitch_shift ~arp_source ~arp_pattern

-- Melody shifter (transpose entire melody)
~melody: saw "220 165 275 220"
~melody_shifted: pitch_shift ~melody 7  -- Transpose up a fifth

-- Glitch effect (random-ish pitch jumps)
~glitch_source: saw 220
~glitch_pattern: "0 12 -12 7"
~glitch: pitch_shift ~glitch_source ~glitch_pattern

-- ========== DIFFERENT SOURCES ==========

-- Square wave harmonizer
~square_harm: pitch_shift (square 220) 7

-- Triangle wave thick sound
~tri_source: tri 220
~tri_oct: pitch_shift ~tri_source 12
~tri_thick: (~tri_source + ~tri_oct) * 0.5

-- Noise pitched (weird but interesting)
~noise_pitched: pitch_shift noise 0

-- ========== CREATIVE EFFECTS ==========

-- Detune effect (slight pitch variation for thickness)
~detune1: pitch_shift (saw 220) 0.1    -- Slight sharp
~detune2: pitch_shift (saw 220) -0.1   -- Slight flat
~detuned: (~detune1 + ~detune2) * 0.5

-- Octave doubling (common in synths)
~synth: saw 220
~synth_octave: pitch_shift ~synth -12
~octave_doubled: (~synth + ~synth_octave * 0.5) * 0.66

-- Formant-like (stack multiple shifts)
~formant_source: saw 110
~formant1: pitch_shift ~formant_source 0
~formant2: pitch_shift ~formant_source 7
~formant3: pitch_shift ~formant_source 12
~formant_stack: (~formant1 + ~formant2 + ~formant3) * 0.33

-- Extreme shifts
~extreme_up: pitch_shift (saw 220) 24    -- 2 octaves up
~extreme_down: pitch_shift (saw 220) -24 -- 2 octaves down

-- ========== THROUGH EFFECTS ==========

-- Pitch shift then filter
~shifted_filtered: pitch_shift (saw 220) 12 # lpf 2000 0.8

-- Pitch shift then reverb
~shifted_reverb: pitch_shift (saw 220) 7 # reverb 0.5 0.8 0.3

-- Pitch shift then distortion (octave fuzz)
~octave_fuzz: pitch_shift (saw 110) 12 # distort 3.0

-- ========== MUSICAL PATTERNS ==========

-- Melody with harmony
~lead: saw "220 275 330 275"
~lead_harmony: pitch_shift ~lead 7
~lead_mix: (~lead + ~lead_harmony * 0.7) * 0.6

-- Bass with sub-octave
~bass: saw "55 55 82.5 110"
~bass_sub: pitch_shift ~bass -12
~bass_thick: (~bass + ~bass_sub * 0.5) * 0.75

-- Rhythmic pitch shifting
~rhythm_source: square 220
~rhythm_shifts: "0 ~ 12 ~"
~rhythmic_shift: pitch_shift ~rhythm_source ~rhythm_shifts

-- ========== OUTPUT ==========

-- Choose your sound!
out: ~harmonizer * 0.3

-- Try these variations:
-- out: ~no_shift * 0.3                   -- No shift (reference)
-- out: ~octave_up * 0.3                  -- Octave up
-- out: ~octave_down * 0.3                -- Octave down
-- out: ~fifth_up * 0.3                   -- Perfect fifth
-- out: ~harmonizer * 0.3                 -- Simple harmonizer
-- out: ~thick_harmony * 0.3              -- Thick harmonizer
-- out: ~shimmer * 0.3                    -- Shimmer effect
-- out: ~major_chord * 0.3                -- Major chord
-- out: ~minor_chord * 0.3                -- Minor chord
-- out: ~seventh_chord * 0.25             -- 7th chord
-- out: ~arpeggiator * 0.3                -- Arpeggiator
-- out: ~melody_shifted * 0.3             -- Transposed melody
-- out: ~glitch * 0.3                     -- Glitch effect
-- out: ~detuned * 0.3                    -- Detune chorus
-- out: ~octave_doubled * 0.3             -- Octave doubling
-- out: ~formant_stack * 0.3              -- Formant-like
-- out: ~extreme_up * 0.3                 -- 2 octaves up
-- out: ~extreme_down * 0.3               -- 2 octaves down
-- out: ~shifted_filtered * 0.3           -- Through filter
-- out: ~shifted_reverb * 0.2             -- Through reverb
-- out: ~octave_fuzz * 0.2                -- Octave fuzz
-- out: ~lead_mix * 0.3                   -- Melody + harmony
-- out: ~bass_thick * 0.3                 -- Thick bass
-- out: ~rhythmic_shift * 0.3             -- Rhythmic shifting

-- ========== CREATIVE TIPS ==========

-- MUSICAL INTERVALS (semitones):
--   0:  Unison (no change)
--   1:  Minor second (half step)
--   2:  Major second (whole step)
--   3:  Minor third
--   4:  Major third
--   5:  Perfect fourth
--   7:  Perfect fifth (most common harmony)
--   12: Octave
--   -12: Octave down

-- HARMONIZER APPLICATIONS:
--   - Vocal doubling (±5-7 semitones)
--   - Guitar harmonies (3rd, 5th, octave)
--   - Thick synth sounds (octave + fifth)
--   - Shimmer reverb (octave up with feedback)

-- CREATIVE USES:
--   - Arpeggiators: pattern-modulate semitones
--   - Chord stacking: layer multiple shifts
--   - Detune: very small shifts (±0.1 to ±0.5)
--   - Formants: stack octaves (0, 12, 19, 24)
--   - Bass thickness: add -12 semitones
--   - Extreme: ±24 or more for special effects

-- QUALITY NOTES:
--   - Granular-based pitch shifting (50ms grains)
--   - Works best with harmonic sources (saw, square, triangle)
--   - May have artifacts with noise or complex signals
--   - Larger pitch shifts = more artifacts
--   - Small shifts (±7 semitones) sound most natural

-- THEORY:
--   - Semitone to frequency ratio: 2^(semitones/12)
--   - Uses two overlapping grains with Hann windows
--   - Grains read at shifted rate, play at normal rate
--   - Maintains duration while changing pitch
--   - Latency: ~50ms (one grain size)
