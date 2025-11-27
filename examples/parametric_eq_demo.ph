-- Parametric EQ Demonstration
-- 3-band peaking equalizer (low/mid/high)

tempo: 0.5

-- Classic "scooped" metal sound (bass boost, mid cut, treble boost)
~guitar: saw 165
~scooped: parametric_eq ~guitar 80 6.0 0.7 500 -6.0 1.0 3000 4.0 1.0
~guitar_env: ad 0.01 0.3
~guitar_out: ~scooped * ~guitar_env * 0.25

-- Bass boost for kick drum emulation
~kick_tone: sine 60
~kick_boosted: parametric_eq ~kick_tone 60 8.0 0.5 400 0.0 1.0 2000 0.0 1.0
~kick_env: ad 0.005 0.15
~kick_out: ~kick_boosted * ~kick_env * 0.3

-- Presence boost for vocal clarity (mid-high boost)
~vocal: saw 220
~vocal_clear: parametric_eq ~vocal 200 0.0 1.0 2000 4.0 1.0 5000 3.0 1.5
~vocal_env: adsr 0.02 0.1 0.7 0.3
~vocal_out: ~vocal_clear * ~vocal_env * 0.2

-- Telephone effect (extreme mid boost, cut everything else)
~phone_in: square 330
~telephone: parametric_eq ~phone_in 200 -8.0 0.7 1000 8.0 0.5 4000 -10.0 0.7
~phone_env: ad 0.01 0.2
~phone_out: ~telephone * ~phone_env * 0.15

-- Bass enhancement (sub boost, low-mid cut for clarity)
~bass: saw 55
~enhanced_bass: parametric_eq ~bass 50 9.0 0.8 200 -3.0 1.0 1000 0.0 1.0
~bass_env: ad 0.015 0.35
~bass_out: ~enhanced_bass * ~bass_env * 0.25

-- Pattern-modulated mid gain (breathing effect)
~mid_gain_pattern: "-6.0 -3.0 0.0 3.0"
~synth: tri 440
~breathing: parametric_eq ~synth 100 0.0 1.0 1000 ~mid_gain_pattern 1.0 4000 0.0 1.0
~breathing_env: ad 0.01 0.3
~breathing_out: ~breathing * ~breathing_env * 0.2

-- White noise shaping (pink-ish noise)
~noise: white_noise
~shaped_noise: parametric_eq ~noise 200 -4.0 0.7 1000 -2.0 1.0 4000 -6.0 1.0
~noise_env: ad 0.001 0.08
~noise_out: ~shaped_noise * ~noise_env * 0.08

-- Classic FM bell with EQ for brightness
~carrier: sine 550
~mod: sine 220
~bell: fm ~carrier ~mod 2.5
~bright_bell: parametric_eq ~bell 100 0.0 1.0 2000 3.0 1.0 6000 6.0 1.5
~bell_env: ad 0.005 0.4
~bell_out: ~bright_bell * ~bell_env * 0.15

-- Flat EQ (all gains = 0, should pass signal unchanged)
~ref_tone: sine 440
~flat_eq: parametric_eq ~ref_tone 100 0.0 1.0 1000 0.0 1.0 4000 0.0 1.0
~ref_env: ad 0.01 0.2
~ref_out: ~flat_eq * ~ref_env * 0.15

-- Extreme treble boost (presence/air)
~dull_sound: tri 220
~airy: parametric_eq ~dull_sound 100 0.0 1.0 500 0.0 1.0 8000 12.0 1.0
~airy_env: ad 0.01 0.25
~airy_out: ~airy * ~airy_env * 0.12

out: ~guitar_out + ~kick_out + ~vocal_out + ~phone_out + ~bass_out + ~breathing_out + ~noise_out + ~bell_out + ~ref_out + ~airy_out
