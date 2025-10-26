-- Peak Follower Demonstration
-- Tracks peak amplitude with fast attack and slow release
-- Classic envelope follower for dynamics and modulation effects

tempo: 2.0

-- Example 1: Envelope extraction from modulated tone
~lfo1: sine 2 * 0.5 + 0.5
~tone1: sine 440 * ~lfo1
~env1: ~tone1 # peak_follower 0.01 0.05
~visual1: ~env1
~out1: ~visual1 * 0.3

-- Example 2: Sidechain ducking effect
~kick2: impulse 4.0
~kick_env2: ~kick2 # peak_follower 0.001 0.2
~bass2: saw 55
~ducked2: ~bass2 * (1.0 - ~kick_env2 * 0.8)
~out2: ~ducked2 * 0.3

-- Example 3: Dynamic filter modulation
~carrier3: saw 110
~mod3: sine 0.5 * 0.5 + 0.5
~modulated3: ~carrier3 * ~mod3
~envelope3: ~modulated3 # peak_follower 0.02 0.1
~cutoff3: ~envelope3 * 3000.0 + 200.0
~filtered3: ~carrier3 # lpf ~cutoff3 0.8
~out3: ~filtered3 * 0.25

-- Example 4: Amplitude-to-pitch tracking
~noise_burst4: white_noise * (sine 3 * 0.5 + 0.5)
~amp4: ~noise_burst4 # peak_follower 0.01 0.1
~pitch4: 110.0 + ~amp4 * 330.0
~tracking4: saw ~pitch4
~out4: ~tracking4 * 0.2

-- Example 5: Rhythmic gating
~input5: saw 165
~gate_pattern5: impulse 8.0
~gate_env5: ~gate_pattern5 # peak_follower 0.001 0.08
~gated5: ~input5 * ~gate_env5
~out5: ~gated5 * 0.3

-- Mix all examples
out: ~out1 + ~out2 + ~out3 + ~out4 + ~out5
