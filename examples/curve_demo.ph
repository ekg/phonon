-- Curve Envelope Demonstration
-- Curved ramps with adjustable shape: linear, exponential, logarithmic

tempo: 2.0

-- Example 1: Exponential filter sweep (slow start, fast end)
~exp_sweep1: curve 200.0 4000.0 2.0 3.0
~carrier1: saw 110
~filtered1: ~carrier1 # lpf ~exp_sweep1 0.8
~out1: ~filtered1 * 0.3

-- Example 2: Logarithmic amplitude fade (fast start, slow end)
~log_fade2: curve 1.0 0.0 3.0 -3.0
~tone2: sine 440
~faded2: ~tone2 * ~log_fade2
~out2: ~faded2 * 0.3

-- Example 3: Linear pitch glide (straight ramp)
~linear_pitch3: curve 110.0 440.0 2.0 0.0
~glide3: saw ~linear_pitch3
~out3: ~glide3 * 0.2

-- Example 4: Exponential resonance sweep
~res_curve4: curve 0.5 8.0 2.5 4.0
~osc4: saw 82.5
~swept4: ~osc4 # lpf 800.0 ~res_curve4
~out4: ~swept4 * 0.25

-- Example 5: Logarithmic tremolo
~trem_curve5: curve 1.0 0.2 3.0 -2.0
~carrier5: saw 165
~tremolo5: ~carrier5 * ~trem_curve5
~out5: ~tremolo5 * 0.3

-- Mix all examples
out: ~out1 * 0.6 + ~out2 * 0.5 + ~out3 * 0.4 + ~out4 * 0.5 + ~out5 * 0.5
