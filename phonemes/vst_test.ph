-- VST3 Effect Plugin Test
-- Tests Surge XT Effects processing audio input

cps: 1.0

-- Create audio source (saw wave at 220 Hz)
~source $ saw 220

-- Process through VST effect (Surge XT Effects)
~effected $ ~source # vst "Surge XT Effects"

out $ ~effected * 0.5
