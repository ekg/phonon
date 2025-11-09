-- Flanger Effect Demo
-- Classic "swooshing" modulation effect
--
-- Flanger creates a comb-filtering effect by mixing the input
-- with a delayed copy where the delay time is modulated by an LFO.
--
-- Parameters: flanger depth rate feedback
-- - depth: Modulation depth (0.0-2.0, higher = more intense)
-- - rate: LFO rate in Hz (0.1-10.0, typical 0.5-2.0)
-- - feedback: Feedback amount (0.0-0.95, higher = more resonance)

tempo: 2.0

-- Example 1: Flanged sawtooth pad
~pad: saw "110 165 220" * 0.15
~flanged_pad: ~pad # flanger 1.0 0.5 0.7
out1: ~flanged_pad

-- Example 2: Flanged drums (classic tape flanging sound)
-- ~drums: s "bd sn hh*4 cp"
-- ~flanged_drums: ~drums # flanger 0.8 0.3 0.5
-- out2: ~flanged_drums

-- Example 3: Subtle flange on hi-hats
-- ~hats: s "hh*16" # gain 0.3
-- ~subtle_flange: ~hats # flanger 0.3 2.0 0.3
-- out3: ~subtle_flange

-- Example 4: Deep flange with pattern-controlled rate
-- ~bass: saw 55 * 0.2
-- ~rate_pattern: sine 0.1 * 0.4 + 0.6  -- Varies between 0.2-1.0 Hz
-- ~deep_flange: ~bass # flanger 1.5 ~rate_pattern 0.8
-- out4: ~deep_flange

-- Example 5: Metallic flange with high feedback
-- ~synth: square 220 * 0.1
-- ~metallic: ~synth # flanger 1.2 1.0 0.9
-- out5: ~metallic
