-- Ring Modulation Demonstration
-- Multiplying two signals creates sidebands at sum and difference frequencies

tempo: 2.0

-- Classic ring mod: creates inharmonic bell-like tones
~carrier1: sine 440
~mod1: sine 333
~bell1: ring_mod ~carrier1 ~mod1
~bell1_env: ad 0.005 0.3
~bell1_out: ~bell1 * ~bell1_env * 0.25

-- Higher frequency pair for brighter metallic sound
~carrier2: sine 880
~mod2: sine 200
~bright: ring_mod ~carrier2 ~mod2
~bright_env: ad 0.001 0.15
~bright_out: ~bright * ~bright_env * 0.2

-- Tremolo effect (ring mod with very low frequency)
~carrier3: sine 330
~lfo: sine 4
~tremolo: ring_mod ~carrier3 ~lfo
~tremolo_out: ~tremolo * 0.18

-- Ring mod with non-harmonic ratio creates complex timbres
~carrier4: sine 550
~mod4: sine 137
~complex: ring_mod ~carrier4 ~mod4
~complex_env: ad 0.01 0.25
~complex_out: ~complex * ~complex_env * 0.22

-- Detuned carriers for chorus-like effect
~carrier5a: sine 220
~carrier5b: sine 223
~mod5: sine 55
~ring5a: ring_mod ~carrier5a ~mod5
~ring5b: ring_mod ~carrier5b ~mod5
~chorus: ~ring5a + ~ring5b
~chorus_env: ad 0.02 0.4
~chorus_out: ~chorus * ~chorus_env * 0.15

-- Pattern-modulated carrier frequency
~carrier_pattern: "110 220 165 275"
~carrier6: sine ~carrier_pattern
~mod6: sine 83
~pattern_ring: ring_mod ~carrier6 ~mod6
~pattern_out: ~pattern_ring * 0.18

-- Filtered ring mod for softer tones
~carrier7: sine 440
~mod7: sine 220
~raw_ring: ring_mod ~carrier7 ~mod7
~soft: ~raw_ring # lpf 3000 0.6
~soft_env: ad 0.015 0.35
~soft_out: ~soft * ~soft_env * 0.2

-- Ring mod with PWM for evolving timbre
~pwm_lfo: sine 0.5
~pwm_width: ~pwm_lfo * 0.3 + 0.5
~carrier8: pulse 165 ~pwm_width
~mod8: sine 41
~evolving: ring_mod ~carrier8 ~mod8
~evolving_out: ~evolving * 0.15

out: ~bell1_out + ~bright_out + ~tremolo_out + ~complex_out + ~chorus_out + ~pattern_out + ~soft_out + ~evolving_out
