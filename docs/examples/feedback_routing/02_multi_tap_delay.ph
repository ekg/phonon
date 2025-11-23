-- Multi-Tap Delay Example
-- Multiple delay taps create dense rhythmic textures
-- Like a delay network or diffusion reverb

tempo: 1.0

-- Source impulse
~source: sine 880 * 0.3

-- Four delay taps with different times
~tap1: ~source # delay 0.037 0.7  -- Early reflection
~tap2: ~source # delay 0.043 0.7  -- Early reflection
~tap3: ~source # delay 0.051 0.7  -- Mid reflection
~tap4: ~source # delay 0.061 0.7  -- Late reflection

-- Mix all taps together (diffuse sound)
out: (~source + ~tap1 + ~tap2 + ~tap3 + ~tap4) * 0.2

-- Try adjusting:
-- - tap times for different textures
-- - feedback amounts (0.7) for longer tails
-- - add filters to each tap for tonal shaping
