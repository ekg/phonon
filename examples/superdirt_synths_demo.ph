-- SuperDirt Synths Demo
-- Demonstrates all SuperDirt synthesizers available in Phonon

tempo: 2.0

-- ========== Drum Synths ==========

-- SuperKick - Classic kick drum
-- Parameters: freq, pitch_env, sustain, noise_amt
~kick: superkick 60 0.5 0.3 0.1

-- SuperSnare - Snare with filtered noise
-- Parameters: freq, snappy, sustain
~snare: supersnare 200 0.8 0.15

-- SuperHat - Hi-hat with filtered noise burst
-- Parameters: bright, sustain
~hat_closed: superhat 0.7 0.05
~hat_open: superhat 0.7 0.3

-- ========== Melodic Synths ==========

-- SuperSaw - Thick detuned saw waves
-- Parameters: freq, detune, voices
~bass: supersaw 55 0.5 7

-- SuperPWM - Pulse width modulation
-- Parameters: freq, pwm_rate, pwm_depth
~pad: superpwm 220 0.5 0.8

-- SuperChip - Chiptune square wave
-- Parameters: freq, vibrato_rate, vibrato_depth
~lead: superchip 440 5.0 0.05

-- SuperFM - 2-operator FM synthesis
-- Parameters: freq, mod_ratio, mod_index
~bell: superfm 880 2.0 1.0

-- ========== Mix ==========
out: ~kick * 0.8 + ~snare * 0.6 + ~hat_closed * 0.4 + ~bass * 0.3 + ~pad * 0.2
