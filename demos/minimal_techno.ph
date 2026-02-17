-- Minimal Techno Pattern Library for Phonon
-- Based on Robert Hood, Richie Hawtin, Ricardo Villalobos aesthetics
-- 10+ patterns demonstrating stripped-down, hypnotic, evolving techno

-- =============================================================================
-- PATTERN 1: Detroit Minimal - Robert Hood Style
-- Pure stripped-down aesthetic: drums, bassline, funky groove
-- =============================================================================

cps: 2.166  -- ~130 BPM (classic minimal techno tempo)

-- Four-on-the-floor kick - the foundation
~kick $ s "bd*4"

-- Sparse offbeat hi-hats with subtle swing
~hats $ s "~ hh ~ hh" $ swing 0.08 # gain 0.4

-- Minimal clap on 2 and 4
~clap $ s "~ cp ~ cp" # gain 0.6

-- Hypnotic filtered bass
~bass $ saw 55 # lpf 500 1.2 * 0.25

out $ ~kick * 0.8 + ~hats + ~clap + ~bass


-- =============================================================================
-- PATTERN 2: Plastikman Pulse - Richie Hawtin Style
-- Filter modulation as the primary movement
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"
~lfo $ sine 0.0625  -- 16-bar filter sweep

-- Pulsing bass with slow filter modulation
~pulse $ saw 55 # lpf (~lfo * 2500 + 300) 1.5 * 0.3

-- Minimal percussion - just kick and filtered pulse
out $ ~kick * 0.7 + ~pulse


-- =============================================================================
-- PATTERN 3: Hypnotic Loop - Ricardo Villalobos Style
-- Extended, evolving, deep hypnotic groove
-- =============================================================================

cps: 2.083  -- ~125 BPM (deeper, more house-influenced)

~kick $ s "bd*4"

-- Shuffled hats - 16ths with heavy swing
~hats $ s "hh*16" $ swing 0.12 $ degradeBy 0.3 # gain 0.35

-- Sparse rimshot pattern
~rim $ s "~ ~ rim ~" $ every 4 (rotL 0.25) # gain 0.5

-- Sub bass with very slow modulation
~sub_lfo $ sine 0.03125  -- 32-bar cycle
~sub $ sine 55 # lpf (~sub_lfo * 200 + 80) 0.7 * 0.4

out $ ~kick * 0.8 + ~hats + ~rim + ~sub


-- =============================================================================
-- PATTERN 4: Tresor Club - Berlin Industrial Minimal
-- Harder edge, industrial textures
-- =============================================================================

cps: 2.25  -- ~135 BPM (harder Berlin tempo)

~kick $ s "bd*4" # gain 1.0

-- Clap with reverb tail
~clap $ s "~ cp ~ ~" # reverb 0.6 0.5 0.4 # gain 0.7

-- Industrial hi-hats with degradation
~hats $ s "hh*8" $ degradeBy 0.4 # hpf 2000 0.6 # gain 0.4

-- Dark rumbling bass
~rumble $ saw 41.2 # lpf 200 2.0 # distortion 1.5 * 0.2

out $ ~kick + ~clap + ~hats + ~rumble


-- =============================================================================
-- PATTERN 5: Minus Label - Surgical Precision
-- Clean, precise, hypnotic repetition
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Locked 16th hat grid
~hats $ s "hh*16" # gain 0.3

-- Snare every 2 beats
~snare $ s "~ sn ~ sn" # gain 0.6

-- Constant filtered saw - no modulation, pure hypnosis
~drone $ saw 55 # lpf 800 0.9 * 0.2

out $ ~kick * 0.8 + ~hats + ~snare + ~drone


-- =============================================================================
-- PATTERN 6: Perlon Groove - Deep Shuffle
-- Deep, shuffling, microhouse-influenced
-- =============================================================================

cps: 2.083  -- ~125 BPM

~kick $ s "bd*4"

-- Heavy shuffle on hats (35%+ swing is typical for microhouse)
~hats $ s "hh*8" $ swing 0.18 # gain 0.4

-- Syncopated percussion
~perc $ s "~ rim [~ rim] ~" $ swing 0.1 # gain 0.5

-- Warm sub bass
~sub $ sine "55 ~ 82.5 ~" * 0.35

-- Subtle pad for depth
~pad $ sine "[110 165]" * 0.06

out $ ~kick * 0.7 + ~hats + ~perc + ~sub + ~pad # reverb 0.4 0.6 0.2


-- =============================================================================
-- PATTERN 7: Dubby Minimal - Dub Techno Crossover
-- Space, delay, echoes
-- =============================================================================

cps: 2.0  -- 120 BPM (dubbed-out tempo)

~kick $ s "bd*4" # gain 0.8

-- Sparse chord stab with long reverb
~stab $ s "~ ~ [stab:0 ~] ~" $ slow 2 # reverb 0.85 0.4 0.6 # gain 0.35

-- Delayed hats - creates polyrhythmic echoes
~hats $ s "~ hh ~ hh" # delay 0.375 0.55 0.5 # gain 0.3

-- Rumbling sub
~sub $ sine 55 # lpf 100 0.8 * 0.35

out $ ~kick + ~stab + ~hats + ~sub


-- =============================================================================
-- PATTERN 8: Evolving Minimal - Progressive Transformation
-- Pattern changes over time using every
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Evolving hat pattern
~hats $ s "hh*8" $ every 4 (fast 2) $ every 8 rev # gain 0.4

-- Rotating rim pattern
~rim $ s "rim ~ ~ rim ~ rim ~ ~" $ every 3 (rotL 0.125) # gain 0.5

-- Bass that changes every 4 bars
~bass_pattern $ saw "55 55 82.5 55" $ every 4 rev
~bass $ ~bass_pattern # lpf 600 1.0 * 0.25

out $ ~kick * 0.8 + ~hats + ~rim + ~bass


-- =============================================================================
-- PATTERN 9: Breakdown Section - Tension & Release
-- Stripped back for breakdowns - drums drop, atmosphere remains
-- =============================================================================

cps: 2.166

-- Very slow modulation for tension
~tension_lfo $ sine 0.0416  -- 24-bar cycle

-- Atmospheric pad only
~atmos $ saw "[82.5 110]" # lpf (~tension_lfo * 1500 + 200) 0.6 * 0.15

-- Subtle percussion hints
~ghost_perc $ s "~ ~ hh? ~" $ swing 0.1 # reverb 0.8 0.5 0.5 # gain 0.2

out $ ~atmos + ~ghost_perc # reverb 0.7 0.4 0.4


-- =============================================================================
-- PATTERN 10: Full Drop - Maximum Intensity
-- All elements combined for peak energy
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Driving hats
~hats $ s "hh*16" $ degradeBy 0.2 $ swing 0.06 # gain 0.4

-- Clap on 2 and 4
~clap $ s "~ cp ~ cp" # gain 0.7

-- Ride for energy
~ride $ s "~ ~ ride ~" # gain 0.4

-- Pounding bass
~bass_lfo $ sine 0.5
~bass $ saw 55 # lpf (~bass_lfo * 300 + 400) 1.5 * 0.3

-- Extra percussion layer
~perc $ s "~ rim ~ [rim rim]" $ swing 0.1 # gain 0.4

out $ ~kick * 0.9 + ~hats + ~clap + ~ride + ~bass + ~perc # reverb 0.2 0.7 0.1


-- =============================================================================
-- PATTERN 11: Ostgut Ton - Deep & Dark
-- Inspired by Berghain's legendary sound system
-- =============================================================================

cps: 2.166

~kick $ s "bd*4" # lpf 200 1.0  -- Subby kick

-- Very sparse elements
~hats $ s "~ ~ ~ hh" $ every 4 rev # gain 0.35

-- Dark sub
~sub $ sine 41.2 * 0.4

-- Tension percussion - irregular
~tension $ s "~ rim ~ ~" $ every 5 (fast 2) $ every 7 (rotR 0.125) # reverb 0.5 0.6 0.3 # gain 0.5

out $ ~kick * 0.9 + ~hats + ~sub + ~tension


-- =============================================================================
-- PATTERN 12: Clicks & Cuts - Microhouse Minimal
-- Glitchy, clicky, granular textures
-- =============================================================================

cps: 2.083  -- ~125 BPM

~kick $ s "bd*4"

-- Clicky hi-hat pattern - sounds like micro-samples
~clicks $ s "hh*32" $ degradeBy 0.7 # hpf 8000 0.8 # gain 0.25

-- Stepped filter modulation (microhouse characteristic)
~cutoff_pattern # "400 800 1200 600 1000 500 900 700"
~bass $ saw 55 # lpf 700 1.0 * 0.25

-- Glitchy percussion
~glitch $ s "rim*8" $ degradeBy 0.6 $ scramble 8 # gain 0.3

out $ ~kick * 0.7 + ~clicks + ~bass + ~glitch


-- =============================================================================
-- PATTERN 13: Ghost Notes - Humanized Groove
-- Using ghost function for natural feel
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Main snare with ghost notes
~snare $ s "~ sn ~ sn" $ ghost # gain 0.5

-- Humanized hats with swing
~hats $ s "hh*8" $ swing 0.1 $ degradeBy 0.15 # gain 0.4

-- Subtle rim accents
~rim $ s "~ ~ [rim ~] ~" # gain 0.45

out $ ~kick * 0.8 + ~snare + ~hats + ~rim


-- =============================================================================
-- PATTERN 14: LFO Madness - Parameter Modulation Showcase
-- Demonstrating Phonon's unique pattern-as-control capability
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Multiple LFOs at different rates
~lfo_slow $ sine 0.125    -- 8-bar cycle
~lfo_medium $ sine 0.5    -- 2-bar cycle
~lfo_fast $ sine 2        -- Half-bar cycle

-- Bass with slow filter sweep
~bass $ saw 55 # lpf (~lfo_slow * 1500 + 300) 1.2 * 0.25

-- Pad with medium modulation
~pad $ sine 110 # lpf (~lfo_medium * 1000 + 500) 0.7 * 0.1

-- Hats with fast tremolo-like effect (via gain)
~hats $ s "hh*8" # gain (0.3 + ~lfo_fast * 0.15)

out $ ~kick * 0.8 + ~bass + ~pad + ~hats


-- =============================================================================
-- PATTERN 15: Call and Response - Dialogue Between Elements
-- Two patterns that interlock and respond to each other
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Call pattern (first half)
~call $ s "~ sn ~ ~"

-- Response pattern (second half)
~response $ s "~ ~ ~ [cp rim]"

-- Interlock them with jux for stereo
~dialogue $ ~call + ~response $ jux rev # gain 0.6

-- Supporting hats
~hats $ s "hh*8" $ swing 0.08 # gain 0.35

out $ ~kick * 0.8 + ~dialogue + ~hats


-- =============================================================================
-- Quick Reference - BPM/CPS Conversion for Minimal Techno
-- =============================================================================
-- 120 BPM = cps: 2.0
-- 125 BPM = cps: 2.083  (microhouse, deep minimal)
-- 128 BPM = cps: 2.133
-- 130 BPM = cps: 2.166  (classic minimal techno sweet spot)
-- 135 BPM = cps: 2.25   (harder Berlin techno)
-- 137 BPM = cps: 2.283

-- =============================================================================
-- Swing Values for Different Feels
-- =============================================================================
-- 0.0     = straight 16ths (robotic)
-- 0.05-08 = subtle swing (Detroit minimal)
-- 0.10-12 = medium swing (Richie Hawtin style)
-- 0.15-20 = heavy shuffle (microhouse/Ricardo Villalobos)
-- 0.25+   = extreme swing (rarely used in minimal)
