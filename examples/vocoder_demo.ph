-- Vocoder Demo
-- Analyzes modulator amplitude envelope in frequency bands and applies to carrier
-- Classic use: Robot voice effect (voice modulating synth)

tempo: 2.0

-- SYNTAX: vocoder modulator carrier num_bands
-- modulator: signal to analyze (usually voice/rhythmic)
-- carrier: signal to shape (usually synth/rich harmonics)
-- num_bands: number of frequency bands (2-32, default 8)

-- ========== BASIC VOCODER ==========

-- Simple vocoder (8 bands)
~modulator1: saw 110
~carrier1: saw 220
~basic: vocoder ~modulator1 ~carrier1 8

-- 16-band vocoder (higher resolution)
~modulator2: saw 110
~carrier2: saw 220
~hires: vocoder ~modulator2 ~carrier2 16

-- 4-band vocoder (lo-fi robot effect)
~modulator3: saw 110
~carrier3: saw 220
~lofi: vocoder ~modulator3 ~carrier3 4

-- ========== CLASSIC ROBOT VOICE ==========

-- Robot voice (saw modulating saw)
~voice: saw 110
~synth: saw 220
~robot: vocoder ~voice ~synth 8

-- Robot with detuned carrier (thicker)
~robot_carrier1: saw 218
~robot_carrier2: saw 222
~robot_carrier: (~robot_carrier1 + ~robot_carrier2) * 0.5
~thick_robot: vocoder ~voice ~robot_carrier 8

-- Robot with chord carrier
~chord1: saw 220
~chord2: saw 275
~chord3: saw 330
~chord_carrier: (~chord1 + ~chord2 + ~chord3) * 0.33
~chord_robot: vocoder ~voice ~chord_carrier 8

-- ========== DIFFERENT MODULATORS ==========

-- Square wave modulator (different rhythm)
~square_mod: square 110
~square_robot: vocoder ~square_mod (saw 220) 8

-- Triangle modulator (mellower)
~tri_mod: tri 110
~tri_robot: vocoder ~tri_mod (saw 220) 8

-- Pattern-modulated modulator (melody)
~melody_mod: saw "110 165 220 165"
~melody_robot: vocoder ~melody_mod (saw 440) 8

-- Bass modulator (low frequencies)
~bass_mod: saw "55 55 82.5 110"
~bass_robot: vocoder ~bass_mod (saw 220) 8

-- ========== DIFFERENT CARRIERS ==========

-- Noise carrier (whisper effect)
~noise_carrier: noise
~whisper: vocoder (saw 110) ~noise_carrier 16

-- Multiple oscillator carrier (rich harmonics)
~rich_carrier1: saw 220
~rich_carrier2: saw 330
~rich_carrier3: saw 440
~rich_carrier: (~rich_carrier1 + ~rich_carrier2 + ~rich_carrier3) * 0.33
~rich_robot: vocoder (saw 110) ~rich_carrier 8

-- High frequency carrier (bright robot)
~high_carrier: saw 880
~bright_robot: vocoder (saw 110) ~high_carrier 8

-- Pattern-modulated carrier (changing timbre)
~pattern_carrier: saw "220 330 440 330"
~morphing_robot: vocoder (saw 110) ~pattern_carrier 8

-- ========== MUSICAL PATTERNS ==========

-- Melody on both modulator and carrier
~melody1: saw "110 165 220 165 110"
~melody2: saw "220 330 440 330 220"
~dual_melody: vocoder ~melody1 ~melody2 8

-- Rhythmic modulator (pulsing)
~rhythmic_mod: saw "110 ~ 110 ~"
~rhythmic_robot: vocoder ~rhythmic_mod (saw 220) 8

-- Fast changing modulator
~fast_mod: saw "110 165 220" $ fast 2
~fast_robot: vocoder ~fast_mod (saw 440) 8

-- Bass line vocoder
~bass_line: saw "55 55 82.5 110"
~bass_carrier: saw 110
~bass_vocoder: vocoder ~bass_line ~bass_carrier 8

-- ========== BAND COUNT COMPARISON ==========

-- 4 bands (lo-fi, retro)
~bands4: vocoder (saw 110) (saw 220) 4

-- 8 bands (classic vocoder sound)
~bands8: vocoder (saw 110) (saw 220) 8

-- 16 bands (high resolution)
~bands16: vocoder (saw 110) (saw 220) 16

-- 32 bands (maximum resolution)
~bands32: vocoder (saw 110) (saw 220) 32

-- ========== THROUGH EFFECTS ==========

-- Vocoder through reverb (spacey robot)
~space_robot: vocoder (saw 110) (saw 220) 8 # reverb 0.5 0.8 0.3

-- Vocoder through delay (echoing robot)
~echo_robot: vocoder (saw 110) (saw 220) 8 # delay 0.25 0.4

-- Vocoder through lowpass filter (muffled robot)
~muffled_robot: vocoder (saw 110) (saw 220) 8 # lpf 2000 0.8

-- Vocoder through distortion (aggressive robot)
~distorted_robot: vocoder (saw 110) (saw 220) 8 # distort 2.0

-- Vocoder through chorus (thick robot)
~chorus_robot: vocoder (saw 110) (saw 220) 8 # chorus 3 0.8 0.2 0.5

-- ========== COMPLEX PATCHES ==========

-- Dual vocoder (parallel processing)
~vocoder1: vocoder (saw 110) (saw 220) 8
~vocoder2: vocoder (saw 165) (saw 330) 8
~dual_vocoder: (~vocoder1 + ~vocoder2) * 0.5

-- Vocoder feedback (modulator through vocoder)
~feedback_mod: saw 110
~feedback_carrier: saw 220
~feedback1: vocoder ~feedback_mod ~feedback_carrier 8
-- Note: True feedback requires delay to prevent infinite loop

-- Vocoder with LFO-modulated carrier
~lfo: sine 0.5
~lfo_carrier_freq: ~lfo * 100 + 320
~lfo_carrier: saw ~lfo_carrier_freq
~lfo_vocoder: vocoder (saw 110) ~lfo_carrier 8

-- Vocoder with filtered modulator
~filtered_mod: saw 110 # lpf 500 0.8
~filtered_vocoder: vocoder ~filtered_mod (saw 220) 8

-- Vocoder with filtered carrier
~filtered_carrier: saw 220 # lpf 1000 0.8
~carrier_filtered_vocoder: vocoder (saw 110) ~filtered_carrier 8

-- ========== STEREO EFFECTS ==========

-- Panned vocoders (stereo field)
~left_vocoder: vocoder (saw 108) (saw 216) 8
~right_vocoder: vocoder (saw 112) (saw 224) 8
~stereo_vocoder: (~left_vocoder + ~right_vocoder) * 0.5

-- Detuned vocoders (chorus-like)
~detune1: vocoder (saw 109) (saw 218) 8
~detune2: vocoder (saw 110) (saw 220) 8
~detune3: vocoder (saw 111) (saw 222) 8
~detuned_vocoder: (~detune1 + ~detune2 + ~detune3) * 0.33

-- ========== EXPERIMENTAL ==========

-- Vocoder with sine carriers (clean)
~sine_carrier: sine 220
~sine_vocoder: vocoder (saw 110) ~sine_carrier 8

-- Vocoder with triangle carriers (warm)
~tri_carrier: tri 220
~tri_vocoder: vocoder (saw 110) ~tri_carrier 8

-- Extreme band counts
~minimal_vocoder: vocoder (saw 110) (saw 220) 2   -- Just 2 bands
~ultra_vocoder: vocoder (saw 110) (saw 220) 32    -- Maximum 32 bands

-- Modulator and carrier same frequency (ring mod effect)
~same_freq: vocoder (saw 220) (saw 220) 8

-- Very high modulator (unusual timbre)
~high_mod: saw 880
~high_mod_vocoder: vocoder ~high_mod (saw 220) 8

-- Very low modulator (sub bass)
~sub_mod: saw 55
~sub_vocoder: vocoder ~sub_mod (saw 220) 8

-- ========== OUTPUT ==========

-- Choose your sound!
out: ~basic * 0.3

-- Try these variations:
-- out: ~basic * 0.3                              -- Basic 8-band
-- out: ~hires * 0.3                              -- 16-band hi-res
-- out: ~lofi * 0.3                               -- 4-band lo-fi
-- out: ~robot * 0.3                              -- Classic robot
-- out: ~thick_robot * 0.3                        -- Detuned carrier
-- out: ~chord_robot * 0.3                        -- Chord carrier
-- out: ~square_robot * 0.3                       -- Square modulator
-- out: ~tri_robot * 0.3                          -- Triangle modulator
-- out: ~melody_robot * 0.3                       -- Melody modulator
-- out: ~bass_robot * 0.3                         -- Bass modulator
-- out: ~whisper * 0.15                           -- Noise carrier
-- out: ~rich_robot * 0.3                         -- Rich harmonics
-- out: ~bright_robot * 0.3                       -- High carrier
-- out: ~morphing_robot * 0.3                     -- Pattern carrier
-- out: ~dual_melody * 0.3                        -- Dual melody
-- out: ~rhythmic_robot * 0.3                     -- Pulsing
-- out: ~fast_robot * 0.3                         -- Fast changing
-- out: ~bass_vocoder * 0.3                       -- Bass line
-- out: ~bands4 * 0.3                             -- 4 bands
-- out: ~bands8 * 0.3                             -- 8 bands
-- out: ~bands16 * 0.3                            -- 16 bands
-- out: ~bands32 * 0.3                            -- 32 bands
-- out: ~space_robot * 0.2                        -- With reverb
-- out: ~echo_robot * 0.3                         -- With delay
-- out: ~muffled_robot * 0.3                      -- With lowpass
-- out: ~distorted_robot * 0.25                   -- With distortion
-- out: ~chorus_robot * 0.3                       -- With chorus
-- out: ~dual_vocoder * 0.3                       -- Dual parallel
-- out: ~lfo_vocoder * 0.3                        -- LFO modulated
-- out: ~filtered_vocoder * 0.3                   -- Filtered modulator
-- out: ~carrier_filtered_vocoder * 0.3           -- Filtered carrier
-- out: ~stereo_vocoder * 0.3                     -- Stereo field
-- out: ~detuned_vocoder * 0.3                    -- Detuned chorus
-- out: ~sine_vocoder * 0.3                       -- Sine carrier
-- out: ~tri_vocoder * 0.3                        -- Triangle carrier
-- out: ~minimal_vocoder * 0.3                    -- 2 bands
-- out: ~ultra_vocoder * 0.3                      -- 32 bands
-- out: ~same_freq * 0.3                          -- Same frequency
-- out: ~high_mod_vocoder * 0.3                   -- High modulator
-- out: ~sub_vocoder * 0.3                        -- Sub bass

-- ========== CREATIVE TIPS ==========

-- MODULATOR CHOICE:
--   - Saw/Square: Strong harmonics, clear modulation
--   - Triangle: Mellower, less aggressive
--   - Low frequencies (55-110 Hz): Deep, bassy robot
--   - High frequencies (440-880 Hz): Bright, thin robot
--   - Rhythmic patterns: Create pulsing, talking effects

-- CARRIER CHOICE:
--   - Saw/Square: Rich harmonics, classic robot sound
--   - Noise: Whisper/breathiness, less tonal
--   - Chords: Harmonic richness, fuller sound
--   - High frequencies: Brighter, more articulate
--   - Pattern-modulated: Changing timbre over time

-- BAND COUNT:
--   - 2-4 bands: Lo-fi, retro, chunky
--   - 8 bands: Classic vocoder sound, balanced
--   - 16-32 bands: High resolution, more natural
--   - More bands = clearer articulation, less robotic
--   - Fewer bands = more robotic, characterful

-- FREQUENCY RANGES:
--   - Vocoder uses logarithmic spacing: 100Hz to 10kHz
--   - Lower bands (100-500 Hz): Warmth, body
--   - Mid bands (500-2kHz): Vowel formants, clarity
--   - Upper bands (2-10kHz): Consonants, brilliance
--   - Match carrier and modulator ranges for best effect

-- CLASSIC VOCODER APPLICATIONS:
--   - Robot/android voices (saw modulating saw)
--   - Whisper effects (voice modulating noise)
--   - Talking instruments (melody modulating synth)
--   - Rhythmic synthesis (drums modulating chords)
--   - Retro sci-fi sounds (lo-fi 4-8 bands)
--   - Modern electronic music (16-32 bands)

-- ADVANCED TECHNIQUES:
--   - Layer multiple vocoders with different band counts
--   - Modulate carrier frequency with LFOs
--   - Use filtered signals for modulator/carrier
--   - Combine with effects (reverb, delay, chorus)
--   - Create stereo field with detuned vocoders
--   - Experiment with same frequency mod/carrier (ring mod)
--   - Try extreme band counts (2 or 32) for special effects

-- THEORY:
--   - Vocoder = VOice CODER (originally for speech compression)
--   - Invented in 1928 by Homer Dudley at Bell Labs
--   - Analyzes modulator into frequency bands
--   - Measures amplitude envelope in each band
--   - Applies those envelopes to carrier bands
--   - Envelope follower: smoothed amplitude detector
--   - Fast attack (0.01) + slow release (0.1) = natural articulation
--   - Logarithmic band spacing matches human hearing
--   - Used in: Kraftwerk, Daft Punk, EDM, film sound design
