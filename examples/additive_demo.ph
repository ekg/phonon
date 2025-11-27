-- Additive Synthesis Demo
-- Creates complex timbres by summing multiple sine wave partials (harmonics)
-- Each partial is a harmonic (integer multiple of fundamental) with independent amplitude

tempo: 0.5

-- SYNTAX: additive frequency amplitudes
-- frequency: fundamental frequency in Hz (pattern-modulatable)
-- amplitudes: space-separated amplitude values for each partial
--   Partial 1 = fundamental
--   Partial 2 = 2× fundamental (1st overtone)
--   Partial 3 = 3× fundamental (2nd overtone), etc.

-- ========== BASIC ADDITIVE ==========

-- Single partial (pure sine wave)
~single: additive 440 "1.0"

-- Two partials (fundamental + octave)
~two: additive 440 "1.0 0.5"

-- Three partials (fundamental + harmonics)
~three: additive 440 "1.0 0.5 0.25"

-- Full harmonic series (organ-like)
~harmonic_series: additive 110 "1.0 0.5 0.33 0.25 0.2 0.17 0.14 0.13"

-- ========== CLASSIC WAVEFORM APPROXIMATIONS ==========

-- Sawtooth approximation (all harmonics, 1/n amplitude)
~saw_approx: additive 220 "1.0 0.5 0.33 0.25 0.2 0.17 0.14 0.13"

-- Square wave approximation (odd harmonics only, 1/n amplitude)
~square_approx: additive 220 "1.0 0.0 0.33 0.0 0.2 0.0 0.14 0.0"

-- Triangle approximation (odd harmonics, 1/n² amplitude)
~triangle_approx: additive 220 "1.0 0.0 0.11 0.0 0.04 0.0 0.02 0.0"

-- ========== INHARMONIC SPECTRA ==========

-- Strong fundamental, weak upper partials (flute-like)
~flute_like: additive 440 "1.0 0.1 0.05 0.02"

-- Strong upper partials (bright, bell-like)
~bright: additive 220 "0.5 1.0 0.8 0.6 0.4"

-- Evenly weighted partials (organ-like)
~organ: additive 110 "1.0 1.0 1.0 1.0 1.0"

-- Decreasing exponentially (natural decay)
~natural: additive 220 "1.0 0.7 0.5 0.35 0.25 0.17 0.12 0.08"

-- ========== MELODIC PATTERNS ==========

-- Melody with harmonic series
~melody: additive "220 330 440 330 220" "1.0 0.5 0.33 0.25"

-- Bass line with rich harmonics
~bass: additive "55 55 82.5 110" "1.0 0.7 0.5 0.35 0.25"

-- Lead melody with bright timbre
~lead: additive "440 550 660 550 440 330 440" "1.0 0.8 0.6 0.4"

-- Arpeggio
~arp: additive "220 330 440" "1.0 0.5 0.25" $ fast 2

-- ========== RHYTHMIC PATTERNS ==========

-- Pulsing additive
~pulsing: additive "220 ~ 220 ~" "1.0 0.5 0.33"

-- Chord (multiple voices)
~chord1: additive 220 "1.0 0.5 0.33"  -- Root (A)
~chord2: additive 275 "1.0 0.5 0.33"  -- Major third (C#)
~chord3: additive 330 "1.0 0.5 0.33"  -- Perfect fifth (E)
~chord: (~chord1 + ~chord2 + ~chord3) * 0.3

-- ========== SPECTRAL MORPHING ==========

-- Morph from sine to square by adding odd harmonics
~morph_source: additive 220
~morph_amps: "1.0" "1.0 0.0 0.33" "1.0 0.0 0.33 0.0 0.2"
-- Note: Currently amplitudes are fixed, but this shows the concept

-- ========== TIMBRE VARIATIONS ==========

-- Hollow (missing even harmonics)
~hollow: additive 220 "1.0 0.0 0.5 0.0 0.25 0.0 0.13"

-- Nasal (strong 2nd-4th harmonics)
~nasal: additive 220 "1.0 1.2 1.0 0.8 0.4 0.2"

-- Dark (fundamental dominant)
~dark: additive 220 "1.0 0.2 0.1 0.05 0.02"

-- Bright (upper partials emphasized)
~bright_high: additive 220 "0.3 0.5 0.7 0.9 1.0 0.8 0.6"

-- ========== BELL-LIKE TONES ==========

-- Bell (inharmonic-ish with strong upper partials)
~bell: additive 220 "1.0 0.8 1.2 0.6 0.9 0.4 0.5"

-- Glockenspiel
~glock: additive 880 "1.0 0.6 0.4 0.2 0.1"

-- ========== THROUGH EFFECTS ==========

-- Additive through reverb
~additive_reverb: additive 220 "1.0 0.5 0.33 0.25" # reverb 0.5 0.8 0.3

-- Additive through delay
~additive_delay: additive 330 "1.0 0.6 0.4 0.2" # delay 0.25 0.4

-- Additive through lowpass filter
~additive_filtered: additive 110 "1.0 0.5 0.33 0.25 0.2 0.17" # lpf 2000 0.8

-- Additive through chorus
~additive_chorus: additive 220 "1.0 0.5 0.33" # chorus 3 0.8 0.2 0.5

-- ========== MULTI-LAYERED ==========

-- Layer multiple additive synths
~layer1: additive 220 "1.0 0.5 0.33"
~layer2: additive 222 "1.0 0.0 0.5"  -- Slight detune + different spectrum
~layered: (~layer1 + ~layer2) * 0.5

-- Pad sound (multiple octaves)
~pad_low: additive 55 "1.0 0.5 0.33"
~pad_mid: additive 110 "1.0 0.5 0.33"
~pad_high: additive 220 "1.0 0.5 0.33"
~pad: (~pad_low + ~pad_mid + ~pad_high) * 0.25

-- ========== OUTPUT ==========

-- Choose your sound!
out: ~harmonic_series * 0.3

-- Try these variations:
-- out: ~single * 0.5                            -- Pure sine
-- out: ~two * 0.4                                -- Octaves
-- out: ~three * 0.4                              -- Basic harmonics
-- out: ~harmonic_series * 0.3                    -- Full series
-- out: ~saw_approx * 0.3                         -- Sawtooth-like
-- out: ~square_approx * 0.3                      -- Square-like
-- out: ~triangle_approx * 0.3                    -- Triangle-like
-- out: ~flute_like * 0.4                         -- Flute-like
-- out: ~bright * 0.3                             -- Bright tone
-- out: ~organ * 0.2                              -- Organ-like
-- out: ~natural * 0.3                            -- Natural decay
-- out: ~melody * 0.3                             -- Melody
-- out: ~bass * 0.4                               -- Bass line
-- out: ~lead * 0.3                               -- Lead melody
-- out: ~arp * 0.3                                -- Arpeggio
-- out: ~pulsing * 0.4                            -- Pulsing
-- out: ~chord * 0.4                              -- Chord
-- out: ~hollow * 0.3                             -- Hollow
-- out: ~nasal * 0.25                             -- Nasal
-- out: ~dark * 0.4                               -- Dark
-- out: ~bright_high * 0.3                        -- Bright high
-- out: ~bell * 0.3                               -- Bell-like
-- out: ~glock * 0.3                              -- Glockenspiel
-- out: ~additive_reverb * 0.2                    -- With reverb
-- out: ~additive_delay * 0.3                     -- With delay
-- out: ~additive_filtered * 0.3                  -- Filtered
-- out: ~additive_chorus * 0.3                    -- With chorus
-- out: ~layered * 0.3                            -- Layered
-- out: ~pad * 0.5                                -- Pad sound

-- ========== CREATIVE TIPS ==========

-- HARMONIC SERIES:
--   - Full series (1.0 0.5 0.33 0.25...): Bright, rich, sawtooth-like
--   - Odd only (1.0 0.0 0.33 0.0 0.2...): Square wave-like, hollow
--   - Even only (0.0 1.0 0.0 0.5 0.0...): Octave-heavy, electronic
--   - Few partials (1.0 0.5 0.25): Clear, simple, flute-like

-- AMPLITUDE ENVELOPES:
--   - Decreasing (1.0 0.5 0.33...): Natural, acoustic
--   - Increasing (0.25 0.5 0.75 1.0): Bright, synthetic
--   - Random (0.8 0.3 0.6 0.2): Metallic, inharmonic
--   - Strong fundamental (1.0 0.1 0.1...): Dark, woody, flute-like
--   - Weak fundamental (0.5 1.0 0.8...): Bright, nasal

-- CLASSIC TIMBRES:
--   - Organ: All harmonics equal (1.0 1.0 1.0 1.0)
--   - Flute: Fundamental dominant (1.0 0.1 0.05 0.02)
--   - Clarinet: Odd harmonics (1.0 0.0 0.5 0.0 0.25)
--   - Trumpet: Many harmonics (1.0 0.8 0.6 0.5 0.4 0.3)
--   - String: Natural decay (1.0 0.7 0.5 0.35 0.25)

-- ADVANCED TECHNIQUES:
--   - Layer multiple additive synths with slight detune
--   - Use pattern-modulated frequency for melodies
--   - Combine with effects (reverb, delay, filters)
--   - Create chords by stacking multiple additive synths
--   - Approximate classic waveforms with harmonic series
--   - Create bell tones with non-standard amplitude ratios
--   - Build pads by layering multiple octaves

-- THEORY:
--   - Each partial is n×fundamental (n=1,2,3,...)
--   - Amplitude determines loudness of each partial
--   - Fourier theorem: ANY periodic waveform can be built
--     from sine waves
--   - Sawtooth = all harmonics at 1/n amplitude
--   - Square = odd harmonics at 1/n amplitude
--   - Triangle = odd harmonics at 1/n² amplitude
--   - Natural instruments have complex harmonic structures
--     that change over time (attack, sustain, decay)
