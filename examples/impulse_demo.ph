-- Impulse Generator Demonstration
-- Generates periodic single-sample spikes at specified frequency
-- Useful for triggering, creating rhythmic gates, clock signals

tempo: 2.0

-- Example 1: Basic impulse as clock
~clock: impulse 4.0
~tone1: sine 440
~gated1: ~tone1 * ~clock * 0.3

-- Example 2: Slow impulse trigger
~slow_clock: impulse 0.5
~bass: saw 55
~bass_hits: ~bass * ~slow_clock * 0.4

-- Example 3: Fast impulse (hi-hat-like clicks)
~fast_pulse: impulse 16.0
~noise: white_noise # hpf 8000 0.5
~hats: ~noise * ~fast_pulse * 0.15

-- Example 4: Impulse driving kick
~kick_clock: impulse 2.0
~kick_tone: sine 60
~kick_env: line 1.0 0.0
~kick_out: ~kick_tone * ~kick_clock * ~kick_env * 0.5

-- Example 5: Dual rate impulses (polyrhythm)
~pulse3: impulse 3.0
~pulse5: impulse 5.0
~poly_tone: square 220
~poly_out: ~poly_tone * (~pulse3 + ~pulse5) * 0.15

-- Example 6: Impulse with FM synthesis
~fm_clock: impulse 8.0
~fm_tone: fm 440 880 3.0
~fm_gated: ~fm_tone * ~fm_clock * 0.2

-- Example 7: Impulse-controlled filter sweep
~sweep_clock: impulse 1.0
~sweep_lfo: line 0.0 1.0
~sweep_input: saw 110
~sweep_filtered: ~sweep_input # lpf (~sweep_lfo * 4000 + 200) 0.8
~sweep_out: ~sweep_filtered * ~sweep_clock * 0.25

-- Example 8: Impulse + reverb (sparse ambience)
~sparse_clock: impulse 0.25
~sparse_tone: tri 330
~sparse_hit: ~sparse_tone * ~sparse_clock
~sparse_verb: reverb ~sparse_hit 0.5 0.7
~sparse_out: ~sparse_verb * 0.3

-- Example 9: Multiple impulse layers
~layer1: impulse 1.0 * (sine 220) * 0.15
~layer2: impulse 2.0 * (sine 330) * 0.12
~layer3: impulse 4.0 * (sine 440) * 0.1
~layered_out: ~layer1 + ~layer2 + ~layer3

-- Example 10: Impulse modulating pulse width
~pwm_clock: impulse 0.5
~pwm_mod: line 0.1 0.9
~pwm_osc: pulse 110 ~pwm_mod
~pwm_out: ~pwm_osc * ~pwm_clock * 0.2

-- Mix all examples
out: ~gated1 * 0.8 + ~bass_hits * 0.7 + ~hats * 0.5 + ~kick_out * 0.0 + ~poly_out * 0.6 + ~fm_gated * 0.4 + ~sweep_out * 0.0 + ~sparse_out * 0.0 + ~layered_out * 0.5 + ~pwm_out * 0.0
