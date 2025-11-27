-- Digital Waveguide Physical Modeling Demo
-- Simulates wave propagation in strings/tubes using bidirectional delay lines
-- More sophisticated than Karplus-Strong with separate forward/backward waves

tempo: 0.5

-- SYNTAX: waveguide frequency damping pickup_position
-- frequency: pitch in Hz (or pattern)
-- damping: energy loss at boundaries (0.0 = no loss, 1.0 = max loss)
-- pickup_position: where to read from string (0.0 to 1.0)
--   - 0.5 = center (emphasizes fundamental)
--   - 0.25 or 0.75 = off-center (emphasizes harmonics)

-- Basic waveguide string
~simple: waveguide 440 0.5 0.5

-- Different pitches with various damping
~low: waveguide 110 0.3 0.5           -- Low pitch, long sustain
~mid: waveguide 220 0.5 0.5           -- Medium pitch, medium sustain
~high: waveguide 880 0.7 0.5          -- High pitch, short sustain

-- Pickup position affects timbre
~center: waveguide 220 0.4 0.5        -- Center: warm, fundamental-rich
~quarter: waveguide 220 0.4 0.25      -- Quarter: brighter, more harmonics
~eighth: waveguide 220 0.4 0.125      -- Near end: very bright, metallic

-- Melody with pattern-modulated frequency
~melody: waveguide "220 330 440 330 220" 0.5 0.5

-- Bass line with low damping (long sustain)
~bass: waveguide "55 55 82.5 110" 0.2 0.5

-- Lead with medium damping
~lead: waveguide "440 550 660 550 440 330 440" 0.5 0.5

-- Percussive sound (high damping = short sustain)
~perc: waveguide "880 1100" 0.9 0.5

-- Pattern-modulated damping (dynamic articulation)
~dynamic_damp: waveguide 330 "0.2 0.8 0.5" 0.5

-- Pattern-modulated pickup position (timbral variation)
~dynamic_pickup: waveguide 220 0.4 "0.5 0.25 0.75 0.5"

-- Arpeggio with transforms
~arp: waveguide "220 330 440" 0.5 0.5 $ fast 2

-- Chord (multiple strings mixed)
~chord1: waveguide 220 0.4 0.5        -- Root
~chord2: waveguide 275 0.4 0.5        -- Major third (5/4 ratio)
~chord3: waveguide 330 0.4 0.5        -- Perfect fifth (3/2 ratio)
~chord: (~chord1 + ~chord2 + ~chord3) * 0.33

-- Rhythmic pattern with rests
~rhythm: waveguide "440 ~ 550 ~" 0.6 0.5

-- Bright tone (off-center pickup)
~bright: waveguide "330 440" 0.5 0.2

-- Dark tone (center pickup)
~dark: waveguide "330 440" 0.5 0.5

-- Bell-like tone (very low damping, off-center)
~bell: waveguide "220 330 440" 0.1 0.25

-- Muted tone (high damping, center)
~muted: waveguide "220 330 440" 0.8 0.5

-- Through effects
~waveguide_reverb: waveguide "330 440" 0.5 0.5 # reverb 0.5 0.8 0.3

-- Filtered waveguide
~filtered: waveguide "110 165 220" 0.3 0.5 # lpf 2000 0.8

-- Output: choose your sound!
out: ~melody * 0.4

-- Try these variations:
-- out: ~simple * 0.5                            -- Single string
-- out: ~bass * 0.5                              -- Bass line
-- out: ~lead * 0.4                              -- Lead melody
-- out: ~perc * 0.5                              -- Percussive
-- out: ~dynamic_damp * 0.4                      -- Dynamic damping
-- out: ~dynamic_pickup * 0.4                    -- Timbral variation
-- out: ~arp * 0.4                               -- Fast arpeggio
-- out: ~chord * 0.5                             -- Chord
-- out: ~rhythm * 0.5                            -- Rhythmic
-- out: ~bright * 0.4                            -- Bright tone
-- out: ~dark * 0.4                              -- Dark tone
-- out: ~bell * 0.3                              -- Bell-like
-- out: ~muted * 0.4                             -- Muted
-- out: ~waveguide_reverb * 0.3                  -- With reverb
-- out: ~filtered * 0.4                          -- Filtered

-- CREATIVE TIPS:
-- DAMPING (energy loss at boundaries):
--   - Low (0.1-0.3): long sustain, bell-like, resonant
--   - Medium (0.4-0.6): realistic guitar/harp/string
--   - High (0.7-0.9): percussive, muted, dry
--   - Pattern-modulate for dynamic articulation

-- PICKUP POSITION (timbral control):
--   - Center (0.5): warm, fundamental-rich, dark
--   - Quarter (0.25/0.75): balanced, natural
--   - Near ends (0.1/0.9): bright, harmonic-rich, metallic
--   - Pattern-modulate for timbral variation

-- COMPARISON TO KARPLUS-STRONG:
--   - Waveguide: bidirectional delay lines, pickup position control
--   - Karplus-Strong: single delay line, simpler but limited timbral control
--   - Waveguide offers more realistic physical modeling

-- USES:
--   - Plucked strings: guitar, harp, harpsichord
--   - Bowed strings: violin, cello (with continuous excitation)
--   - Struck strings: piano (with impulse excitation)
--   - Wind instruments: flute, clarinet (tube modeling)
--   - Bells and resonant bodies: metallic tones

-- ADVANCED TECHNIQUES:
--   - Combine multiple waveguides for coupled strings
--   - Modulate pickup position for wah-wah effect
--   - Use low damping + reverb for ambient pads
--   - Fast patterns create realistic tremolos
--   - Layer with effects for hybrid synthesis
