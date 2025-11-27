-- Latch (Sample & Hold) Demonstration
-- Classic modular synth building block that samples input when triggered
-- Holds sampled value until next trigger, creating stepped/quantized outputs
-- Perfect for random melodies, stepped filter sweeps, and rhythmic effects

tempo: 0.5

-- Example 1: Random stepped melody
~noise1: white_noise * 200.0 + 440.0
~clock1: impulse 8.0
~freq1: ~noise1 # latch ~clock1
~melody1: sine ~freq1
~out1: ~melody1 * 0.2

-- Example 2: Stepped filter sweep
~carrier2: saw 110
~sweep2: sine 0.5 * 0.5 + 0.5
~s_h_clock2: impulse 16.0
~stepped_sweep2: ~sweep2 # latch ~s_h_clock2
~cutoff2: ~stepped_sweep2 * 2000.0 + 200.0
~filtered2: ~carrier2 # lpf ~cutoff2 0.8
~out2: ~filtered2 * 0.3

-- Example 3: Quantized tremolo
~carrier3: saw 165
~trem_lfo3: sine 3
~trem_clock3: impulse 12.0
~stepped_trem3: ~trem_lfo3 # latch ~trem_clock3
~tremolo3: ~carrier3 * (~stepped_trem3 * 0.5 + 0.5)
~out3: ~tremolo3 * 0.25

-- Example 4: Rhythmic sample & hold effect
~bass4: saw 55
~sh_source4: white_noise
~sh_clock4: impulse 4.0
~sh_out4: ~sh_source4 # latch ~sh_clock4
~mod_freq4: (~sh_out4 * 0.5 + 0.5) * 1000.0 + 200.0
~modulated4: ~bass4 # lpf ~mod_freq4 1.2
~out4: ~modulated4 * 0.3

-- Example 5: Stepped arpeggio
~arp_input5: sine 0.25 * 300.0 + 440.0
~arp_clock5: impulse 16.0
~arp_freq5: ~arp_input5 # latch ~arp_clock5
~arp5: sine ~arp_freq5
~out5: ~arp5 * 0.2

-- Example 6: Dual sample & hold (stereo-like effect)
~source6: white_noise * 150.0 + 300.0
~clock_a6: impulse 10.0
~clock_b6: impulse 7.0
~freq_a6: ~source6 # latch ~clock_a6
~freq_b6: ~source6 # latch ~clock_b6
~voice_a6: sine ~freq_a6
~voice_b6: saw ~freq_b6
~out6: (~voice_a6 + ~voice_b6) * 0.15

-- Example 7: Stepped resonance modulation
~carrier7: saw 110
~res_lfo7: sine 1 * 0.5 + 0.5
~res_clock7: impulse 8.0
~stepped_res7: ~res_lfo7 # latch ~res_clock7
~resonant7: ~carrier7 # lpf 800.0 (~stepped_res7 * 3.0 + 0.5)
~out7: ~resonant7 * 0.25

-- Example 8: Random rhythm generator
~rhythm_noise8: white_noise
~rhythm_clock8: impulse 16.0
~rhythm_gate8: ~rhythm_noise8 # latch ~rhythm_clock8 # schmidt 0.3 -0.3
~kick8: sine 55 * ~rhythm_gate8
~out8: ~kick8 * 0.3

-- Example 9: Stepped amplitude modulation
~carrier9: saw 220
~am_source9: sine 2 * 0.5 + 0.5
~am_clock9: impulse 8.0
~am_level9: ~am_source9 # latch ~am_clock9
~modulated9: ~carrier9 * ~am_level9
~out9: ~modulated9 * 0.25

-- Example 10: Complex random sequence
~seq_noise10: white_noise * 300.0 + 220.0
~seq_clock_a10: impulse 8.0
~seq_clock_b10: impulse 12.0
~freq_a10: ~seq_noise10 # latch ~seq_clock_a10
~freq_b10: ~seq_noise10 # latch ~seq_clock_b10
~osc_a10: sine ~freq_a10
~osc_b10: saw ~freq_b10
~mixed10: ~osc_a10 + ~osc_b10
~out10: ~mixed10 * 0.15

-- Mix all examples (adjust weights for different emphasis)
out: ~out1 * 0.6 + ~out2 * 0.7 + ~out3 * 0.5 + ~out4 * 0.6 + ~out5 * 0.5 + ~out6 * 0.6 + ~out7 * 0.5 + ~out8 * 0.4 + ~out9 * 0.6 + ~out10 * 0.7
