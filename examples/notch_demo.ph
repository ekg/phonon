-- Notch Filter (Band-Reject) Demonstration
-- Removes frequencies at center frequency while passing all others
-- Opposite of bandpass: rejects a narrow band, passes everything else
-- Useful for removing hum, feedback, resonances, or unwanted tones

tempo: 0.5

-- Example 1: Remove 60Hz hum
~signal1: sine 440 + sine 880
~hum1: sine 60 * 0.4
~noisy1: ~signal1 + ~hum1
~clean1: ~noisy1 # notch 60 5.0
~out1: ~clean1 * 0.25

-- Example 2: Remove feedback frequency
~source2: saw 110
~feedback2: sine 1200 * 0.5
~with_feedback: ~source2 + ~feedback2
~no_feedback: ~with_feedback # notch 1200 8.0
~out2: ~no_feedback * 0.3

-- Example 3: Remove resonant peak
~resonant3: saw 110 # lpf 800 10.0
~smooth3: ~resonant3 # notch 800 4.0
~out3: ~smooth3 * 0.35

-- Example 4: Narrow notch (high Q)
~input4: white_noise
~narrow_notch: ~input4 # notch 1000 15.0
~out4: ~narrow_notch * 0.15

-- Example 5: Wide notch (low Q)
~input5: white_noise
~wide_notch: ~input5 # notch 2000 0.7
~out5: ~wide_notch * 0.15

-- Example 6: Multiple notches (harmonic removal)
~harmonic6: sine 220 + sine 440 + sine 660 + sine 880 + sine 1100
~notch1: ~harmonic6 # notch 440 5.0
~notch2: ~notch1 # notch 880 5.0
~out6: ~notch2 * 0.2

-- Example 7: Sweep notch (phaser-like effect)
~notch_freq7: sine 0.25 * 1500 + 2000
~phased7: saw 110 # notch ~notch_freq7 3.0
~out7: ~phased7 * 0.3

-- Example 8: Notch on filtered signal
~filtered8: square 110 # lpf 2000 0.8
~notched8: ~filtered8 # notch 1100 6.0
~out8: ~notched8 * 0.25

-- Example 9: Remove mains hum (50Hz or 60Hz)
~music9: tri 330 + tri 440 + tri 550
~mains_hum: sine 50 * 0.3
~with_hum9: ~music9 + ~mains_hum
~dehum9: ~with_hum9 # notch 50 4.0
~out9: ~dehum9 * 0.2

-- Example 10: Cascaded notches for comb effect
~input10: white_noise
~comb1: ~input10 # notch 500 2.0
~comb2: ~comb1 # notch 1000 2.0
~comb3: ~comb2 # notch 1500 2.0
~comb4: ~comb3 # notch 2000 2.0
~out10: ~comb4 * 0.2

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.7 + ~out2 * 0.6 + ~out3 * 0.5 + ~out4 * 0.0 + ~out5 * 0.0 + ~out6 * 0.6 + ~out7 * 0.7 + ~out8 * 0.5 + ~out9 * 0.6 + ~out10 * 0.4
