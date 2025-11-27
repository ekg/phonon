-- Brown Noise Demonstration
-- 6dB/octave rolloff (steeper than pink) - very warm, rumbly sound
-- Also called Brownian noise or red noise

tempo: 0.5

-- Classic brown noise (reference)
~brown: brown_noise * 0.2

-- Brown noise thunder rumble (low-passed)
~thunder: brown_noise # lpf 300 0.5
~thunder_env: ad 0.1 0.8
~thunder_out: ~thunder * ~thunder_env * 0.25

-- Brown noise kick drum (very low frequency)
~kick_tone: brown_noise # lpf 100 0.7
~kick_env: ad 0.005 0.15
~kick_out: ~kick_tone * ~kick_env * 0.35

-- Brown noise earthquake/rumble bed
~rumble: brown_noise # lpf 60 0.4
~rumble_env: line 0.0 1.0
~rumble_out: ~rumble * ~rumble_env * 0.12

-- Filtered brown noise (mid-range warmth)
~warm: brown_noise # lpf 1200 0.6
~warm_env: ad 0.01 0.3
~warm_out: ~warm * ~warm_env * 0.2

-- Brown noise with reverb (cinematic ambience)
~ambient: brown_noise # lpf 400 0.5
~ambient_verb: reverb ~ambient 0.4 0.6
~ambient_env: ad 0.05 0.5
~ambient_out: ~ambient_verb * ~ambient_env * 0.15

-- Brown noise wind gust
~wind: brown_noise # lpf 500 0.4 # hpf 50 0.3
~wind_env: ad 0.3 0.6
~wind_out: ~wind * ~wind_env * 0.18

-- Brown noise with slow LFO modulation
~modulated: brown_noise
~lfo: sine 0.2
~mod_gain: ~lfo * 0.4 + 0.6
~mod_env: ad 0.02 0.35
~mod_out: ~modulated * ~mod_gain * ~mod_env * 0.15

-- Brown noise through Moog ladder (smooth sub-bass)
~sub: moog_ladder brown_noise 150 0.6
~sub_env: ad 0.02 0.4
~sub_out: ~sub * ~sub_env * 0.25

-- Bandpass brown noise (focused rumble)
~focused: brown_noise # bpf 200 0.4
~focused_env: ad 0.01 0.25
~focused_out: ~focused * ~focused_env * 0.22

-- Brown noise with EQ shaping (presence boost)
~shaped: parametric_eq brown_noise 80 3.0 0.7 400 -2.0 1.0 2000 0.0 1.0
~shaped_env: ad 0.015 0.3
~shaped_out: ~shaped * ~shaped_env * 0.18

out: ~brown * 0.0 + ~thunder_out + ~kick_out + ~rumble_out * 0.5 + ~warm_out + ~ambient_out + ~wind_out + ~mod_out + ~sub_out + ~focused_out + ~shaped_out
