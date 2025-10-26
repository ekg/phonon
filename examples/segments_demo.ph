-- Segments Envelope Demonstration
-- Arbitrary breakpoint envelopes with linear interpolation
-- Create complex envelopes with any number of stages

tempo: 2.0

-- Example 1: ADSR-style envelope (4 breakpoints)
~adsr1: segments "0 1 0.7 0" "0.1 0.2 0.3"
~tone1: sine 440
~shaped1: ~tone1 * ~adsr1
~out1: ~shaped1 * 0.3

-- Example 2: Percussion envelope (fast attack, exponential-like decay)
~perc2: segments "0 1 0.3 0.1 0" "0.01 0.1 0.1 0.2"
~osc2: sine 220
~hit2: ~osc2 * ~perc2
~out2: ~hit2 * 0.4

-- Example 3: Complex filter sweep (5 breakpoints)
~filter_env3: segments "200 2000 500 3000 300" "0.2 0.2 0.3 0.3"
~carrier3: saw 110
~filtered3: ~carrier3 # lpf ~filter_env3 0.7
~out3: ~filtered3 * 0.25

-- Example 4: Wobbling amplitude modulation (6 breakpoints)
~wobble4: segments "0 1 0.5 1 0.3 0" "0.1 0.1 0.1 0.1 0.2"
~bass4: saw 55
~wobbled4: ~bass4 * ~wobble4
~out4: ~wobbled4 * 0.3

-- Example 5: Stepped envelope (many short segments for sequencer-like effect)
~steps5: segments "0 0.3 0.6 0.9 0.6 0.3 0.1 0" "0.08 0.08 0.08 0.08 0.08 0.08 0.08"
~synth5: square 330
~stepped5: ~synth5 * ~steps5
~out5: ~stepped5 * 0.25

-- Mix all examples
out: ~out1 * 0.5 + ~out2 * 0.6 + ~out3 * 0.5 + ~out4 * 0.5 + ~out5 * 0.4
