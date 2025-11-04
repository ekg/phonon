-- Karplus-Strong Physical Modeling Demo
-- Realistic plucked string synthesis using delay line + lowpass filter

tempo: 2.0

-- SYNTAX: pluck frequency [damping]
-- frequency: pitch in Hz (or pattern)
-- damping: 0.0 = long sustain, 1.0 = short sustain (default: 0.5)

-- Basic plucked string
~simple: pluck 440

-- Different pitches
~low: pluck 110 0.3           -- Low pitch, long sustain
~mid: pluck 220 0.5           -- Medium pitch, medium sustain
~high: pluck 880 0.7          -- High pitch, short sustain

-- Melody with pattern-modulated frequency
~melody: pluck "220 330 440 330 220"

-- Bass line with low damping (long sustain)
~bass: pluck "55 55 82.5 110" 0.2

-- Lead with medium damping
~lead: pluck "440 550 660 550 440 330 440" 0.5

-- Percussive pluck (high damping = short sustain)
~perc: pluck "880 1100" 0.8

-- Pattern-modulated damping
~dynamic_damp: pluck 330 "0.3 0.7 0.5"

-- Arpeggio with transforms
~arp: pluck "220 330 440" $ fast 2

-- Chord (multiple plucks mixed)
~chord1: pluck 220 0.4
~chord2: pluck 275 0.4       -- E (5/4 ratio)
~chord3: pluck 330 0.4       -- A (3/2 ratio)
~chord: (~chord1 + ~chord2 + ~chord3) * 0.33

-- Rhythmic pattern
~rhythm: pluck "440 ~ 550 ~" 0.6

-- Through effects
~pluck_reverb: pluck "330 440" 0.5 # reverb 0.5 0.8 0.3

-- Output: choose your sound!
out: ~melody * 0.4

-- Try these variations:
-- out: ~simple * 0.5                            -- Single string
-- out: ~bass * 0.5                              -- Bass line
-- out: ~lead * 0.4                              -- Lead melody
-- out: ~perc * 0.5                              -- Percussive
-- out: ~dynamic_damp * 0.4                      -- Dynamic damping
-- out: ~arp * 0.4                               -- Fast arpeggio
-- out: ~chord * 0.5                             -- Chord
-- out: ~rhythm * 0.5                            -- Rhythmic
-- out: ~pluck_reverb * 0.3                      -- With reverb

-- CREATIVE TIPS:
-- - Low damping (0.1-0.3): long sustain, bell-like
-- - Medium damping (0.4-0.6): realistic guitar/harp
-- - High damping (0.7-0.9): percussive, muted
-- - Pattern-modulate damping for dynamic expression
-- - Combine multiple plucks for chords
-- - Use with effects (reverb, delay) for ambient textures
-- - Fast patterns create arpeggiator effect
