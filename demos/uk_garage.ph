-- 🎵 UK GARAGE / 2-STEP PATTERNS - 130-140 BPM
-- Comprehensive pattern library for authentic UKG production
-- Based on research from docs/UK_GARAGE_2STEP_RESEARCH.md

-- ============================================
-- THE DEFINING FEATURE: 2-STEP KICK PATTERN
-- ============================================
-- Kicks SKIP beats, creating space and bounce
-- Beat 1 and the "and" of beat 3 (7/16 = ~position 0.4375)
-- This is what separates 2-step from four-on-floor house

tempo: 2.2  -- 132 BPM (classic UKG tempo)

-- ============================================
-- 1. CLASSIC 2-STEP (Basic)
-- ============================================
-- The foundation: kick on 1, kick between 3 and 4, snares on 2 & 4

~classic_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~classic_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~classic_hats: s "~ hh ~ hh ~ hh ~ hh"
~classic_opens: s "~ oh ~ oh ~ ~ ~ oh"

-- Simple one-liner version:
-- "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~, ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~, ~ hh ~ hh ~ hh ~ hh"

-- ============================================
-- 2. CLASSIC 2-STEP (with Swing)
-- ============================================
-- Swing on hats/percussion is CRITICAL for the UKG groove
-- The kick and snare stay straight, hats get swung

~swung_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~swung_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~swung_hats: s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
~swung_opens: s "~ oh ~ oh ~ ~ ~ oh" $ swing 0.08

-- ============================================
-- 3. BOUNCY 2-STEP
-- ============================================
-- Ghost kick on 16th before beat 3 adds bounce
-- This creates the "skippy" feel that defines the genre

~bouncy_kick: s "bd ~ ~ ~ ~ ~ bd bd ~ ~ ~ ~ ~ ~ ~ ~"
~bouncy_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~bouncy_hats: s "hh*8" $ swing 0.1

-- ============================================
-- 4. GHOST SNARE 2-STEP
-- ============================================
-- Ghost snares add complexity and drive
-- The ghost function adds quieter copies at offsets

~ghost_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~ghost_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ ghost
~ghost_hats: s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08

-- ============================================
-- 5. ROLLING 2-STEP (Slower, Deeper)
-- ============================================
-- 124-128 BPM, more spacious feel
-- Alternate bars with different kick patterns for movement
-- tempo: 2.07 for 124 BPM

~rolling_kick: s "<[bd ~ ~ ~ ~ ~ ~ bd] [~ ~ ~ ~ ~ ~ ~ bd]>"
~rolling_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~rolling_hats: s "hh*8" $ swing 0.12
~rolling_rim: s "~ ~ ~ rs ~ rs ~ ~" $ swing 0.08

-- ============================================
-- 6. SPEED GARAGE
-- ============================================
-- Faster (135-140 BPM), more four-on-floor influence
-- tempo: 2.33 for 140 BPM

~speed_kick: s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
~speed_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~speed_hats: s "hh hh oh hh hh hh oh hh" $ swing 0.06

-- ============================================
-- 7. SHUFFLING 2-STEP
-- ============================================
-- Heavy swing on everything, more MPC60-style
-- The shuffle feel from straight + swung elements together

~shuffle_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~shuffle_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~shuffle_hats: s "hh hh hh hh hh hh hh hh" $ swing 0.15
~shuffle_tamb: s "~ tamb ~ tamb" $ swing 0.15

-- ============================================
-- 8. MINIMAL 2-STEP
-- ============================================
-- Stripped back, space is the key element
-- Common in deep/vocal garage

~minimal_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~minimal_snare: s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
~minimal_hats: s "~ ~ oh ~ ~ ~ oh ~" $ swing 0.08

-- ============================================
-- 9. SYNCOPATED 2-STEP
-- ============================================
-- More complex kick pattern with extra syncopation
-- Keeps the 2-step feel but adds movement

~synco_kick: s "bd ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~ bd ~ ~"
~synco_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~synco_hats: s "hh hh oh hh hh hh oh hh" $ swing 0.1
~synco_rim: s "rs(5,16)" $ swing 0.08

-- ============================================
-- 10. BROKEN 2-STEP
-- ============================================
-- Irregular kick pattern, almost jungle influence
-- Still maintains the skippy garage feel

~broken_kick: s "bd ~ bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~"
~broken_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ ghost
~broken_hats: s "hh*16" * 0.5 $ swing 0.08

-- ============================================
-- 11. 4X4 GARAGE (UK Funky influence)
-- ============================================
-- Four-on-floor but with 2-step percussion
-- Bridge between house and garage

~funky_kick: s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
~funky_snare: s "~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~"
~funky_hats: s "~ hh oh hh ~ hh oh hh" $ swing 0.06
~funky_perc: s "~ ~ ~ ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~"

-- ============================================
-- 12. PERCUSSIVE 2-STEP
-- ============================================
-- Extra percussion layers for complexity
-- Shakers, tambourines, rimshots

~perc_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~perc_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~perc_hats: s "hh*8" $ swing 0.08
~perc_shaker: s "~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~" -- placeholder: use shaker sample
~perc_tamb: s "tamb tamb tamb tamb" * 0.4 $ swing 0.1
~perc_rim: s "~ rs ~ ~ ~ rs ~ ~" * 0.5 $ swing 0.08

-- ============================================
-- 13. OPEN HAT GROOVE
-- ============================================
-- Emphasis on open hats for the "cutting high-end"
-- Signature of many classic UKG tracks

~openhat_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~openhat_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~openhat_hats: s "~ oh ~ oh ~ oh ~ oh" $ swing 0.08
~openhat_closed: s "hh ~ hh ~ hh ~ hh ~" * 0.4

-- ============================================
-- 14. TRIPLET 2-STEP
-- ============================================
-- Triplet feel within the 2-step framework
-- Common in later UKG productions

~triplet_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~ ~"
~triplet_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~triplet_hats: s "hh(6,8)" $ swing 0.1

-- ============================================
-- 15. DUBBY 2-STEP
-- ============================================
-- Dub reggae influenced, spacey, heavy on reverb feel
-- Rimshots and delays characteristic

~dub_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~dub_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~dub_rim: s "~ ~ rs ~ ~ ~ ~ ~ ~ ~ rs ~ ~ ~ ~ ~" $ swing 0.1
~dub_hats: s "~ hh ~ hh ~ hh ~ hh" $ swing 0.1

-- ============================================
-- 16. CHOPPY 2-STEP
-- ============================================
-- More staccato, chopped feel
-- Emphasizes the gaps in the rhythm

~choppy_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~choppy_snare: s "~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~"
~choppy_hats: s "hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~" * 0.6

-- ============================================
-- 17. VOCAL GARAGE GROOVE
-- ============================================
-- Pattern designed to complement vocals
-- More minimal, leaves space for voice

~vocal_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~vocal_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~vocal_hats: s "~ oh ~ ~ ~ oh ~ ~" $ swing 0.06

-- ============================================
-- BASS PATTERNS
-- ============================================

-- Classic garage sub bass (follows chord changes)
~bass_sub: sine "55 55 82.5 73.4"

-- Wobbly bassline with filter modulation
~bass_lfo: sine 0.5 * 0.5 + 0.5
~bass_wobble: saw "55 55 82.5 73.4" # lpf (~bass_lfo * 800 + 400) 0.7

-- Punchy 909-style bass stabs
~bass_stab: saw "110 ~ 82.5 ~" # lpf 1200 0.8

-- ============================================
-- EUCLIDEAN PATTERNS FOR UKG
-- ============================================

-- Euclidean rhythms can approximate UKG feel
~euclidean_kick: s "bd(3,16,0)"  -- 3 hits in 16 steps
~euclidean_snare: s "sn(2,8,4)"  -- snares on 2 and 4
~euclidean_hats: s "hh(7,8)"    -- nearly continuous hats
~euclidean_rim: s "rs(5,16)"    -- scattered rimshots

-- ============================================
-- FULL PRODUCTION EXAMPLES
-- ============================================

-- EXAMPLE 1: Classic 2-Step Track
~ex1_kick: s "bd ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ ~"
~ex1_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~ex1_hats: s "~ hh ~ hh ~ hh ~ hh" $ swing 0.08
~ex1_opens: s "~ oh ~ oh ~ ~ ~ oh" $ swing 0.08
~ex1_bass: sine "55 55 82.5 73.4" * 0.3
~ex1_drums: ~ex1_kick + ~ex1_snare + ~ex1_hats * 0.6 + ~ex1_opens * 0.5
-- out: ~ex1_drums + ~ex1_bass

-- EXAMPLE 2: Rolling Deep
~ex2_kick: s "<[bd ~ ~ ~ ~ ~ ~ bd] [~ ~ ~ ~ ~ ~ ~ bd]>"
~ex2_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ ghost
~ex2_hats: s "hh*8" $ swing 0.12
~ex2_rim: s "~ ~ ~ rs ~ rs ~ ~" $ swing 0.08
~ex2_sub: sine "55 55 55 82.5" * 0.25
~ex2_drums: ~ex2_kick + ~ex2_snare + ~ex2_hats * 0.5 + ~ex2_rim * 0.4
-- out: ~ex2_drums + ~ex2_sub

-- EXAMPLE 3: Speed Garage Energy
~ex3_kick: s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~"
~ex3_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~ex3_hats: s "hh hh oh hh hh hh oh hh" $ swing 0.06
~ex3_bass: saw "110 ~ 82.5 ~" # lpf 1000 0.7 * 0.2
~ex3_drums: ~ex3_kick + ~ex3_snare + ~ex3_hats * 0.6
-- out: ~ex3_drums + ~ex3_bass

-- ============================================
-- DEFAULT OUTPUT: Classic 2-Step Demo
-- ============================================
out: ~ex1_drums + ~ex1_bass

-- ============================================
-- PRODUCTION TIPS
-- ============================================
-- 1. Tempo: 130-140 BPM (132 is the sweet spot)
-- 2. Swing: 60-65% (MPC swing 68-69, or 0.08-0.12 in Phonon)
-- 3. Kick: 909 for punch, 808 for body - layer both
-- 4. Snare: Pitched up 808/909, snappy with short decay
-- 5. Hats: Swing the hats, keep kick/snare straight
-- 6. Ghost notes: Lower velocity for ghost snares/kicks
-- 7. Space: The gaps are as important as the hits
-- 8. Bass: Sub-heavy, follows chord progression
-- 9. Open hats: Create the "cutting" high-end texture
-- 10. Rimshots: Swung, fills the gaps with texture
