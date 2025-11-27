-- Stack operation demo - Per-voice gain control
-- This demonstrates THE KEY feature for controlling individual voices

tempo: 0.5

-- Example 1: Stack oscillators with different gains
~low: sine 110 * 0.3
~mid: sine 220 * 0.5
~high: sine 440 * 0.2
~chord: stack [~low, ~mid, ~high]

-- Example 2: Stack drum patterns with individual gains
~kick: s "bd" * 0.8
~snare: s "~ sn" * 1.0
~hh: s "hh*4" * 0.4

~drums: stack [~kick, ~snare, ~hh]

-- Example 3: Stack with transforms applied to individual layers
~normal_beat: s "bd sn"
~fast_hats: s "hh*4" $ fast 2
~reversed_snare: s "~ sn" $ rev

~layered: stack [~normal_beat, ~fast_hats, ~reversed_snare]

-- Output mix
out: ~drums * 0.6 + ~chord * 0.3
