-- Limiter (Brick-Wall) Demonstration
-- Prevents signals from exceeding threshold (mastering/safety)

tempo: 0.5

-- Basic limiting: prevent clipping from hot signal
~hot_saw: saw 220 * 2.0
~safe: limiter ~hot_saw 0.8
~safe_env: ad 0.01 0.3
~safe_out: ~safe * ~safe_env * 0.4

-- Multiple oscillators that would clip when summed
~osc1: sine 220 * 0.7
~osc2: sine 330 * 0.7
~osc3: sine 440 * 0.7
~mix: ~osc1 + ~osc2 + ~osc3
~limited_mix: limiter ~mix 1.0
~mix_out: ~limited_mix * 0.25

-- Dynamic range control for percussion
~noise: white_noise
~perc_env: ad 0.001 0.05
~loud_perc: ~noise * ~perc_env * 3.0
~controlled: limiter ~loud_perc 0.5
~perc_out: ~controlled * 0.3

-- Ring mod can get very hot - limit it
~carrier: sine 440
~mod: sine 337
~ring: ring_mod ~carrier ~mod
~ring_env: ad 0.005 0.2
~hot_ring: ~ring * ~ring_env * 2.0
~safe_ring: limiter ~hot_ring 0.7
~ring_out: ~safe_ring * 0.3

-- Master limiter on entire mix (mastering use case)
~synth1: saw 165 * 1.5
~synth2: square 110 * 1.2
~synth3: tri 82 * 1.3
~unmastered: ~synth1 + ~synth2 + ~synth3
~mastered: limiter ~unmastered 0.9
~master_env: adsr 0.02 0.1 0.6 0.4
~master_out: ~mastered * ~master_env * 0.25

-- Pattern-modulated threshold for dynamic limiting
~threshold_pattern: "0.3 0.5 0.7 0.9"
~pwm_lfo: sine 2
~pwm_width: ~pwm_lfo * 0.3 + 0.5
~pwm: pulse 220 ~pwm_width
~hot_pwm: ~pwm * 1.5
~dynamic_limit: limiter ~hot_pwm ~threshold_pattern
~dynamic_out: ~dynamic_limit * 0.2

-- Limiter before distortion (tame peaks first)
~raw: saw 330 * 2.0
~tamed: limiter ~raw 0.6
~dist: distortion ~tamed 0.5
~dist_env: ad 0.01 0.25
~dist_out: ~dist * ~dist_env * 0.3

-- Protect filter resonance peaks
~filtered: saw 220 # lpf 800 12.0
~resonance_control: limiter ~filtered 0.8
~filter_env: ad 0.015 0.35
~filter_out: ~resonance_control * ~filter_env * 0.25

out: ~safe_out + ~mix_out + ~perc_out + ~ring_out + ~master_out + ~dynamic_out + ~dist_out + ~filter_out
