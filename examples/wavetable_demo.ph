-- Wavetable Oscillator Demo
-- Demonstrates wavetable synthesis with pattern modulation

tempo: 2.0

-- Basic wavetable oscillator at 440Hz
-- Default wavetable is a sine wave (2048 samples)
~simple: wavetable 440

-- Pattern-modulated frequencies (arpegg io)
~arp: wavetable "220 330 440 330"

-- Bass line with envelope
~bass_freq: wavetable "55 55 82.5 110"
~bass_env: adsr 0.01 0.1 0.0 0.05
~bass: ~bass_freq * ~bass_env

-- Lead melody with longer notes
~lead_freq: wavetable "440 550 660 550 440 330 440"
~lead_env: adsr 0.05 0.2 0.3 0.2
~lead: ~lead_freq * ~lead_env * 0.6

-- Pad with slow changes
~pad_freq: wavetable "110 165 220" $ slow 4
~pad_env: adsr 1.0 1.0 0.8 2.0
~pad: ~pad_freq * ~pad_env * 0.3

-- Wavetable through filter (classic synth sound)
~filtered: wavetable 110 # lpf 800 1.5 * 0.5

-- Wavetable with pattern transforms
~transformed: wavetable "220 330" $ fast 4 $ every 4 rev

-- Mix everything
out: (~bass + ~lead * 0.5 + ~pad) * 0.4

-- Try these variations:
-- out: ~simple * 0.3                           -- Simple tone
-- out: ~arp * 0.3                              -- Arpeggio
-- out: ~bass                                   -- Bass line
-- out: ~lead                                   -- Melody
-- out: ~pad                                    -- Ambient pad
-- out: ~filtered                               -- Filtered
-- out: ~transformed * 0.3                      -- With transforms
