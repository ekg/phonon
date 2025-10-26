-- Pink Noise Demonstration
-- 1/f spectrum (equal energy per octave) - warmer than white noise

tempo: 2.0

-- Classic pink noise (reference)
~pink: pink_noise * 0.15

-- Pink noise percussion (hi-hat substitute)
~pink_perc: pink_noise
~perc_env: ad 0.001 0.08
~perc_out: ~pink_perc * ~perc_env * 0.12

-- Filtered pink noise (darker tone)
~dark_pink: pink_noise # lpf 1500 0.7
~dark_env: ad 0.005 0.15
~dark_out: ~dark_pink * ~dark_env * 0.2

-- Bandpass pink noise (snare-like)
~snare_pink: pink_noise # bpf 800 0.5
~snare_env: ad 0.01 0.18
~snare_out: ~snare_pink * ~snare_env * 0.25

-- Pink noise with reverb (ambient pad)
~ambient_pink: pink_noise # lpf 3000 0.6
~with_reverb: reverb ~ambient_pink 0.3 0.5
~ambient_env: ad 0.02 0.4
~ambient_out: ~with_reverb * ~ambient_env * 0.15

-- Pink noise wind texture
~wind: pink_noise # lpf 600 0.4
~wind_env: line 0.0 1.0
~wind_out: ~wind * ~wind_env * 0.1

-- Pink noise with slow modulation
~modulated: pink_noise
~lfo: sine 0.25
~mod_gain: ~lfo * 0.5 + 0.5
~mod_env: ad 0.01 0.3
~mod_out: ~modulated * ~mod_gain * ~mod_env * 0.15

-- Bitcrushed pink noise (lo-fi texture)
~lofi_pink: bitcrush pink_noise 8 0.5
~lofi_env: ad 0.005 0.12
~lofi_out: ~lofi_pink * ~lofi_env * 0.18

-- Pink noise through Moog ladder (smooth filtered)
~moog_pink: moog_ladder pink_noise 2000 0.5
~moog_env: ad 0.01 0.25
~moog_out: ~moog_pink * ~moog_env * 0.2

-- Pink noise breath effect (slow attack/decay)
~breath: pink_noise # lpf 1000 0.6
~breath_env: ad 0.3 0.5
~breath_out: ~breath * ~breath_env * 0.08

-- Pink noise with EQ shaping (presence boost)
~shaped: parametric_eq pink_noise 200 -3.0 1.0 2000 4.0 1.0 6000 3.0 1.0
~shaped_env: ad 0.01 0.15
~shaped_out: ~shaped * ~shaped_env * 0.12

out: ~pink * 0.0 + ~perc_out + ~dark_out + ~snare_out + ~ambient_out + ~wind_out * 0.5 + ~mod_out + ~lofi_out + ~moog_out + ~breath_out + ~shaped_out
