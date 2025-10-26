-- AD (Attack-Decay) Envelope Demonstration
-- Perfect for percussive sounds - attack then decay to silence

tempo: 2.0

-- Classic kick drum: quick attack, medium decay on low sine
~kick_env: ad 0.005 0.15
~kick: sine 55 * ~kick_env * 0.8

-- Snare: fast attack, quick decay on noise + tone
~snare_env: ad 0.003 0.08
~snare_tone: sine 200 * ~snare_env * 0.3
~snare_noise: noise 0 * ~snare_env * 0.15

-- Hi-hat: instant attack, very short decay
~hat_env: ad 0.001 0.05
~hat: noise 0 * ~hat_env * 0.2

-- Bass pluck: medium attack, long decay
~bass_env: ad 0.02 0.4
~bass: saw 110 * ~bass_env * 0.5

-- Pattern-modulated decay time creates variation
~decay_var: "0.1 0.3 0.2 0.4"
~var_env: ad 0.01 ~decay_var
~melody: square 440 * ~var_env * 0.3

out: ~kick + ~snare_tone + ~snare_noise + ~hat + ~bass + ~melody
