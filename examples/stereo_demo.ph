-- Stereo Output Demonstration
-- Use out1 for left channel, out2 for right channel
-- This example creates a rich stereo soundscape with different sounds in each channel

tempo: 1.5

-- Left channel: Low bass with subtle movement
~bass_tone: saw 55 * 0.3
~bass_lfo: sine 0.1
~bass_filtered: ~bass_tone # lpf (~bass_lfo * 300 + 400) 0.8
~bass_env: ad 0.01 0.4

-- Right channel: Higher melody with stereo contrast
~melody_tone: square 440 * 0.2
~melody_lfo: sine 0.15
~melody_filtered: ~melody_tone # hpf (~melody_lfo * 500 + 1000) 0.5
~melody_env: ad 0.005 0.3

-- Both channels: Center pad (mono-compatible)
~pad: tri 220 * 0.15
~pad_verb: reverb ~pad 0.3 0.5

-- Left channel: Pink noise (warm ambience)
~left_noise: (pink_noise # lpf 800 0.6) * 0.08

-- Right channel: White noise (bright texture)
~right_noise: (white_noise # hpf 2000 0.4) * 0.06

-- Left channel: Deep sub bass
~sub: sine 55 * 0.2

-- Right channel: Harmonic overtone
~overtone: sine 165 * 0.12

-- Stereo outputs with distinct character per channel
out1: ~bass_filtered * ~bass_env + ~pad_verb + ~left_noise + ~sub
out2: ~melody_filtered * ~melody_env + ~pad_verb + ~right_noise + ~overtone
