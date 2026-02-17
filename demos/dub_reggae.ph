-- =============================================================================
-- DUB & REGGAE PATTERN LIBRARY
-- =============================================================================
--
-- This file contains 12+ authentic dub and reggae patterns for Phonon.
-- Based on the research in docs/DUB_REGGAE_PATTERNS.md
--
-- Key concepts:
-- - ONE DROP: Beat 1 is silent, kick+snare on beat 3
-- - ROCKERS: Kick on 1 and 3, snare on 2 and 4
-- - STEPPERS: Four-on-floor kick, driving feel
-- - SKANK: Offbeat chord stabs (the "choppy" reggae sound)
-- - BUBBLE: Pulsing organ/keyboard on 8ths
-- - DUB BASS: Deep, melodic, emphasizing root and fifth
--
-- Typical tempos:
-- - One Drop: 60-80 BPM (cps 1.0-1.33)
-- - Rockers: 60-90 BPM (cps 1.0-1.5)
-- - Steppers: 110-140 BPM (cps 1.83-2.33)
-- =============================================================================


-- =============================================================================
-- PATTERN 1: Classic One Drop (70 BPM)
-- =============================================================================
-- The most iconic reggae pattern. Beat 1 is "dropped" (silent).
-- Kick and rimshot together on beat 3 only.

-- Simple one-liner:
"~ ~ bd ~ ~ ~ rim ~, hh hh hh hh hh hh hh hh"

-- Full version with proper layering:
-- cps: 1.167
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "~ ~ rim ~"
-- ~hats $ s "hh*8" $ swing 0.12
-- out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4


-- =============================================================================
-- PATTERN 2: One Drop with Bass (70 BPM)
-- =============================================================================
-- Adds the classic reggae bassline - root and fifth with space

-- One-liner (drums only):
"~ ~ [bd,rim] ~, hh hh hh hh hh hh hh hh"

-- Full version with bass:
-- cps: 1.167
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "~ ~ rim ~"
-- ~hats $ s "hh*8" $ swing 0.1
-- ~bass $ saw "[~ 55] [~ ~] [82.5 ~] [~ 55]" # lpf 180 0.9
-- out $ ~kick * 0.8 + ~rim * 0.5 + ~hats * 0.4 + ~bass * 0.5


-- =============================================================================
-- PATTERN 3: Rockers / Roots Rock (75 BPM)
-- =============================================================================
-- More driving than one drop. Kick on 1 and 3, snare on 2 and 4.
-- Influenced by R&B and soul rhythms.

-- Simple one-liner:
"bd ~ bd ~, ~ sn ~ sn, hh hh hh hh hh hh hh hh"

-- Full version:
-- cps: 1.25
-- ~kick $ s "bd ~ bd ~"
-- ~snare $ s "~ sn ~ sn"
-- ~hats $ s "hh*8" $ swing 0.1
-- out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4


-- =============================================================================
-- PATTERN 4: Rockers with Skank (75 BPM)
-- =============================================================================
-- Adds the offbeat "skank" - the rhythmic backbone of reggae

-- One-liner:
"bd ~ bd ~, ~ sn ~ sn, [~ rim]*4"

-- Full version with skank:
-- cps: 1.25
-- ~kick $ s "bd ~ bd ~"
-- ~snare $ s "~ sn ~ sn"
-- ~hats $ s "hh*8" $ swing 0.1
-- ~skank $ s "[~ rim:1]*4" # gain 0.5
-- out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~skank * 0.4


-- =============================================================================
-- PATTERN 5: Steppers (120 BPM)
-- =============================================================================
-- The most driving reggae pattern. Four-on-floor kick.
-- Often used in roots reggae and dub.

-- Simple one-liner:
"bd bd bd bd, ~ sn ~ sn, hh*16"

-- Full version:
-- cps: 2.0
-- ~kick $ s "bd*4"
-- ~snare $ s "~ sn ~ sn"
-- ~hats $ s "hh*16" # gain 0.6
-- out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4


-- =============================================================================
-- PATTERN 6: Steppers Dub (120 BPM)
-- =============================================================================
-- Steppers with filter modulation - classic dub technique

-- One-liner:
"bd*4, ~ sn ~ sn, hh*16"

-- Full version with filter sweep:
-- cps: 2.0
-- ~kick $ s "bd*4"
-- ~snare $ s "~ sn ~ sn"
-- ~ghost $ s "~ [~ sn:1] ~ [~ sn:1]" # gain 0.3
-- ~hats $ s "hh*16" # gain "0.5 0.3 0.6 0.3"
-- ~bass $ saw "55*4" # lpf 150 0.9
-- ~lfo $ sine 0.5
-- ~mix $ (~kick + ~snare + ~ghost + ~hats) * 0.6 + ~bass * 0.4
-- out $ ~mix # lpf (~lfo * 1500 + 800) 0.5


-- =============================================================================
-- PATTERN 7: Minimal One-Drop Dub (60 BPM)
-- =============================================================================
-- Very slow, spacious dub. Heavy on atmosphere.

-- One-liner:
"~ ~ bd ~, ~ ~ rim ~, hh*8"

-- Full version:
-- cps: 1.0
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "~ ~ rim ~"
-- ~hats $ s "hh*8" $ swing 0.15 # gain 0.4
-- ~bass $ saw "[~ 55] [~ ~] [82.5 ~] [~ 55]" # lpf 180 0.9
-- ~skank $ s "[~ perc]*4" # gain 0.5
-- out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4 + ~bass * 0.5 + ~skank * 0.3


-- =============================================================================
-- PATTERN 8: Dub Echo Simulation (70 BPM)
-- =============================================================================
-- Simulates classic dub echo/delay effect using pattern offsets

-- One-liner (just the rim with "echoes"):
"rim ~ ~ ~, ~ rim:1 ~ ~, ~ ~ rim:2 ~"

-- Full version:
-- cps: 1.167
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "rim ~ ~ ~"
-- ~echo1 $ s "~ rim:1 ~ ~" # gain 0.5
-- ~echo2 $ s "~ ~ rim:1 ~" # gain 0.25
-- ~echo3 $ s "~ ~ ~ rim:1" # gain 0.125
-- ~hats $ s "hh*8" $ swing 0.1
-- out $ ~kick * 0.8 + ~rim * 0.6 + ~echo1 * 0.5 + ~echo2 * 0.4 + ~echo3 * 0.3 + ~hats * 0.4


-- =============================================================================
-- PATTERN 9: Rockers with Bubble Organ (80 BPM)
-- =============================================================================
-- Adds the hypnotic organ "bubble" - pulsing 8th notes

-- One-liner (drums only):
"bd ~ bd ~, ~ sn ~ sn, hh*8"

-- Full version with bubble:
-- cps: 1.333
-- ~kick $ s "bd ~ bd ~"
-- ~snare $ s "~ sn ~ sn"
-- ~hats $ s "hh*8" $ swing 0.1
-- ~bubble $ sine 220 # lpf 500 0.3 * "0.3 0.5"
-- ~bass $ saw "[55 ~] [~ 82.5] [55 110] [82.5 ~]" # lpf 200 0.8
-- out $ ~kick * 0.8 + ~snare * 0.6 + ~hats * 0.4 + ~bubble * 0.2 + ~bass * 0.5


-- =============================================================================
-- PATTERN 10: Heavy Dub (65 BPM)
-- =============================================================================
-- Deep, subsonic dub with filtered drums

-- One-liner:
"~ ~ [bd,rim] ~, hh*8"

-- Full version with filter modulation:
-- cps: 1.083
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "~ ~ rim ~"
-- ~hats $ s "hh*8" $ swing 0.15
-- ~bass $ saw "55 ~ 82.5 55" # lpf 120 0.95
-- ~lfo $ sine 0.25
-- ~drums $ ~kick * 0.8 + ~rim * 0.5 + ~hats * 0.4
-- out $ ~drums # lpf (~lfo * 2000 + 500) 0.6 + ~bass * 0.6


-- =============================================================================
-- PATTERN 11: Lovers Rock (72 BPM)
-- =============================================================================
-- Smoother, more romantic reggae subgenre. Softer dynamics.

-- One-liner:
"bd ~ bd ~, ~ rim ~ rim, hh*8"

-- Full version:
-- cps: 1.2
-- ~kick $ s "bd ~ bd ~" # gain 0.7
-- ~rim $ s "~ rim ~ rim" # gain 0.5
-- ~hats $ s "hh*8" $ swing 0.08 # gain 0.4
-- ~skank $ s "[~ perc:1]*4" # gain 0.3
-- out $ ~kick + ~rim + ~hats + ~skank


-- =============================================================================
-- PATTERN 12: Militant Steppers (130 BPM)
-- =============================================================================
-- Faster, more aggressive steppers. Marching, militant feel.

-- One-liner:
"bd*4, ~ cp ~ cp, hh*16"

-- Full version:
-- cps: 2.167
-- ~kick $ s "bd*4"
-- ~clap $ s "~ cp ~ cp"
-- ~hats $ s "hh*16" # gain 0.5
-- ~bass $ saw "55 55 82.5 55" # lpf 140 0.9
-- out $ ~kick * 0.8 + ~clap * 0.7 + ~hats * 0.4 + ~bass * 0.5


-- =============================================================================
-- PATTERN 13: Dub Siren Pattern (70 BPM)
-- =============================================================================
-- Uses oscillator for the classic dub siren effect

-- Full version with siren:
-- cps: 1.167
-- ~kick $ s "~ ~ bd ~"
-- ~rim $ s "~ ~ rim ~"
-- ~hats $ s "hh*8" $ swing 0.12
-- ~siren $ sine "880 1320 880 660" # lpf 2000 0.5 # gain 0.3
-- out $ ~kick * 0.8 + ~rim * 0.5 + ~hats * 0.4 + ~siren * 0.2


-- =============================================================================
-- PATTERN 14: Euclidean One Drop (70 BPM)
-- =============================================================================
-- Using Euclidean rhythms for interesting variations

-- One-liner with Euclidean:
"bd(1,4,2), rim(1,4,2), hh(7,8)"

-- Full version:
-- cps: 1.167
-- ~kick $ s "bd(1,4,2)"
-- ~rim $ s "rim(1,4,2)"
-- ~hats $ s "hh(7,8)" $ swing 0.1
-- out $ ~kick * 0.8 + ~rim * 0.6 + ~hats * 0.4


-- =============================================================================
-- PATTERN 15: Dub Techno Hybrid (110 BPM)
-- =============================================================================
-- Combining dub's spaciousness with techno's drive

-- One-liner:
"bd*4, ~ rim ~ rim, hh*16"

-- Full version:
-- cps: 1.833
-- ~kick $ s "bd*4"
-- ~rim $ s "~ rim ~ rim" # gain 0.5
-- ~hats $ s "hh*16" # gain 0.4
-- ~bass $ saw "55 55 82.5 110" # lpf 200 0.8
-- ~lfo $ sine 0.125
-- ~mix $ ~kick * 0.7 + ~rim * 0.5 + ~hats * 0.4 + ~bass * 0.5
-- out $ ~mix # lpf (~lfo * 3000 + 500) 0.4


-- =============================================================================
-- VARIATIONS AND TECHNIQUES
-- =============================================================================

-- Swing variations:
-- ~hats $ s "hh*8" $ swing 0.05   -- subtle
-- ~hats $ s "hh*8" $ swing 0.1    -- moderate
-- ~hats $ s "hh*8" $ swing 0.15   -- heavy

-- Euclidean variations:
-- ~kick $ s "bd(3,8)"             -- 3 hits over 8 steps
-- ~hats $ s "hh(5,16)"            -- 5 hits over 16 steps
-- ~rim $ s "rim(2,8,1)"           -- 2 hits, 8 steps, rotated by 1

-- Ghost notes:
-- ~ghost $ s "~ [~ sn:1] ~ [~ sn:1]" # gain 0.3

-- Drop outs (remove every 4th bar):
-- ~snare $ s "~ sn ~ sn" $ every 4 (const silence)

-- Speed up every 8 bars:
-- ~drums $ (s "bd ~ bd ~, ~ sn ~ sn, hh*8") $ every 8 (fast 2)
