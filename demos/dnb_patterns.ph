-- ============================================
-- DRUM & BASS PATTERN LIBRARY
-- 20 Authentic DnB Patterns for Phonon
-- ============================================
-- Based on research: docs/DnB_PATTERN_RESEARCH.md
-- See also: demos/dnb.ph for simpler examples
-- Standard tempo: 174 BPM (tempo: 2.9)
--
-- Uses new $ bus syntax (: also works for backwards compat)

tempo: 2.9  -- 174 BPM (DnB standard)

-- ============================================
-- PATTERN 1: CLASSIC TWO-STEP
-- ============================================
-- The quintessential DnB pattern
-- Kicks on 1st and 6th eighth notes
-- Snares on beats 2 and 4

~twostep_kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
~twostep_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~twostep_hats $ s "hh*8"

-- out $ ~twostep_kick + ~twostep_snare + ~twostep_hats * 0.5


-- ============================================
-- PATTERN 2: TWO-STEP TIGHT
-- ============================================
-- Compressed two-step with 16th note hats
-- More energy, tighter feel

~twostep_tight $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*16"

-- out $ ~twostep_tight


-- ============================================
-- PATTERN 3: HALF-TIME DnB
-- ============================================
-- Single snare on beat 3 - "halves" perceived speed
-- Popular in dubstep-influenced DnB

~halftime_kick $ s "bd ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
~halftime_snare $ s "~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~"
~halftime_hats $ s "hh*8" * 0.5

-- out $ ~halftime_kick + ~halftime_snare + ~halftime_hats


-- ============================================
-- PATTERN 4: LIQUID DnB
-- ============================================
-- Smooth, melodic, soulful (174 BPM)
-- Softer dynamics, flowing feel

~liquid_kick $ s "bd ~ ~ ~ ~ bd ~ ~"
~liquid_snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~liquid_hats $ s "hh*16" * 0.4
~liquid_ride $ s "~ ~ ~ ~ ~ ~ ride ~" * 0.3

~liquid_drums $ ~liquid_kick + ~liquid_snare + ~liquid_hats + ~liquid_ride

-- out $ ~liquid_drums


-- ============================================
-- PATTERN 5: LIQUID FULL MIX
-- ============================================
-- Complete liquid production with bass

~liquid_bass $ saw "55 55 82.5 73.4" # lpf 600 0.6 * 0.25
~liquid_sub $ sine "55 55 82.5 73.4" * 0.2

-- out $ ~liquid_drums + ~liquid_bass + ~liquid_sub


-- ============================================
-- PATTERN 6: NEUROFUNK
-- ============================================
-- Dark, mechanical, 180 BPM typically
-- Second kick shifted to last 16th before beat 3
-- 32nd note hats for intensity

~neuro_kick $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
~neuro_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~neuro_hats $ s "hh*32" * 0.25
~neuro_perc $ s "~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ ~" * 0.4

~neuro_drums $ ~neuro_kick + ~neuro_snare + ~neuro_hats + ~neuro_perc

-- out $ ~neuro_drums


-- ============================================
-- PATTERN 7: NEUROFUNK FULL MIX
-- ============================================
-- Complete neuro with modulated bass

~neuro_lfo $ sine 4
~neuro_bass $ saw 55 # lpf (~neuro_lfo * 1500 + 500) 0.9 * 0.2
~neuro_sub $ sine 55 * 0.15

-- out $ ~neuro_drums + ~neuro_bass + ~neuro_sub


-- ============================================
-- PATTERN 8: JUMP-UP
-- ============================================
-- Bouncy, high energy, raw
-- Multiple kicks, punchy feel

~jumpup_kick $ s "bd bd ~ ~ bd ~ bd ~"
~jumpup_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~jumpup_hats $ s "hh*16" * 0.5

~jumpup_drums $ ~jumpup_kick + ~jumpup_snare + ~jumpup_hats

-- out $ ~jumpup_drums


-- ============================================
-- PATTERN 9: JUMP-UP AGGRESSIVE
-- ============================================
-- Extra punchy with claps

~jumpup_agg $ s "bd*2 bd ~ bd ~ bd ~, ~ sn ~ sn ~ ~ sn ~, hh*16, ~ ~ cp ~ ~ ~ cp ~"

-- out $ ~jumpup_agg


-- ============================================
-- PATTERN 10: ROLLERS
-- ============================================
-- Hypnotic, minimal, groovy
-- Extra kick for rolling feel

~roller_kick $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
~roller_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~roller_hats $ s "hh*8" * 0.35

~roller_drums $ ~roller_kick + ~roller_snare + ~roller_hats

-- out $ ~roller_drums


-- ============================================
-- PATTERN 11: ROLLER FULL MIX
-- ============================================
-- Complete roller with long rolling bass

~roller_bass $ saw 55 # lpf 500 0.8 * 0.3
~roller_verb $ reverb ~roller_drums 0.2 0.4 0.1

-- out $ ~roller_verb + ~roller_bass


-- ============================================
-- PATTERN 12: TECHSTEP
-- ============================================
-- Dark, industrial, mechanical (170-180 BPM)
-- Euclidean hi-hats, cold metallic feel

~techstep_kick $ s "bd ~ ~ bd ~ ~ bd ~"
~techstep_snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~techstep_hats $ s "hh(7,16)" * 0.4
~techstep_perc $ s "industrial(3,16)" * 0.2

~techstep_drums $ ~techstep_kick + ~techstep_snare + ~techstep_hats + ~techstep_perc

-- out $ ~techstep_drums


-- ============================================
-- PATTERN 13: JUNGLE
-- ============================================
-- Early 90s, 160-170 BPM feel
-- Heavy breakbeat chopping

~jungle_kick $ s "bd ~ bd ~ ~ ~ bd ~"
~jungle_snare $ s "~ sn ~ sn ~ ~ ~ ~"
~jungle_break $ s "amen*4" * 0.4

~jungle_drums $ ~jungle_kick + ~jungle_snare + ~jungle_break

-- out $ ~jungle_drums


-- ============================================
-- PATTERN 14: JUNGLE RAGGA
-- ============================================
-- Jungle with reggae influence
-- Syncopated snares

~ragga_kick $ s "bd ~ ~ bd ~ bd ~ ~"
~ragga_snare $ s "~ ~ sn ~ ~ ~ sn ~"
~ragga_hats $ s "hh*8" * 0.4
~ragga_sub $ sine "55 ~ ~ 55 ~ ~ ~ ~" * 0.25

~ragga_full $ ~ragga_kick + ~ragga_snare + ~ragga_hats + ~ragga_sub

-- out $ ~ragga_full


-- ============================================
-- PATTERN 15: GHOST NOTES
-- ============================================
-- Human feel with ghost snares
-- Main snares + quieter ghost hits

~ghost_kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
~ghost_main $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~ghost_quiet $ s "~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~" * 0.25
~ghost_hats $ s "hh*16" * 0.4

~ghost_drums $ ~ghost_kick + ~ghost_main + ~ghost_quiet + ~ghost_hats

-- out $ ~ghost_drums


-- ============================================
-- PATTERN 16: EUCLIDEAN DnB
-- ============================================
-- Complex polyrhythmic patterns
-- Great for experimental DnB

~euclidean_kick $ s "bd(3,16)"
~euclidean_snare $ s "sn(2,8,4)"
~euclidean_hats $ s "hh*16" * 0.4
~euclidean_perc $ s "cp(7,16)" * 0.3

~euclidean_drums $ ~euclidean_kick + ~euclidean_snare + ~euclidean_hats + ~euclidean_perc

-- out $ ~euclidean_drums


-- ============================================
-- PATTERN 17: SYNCOPATED BREAK
-- ============================================
-- Think break inspired
-- Heavy syncopation

~synco_kick $ s "bd ~ ~ bd ~ bd ~ ~"
~synco_snare $ s "~ ~ sn ~ ~ ~ ~ ~"
~synco_hats $ s "hh hh ~ hh hh ~ hh ~" * 0.5

~synco_drums $ ~synco_kick + ~synco_snare + ~synco_hats

-- out $ ~synco_drums


-- ============================================
-- PATTERN 18: FAST ENERGY
-- ============================================
-- Maximum energy with 32nd hats
-- For peak moments

~fast_kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ bd ~"
~fast_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~fast_hats $ s "hh*32" * 0.3
~fast_ride $ s "ride*4" * 0.2

~fast_drums $ ~fast_kick + ~fast_snare + ~fast_hats + ~fast_ride

-- out $ ~fast_drums


-- ============================================
-- PATTERN 19: MINIMAL DnB
-- ============================================
-- Stripped back, sparse
-- Focus on space and groove

~minimal_kick $ s "bd ~ ~ ~ ~ bd ~ ~"
~minimal_snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~minimal_hats $ s "~ ~ hh ~ ~ ~ hh ~" * 0.4

~minimal_drums $ ~minimal_kick + ~minimal_snare + ~minimal_hats

-- out $ ~minimal_drums


-- ============================================
-- PATTERN 20: REESE BASS PRODUCTION
-- ============================================
-- Full production with classic Reese bass

~reese_drums $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*16" * 0.8

~reese_bass $ supersaw "55 55 55 82.5" 0.6 12
~reese_lfo $ sine 0.5 * 0.5 + 0.5
~reese_filt $ ~reese_bass # lpf (~reese_lfo * 1500 + 400) 0.85 * 0.2

~reese_sub $ sine "55 55 55 82.5" * 0.15

~reese_full $ ~reese_drums + ~reese_filt + ~reese_sub

-- out $ ~reese_full


-- ============================================
-- FULL DEMO: LIQUID PRODUCTION
-- ============================================
-- Complete track structure

~demo_kick $ s "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~"
~demo_snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~demo_hats $ s "hh*16" * 0.35
~demo_ride $ s "~ ~ ~ ~ ~ ~ ride ~ ~ ~ ~ ~ ~ ~ ride ~" * 0.25
~demo_perc $ s "cp(5,16)" * 0.2

~demo_drums $ ~demo_kick + ~demo_snare + ~demo_hats + ~demo_ride + ~demo_perc

~demo_bass $ supersaw "55 55 55 82.5" 0.5 8
~demo_lfo $ sine 0.3 * 0.5 + 0.5
~demo_bass_filt $ ~demo_bass # lpf (~demo_lfo * 1200 + 500) 0.7 * 0.2

~demo_sub $ sine "55 55 55 82.5" * 0.18

~demo_drums_verb $ reverb ~demo_drums 0.15 0.3 0.1

out $ ~demo_drums_verb + ~demo_bass_filt + ~demo_sub


-- ============================================
-- QUICK REFERENCE
-- ============================================
-- All patterns in one-liner format for easy copy/paste:
--
-- Two-Step:     "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*8"
-- Half-Time:    "bd ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~, hh*8"
-- Liquid:       "bd ~ ~ ~ ~ bd ~ ~, ~ ~ ~ ~ sn ~ ~ ~, hh*16"
-- Neurofunk:    "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*32"
-- Jump-Up:      "bd bd ~ ~ bd ~ bd ~, ~ ~ ~ ~ sn ~ ~ ~, hh*16"
-- Roller:       "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*8"
-- Techstep:     "bd ~ ~ bd ~ ~ bd ~, ~ ~ ~ ~ sn ~ ~ ~, hh(7,16)"
-- Jungle:       "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, amen*4"
-- Ghost:        "bd ~ ~ ~ ~ bd ~ ~, ~ ~ ~ ~ sn ~ ~ ~, ~ ~ sn ~ ~ ~ ~ ~, hh*16"
-- Euclidean:    "bd(3,16), sn(2,8,4), hh*16, cp(7,16)"
-- Minimal:      "bd ~ ~ ~ ~ bd ~ ~, ~ ~ ~ ~ sn ~ ~ ~, ~ ~ hh ~ ~ ~ hh ~"
-- Fast:         "bd ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ bd ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, hh*32"
