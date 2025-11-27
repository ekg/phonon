-- ASR (Attack-Sustain-Release) Envelope Demonstration
-- Gate-based envelope: attacks when gate rises, sustains while high, releases when gate falls
-- Perfect for organ-style sounds, held notes, and gated synthesis
-- Unlike ADSR, sustain holds indefinitely while gate is high

tempo: 0.5

-- Example 1: Classic organ tone
~organ_gate: "1.0 1.0 0.0 0.0"
~organ_env: asr ~organ_gate 0.05 0.1
~organ_tone: sine 220 + sine 440 + sine 660
~organ_out: ~organ_tone * ~organ_env * 0.2

-- Example 2: Fast attack, slow release (pad)
~pad_gate: "1.0 0.0 1.0 0.0"
~pad_env: asr ~pad_gate 0.001 0.5
~pad_tone: saw 110 # lpf 800 0.7
~pad_out: ~pad_tone * ~pad_env * 0.3

-- Example 3: Slow attack, fast release (swell)
~swell_gate: "1.0 0.0 0.0 0.0"
~swell_env: asr ~swell_gate 0.3 0.01
~swell_tone: tri 330
~swell_out: ~swell_tone * ~swell_env * 0.25

-- Example 4: Gated rhythmic pattern
~rhythm_gate: "1.0 0.0 1.0 1.0"
~rhythm_env: asr ~rhythm_gate 0.01 0.05
~rhythm_tone: square 165
~rhythm_out: ~rhythm_tone * ~rhythm_env * 0.2

-- Example 5: Sustained chord
~chord_gate_const: sine 0.0
~chord_gate: ~chord_gate_const * 0.0 + 1.0
~chord_env: asr ~chord_gate 0.1 0.2
~chord_tone: sine 220 + sine 277 + sine 330 + sine 440
~chord_out: ~chord_tone * ~chord_env * 0.15

-- Example 6: ASR with filter modulation
~filter_gate: "1.0 1.0 0.0 1.0"
~filter_env: asr ~filter_gate 0.02 0.15
~filter_source: saw 110
~filter_cutoff: ~filter_env * 3000 + 200
~filtered: ~filter_source # lpf ~filter_cutoff 0.8
~filter_out: ~filtered * 0.3

-- Example 7: Percussive with quick attack/release
~perc_gate: "1.0 0.0 0.0 1.0"
~perc_env: asr ~perc_gate 0.001 0.1
~perc_tone: sine 55
~perc_out: ~perc_tone * ~perc_env * 0.4

-- Example 8: Tremolo effect with ASR
~trem_gate: "1.0 1.0 1.0 0.0"
~trem_env: asr ~trem_gate 0.05 0.1
~trem_lfo: sine 6.0 * 0.3 + 0.7
~trem_tone: sine 440
~trem_out: ~trem_tone * ~trem_env * ~trem_lfo * 0.2

-- Example 9: Dual envelope (amplitude + filter)
~dual_gate: "1.0 0.0 1.0 1.0"
~amp_env: asr ~dual_gate 0.02 0.2
~filt_env: asr ~dual_gate 0.05 0.3
~dual_source: saw 165
~dual_cutoff: ~filt_env * 2000 + 500
~dual_filtered: ~dual_source # lpf ~dual_cutoff 0.7
~dual_out: ~dual_filtered * ~amp_env * 0.3

-- Example 10: Long sustain with reverb
~verb_gate: "1.0 1.0 1.0 1.0"
~verb_env: asr ~verb_gate 0.1 0.5
~verb_tone: tri 220
~verb_wet: reverb ~verb_tone 0.6 0.5
~verb_out: ~verb_wet * ~verb_env * 0.2

-- Mix all examples (adjust weights for different emphasis)
out: ~organ_out * 0.7 + ~pad_out * 0.5 + ~swell_out * 0.6 + ~rhythm_out * 0.5 + ~chord_out * 0.4 + ~filter_out * 0.6 + ~perc_out * 0.5 + ~trem_out * 0.4 + ~dual_out * 0.5 + ~verb_out * 0.3
