-- Lag (Exponential Slew Limiter) Demonstration
-- Smooths abrupt changes with exponential approach to target
-- Useful for portamento, click removal, parameter smoothing

tempo: 2.0

-- Example 1: Portamento (pitch glide)
~notes: "220 330 440 550"
~smooth_freq: lag ~notes 0.05
~porta_tone: sine ~smooth_freq
~porta_out: ~porta_tone * 0.25

-- Example 2: Click removal (smooth gate)
~gate: "0.0 1.0 0.0 1.0"
~smooth_gate: lag ~gate 0.01
~click_free: sine 440 * ~smooth_gate * 0.2

-- Example 3: Very fast lag (almost bypass)
~fast_input: square 220
~fast_lag: lag ~fast_input 0.0001
~fast_out: ~fast_lag * 0.15

-- Example 4: Slow lag (heavy smoothing)
~slow_input: square 110
~slow_lag: lag ~slow_input 0.2
~slow_out: ~slow_lag * 0.2

-- Example 5: Lag on filter cutoff (smooth sweep)
~sweep_mod: line 0.0 1.0
~smooth_sweep: lag ~sweep_mod 0.1
~sweep_freq: ~smooth_sweep * 3000 + 200
~sweep_tone: saw 110 # lpf ~sweep_freq 0.8
~sweep_out: ~sweep_tone * 0.25

-- Example 6: Lag on FM modulation index
~mod_index_raw: "0.5 5.0"
~mod_index_smooth: lag ~mod_index_raw 0.05
~fm_smooth: fm 220 440 ~mod_index_smooth
~fm_out: ~fm_smooth * 0.15

-- Example 7: Lag for tremolo smoothing
~trem_lfo_raw: square 4.0
~trem_lfo: lag ~trem_lfo_raw 0.02
~trem_amount: ~trem_lfo * 0.5 + 0.5
~trem_tone: sine 330
~trem_out: ~trem_tone * ~trem_amount * 0.2

-- Example 8: Cascaded lags (ultra-smooth)
~input: "0.0 1.0"
~lag1: lag ~input 0.05
~lag2: lag ~lag1 0.05
~cascaded_tone: sine 440 * ~lag2 * 0.2

-- Example 9: Lag on pulse width (smooth PWM)
~pwm_mod_raw: sine 0.5
~pwm_mod: lag ~pwm_mod_raw 0.03
~pwm_width: ~pwm_mod * 0.4 + 0.5
~pwm_osc: pulse 110 ~pwm_width
~pwm_out: ~pwm_osc * 0.15

-- Example 10: Lag with reverb (smooth ambience)
~verb_input: "0.0 0.5 1.0 0.5"
~verb_smooth: lag ~verb_input 0.08
~verb_tone: tri 440
~verb_wet: reverb ~verb_tone 0.4 0.5
~verb_out: ~verb_wet * ~verb_smooth * 0.2

-- Mix all examples
out: ~porta_out * 0.8 + ~click_free * 0.7 + ~fast_out * 0.0 + ~slow_out * 0.6 + ~sweep_out * 0.5 + ~fm_out * 0.4 + ~trem_out * 0.6 + ~cascaded_tone * 0.5 + ~pwm_out * 0.4 + ~verb_out * 0.3
