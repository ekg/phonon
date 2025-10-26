-- White Noise Generator Demonstration
-- Generates uniformly distributed random samples

tempo: 2.0

-- Raw white noise (useful for percussion, hi-hats, effects)
~noise_raw: white_noise * 0.2

-- Filtered white noise creates different timbres
~noise_low: white_noise # lpf 800 0.5
~filtered: ~noise_low * 0.15

-- High-pass filtered noise (crisp, airy)
~noise_high: white_noise # hpf 4000 0.7
~crisp: ~noise_high * 0.1

-- Band-pass filtered noise (snare-like)
~noise_band: white_noise # bpf 2000 0.3
~snare: ~noise_band * 0.2

-- Percussive white noise burst (hi-hat)
~hihat_env: ad 0.001 0.05
~hihat: white_noise * ~hihat_env * 0.25

-- Longer noise with envelope (cymbal-like)
~cymbal_env: ad 0.005 0.3
~cymbal: white_noise # lpf 8000 0.6
~cymbal_shaped: ~cymbal * ~cymbal_env * 0.15

-- Amplitude-modulated noise (tremolo effect)
~am_lfo: sine 6
~am_noise: white_noise * ((~am_lfo + 1) * 0.5) * 0.15

-- Noise with resonant filter sweep
~sweep: line 200 4000
~swept_noise: white_noise # lpf ~sweep 0.8
~swept: ~swept_noise * 0.15

out: ~noise_raw + ~filtered + ~crisp + ~snare + ~hihat + ~cymbal_shaped + ~am_noise + ~swept
