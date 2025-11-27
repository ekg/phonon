-- Moog Ladder Filter Demonstration
-- Classic 4-pole 24dB/octave lowpass with resonance

tempo: 0.5

-- Classic Moog bass sound (low cutoff, high resonance)
~bass: saw 55
~moog_bass: moog_ladder ~bass 400 0.7
~bass_env: ad 0.01 0.3
~bass_out: ~moog_bass * ~bass_env * 0.35

-- Sweeping filter with pattern-modulated cutoff
~sweep_pattern: "500 1000 2000 4000"
~synth: saw 110
~swept: moog_ladder ~synth ~sweep_pattern 0.6
~sweep_env: ad 0.005 0.25
~sweep_out: ~swept * ~sweep_env * 0.25

-- High resonance for squelchy acid sound
~acid: saw 82.5
~squelchy: moog_ladder ~acid 800 0.85
~acid_env: ad 0.01 0.2
~acid_out: ~squelchy * ~acid_env * 0.3

-- Pattern-modulated resonance (breathing effect)
~res_pattern: "0.2 0.5 0.8 0.95"
~pad: tri 220
~breathing: moog_ladder ~pad 1000 ~res_pattern
~breathing_env: adsr 0.05 0.1 0.7 0.3
~breathing_out: ~breathing * ~breathing_env * 0.2

-- Filtered white noise for hi-hat/cymbals
~noise: white_noise
~filtered_noise: moog_ladder ~noise 3000 0.4
~noise_env: ad 0.001 0.05
~noise_out: ~filtered_noise * ~noise_env * 0.15

-- Classic lead sound with moderate filtering
~lead: saw 330
~lead_filt: moog_ladder ~lead 1200 0.5
~lead_env: ad 0.005 0.2
~lead_out: ~lead_filt * ~lead_env * 0.25

-- Cascaded Moog filters for steeper rolloff
~raw: saw 165
~stage1: moog_ladder ~raw 800 0.3
~stage2: moog_ladder ~stage1 800 0.3
~cascade_env: ad 0.015 0.3
~cascade_out: ~stage2 * ~cascade_env * 0.25

-- Self-oscillating filter (very high resonance)
~tiny_input: sine 110 * 0.01
~self_osc: moog_ladder ~tiny_input 1500 0.98
~osc_env: ad 0.02 0.35
~osc_out: ~self_osc * ~osc_env * 0.2

-- Moog ladder on FM tone
~carrier: sine 440
~modulator: sine 220
~fm_tone: fm ~carrier ~modulator 2.5
~fm_moog: moog_ladder ~fm_tone 2000 0.6
~fm_env: ad 0.01 0.3
~fm_out: ~fm_moog * ~fm_env * 0.22

-- Dynamic cutoff sweep using LFO
~lfo: sine 0.5
~cutoff_sweep: ~lfo * 1500 + 2000
~square: square 165
~dynamic_sweep: moog_ladder ~square ~cutoff_sweep 0.7
~dynamic_env: ad 0.02 0.35
~dynamic_out: ~dynamic_sweep * ~dynamic_env * 0.2

out: ~bass_out + ~sweep_out + ~acid_out + ~breathing_out + ~noise_out + ~lead_out + ~cascade_out + ~osc_out + ~fm_out + ~dynamic_out
