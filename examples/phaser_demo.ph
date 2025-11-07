-- Phaser Demo: Classic Spectral Sweeping Effect
-- Phaser creates moving notches in the frequency spectrum using allpass filters
-- Syntax: signal # phaser rate depth feedback stages
--   rate: LFO frequency in Hz (0.05 to 5.0)
--   depth: Modulation depth (0.0 to 1.0)
--   feedback: Resonance amount (0.0 to 0.95)
--   stages: Number of allpass filters (2 to 12, typically 4-6)

tempo: 1.5

-- Example 1: Classic 4-stage phaser (vintage analog sound)
~synth1: saw 110 # phaser 0.4 0.7 0.5 4

-- Example 2: Deep 6-stage phaser (more dramatic sweep)
~synth2: square 220 # phaser 0.3 0.8 0.6 6

-- Example 3: Fast subtle phaser (shimmering effect)
~synth3: tri 165 # phaser 1.5 0.4 0.3 4

-- Example 4: Slow 8-stage phaser (deep space pad)
~pad: saw 82.5 # lpf 800 0.4 # phaser 0.15 0.9 0.7 8

-- Example 5: Pattern-modulated phaser rate (evolving)
~rate_lfo: sine 0.1 * 1.5 + 1.0
~depth_lfo: sine 0.15 * 0.3 + 0.6
~modulated: saw 330 # phaser ~rate_lfo ~depth_lfo 0.5 6

-- Mix all examples
out: (~synth1 + ~synth2 + ~synth3 + ~pad + ~modulated) * 0.15
