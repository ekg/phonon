-- Phase 3: Dynamic Pattern Transforms
-- Pattern-to-pattern modulation examples
-- Patterns can now modulate other pattern transforms dynamically!

tempo: 0.5

-- ============================================================
-- BASIC PATTERN MODULATION
-- ============================================================

-- Define a pattern that controls speed
%speed: "1 2 3 4"

-- Use the speed pattern to modulate fast transform
~drums: s "bd*4" $ fast %speed

-- Result: kick density changes each cycle
-- Cycle 0: 1x speed (4 kicks)
-- Cycle 1: 2x speed (8 kicks)
-- Cycle 2: 3x speed (12 kicks)
-- Cycle 3: 4x speed (16 kicks)

-- ============================================================
-- PROBABILITY MODULATION
-- ============================================================

-- Pattern controls how many events get removed
%density: "0.1 0.3 0.5 0.8"

~hats: s "hh*8" $ degradeBy %density

-- Result: hi-hat density evolves over cycles
-- Low density -> sparse pattern
-- High density -> full pattern

-- ============================================================
-- SHUFFLE MODULATION
-- ============================================================

-- Pattern controls shuffle amount
%shuffle_amt: "0.0 0.25 0.5 0.75"

~snare: s "sn*4" $ shuffle %shuffle_amt

-- Result: timing randomness increases over cycles

-- ============================================================
-- COMBINING MULTIPLE PATTERN MODULATIONS
-- ============================================================

-- Multiple control patterns
%kick_speed: "1 2 1 4"
%snare_speed: "2 3 2 1"
%kick_prob: "0.1 0.3 0.5 0.7"
%snare_prob: "0.8 0.6 0.4 0.2"

~kick: s "bd*4" $ fast %kick_speed $ degradeBy %kick_prob
~snare2: s "sn*4" $ fast %snare_speed $ degradeBy %snare_prob

-- Result: Complex evolving drum patterns with independent evolution

-- ============================================================
-- CONSTANT PATTERN ASSIGNMENT
-- ============================================================

-- You can assign constants to patterns too
%constant_speed: 3.0

~fast_hats: s "hh*4" $ fast %constant_speed

-- ============================================================
-- AUDIO SIGNAL AS PATTERN (LFO)
-- ============================================================

-- Use audio signal to control pattern transforms
~lfo: sine 0.25

-- Convert LFO to pattern (auto-scaled to 0-1 range)
%lfo_pattern: ~lfo

-- Use LFO to modulate probability
~modulated_drums: s "bd*8" $ degradeBy %lfo_pattern

-- Result: Kick density waves in and out smoothly

-- ============================================================
-- COMPLEX REAL-WORLD EXAMPLE
-- ============================================================

-- Build an evolving techno pattern

-- Speed patterns that create momentum
%kick_evo: "1 1 2 4"
%hat_evo: "2 4 8 16"

-- Probability patterns for tension/release
%kick_density: "0.9 0.7 0.5 0.3"
%hat_density: "0.3 0.5 0.7 0.9"

-- Build layers with independent evolution
~techno_kick: s "bd*4" $ fast %kick_evo $ degradeBy %kick_density
~techno_hats: s "hh*8" $ fast %hat_evo $ degradeBy %hat_density
~techno_snare: s "sn(8,3)" -- Euclidean for contrast

-- Mix with simple gain control
out: (~techno_kick + ~techno_hats + ~techno_snare) * 0.7

-- Result: A 4-bar evolving techno groove where:
-- - Kick becomes sparse as pattern speeds up
-- - Hi-hats become dense as pattern speeds up
-- - Creates natural tension and release
-- - Snare provides steady Euclidean counterpoint
