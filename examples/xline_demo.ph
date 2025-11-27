-- XLine (Exponential Envelope) Demonstration
-- Generates exponential ramp from start to end over duration
-- More natural sounding than linear ramps for pitch/amplitude
-- Useful for realistic fades, pitch glides, parameter sweeps

tempo: 0.5

-- Example 1: Exponential pitch glide (descending)
~pitch1: xline 880.0 110.0 1.0
~tone1: sine ~pitch1
~out1: ~tone1 * 0.25

-- Example 2: Exponential fade out
~amplitude2: xline 1.0 0.001 1.0
~tone2: saw 220
~out2: ~tone2 * ~amplitude2 * 0.3

-- Example 3: Exponential fade in (reverse)
~amplitude3: xline 0.001 1.0 0.5
~tone3: tri 330
~out3: ~tone3 * ~amplitude3 * 0.25

-- Example 4: Fast exponential drop (percussive)
~pitch4: xline 440.0 55.0 0.1
~env4: xline 1.0 0.001 0.15
~kick: sine ~pitch4 * ~env4
~out4: ~kick * 0.5

-- Example 5: Exponential filter sweep (descending)
~cutoff5: xline 8000.0 200.0 1.0
~source5: saw 110
~filtered5: ~source5 # lpf ~cutoff5 0.8
~out5: ~filtered5 * 0.3

-- Example 6: Exponential filter sweep (ascending)
~cutoff6: xline 200.0 8000.0 1.0
~source6: square 110
~filtered6: ~source6 # hpf ~cutoff6 0.5
~out6: ~filtered6 * 0.2

-- Example 7: Exponential FM modulation index
~mod_idx: xline 0.1 10.0 1.0
~fm_tone: fm 220 440 ~mod_idx
~out7: ~fm_tone * 0.2

-- Example 8: Cascaded exponential envelopes
~env8a: xline 1.0 0.3 0.5
~env8b: xline 1.0 0.1 1.0
~cascaded: ~env8a * ~env8b
~tone8: sine 440
~out8: ~tone8 * ~cascaded * 0.3

-- Example 9: Exponential pulse width modulation
~pw_mod: xline 0.1 0.9 1.0
~pwm_osc: pulse 110 ~pw_mod
~out9: ~pwm_osc * 0.2

-- Example 10: Exponential resonance sweep
~cutoff10: sine 0.5 * 2000 + 3000
~res10: xline 0.1 0.95 2.0
~source10: saw 55
~filtered10: ~source10 # lpf ~cutoff10 ~res10
~out10: ~filtered10 * 0.25

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.8 + ~out2 * 0.0 + ~out3 * 0.7 + ~out4 * 0.6 + ~out5 * 0.0 + ~out6 * 0.0 + ~out7 * 0.5 + ~out8 * 0.6 + ~out9 * 0.4 + ~out10 * 0.5
