-- Granular Synthesis Demo
-- Breaks audio into small grains and overlaps them with varying parameters

tempo: 1.0

-- PARAMETERS: granular source grain_size_ms density pitch
-- source: input audio to granulate
-- grain_size_ms: grain duration (5-500ms)
-- density: grain spawn rate (0.0=sparse, 1.0=dense/continuous)
-- pitch: playback speed/pitch multiplier (0.1-4.0, 1.0=normal)

-- Basic granular synthesis from a sine wave
~source: sine 220
~basic: granular ~source 50 0.5 1.0

-- Ambient pad: large grains, high density, slow source
~pad_source: sine "110 165 220" $ slow 8
~ambient: granular ~pad_source 100 0.8 0.9

-- Rhythmic texture: varying density creates rhythm
~density_pattern: "0.9 0.3 0.6 0.2"
~rhythmic: granular (sine 220) 30 ~density_pattern 1.0

-- Pitch shifting: different grain sizes create different textures
~pitch_shifted: granular (sine 330) 50 0.6 2.0   -- Double speed/pitch

-- Evolving texture: modulate grain size
~grain_size_pattern: "25 50 100 50"
~evolving: granular (sine 440) ~grain_size_pattern 0.7 1.0

-- Granulate a complex source (sawtooth with harmonics)
~rich_source: saw 110 # lpf 2000 1.0
~granulated_saw: granular ~rich_source 75 0.6 1.2

-- Cloud texture: high density, variable pitch
~cloud: granular (sine "165 220 275") 40 0.9 "0.8 1.2 1.0"

-- Sparse grains for rhythmic clicks
~sparse: granular (sine 550) 10 0.1 1.0

-- Output: choose your texture!
out: ~ambient * 0.3

-- Try these variations:
-- out: ~basic * 0.4                              -- Basic granular
-- out: ~ambient * 0.3                            -- Ambient pad
-- out: ~rhythmic * 0.4                           -- Rhythmic pattern
-- out: ~pitch_shifted * 0.3                      -- Pitch shifted
-- out: ~evolving * 0.3                           -- Evolving texture
-- out: ~granulated_saw * 0.25                    -- Rich harmonics
-- out: ~cloud * 0.25                             -- Dense cloud
-- out: ~sparse * 0.5                             -- Sparse clicks

-- CREATIVE TIPS:
-- - Small grains (5-20ms): glitchy, metallic textures
-- - Medium grains (30-70ms): smooth, continuous textures
-- - Large grains (80-200ms): recognizable source material
-- - Low density (0.1-0.3): sparse, rhythmic
-- - High density (0.7-1.0): continuous, pad-like
-- - Pitch < 1.0: lower, slower playback
-- - Pitch > 1.0: higher, faster playback
-- - Pattern modulation creates evolution and movement
