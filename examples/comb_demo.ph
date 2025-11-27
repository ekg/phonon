-- Comb Filter (Feedback Delay Line) Demonstration
-- Creates resonant peaks by feeding delayed signal back into itself
-- Useful for physical modeling, bells, metallic sounds, and adding character
-- Formula: output = input + feedback * delayed_output
-- Delay time = sample_rate / frequency

tempo: 0.5

-- Example 1: Bell Sound (impulse + comb)
~strike1: impulse 1.0
~bell1: ~strike1 # comb 440 0.98
~out1: ~bell1 * 0.3

-- Example 2: Low Feedback (short resonance)
~strike2: impulse 1.0
~short2: ~strike2 # comb 330 0.5
~out2: ~short2 * 0.4

-- Example 3: High Feedback (long resonance)
~strike3: impulse 1.0
~long3: ~strike3 # comb 550 0.97
~out3: ~long3 * 0.25

-- Example 4: Comb on Continuous Tone (adds resonance)
~tone4: saw 55
~resonant4: ~tone4 # comb 440 0.8
~out4: ~resonant4 * 0.3

-- Example 5: Multiple Combs (complex resonance)
~strike5: impulse 0.5
~body1: ~strike5 # comb 220 0.95
~body2: ~body1 # comb 330 0.93
~body3: ~body2 # comb 440 0.91
~out5: ~body3 * 0.3

-- Example 6: Tuned Combs (chord-like)
~impulse6: impulse 2.0
~comb_a: ~impulse6 # comb 220 0.9
~comb_b: ~impulse6 # comb 275 0.9
~comb_c: ~impulse6 # comb 330 0.9
~out6: (~comb_a + ~comb_b + ~comb_c) * 0.2

-- Example 7: Pattern-Modulated Frequency
~impulse7: impulse 2.0
~freqs7: "220 330 440 550"
~combed7: ~impulse7 # comb ~freqs7 0.9
~out7: ~combed7 * 0.25

-- Example 8: Comb on Noise (filtered noise texture)
~noise8: white_noise
~combed8: ~noise8 # comb 1000 0.95
~out8: ~combed8 * 0.15

-- Example 9: String-like Sound (saw + comb)
~exciter9: saw 110
~string9: ~exciter9 # comb 110 0.85
~out9: ~string9 * 0.3

-- Example 10: Metallic Percussion (multiple impulses + combs)
~hits10: impulse 4.0
~metal1: ~hits10 # comb 523 0.96
~metal2: ~hits10 # comb 659 0.94
~metal3: ~hits10 # comb 784 0.92
~out10: (~metal1 + ~metal2 + ~metal3) * 0.2

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.8 + ~out2 * 0.5 + ~out3 * 0.7 + ~out4 * 0.6 + ~out5 * 0.7 + ~out6 * 0.6 + ~out7 * 0.5 + ~out8 * 0.3 + ~out9 * 0.6 + ~out10 * 0.7
