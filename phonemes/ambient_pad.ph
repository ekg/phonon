-- Ambient Pad - Slowly evolving soundscape
-- Render with: phonon render ambient_pad.phonon ambient_pad.wav --duration 10

-- Slow LFO for modulation
lfo = sine 0.2 * 0.5

-- Three detuned oscillators for rich pad sound
osc1 = saw 110 # lpf("800 1200 1600 1200", 2)
osc2 = saw 110.5 # lpf("900 1300 1700 1300", 2)
osc3 = saw 109.5 # lpf("700 1100 1500 1100", 2)

-- Mix the oscillators
pad = osc1 * 0.15

-- You can create richer sound by mixing multiple oscillators
-- (parser doesn't support addition of buses yet, so use one at a time)
-- Alternative outputs to try:
-- out osc2 * 0.15
-- out osc3 * 0.15

-- Final output
out pad