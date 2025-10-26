-- Line Envelope Demonstration
-- Linear ramps for fades, sweeps, and automation

tempo: 1.0

-- Fade in from silence to full volume
~fade_in: line 0 1
~melody: sine 440 * ~fade_in * 0.4

-- Fade out (reverse - from 1 to 0)
~fade_out: line 1 0
~bass: saw 110 * ~fade_out * 0.3

-- Frequency sweep using Line
~freq_sweep: line 200 800
~sweep: sine ~freq_sweep * 0.2

-- Cutoff sweep for filter
~cutoff_ramp: line 200 4000
~filtered: saw 220 # lpf ~cutoff_ramp 0.7
~filtered_out: ~filtered * 0.25

-- Pattern-modulated end value creates varying ramps
~end_var: "0.5 1.0"
~var_ramp: line 0 ~end_var
~variation: square 330 * ~var_ramp * 0.2

out: ~melody + ~bass + ~sweep + ~filtered_out + ~variation
