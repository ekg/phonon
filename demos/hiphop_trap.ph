-- 🎤 HIP-HOP & TRAP PATTERN LIBRARY
-- 20+ authentic patterns for boom-bap, lo-fi, trap, drill, and phonk
-- Research-based patterns following genre conventions

-- ============================================
-- TEMPO REFERENCE
-- ============================================
-- Boom-Bap: 85-95 BPM
-- Lo-Fi Hip-Hop: 70-85 BPM
-- Modern Hip-Hop: 80-100 BPM
-- Trap: 130-150 BPM (counted half-time = 65-75 BPM feel)
-- Drill: 140-145 BPM (half-time)
-- Phonk: 130-145 BPM

-- ============================================
-- SECTION 1: BOOM-BAP PATTERNS (90s East Coast)
-- ============================================

tempo: 1.5  -- ~90 BPM

-- PATTERN 1: Classic Boom-Bap
-- The quintessential 90s hip-hop groove
~boombap_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
~boombap_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~boombap_hats: s "hh*8"
~boombap: ~boombap_kick + ~boombap_snare + ~boombap_hats

-- PATTERN 2: Boom-Bap with Ghost Notes
-- Adds humanization through quiet ghost snares
~boom_ghost_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ bd"
~boom_ghost_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~boom_ghost_ghost: s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ sn:1 ~ ~ ~ sn:1 ~" * 0.3
~boom_ghost_hats: s "hh*8" $ swing 0.1
~boom_ghost: ~boom_ghost_kick + ~boom_ghost_snare + ~boom_ghost_ghost + ~boom_ghost_hats

-- PATTERN 3: Pete Rock Style
-- Heavy swing, off-beat kicks
~pete_kick: s "bd ~ ~ bd ~ ~ bd ~ bd ~ ~ ~ ~ ~ bd ~"
~pete_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~pete_hats: s "hh hh oh hh hh hh hh oh" $ swing 0.12
~pete: ~pete_kick + ~pete_snare + ~pete_hats

-- PATTERN 4: DJ Premier Style
-- Hard-hitting, minimal, punchy
~premo_kick: s "bd ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ bd ~"
~premo_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~premo_hats: s "hh*16" * 0.6
~premo: ~premo_kick + ~premo_snare + ~premo_hats

-- ============================================
-- SECTION 2: LO-FI HIP-HOP PATTERNS
-- ============================================

-- PATTERN 5: Lo-Fi Chill
-- Slow, relaxed, heavy swing
~lofi_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~ bd ~"
~lofi_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~lofi_hats: s "hh*8" * 0.5 $ swing 0.15
~lofi: ~lofi_kick + ~lofi_snare + ~lofi_hats

-- PATTERN 6: Lo-Fi Study Beats
-- Mellow, continuous groove
~study_kick: s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~"
~study_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ sn:1" * 0.8
~study_hats: s "hh hh oh hh hh oh hh hh" * 0.4 $ swing 0.12
~study: ~study_kick + ~study_snare + ~study_hats

-- PATTERN 7: Lo-Fi with Vinyl Texture
-- Sparse drums, room for samples
~vinyl_kick: s "bd ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ ~ ~"
~vinyl_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~vinyl_hats: s "hh*4" * 0.3 $ swing 0.18
~vinyl_perc: s "~ ~ ~ ~ ~ ~ rim ~ ~ ~ ~ ~ ~ ~ rim ~" * 0.4
~vinyl: ~vinyl_kick + ~vinyl_snare + ~vinyl_hats + ~vinyl_perc

-- ============================================
-- SECTION 3: J DILLA STYLE PATTERNS
-- ============================================

-- PATTERN 8: Dilla Drums
-- Extremely off-grid feel, humanized timing
~dilla_kick: s "bd ~ ~ bd ~ bd bd ~ bd ~ ~ ~ ~ bd ~ ~"
~dilla_snare: s "~ ~ ~ ~ sn ~ ~ sn ~ ~ ~ ~ sn ~ ~ sn:1" $ swing 0.08
~dilla_hats: s "hh*8" * 0.5 $ swing 0.2
~dilla: ~dilla_kick + ~dilla_snare + ~dilla_hats

-- PATTERN 9: Donuts Style
-- Chopped feel, unpredictable
~donuts_kick: s "bd ~ bd ~ ~ ~ bd ~ ~ bd ~ ~ bd ~ ~ ~"
~donuts_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ sn ~ sn ~ ~ ~"
~donuts_hats: s "hh oh hh hh oh hh hh oh" * 0.6 $ swing 0.15
~donuts: ~donuts_kick + ~donuts_snare + ~donuts_hats

-- ============================================
-- SECTION 4: NEO-SOUL / MODERN HIP-HOP
-- ============================================

-- PATTERN 10: Neo-Soul
-- Smooth, laid-back, jazzy
~neosoul_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~ bd ~ ~ ~"
~neosoul_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~neosoul_hats: s "hh hh oh hh hh oh hh hh" * 0.5 $ swing 0.1
~neosoul_rim: s "~ ~ rim ~ ~ ~ ~ ~ ~ ~ rim ~ ~ ~ ~ ~" * 0.4
~neosoul: ~neosoul_kick + ~neosoul_snare + ~neosoul_hats + ~neosoul_rim

-- PATTERN 11: Kendrick/TDE Style
-- Modern bounce, syncopated
~tde_kick: s "bd ~ ~ bd ~ ~ bd ~ ~ ~ bd ~ bd ~ ~ ~"
~tde_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~tde_hats: s "hh*16" * 0.5 $ swing 0.05
~tde: ~tde_kick + ~tde_snare + ~tde_hats

-- ============================================
-- SECTION 5: TRAP PATTERNS
-- ============================================

tempo: 2.33  -- ~140 BPM (half-time feel = 70 BPM)

-- PATTERN 12: Classic Trap
-- Metro Boomin / Southside style
~trap_kick: s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"
~trap_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~trap_hats: s "hh*16"
~trap_oh: s "~ ~ ~ ~ 808oh ~ ~ ~ ~ ~ ~ ~ 808oh ~ ~ ~" * 0.5
~trap: ~trap_kick + ~trap_clap + ~trap_hats + ~trap_oh

-- PATTERN 13: Trap with Hi-Hat Rolls
-- Signature trap hi-hat patterns
~traproll_kick: s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~"
~traproll_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~traproll_hats: s "hh hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] hh hh"
~traproll: ~traproll_kick + ~traproll_clap + ~traproll_hats

-- PATTERN 14: Atlanta Trap
-- Bouncy 808 pattern
~atl_kick: s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~"
~atl_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~atl_snare: s "~ ~ ~ ~ ~ ~ sn:1 ~ ~ ~ ~ ~ ~ ~ sn:1 ~" * 0.6
~atl_hats: s "hh*16" $ swing 0.04
~atl: ~atl_kick + ~atl_clap + ~atl_snare + ~atl_hats

-- PATTERN 15: Dark Trap
-- Minimalist, heavy low-end
~dark_kick: s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ ~ ~ ~ ~"
~dark_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~dark_hats: s "hh*8" * 0.4
~dark_perc: s "~ ~ ~ ~ ~ ~ ~ ~ ~ ~ perc ~ ~ ~ ~ ~" * 0.5
~dark: ~dark_kick + ~dark_clap + ~dark_hats + ~dark_perc

-- ============================================
-- SECTION 6: DRILL PATTERNS
-- ============================================

-- PATTERN 16: UK Drill
-- Sliding 808s, distinctive hi-hat pattern
~ukdrill_kick: s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~"
~ukdrill_snare: s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~"
~ukdrill_hats: s "hh hh hh hh hh [hh*3] hh hh hh hh hh hh hh [hh*3] hh hh"
~ukdrill: ~ukdrill_kick + ~ukdrill_snare + ~ukdrill_hats

-- PATTERN 17: Chicago Drill
-- Raw, aggressive, minimal
~chi_kick: s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ 808bd ~ ~ ~ ~"
~chi_snare: s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~"
~chi_hats: s "hh*16" * 0.6 $ swing 0.03
~chi: ~chi_kick + ~chi_snare + ~chi_hats

-- PATTERN 18: Brooklyn Drill
-- Pop Smoke style
~bk_kick: s "808bd ~ ~ ~ ~ 808bd ~ ~ 808bd ~ ~ ~ ~ ~ 808bd ~"
~bk_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~bk_hats: s "hh hh hh hh hh hh [hh hh hh] hh hh hh hh hh [hh*4] hh hh"
~bk_shaker: s "~ shaker ~ shaker" * 0.3
~bk: ~bk_kick + ~bk_clap + ~bk_hats + ~bk_shaker

-- ============================================
-- SECTION 7: PHONK PATTERNS
-- ============================================

-- PATTERN 19: Memphis Phonk
-- Cowbell, dark atmosphere
~memphis_kick: s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~"
~memphis_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~memphis_hats: s "hh*8" * 0.5
~memphis_cow: s "cb ~ cb ~ cb ~ cb ~" * 0.4
~memphis: ~memphis_kick + ~memphis_clap + ~memphis_hats + ~memphis_cow

-- PATTERN 20: Drift Phonk
-- Heavy bass, aggressive
~drift_kick: s "808bd 808bd ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ 808bd 808bd ~"
~drift_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~drift_hats: s "hh*16" * 0.4 $ swing 0.05
~drift_cow: s "~ cb ~ ~ cb ~ ~ cb ~ cb ~ ~ cb ~ ~ ~" * 0.5
~drift: ~drift_kick + ~drift_clap + ~drift_hats + ~drift_cow

-- ============================================
-- SECTION 8: HI-HAT TECHNIQUES
-- ============================================

-- Hi-Hat Roll Library (use with any pattern)

-- Triplet roll (3 hits)
~roll_triplet: s "[hh hh hh]"

-- 32nd roll (rapid fire)
~roll_32: s "[hh*6]"

-- Building roll (crescendo)
~roll_build: s "[hh*4]" * 0.6 + s "[hh*4]" * 0.8 + s "[hh*4]"

-- Machine gun roll (16 hits per step)
~roll_machinegun: s "[hh*8]" * 0.5

-- Trap hi-hat pattern with strategic rolls
~trap_hats_v2: s "hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh"

-- Velocity-varied hi-hats (dynamics pattern)
~hats_velocity: s "hh*16" * "1 0.6 0.8 0.5 1 0.6 0.8 0.5 1 0.6 0.8 0.5 1 0.7 0.9 1"

-- Open hat accents
~oh_accents: s "~ ~ ~ oh ~ ~ ~ ~ ~ ~ ~ oh ~ ~ ~ ~" * 0.6

-- ============================================
-- SECTION 9: 808 BASS PATTERNS
-- ============================================

-- 808 Bass using samples
~bass_808_simple: s "808:0 ~ ~ ~ ~ ~ ~ ~ 808:0 ~ ~ ~ ~ ~ 808:0 ~"
~bass_808_bounce: s "808:0 ~ ~ 808:0 ~ ~ ~ ~ 808:0 ~ ~ ~ 808:0 ~ ~ ~"
~bass_808_slide: s "808:0 ~ 808:1 ~ ~ ~ ~ ~ 808:0 ~ ~ ~ ~ 808:1 ~ ~"

-- Synth 808 bass (sustained)
~synth_808: sine "55 ~ ~ ~ ~ ~ ~ ~ 55 ~ ~ ~ ~ ~ 55 ~" * 0.5

-- Sub bass layer
~sub_layer: sine "55 55 55 55" * 0.3

-- ============================================
-- SECTION 10: FULL PRODUCTION EXAMPLES
-- ============================================

-- EXAMPLE 1: Complete Boom-Bap Track
tempo: 1.5  -- 90 BPM
~full_boom_kick: s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
~full_boom_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
~full_boom_ghost: s "~ ~ sn:1 ~ ~ ~ ~ sn:1 ~ ~ sn:1 ~ ~ ~ ~ ~" * 0.25
~full_boom_hats: s "hh hh oh hh hh hh hh oh" $ swing 0.1
~full_boom_bass: saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
~full_boombap: ~full_boom_kick + ~full_boom_snare + ~full_boom_ghost + ~full_boom_hats + ~full_boom_bass

-- EXAMPLE 2: Complete Trap Track
tempo: 2.33  -- 140 BPM
~full_trap_kick: s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~"
~full_trap_clap: s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~"
~full_trap_snare: s "~ ~ ~ ~ ~ ~ sn:1 ~ ~ ~ ~ ~ ~ ~ sn:1 ~" * 0.5
~full_trap_hats: s "hh hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*6] [hh*3] hh hh" * 0.7
~full_trap_oh: s "~ ~ ~ ~ 808oh ~ ~ ~ ~ ~ ~ ~ ~ ~ 808oh ~" * 0.4
~full_trap_808: s "808:0 ~ ~ ~ ~ ~ ~ ~ 808:0 ~ ~ ~ 808:0 ~ ~ ~" * 0.6
~full_trap: ~full_trap_kick + ~full_trap_clap + ~full_trap_snare + ~full_trap_hats + ~full_trap_oh + ~full_trap_808

-- EXAMPLE 3: Lo-Fi Production
tempo: 1.25  -- 75 BPM
~full_lofi_kick: s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~"
~full_lofi_snare: s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" $ swing 0.15
~full_lofi_hats: s "hh*8" * 0.4 $ swing 0.18
~full_lofi_rim: s "~ ~ rim ~ ~ ~ ~ rim" * 0.3
~full_lofi_keys: sine "261 ~ 329 ~ 392 ~ 329 ~" * 0.15
~full_lofi: ~full_lofi_kick + ~full_lofi_snare + ~full_lofi_hats + ~full_lofi_rim + ~full_lofi_keys

-- EXAMPLE 4: UK Drill Production
tempo: 2.33  -- 140 BPM
~full_drill_kick: s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~"
~full_drill_snare: s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~"
~full_drill_hats: s "hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*3] hh hh hh hh"
~full_drill_perc: s "~ ~ ~ ~ rim ~ ~ ~ ~ ~ ~ ~ rim ~ ~ ~" * 0.4
~full_drill: ~full_drill_kick + ~full_drill_snare + ~full_drill_hats + ~full_drill_perc

-- ============================================
-- EUCLIDEAN APPROXIMATIONS
-- ============================================

-- Euclidean rhythms for hip-hop grooves
-- bd(3,8) = X..X..X. (boom-bap kick approx)
-- sn(2,8,2) = ..X...X. (snare on 2 and 4)

~euclidean_boombap: s "bd(3,8) sn(2,8,2) hh*8"
~euclidean_trap: s "808bd(3,16) cp(2,16,6) hh*16"
~euclidean_drill: s "808bd(5,16) sn(2,16,6) hh*16"

-- ============================================
-- PATTERN VARIATION TECHNIQUES
-- ============================================

-- Use `every` for variation
-- ~kick $ every 4 (fast 2)         -- double speed every 4 bars
-- ~hats $ every 8 (rev)            -- reverse every 8 bars

-- Use `cat` for multi-bar phrases
-- ~two_bar: cat [~bar1, ~bar2]

-- Layer patterns with different cycles
-- ~main: ~drums + (~perc $ slow 2)  -- percussion half speed

-- ============================================
-- QUICK REFERENCE: SWING VALUES
-- ============================================
-- Boom-Bap:  0.10 - 0.15 (heavy swing)
-- Lo-Fi:     0.12 - 0.20 (lazy, drunk feel)
-- J Dilla:   0.15 - 0.25 (extreme humanization)
-- Trap:      0.03 - 0.08 (subtle, machine-like)
-- Drill:     0.03 - 0.06 (tight, aggressive)

-- ============================================
-- SAMPLE SELECTION GUIDE
-- ============================================
-- 808 drums: 808bd, 808sd, 808oh, 808hc, 808
-- Standard:  bd, sn, hh, cp, oh
-- Percussion: rim, perc, cb (cowbell)
-- Claps: cp, realclaps
-- Use :N for variations (bd:0, bd:1, sn:2, etc.)

-- ============================================
-- DEFAULT OUTPUT
-- ============================================

-- Uncomment one of these to play:
-- out: ~boombap
-- out: ~trap
-- out: ~ukdrill
-- out: ~memphis
out: ~full_boombap
