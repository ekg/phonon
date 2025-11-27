-- Pan2 (Stereo Panning) Demonstration
-- Equal-power panning: constant perceived loudness across stereo field
-- Position: -1.0 = hard left, 0.0 = center, 1.0 = hard right

tempo: 0.5

-- Example 1: Static hard left panning
~bass: saw 55 * 0.3
~bass_left: pan2_l ~bass -1.0
~bass_right: pan2_r ~bass -1.0

-- Example 2: Static hard right panning
~lead: square 440 * 0.2
~lead_left: pan2_l ~lead 1.0
~lead_right: pan2_r ~lead 1.0

-- Example 3: Static center panning (equal power to both channels)
~pad: tri 220 * 0.15
~pad_left: pan2_l ~pad 0.0
~pad_right: pan2_r ~pad 0.0

-- Example 4: Partial left panning
~sub: sine 110 * 0.25
~sub_left: pan2_l ~sub -0.5
~sub_right: pan2_r ~sub -0.5

-- Example 5: LFO-modulated auto-pan
~hihat: (white_noise # hpf 8000 0.5) * 0.1
~pan_lfo: sine 0.5
~hihat_left: pan2_l ~hihat ~pan_lfo
~hihat_right: pan2_r ~hihat ~pan_lfo

-- Example 6: Pattern-controlled pan positions
~snare: (white_noise # bpf 2000 0.8) * 0.2
~pan_pattern: "-1.0 -0.3 0.3 1.0"
~snare_left: pan2_l ~snare ~pan_pattern
~snare_right: pan2_r ~snare ~pan_pattern

-- Example 7: Stereo width with complementary panning
~chord1: saw 220 * 0.12
~chord2: saw 277 * 0.12
~chord3: saw 330 * 0.12
~chord1_l: pan2_l ~chord1 -0.7
~chord1_r: pan2_r ~chord1 -0.7
~chord2_l: pan2_l ~chord2 0.0
~chord2_r: pan2_r ~chord2 0.0
~chord3_l: pan2_l ~chord3 0.7
~chord3_r: pan2_r ~chord3 0.7

-- Example 8: Stereo noise with panning
~noise_left_src: (pink_noise # lpf 1000 0.6) * 0.08
~noise_right_src: (pink_noise # lpf 1000 0.6) * 0.08
~noise_left: pan2_l ~noise_left_src -0.6
~noise_right: pan2_r ~noise_right_src 0.6

-- Example 9: Rhythmic auto-pan with triangle LFO
~kick: (brown_noise # lpf 80 0.7) * 0.3
~tri_pan: tri 1.0
~kick_left: pan2_l ~kick ~tri_pan
~kick_right: pan2_r ~kick ~tri_pan

-- Example 10: Extreme panning (position > 1.0 clamped to 1.0)
~test_clamp: sine 880 * 0.15
~test_left: pan2_l ~test_clamp 5.0
~test_right: pan2_r ~test_clamp 5.0

-- Mix all examples to stereo outputs
out1: ~bass_left + ~lead_left + ~pad_left + ~sub_left + ~hihat_left + ~snare_left + ~chord1_l + ~chord2_l + ~chord3_l + ~noise_left + ~kick_left * 0.5 + ~test_left * 0.0
out2: ~bass_right + ~lead_right + ~pad_right + ~sub_right + ~hihat_right + ~snare_right + ~chord1_r + ~chord2_r + ~chord3_r + ~noise_right + ~kick_right * 0.5 + ~test_right * 0.0
