-- Live MIDI Piano (Composable Polyphonic Synthesis)
-- Your AXIS-49 keyboard should auto-connect now
-- If not: Alt+M cycles through MIDI devices

tempo: 0.5

-- NEW SYNTAX: saw ~midi creates a polyphonic MIDI synth
-- Each note-on triggers a new voice with its own oscillator and ASR envelope
-- Note-off releases the voice's envelope naturally

-- Simple piano: saw wave with automatic per-voice ASR envelope
-- Now with customizable attack/release via keyword args!
~piano $ sine ~midi :attack 0.1 :release 2.0

-- With filter for warmer sound
~warm $ ~piano # bitcrush 4 800
-- Add some reverb (shared effects)
-- # delay 0.3 0.9
~verb $ ~warm 
-- Output
out $ ~verb * 0.6



-- WORKFLOW:
-- 1. C-x to evaluate (should hear your keyboard instantly)
-- 2. Alt+M if no sound (cycles MIDI devices)
-- 3. Play your AXIS-49!
--
-- The new syntax is COMPOSABLE:
-- saw ~midi     -> polyphonic saw oscillator
-- sine ~midi    -> polyphonic sine oscillator
-- tri ~midi     -> polyphonic triangle oscillator
-- square ~midi  -> polyphonic square oscillator
--
-- Effects chain after the oscillator is shared (all voices go through same filter/reverb)
