-- Amp Follower Demonstration
-- RMS-based envelope follower with attack/release smoothing
-- Smoother than peak follower - perfect for musical dynamics and ducking

tempo: 2.0

-- Example 1: Smooth envelope extraction
~lfo1: sine 2 * 0.5 + 0.5
~carrier1: sine 440 * ~lfo1
~envelope1: ~carrier1 # amp_follower 0.02 0.1 0.01
~out1: ~envelope1 * 0.3

-- Example 2: Sidechain ducking effect
~kick2: impulse 4.0
~kick_env2: ~kick2 # amp_follower 0.001 0.2 0.01
~bass2: saw 55
~ducked2: ~bass2 * (1.0 - ~kick_env2 * 0.8)
~out2: ~ducked2 * 0.3

-- Example 3: Smooth tremolo effect
~carrier3: saw 220
~trem_lfo3: sine 6 * 0.5 + 0.5
~modulated3: ~carrier3 * ~trem_lfo3
~smooth_env3: ~modulated3 # amp_follower 0.02 0.05 0.01
~out3: ~smooth_env3 * 0.25

-- Example 4: Dynamic filter modulation
~source4: saw 110
~mod4: sine 0.5 * 0.5 + 0.5
~modulated4: ~source4 * ~mod4
~env4: ~modulated4 # amp_follower 0.03 0.1 0.02
~cutoff4: ~env4 * 3000.0 + 300.0
~filtered4: ~source4 # lpf ~cutoff4 0.8
~out4: ~filtered4 * 0.25

-- Example 5: Smooth noise gate
~noise5: white_noise
~noise_env5: ~noise5 # amp_follower 0.02 0.15 0.02
~gated5: ~noise5 * ~noise_env5
~out5: ~gated5 * 0.2

-- Mix all examples
out: ~out1 + ~out2 + ~out3 + ~out4 + ~out5
