-- Hadamard Diffuser Demo
-- A multi-channel diffusion network for high-quality reverb

-- Simple demonstration
cps: 2.0

-- Dry kick drum
~kick $ s "bd"

-- Kick with light diffusion (0.3)
~diffused_light $ ~kick # diffuser 0.3

-- Kick with heavy diffusion (0.9) - creates dense cloud of reflections
~diffused_heavy $ ~kick # diffuser 0.9

-- Mix: dry + light diffusion
out $ ~kick * 0.5 + ~diffused_light * 0.3

-- Try this: switch to heavy diffusion for a more reverberant sound
-- out $ ~kick * 0.3 + ~diffused_heavy * 0.5

-- Example: Snare with diffusion creates natural room ambience
-- ~snare $ s "sn"
-- ~diffused_snare $ ~snare # diffuser 0.7
-- out $ ~snare * 0.4 + ~diffused_snare * 0.4

-- Example: Modulated diffusion amount
-- ~lfo $ sine 0.25
-- ~var_diffusion $ ~kick # diffuser (~lfo * 0.5 + 0.5)
-- out $ ~var_diffusion

-- Technical details:
-- - 8 parallel channels with 4 cascaded diffusion steps
-- - Each step: variable delays → channel shuffle → Hadamard transform
-- - diffusion=0.0: pure delays (minimal spreading)
-- - diffusion=1.0: maximum allpass character (dense reflections)
-- - Creates 8^4 = 4096 virtual reflections from a single impulse
