-- FM (Frequency Modulation) Synthesis Demonstration
-- Classic FM synthesis with carrier, modulator, and modulation index

tempo: 2.0

-- Classic FM bell tone
-- Carrier:Modulator ratio of 1:2 creates harmonic bell timbre
~bell_env: ad 0.005 0.3
~bell: fm 440 880 2.5 * ~bell_env * 0.3

-- Electric piano tone
-- Low modulation index with harmonic ratio
~ep_env: adsr 0.01 0.1 0.3 0.2
~ep: fm 220 220 1.2 * ~ep_env * 0.25

-- Brass-like tone
-- Higher modulation index creates brighter sound
~brass_env: ad 0.02 0.4
~brass: fm 110 110 4.0 * ~brass_env * 0.2

-- Inharmonic FM (bell-like, metallic)
-- Non-integer ratio creates inharmonic partials
~metal_env: ad 0.003 0.5
~metal: fm 330 253 3.5 * ~metal_env * 0.15

-- Pattern-modulated modulation index creates dynamic timbre
~index_var: "1.0 3.0 2.0 4.0"
~dynamic: fm 440 220 ~index_var * 0.2

-- Frequency sweep with FM
~carrier_sweep: line 200 800
~fm_sweep: fm ~carrier_sweep 100 2.0 * 0.15

out: ~bell + ~ep + ~brass + ~metal + ~dynamic + ~fm_sweep
