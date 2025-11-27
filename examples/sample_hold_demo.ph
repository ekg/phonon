-- Sample-and-Hold Demo
-- Demonstrates classic analog-style sample-and-hold behavior

tempo: 0.5

-- Example 1: Random voltage generation
-- Sample random noise on each clock pulse
~noise: white_noise
~clock: square 4.0
~random_voltage: sample_hold ~noise ~clock
~freq: (~random_voltage + 1.0) * 220.0
~melody: sine ~freq
o1: ~melody * 0.3

-- Example 2: Stepped LFO modulation
-- Sample a slow sine wave at regular intervals
~lfo: sine 0.5
~fast_clock: square 16.0
~stepped_lfo: sample_hold ~lfo ~fast_clock
~modulated_freq: 440.0 + (~stepped_lfo * 110.0)
~synth: saw ~modulated_freq
o2: ~synth * 0.2

-- Example 3: Rhythmic parameter automation
-- Create stepped filter cutoff modulation
~filter_mod: sine 0.25
~rhythm: square 8.0
~stepped_cutoff: sample_hold ~filter_mod ~rhythm
~cutoff_freq: (~stepped_cutoff + 1.0) * 1000.0 + 500.0
~bass: saw 110.0
~filtered: lpf ~bass ~cutoff_freq 0.8
o3: ~filtered * 0.25
