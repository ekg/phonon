-- Flanger Effect Demonstration
-- Sweeping comb filter via delay modulation

tempo: 0.5

-- Classic slow flanger on guitar-like sound
~guitar: saw 220
~slow_flanger: flanger ~guitar 0.7 0.3 0.5
~slow_env: ad 0.01 0.4
~slow_out: ~slow_flanger * ~slow_env * 0.25

-- Fast jet-plane flanger
~synth1: square 165
~jet_flanger: flanger ~synth1 0.8 4.0 0.7
~jet_env: ad 0.005 0.3
~jet_out: ~jet_flanger * ~jet_env * 0.2

-- Subtle flanger with low feedback
~pad: tri 110
~subtle: flanger ~pad 0.3 0.5 0.2
~pad_env: adsr 0.05 0.1 0.7 0.3
~subtle_out: ~subtle * ~pad_env * 0.2

-- Extreme flanger with high feedback (resonant)
~bass: saw 55
~extreme: flanger ~bass 0.9 1.5 0.85
~bass_env: ad 0.02 0.35
~extreme_out: ~extreme * ~bass_env * 0.2

-- Flanger on white noise (whooshing effect)
~noise: white_noise
~whoosh: flanger ~noise 0.6 2.0 0.4
~whoosh_env: ad 0.1 0.5
~whoosh_out: ~whoosh * ~whoosh_env * 0.15

-- Pattern-modulated depth (breathing flanger)
~depth_pattern: "0.2 0.5 0.8 1.0"
~organ: square 220
~breathing: flanger ~organ ~depth_pattern 1.0 0.6
~organ_env: ad 0.01 0.35
~breathing_out: ~breathing * ~organ_env * 0.18

-- Pattern-modulated rate (variable speed sweep)
~rate_pattern: "0.5 1.0 2.0 4.0"
~lead: saw 330
~variable_speed: flanger ~lead 0.7 ~rate_pattern 0.5
~lead_env: ad 0.005 0.25
~variable_out: ~variable_speed * ~lead_env * 0.2

-- Flanger into filter (classic combo)
~raw: saw 440
~flanged: flanger ~raw 0.6 1.2 0.6
~filtered: ~flanged # lpf 1500 3.0
~combo_env: ad 0.01 0.3
~combo_out: ~filtered * ~combo_env * 0.2

-- Gentle flanger on bell-like FM tone
~carrier: sine 440
~mod: sine 220
~fm_bell: fm ~carrier ~mod 2.5
~gentle: flanger ~fm_bell 0.4 0.8 0.3
~bell_env: ad 0.005 0.4
~bell_out: ~gentle * ~bell_env * 0.2

-- Dual flanger (flanger -> flanger cascade)
~source: tri 165
~flanger1: flanger ~source 0.5 0.7 0.4
~flanger2: flanger ~flanger1 0.6 1.3 0.5
~dual_env: ad 0.015 0.35
~dual_out: ~flanger2 * ~dual_env * 0.18

out: ~slow_out + ~jet_out + ~subtle_out + ~extreme_out + ~whoosh_out + ~breathing_out + ~variable_out + ~combo_out + ~bell_out + ~dual_out
