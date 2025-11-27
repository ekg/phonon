-- Schmidt Trigger (Gate with Hysteresis) Demonstration
-- Converts continuous signals to gates with noise immunity
-- Hysteresis: different on/off thresholds prevent rapid oscillation
-- Formula: output HIGH if input > high_threshold, LOW if input < low_threshold

tempo: 0.5

-- Example 1: Basic gate from LFO
~lfo1: sine 4
~gate1: ~lfo1 # schmidt 0.3 -0.3
~pulse1: saw 220 * ~gate1
~out1: ~pulse1 * 0.2

-- Example 2: Rhythmic chopping with Schmidt
~signal2: saw 110
~chop_lfo2: sine 8
~chop_gate2: ~chop_lfo2 # schmidt 0.5 -0.5
~chopped2: ~signal2 * ~chop_gate2
~out2: ~chopped2 * 0.3

-- Example 3: Envelope to gate conversion
~env3: line 1.0 0.0
~gate3: ~env3 # schmidt 0.6 0.4
~beep3: sine 440 * ~gate3
~out3: ~beep3 * 0.25

-- Example 4: Pattern-modulated thresholds
~lfo4: sine 3
~highs4: "0.5 0.7"
~lows4: "-0.5 -0.3"
~gate4: ~lfo4 # schmidt ~highs4 ~lows4
~tone4: sine 330 * ~gate4
~out4: ~tone4 * 0.2

-- Example 5: Multi-voice gating
~lfo5: sine 2
~gate5: ~lfo5 # schmidt 0.2 -0.2
~bass5: saw 55 * ~gate5
~mid5: saw 110 * ~gate5
~high5: sine 220 * ~gate5
~out5: (~bass5 + ~mid5 * 0.7 + ~high5 * 0.5) * 0.15

-- Example 6: Tremolo with hard gating
~carrier6: saw 165
~trem_lfo6: sine 6
~trem_gate6: ~trem_lfo6 # schmidt 0.4 -0.4
~tremolo6: ~carrier6 * (~trem_gate6 * 0.5 + 0.5)
~out6: ~tremolo6 * 0.25

-- Example 7: Noise burst generator
~noise7: white_noise
~burst_lfo7: sine 1
~burst_gate7: ~burst_lfo7 # schmidt 0.6 -0.6
~bursts7: ~noise7 * ~burst_gate7
~out7: ~bursts7 * 0.2

-- Example 8: Dual gate patterns
~lfo_a8: sine 3
~lfo_b8: sine 5
~gate_a8: ~lfo_a8 # schmidt 0.5 -0.5
~gate_b8: ~lfo_b8 # schmidt 0.3 -0.3
~tone_a8: saw 220 * ~gate_a8
~tone_b8: sine 330 * ~gate_b8
~out8: (~tone_a8 + ~tone_b8) * 0.15

-- Example 9: Gate-controlled filter sweep
~input9: saw 110
~sweep_lfo9: sine 0.5
~sweep_gate9: ~sweep_lfo9 # schmidt 0.0 -0.8
~cutoff9: (~sweep_gate9 * 2000.0 + 500.0)
~swept9: ~input9 # lpf ~cutoff9 0.8
~out9: ~swept9 * 0.3

-- Example 10: Polyrhythmic gating
~carrier10: saw 82.5
~gate_a10: sine 2 # schmidt 0.3 -0.3
~gate_b10: sine 3 # schmidt 0.5 -0.5
~gate_c10: sine 5 # schmidt 0.7 -0.7
~poly10: ~carrier10 * (~gate_a10 + ~gate_b10 + ~gate_c10)
~out10: ~poly10 * 0.1

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.5 + ~out2 * 0.7 + ~out3 * 0.4 + ~out4 * 0.6 + ~out5 * 0.5 + ~out6 * 0.6 + ~out7 * 0.4 + ~out8 * 0.7 + ~out9 * 0.6 + ~out10 * 0.5
