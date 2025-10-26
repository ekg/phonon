-- ADSR Envelope Demonstration
-- Shows how ADSR shapes amplitude over time

tempo: 2.0

-- Classic ADSR envelope: quick attack, smooth decay, moderate sustain, slow release
~env: adsr 0.01 0.1 0.6 0.3

-- Apply envelope to a saw wave
~bass: saw 55 * ~env

-- Apply same envelope to a higher frequency
~lead: saw 440 * ~env * 0.3

-- Different envelope: slower attack, no sustain, long release
~pad_env: adsr 0.2 0.1 0.0 0.5
~pad: saw 220 * ~pad_env * 0.2

-- Pattern-modulated envelope parameters
-- Attack time alternates between fast and slow
~dynamic_attack: "0.01 0.2"
~dynamic_env: adsr ~dynamic_attack 0.1 0.5 0.2
~dynamic: square 330 * ~dynamic_env * 0.15

out: ~bass * 0.6 + ~lead * 0.4 + ~pad * 0.3 + ~dynamic * 0.5
