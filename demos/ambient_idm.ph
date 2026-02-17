-- 🌌 AMBIENT/IDM TEXTURE LIBRARY
-- Research-based patterns for ambient textures and IDM rhythms
-- Based on AMBIENT_IDM_RESEARCH.md

-- ============================================
-- SECTION 1: AMBIENT DRONE PATTERNS
-- ============================================

-- Tempo: Very slow for ambient (60-90 BPM equivalent)
tempo: 0.5

-- PATTERN 1: EVOLVING DRONE
-- Multiple asynchronous LFOs create ever-changing texture
-- (The pattern never exactly repeats due to conflicting LFO periods)
~drone_base: saw "55 55"
~lfo_slow: sine 0.07      -- 14 second cycle
~lfo_med: sine 0.13       -- 7.7 second cycle
~lfo_fast: sine 0.31      -- 3.2 second cycle
~evolving_drone: ~drone_base # lpf (~lfo_slow * 500 + ~lfo_med * 300 + 800) (~lfo_fast * 0.3 + 0.5)

-- PATTERN 2: DEEP SUB DRONE
-- Pure low-end foundation with gentle modulation
~sub_drone: sine 55 * 0.4
~sub_lfo: sine 0.05
~sub_drone_mod: ~sub_drone * (~sub_lfo * 0.2 + 0.8)

-- PATTERN 3: HARMONIC STACK DRONE
-- Multiple octaves layered for richness
~stack_fund: saw 55 * 0.3
~stack_oct1: saw 110 * 0.2
~stack_oct2: saw 220 * 0.1
~stack_lfo: sine 0.09
~harmonic_drone: (~stack_fund + ~stack_oct1 + ~stack_oct2) # lpf (~stack_lfo * 600 + 500) 0.4

-- PATTERN 4: DETUNED PAD DRONE
-- Supersaw creates thick, evolving texture
~pad_lfo: sine 0.11
~detuned_pad: supersaw "110 110" 0.6 # lpf (~pad_lfo * 800 + 600) 0.5

-- ============================================
-- SECTION 2: EVOLVING TEXTURES
-- ============================================

-- PATTERN 5: GRANULAR CLOUD
-- Pitch-shifted grains creating atmospheric clouds
~cloud_source: pink_noise
~cloud_grains: granular ~cloud_source "40 60 80" 0.7 "0.8 1.0 1.2"
~granular_cloud: ~cloud_grains # lpf 2000 0.3 * 0.4

-- PATTERN 6: FROZEN TEXTURE
-- Long-sustain granular for static textures
~freeze_source: saw "165 220 275" $ slow 8
~frozen_texture: granular ~freeze_source 100 0.9 0.7 * 0.3

-- PATTERN 7: SPECTRAL AIR
-- High-frequency noise bed for "air" quality
~air_noise: pink_noise # hpf 4000 0.2
~air_verb: reverb ~air_noise 0.9 0.8 0.5
~spectral_air: ~air_verb * 0.15

-- PATTERN 8: SLOWLY MORPHING PAD
-- Filter sweep across multiple octaves
~morph_lfo1: sine 0.03   -- 33 second cycle
~morph_lfo2: sine 0.07   -- 14 second cycle
~morph_source: supersaw "82.5 110" 0.4
~morphing_pad: ~morph_source # lpf (~morph_lfo1 * 1500 + ~morph_lfo2 * 500 + 400) 0.6

-- PATTERN 9: SHIMMER TEXTURE
-- Layered pitches creating ethereal quality
~shim_low: sine "110 165" * 0.3
~shim_mid: sine "220 330" * 0.2
~shim_high: sine "440 660" * 0.1
~shim_lfo: sine 0.17
~shimmer_texture: (~shim_low + ~shim_mid + ~shim_high) * (~shim_lfo * 0.3 + 0.7)

-- ============================================
-- SECTION 3: IDM RHYTHMIC PATTERNS
-- ============================================

-- (Switch to IDM tempo for rhythmic patterns)
-- tempo: 2.0   -- Uncomment for 120 BPM

-- PATTERN 10: EUCLIDEAN COMPLEXITY
-- Polyrhythmic Euclidean patterns for mathematical rhythms
~euclid_kick: s "bd(3,8)"
~euclid_snare: s "sn(5,16,2)"
~euclid_hat: s "hh(7,12)"
~euclidean_drums: ~euclid_kick * 0.8 + ~euclid_snare * 0.6 + ~euclid_hat * 0.4

-- PATTERN 11: PROBABILISTIC GLITCH
-- Random event drops for controlled chaos
~glitch_base: s "bd sn hh hh cp hh sn hh"
~glitch_drops: ~glitch_base $ degrade_by 0.3
~probabilistic_glitch: ~glitch_drops $ sometimes rev

-- PATTERN 12: STUTTER MACHINE
-- Rapid repetition for machine-gun effects
~stutter_src: s "bd sn"
~stutter_4x: ~stutter_src $ stutter 4
~stutter_machine: ~stutter_4x $ every 4 (fast 2)

-- PATTERN 13: POLYRHYTHMIC LAYERS
-- 3 against 4 against 7 for complex phasing
~poly_3: s "bd ~ bd" $ fast 3
~poly_4: s "sn ~ ~ sn" $ fast 4
~poly_7: s "hh*7" $ fast 7
~polyrhythmic_layers: ~poly_3 + ~poly_4 * 0.5 + ~poly_7 * 0.3

-- PATTERN 14: MICRO-TIMED BEATS
-- Subtle timing shifts for human feel
~micro_kick: s "bd ~ ~ bd ~ bd ~ ~"
~micro_snare: s "~ ~ sn ~ ~ ~ sn ~"
~micro_hats: s "hh*16" * 0.4
~micro_timed: (~micro_kick + ~micro_snare + ~micro_hats) $ swing 0.15

-- ============================================
-- SECTION 4: HYBRID AMBIENT-IDM
-- ============================================

-- PATTERN 15: RHYTHMIC TEXTURE
-- Ambient pad with subtle IDM pulse
~rt_pad: supersaw "55 82.5" 0.5 # lpf (sine 0.1 * 800 + 600) 0.4
~rt_pulse: s "bd ~ ~ ~ ~ bd ~ ~" * 0.3
~rhythmic_texture: ~rt_pad * 0.4 + ~rt_pulse

-- PATTERN 16: GRANULAR RHYTHM
-- Granular synthesis with rhythmic density modulation
~gr_density: "0.9 0.3 0.7 0.2 0.8 0.4"
~gr_source: saw 110
~granular_rhythm: granular ~gr_source 30 ~gr_density 1.0 * 0.4

-- PATTERN 17: SPARSE MELODIC IDM
-- Melodic fragments with probabilistic triggering
~sparse_base: sine "~ 220 ~ ~ 330 ~ 440 ~"
~sparse_proc: ~sparse_base $ degrade_by 0.4 $ every 3 rev
~sparse_verb: reverb ~sparse_proc 0.7 0.8 0.4
~sparse_melodic: ~sparse_verb * 0.3

-- PATTERN 18: DIGITAL MIST
-- Layered ambient with subtle rhythmic elements
~mist_drone: saw 55 # lpf (sine 0.05 * 400 + 600) 0.3
~mist_air: pink_noise # hpf 6000 0.1 * 0.15
~mist_rhythm: s "bd ~ ~ sn ~ bd ~ ~" $ degrade_by 0.3 $ swing 0.2
~digital_mist: (~mist_drone + ~mist_air) * 0.5 + ~mist_rhythm * 0.4

-- ============================================
-- SECTION 5: GRANULAR/SPECTRAL PATTERNS
-- ============================================

-- PATTERN 19: GLITCH GRAINS
-- Very small grains for metallic, glitchy textures
~glitch_grain_src: sine 440
~glitch_grains: granular ~glitch_grain_src "5 10 15" 0.6 "0.5 1.0 2.0"

-- PATTERN 20: PITCHED CLOUDS
-- Granular with melodic pitch patterns
~cloud_pitch_src: saw "110 165 220"
~pitched_clouds: granular ~cloud_pitch_src 50 0.8 "0.5 0.75 1.0 1.5"

-- PATTERN 21: SPARSE CLICKS
-- Low-density grains for rhythmic texture
~click_src: white_noise # hpf 2000 0.6
~sparse_clicks: granular ~click_src 8 0.15 1.0 * 0.5

-- ============================================
-- COMPLETE AMBIENT COMPOSITIONS
-- ============================================

-- COMPOSITION A: "ENDLESS HORIZONS"
-- Full ambient piece with layered textures
~eh_sub: sine 55 * 0.3
~eh_lfo1: sine 0.07
~eh_lfo2: sine 0.13
~eh_pad: supersaw 110 0.02 # lpf (~eh_lfo1 * 800 + ~eh_lfo2 * 400 + 600) 0.4
~eh_texture: granular pink_noise "32 48 64" 0.6 "0.5 0.7 1.0" * 0.2
~eh_melody: sine "~ 220 ~ ~ 330 ~ 440 ~" * 0.1
~endless_horizons: (~eh_sub + ~eh_pad * 0.4 + ~eh_texture + ~eh_melody) # reverb 0.8 0.85 0.6

-- COMPOSITION B: "FRACTURED GRID"
-- IDM rhythm piece with polyrhythmic complexity
~fg_kick: s "bd(3,8)" $ degrade_by 0.1
~fg_snare: s "sn(5,16)" $ swing 0.15
~fg_hat: s "hh(7,12)" $ sometimes rev
~fg_melody: saw "110 165 220 ~ 330 ~ 165 220" $ every 4 (stutter 3) $ degrade_by 0.2
~fg_melody_proc: ~fg_melody # lpf "1500 2000 1000 3000" 0.7
~fractured_grid: (~fg_kick * 0.8 + ~fg_snare * 0.6 + ~fg_hat * 0.3 + ~fg_melody_proc * 0.4)

-- COMPOSITION C: "DIGITAL MEDITATION"
-- Slow, evolving ambient with subtle movement
~dm_fund: sine 55 * 0.25
~dm_fifth: sine 82.5 * 0.15
~dm_lfo: sine 0.03
~dm_pad: (~dm_fund + ~dm_fifth) * (~dm_lfo * 0.3 + 0.7)
~dm_grains: granular (sine "110 165") 80 0.7 0.9 * 0.2
~dm_verb: reverb (~dm_pad + ~dm_grains) 0.9 0.9 0.7
~digital_meditation: ~dm_verb * 0.8

-- ============================================
-- OUTPUT SELECTION
-- ============================================

-- Choose your texture by uncommenting:

-- AMBIENT DRONES
-- out: ~evolving_drone * 0.3
-- out: ~sub_drone_mod * 0.5
-- out: ~harmonic_drone * 0.3
-- out: ~detuned_pad * 0.25

-- EVOLVING TEXTURES
-- out: ~granular_cloud
-- out: ~frozen_texture
-- out: ~spectral_air
-- out: ~morphing_pad * 0.25
-- out: ~shimmer_texture * 0.4

-- IDM RHYTHMS (set tempo: 2.0 first)
-- out: ~euclidean_drums
-- out: ~probabilistic_glitch
-- out: ~stutter_machine
-- out: ~polyrhythmic_layers
-- out: ~micro_timed

-- HYBRID AMBIENT-IDM
-- out: ~rhythmic_texture
-- out: ~granular_rhythm
-- out: ~sparse_melodic
-- out: ~digital_mist

-- GRANULAR/SPECTRAL
-- out: ~glitch_grains * 0.3
-- out: ~pitched_clouds * 0.3
-- out: ~sparse_clicks

-- COMPLETE COMPOSITIONS
out: ~endless_horizons * 0.5
-- out: ~fractured_grid * 0.4
-- out: ~digital_meditation * 0.5

-- ============================================
-- PRODUCTION TIPS
-- ============================================
-- 1. Use slow LFO rates (0.01-0.1 Hz) for evolving textures
-- 2. Layer multiple LFOs at unrelated rates for non-repeating patterns
-- 3. Granular density patterns create rhythm from texture
-- 4. Use degrade_by for controlled randomness
-- 5. Combine ambient beds with sparse IDM elements
-- 6. Heavy reverb transforms short sounds into pads
-- 7. High-pass filtered noise adds "air" to mixes
-- 8. Euclidean rhythms create complex but natural patterns
-- 9. Swing and micro-timing add human feel
-- 10. Granular with small grains (5-20ms) = glitchy textures
