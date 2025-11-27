-- RMS (Root Mean Square) Analyzer Demonstration
-- Measures the average power/amplitude of a signal
-- Useful for envelope following, loudness metering, VU meters, and sidechain effects
-- Formula: RMS = sqrt(sum(x²) / N) where N is window size

tempo: 0.5

-- Example 1: Basic amplitude measurement
~sine1: sine 440
~level1: ~sine1 # rms 0.01
~out1: ~level1

-- Example 2: Envelope follower (fast window)
~impulse2: impulse 4.0
~envelope2: ~impulse2 # rms 0.005
~tone2: saw 220 * ~envelope2 * 3.0
~out2: ~tone2 * 0.3

-- Example 3: Sidechain ducking (medium window)
~kick3: impulse 2.0
~kick_level3: ~kick3 # rms 0.02
~pad3: saw 110 + saw 165
~ducked3: ~pad3 * (1.0 - ~kick_level3 * 3.0)
~out3: (~kick3 * 0.5 + ~ducked3) * 0.2

-- Example 4: VU meter (slow window for average level)
~music4: saw 110 + sine 220 * 0.5 + sine 440 * 0.25
~vu4: ~music4 # rms 0.1
~out4: ~vu4 * 0.5

-- Example 5: Dynamic range compression (auto-gain)
~noise5: white_noise
~level5: ~noise5 # rms 0.05
~target_level: 0.3
~gain5: ~target_level / (~level5 + 0.001)
~compressed5: ~noise5 * ~gain5
~out5: ~compressed5 * 0.15

-- Example 6: Pattern-modulated window size
~signal6: sine 330
~windows6: "0.01 0.05 0.1"
~varying_rms6: ~signal6 # rms ~windows6
~out6: ~varying_rms6 * 0.5

-- Example 7: Tremolo depth metering
~carrier7: saw 220
~lfo7: sine 4 * 0.5 + 0.5
~tremolo7: ~carrier7 * ~lfo7
~tremolo_level7: ~tremolo7 # rms 0.02
~out7: ~tremolo_level7 * 0.3

-- Example 8: Attack/Decay envelope tracking
~impulse8: impulse 1.0
~decay8: ~impulse8 # lag 0.2
~decay_rms8: ~decay8 # rms 0.01
~tone8: sine 440 * ~decay_rms8 * 2.0
~out8: ~tone8 * 0.3

-- Example 9: Level-dependent effect (auto-wah)
~input9: saw 110
~input_level9: ~input9 # rms 0.02
~wah_freq9: ~input_level9 * 2000.0 + 500.0
~wah9: ~input9 # lpf ~wah_freq9 1.5
~out9: ~wah9 * 0.3

-- Example 10: Multi-band level metering
~full_spectrum10: saw 55 + saw 110 + saw 220 + saw 440
~low10: ~full_spectrum10 # lpf 200 0.8
~mid10: ~full_spectrum10 # bpf 1000 2.0
~high10: ~full_spectrum10 # hpf 3000 0.8
~low_level10: ~low10 # rms 0.05
~mid_level10: ~mid10 # rms 0.05
~high_level10: ~high10 # rms 0.05
~out10: (~low_level10 + ~mid_level10 + ~high_level10) * 0.2

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.0 + ~out2 * 0.6 + ~out3 * 0.7 + ~out4 * 0.0 + ~out5 * 0.5 + ~out6 * 0.0 + ~out7 * 0.5 + ~out8 * 0.6 + ~out9 * 0.7 + ~out10 * 0.4
