-- Pulse Wave (PWM) Oscillator Demonstration
-- Variable pulse width creates different harmonic content

tempo: 2.0

-- Square wave (50% duty cycle - only odd harmonics)
~square_env: ad 0.01 0.2
~square: pulse 220 0.5 * ~square_env * 0.25

-- Narrow pulse (10% - bright, rich harmonics)
~narrow_env: ad 0.005 0.15
~narrow: pulse 440 0.1 * ~narrow_env * 0.2

-- Wide pulse (90% - similar to 10% but inverted)
~wide_env: ad 0.01 0.25
~wide: pulse 165 0.9 * ~wide_env * 0.2

-- Medium pulse width (30% - different timbre)
~medium_env: ad 0.008 0.18
~medium: pulse 330 0.3 * ~medium_env * 0.22

-- Pulse Width Modulation (PWM) - varying timbre
~pwm_lfo: sine 2
~pwm_width: ~pwm_lfo * 0.3 + 0.5
~pwm: pulse 110 ~pwm_width * 0.15

-- Fast PWM (classic analog synth sound)
~fast_lfo: sine 6
~fast_width: ~fast_lfo * 0.4 + 0.5
~fast_pwm: pulse 220 ~fast_width * 0.18

-- Pattern-modulated pulse width
~width_pattern: "0.1 0.3 0.5 0.7 0.9"
~pattern_pulse: pulse 440 ~width_pattern * 0.15

-- Filtered pulse for softer tone
~pulse_raw: pulse 330 0.2
~filtered: ~pulse_raw # lpf 2000 0.7
~filtered_out: ~filtered * 0.2

-- PWM bass with envelope
~bass_lfo: sine 0.25
~bass_width: ~bass_lfo * 0.4 + 0.5
~bass_env: adsr 0.01 0.1 0.5 0.3
~bass: pulse 55 ~bass_width * ~bass_env * 0.25

out: ~square + ~narrow + ~wide + ~medium + ~pwm + ~fast_pwm + ~pattern_pulse + ~filtered_out + ~bass
