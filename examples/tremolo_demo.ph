-- Tremolo Effect Demonstration
-- Classic amplitude modulation for rhythmic pulsing effects

tempo: 0.5

-- Example 1: Classic Electric Guitar Tremolo (6 Hz, medium depth)
~guitar1: saw 220
~trem1: ~guitar1 # tremolo 6.0 0.6
~out1: ~trem1 * 0.3

-- Example 2: Slow Swell / Pad Tremolo (0.5 Hz, deep)
~pad2: sine 165 + sine 220
~trem2: ~pad2 # tremolo 0.5 0.8
~out2: ~trem2 * 0.2

-- Example 3: Fast Helicopter Effect (12 Hz, very deep)
~carrier3: saw 110
~heli3: ~carrier3 # tremolo 12.0 0.9
~out3: ~heli3 * 0.25

-- Example 4: Subtle Shimmer (4 Hz, shallow depth)
~synth4: square 330
~shimmer4: ~synth4 # tremolo 4.0 0.3
~out4: ~shimmer4 * 0.2

-- Example 5: Pattern-Modulated Rate (3-7 Hz sweep)
~rate_lfo5: sine 0.3 * 2.0 + 5.0
~bass5: saw 55
~dynamic5: ~bass5 # tremolo ~rate_lfo5 0.7
~out5: ~dynamic5 * 0.3

-- Example 6: Pattern-Modulated Depth (0.2-0.8 sweep)
~depth_lfo6: sine 0.4 * 0.3 + 0.5
~lead6: saw 440
~expressive6: ~lead6 # tremolo 5.0 ~depth_lfo6
~out6: ~expressive6 * 0.2

-- Example 7: Rhythmic Tremolo with Gate (auto-pan effect)
~gate7: impulse 4.0
~carrier7: saw 165
~gated_trem7: ~carrier7 # tremolo 8.0 0.5
~rhythmic7: ~gated_trem7 * ~gate7
~out7: ~rhythmic7 * 0.15

-- Example 8: Stereo Tremolo (opposing phases for auto-pan)
~synth8: square 275
~trem_l8: ~synth8 # tremolo 3.0 0.7
~trem_r8: ~synth8 # tremolo 3.0 0.7
-- Note: would need phase offset for true stereo effect
~out8: ~trem_l8 * 0.15

-- Example 9: Extreme Tremolo (threshold effect)
~noise9: white_noise
~extreme9: ~noise9 # tremolo 15.0 0.95
~out9: ~extreme9 * 0.1

-- Example 10: Combining Tremolo with Filter Sweep
~carrier10: saw 82.5
~trem10: ~carrier10 # tremolo 7.0 0.65
~sweep10: line 200.0 2000.0 3.0
~filtered10: ~trem10 # lpf ~sweep10 0.8
~out10: ~filtered10 * 0.25

-- Mix all examples
out: ~out1 + ~out2 * 0.7 + ~out3 * 0.6 + ~out4 + ~out5 * 0.8 + ~out6 + ~out7 * 0.9 + ~out8 * 0.7 + ~out9 * 0.5 + ~out10 * 0.7
