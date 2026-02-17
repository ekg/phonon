-- BREAKBEAT PATTERN LIBRARY
-- Research-based patterns from the jungle/breakbeat/breakcore tradition
-- Based on docs/BREAKBEAT_JUNGLE_RESEARCH.md

-- ============================================
-- TEMPO SETTINGS
-- ============================================
-- Jungle: 160-175 BPM (tempo: 2.67-2.92)
-- Drum & Bass: 170-180 BPM (tempo: 2.83-3.0)
-- Breakcore: 160-200+ BPM (can go faster)
-- Original Amen: 137 BPM (tempo: 2.28)

tempo: 2.8  -- ~168 BPM (classic jungle tempo)


-- ============================================
-- 1. THE AMEN BREAK - ORIGINAL PATTERN
-- ============================================
-- The most sampled drum break in history
-- From "Amen, Brother" by The Winstons (1969)
-- Gregory C. Coleman on drums

-- Full 4-bar Amen pattern (programmatic recreation)
-- Bars 1-2: Standard funk groove
-- Bars 3-4: The "tumbling" syncopated feel

~amen_kick_1 $ s "[bd ~ bd ~] [~ ~ ~ ~] [~ ~ bd bd] [~ ~ ~ ~]"
~amen_snare_1 $ s "[~ ~ ~ ~] [sn ~ ~ sn] [~ sn ~ ~] [sn ~ ~ sn]"
~amen_hh_1 $ s "[hh ~ hh ~]*4"

-- Simplified single-cycle Amen
-- Pattern 1: Basic Amen Loop
~amen_basic $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn ~ ~ ~ sn, hh*8"

-- Pattern 2: Amen with open hats (bar 3 feel)
~amen_open $ s "bd ~ bd bd ~ ~ bd ~, ~ sn ~ ~ sn ~ ~ sn, hh ~ hh ~ hh ~ oh ~"


-- ============================================
-- 2. CLASSIC JUNGLE PATTERNS
-- ============================================

-- Pattern 3: Basic Jungle Break
-- The quintessential jungle rhythm - syncopated kicks with snares
~jungle_basic $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8"

-- Pattern 4: Ragga Jungle
-- Reggae-influenced with more space
~ragga_jungle $ s "bd ~ ~ ~ bd ~ ~ ~, ~ ~ sn ~ ~ sn ~ ~, hh*4"

-- Pattern 5: Rolling Jungle
-- Continuous 16th note hats, driving energy
~rolling_jungle $ s "bd ~ ~ bd ~ bd ~ ~, ~ ~ sn ~ ~ ~ sn ~, hh*16" * 0.8

-- Pattern 6: Darkside Jungle
-- Harder, more aggressive pattern (higher tempo recommended)
~darkside $ s "[bd bd] ~ bd ~ ~ bd [bd ~] ~, ~ sn ~ sn ~ ~ sn sn, hh*16"


-- ============================================
-- 3. CHOPPED BREAK PATTERNS
-- ============================================

-- Pattern 7: Amencutup Classic
-- Using pre-sliced amen samples (slices 0-31)
~amen_chop_1 $ s "amencutup:0 amencutup:1 ~ amencutup:2 ~ ~ amencutup:3 ~ amencutup:4 ~ ~ ~ amencutup:5 ~ ~ ~"

-- Pattern 8: Reversed Slice Pattern
-- Order reversed for variation
~amen_chop_rev $ s "amencutup:7 amencutup:6 amencutup:5 amencutup:4 amencutup:3 amencutup:2 amencutup:1 amencutup:0"

-- Pattern 9: Random Cutup Feel
-- Classic live-coding pattern with bracketed groups
~amen_random $ s "amencutup:0 [amencutup:1 amencutup:5] amencutup:2*2 [amencutup:3 amencutup:4]"

-- Pattern 10: Think Break Style
-- Inspired by "Think (About It)" by Lyn Collins
~think_style $ s "bd ~ ~ bd ~ bd ~ ~, ~ ~ sn ~, hh*8"


-- ============================================
-- 4. BREAKCORE PATTERNS
-- ============================================

-- Pattern 11: Breakcore Blast
-- Fast, aggressive, distorted (use tempo: 3.3+ for authentic feel)
~breakcore_blast $ s "[bd bd bd] sn [bd bd] sn [bd bd bd bd] sn bd sn"

-- Pattern 12: Glitch Break
-- Irregular subdivisions and stutters
~glitch_break $ s "bd [sn sn sn] ~ [[bd bd] cp] ~ bd [bd sn] ~, hh*16"

-- Pattern 13: Drill Break
-- Rapid-fire snare rolls
~drill_break $ s "bd ~ [sn sn sn sn] ~ bd ~ [sn*8] ~"


-- ============================================
-- 5. TRANSFORMED PATTERNS
-- ============================================

-- Pattern 14: Swung Jungle
-- Adding swing for human feel
~swung_jungle $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8" $ swing 0.15

-- Pattern 15: Euclidean Jungle
-- Using Euclidean rhythms for variation
~euclidean_jungle $ s "bd(5,16), sn(3,8,4), hh*16"

-- Pattern 16: Ghost Note Pattern
-- Main hits + quiet ghost notes for groove
~ghost_main $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn"
~ghost_quiet $ s "~ ~ ~ bd ~ ~ ~ ~, sn ~ ~ ~ ~ sn ~ ~" * 0.3

-- Pattern 17: Polyrhythmic Break
-- 3-against-4 feel
~poly_break $ s "[bd bd bd, sn sn sn sn], hh*12"


-- ============================================
-- 6. LAYERED BREAK TECHNIQUES
-- ============================================

-- Classic technique: layer break + clean hits
-- High-pass the break, let layered kick provide low-end

-- Pattern 18: Layered Jungle
~break_layer $ s "breaks152" $ loopAt 2 $ chop 8 # hpf 200 0.7
~clean_kick $ s "bd ~ ~ ~ ~ bd ~ ~"
~clean_snare $ s "~ ~ ~ ~ sn ~ ~ ~"
~layered $ ~break_layer * 0.5 + ~clean_kick * 0.8 + ~clean_snare * 0.9


-- ============================================
-- 7. VARIATIONS WITH EFFECTS
-- ============================================

-- Pattern 19: Filtered Jungle
~filtered_jungle $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*16" # lpf 4000 0.7

-- Pattern 20: Delayed Break
~delayed_break $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn" # delay 0.125 0.4 0.25

-- Pattern 21: Lo-Fi Break
~lofi_break $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn, hh*8" # bitcrush 10 22050 # lpf 6000 0.6


-- ============================================
-- FULL PRODUCTION EXAMPLES
-- ============================================

-- Example A: Classic Jungle Track
~jungle_kicks $ s "bd ~ bd ~ ~ ~ bd ~"
~jungle_snares $ s "~ sn ~ sn ~ ~ ~ sn"
~jungle_hats $ s "hh*16" * 0.5
~jungle_drums $ ~jungle_kicks + ~jungle_snares + ~jungle_hats

-- Sub bass following the kicks
~jungle_sub $ sine "55 ~ 55 ~ ~ ~ 55 ~" * 0.3

-- Full mix
~jungle_full $ ~jungle_drums # reverb 0.3 0.6 0.15 + ~jungle_sub


-- Example B: Breakcore Chaos
-- tempo: 3.3 for authentic feel
~chaos_drums $ s "[bd bd] sn [bd bd] sn [bd bd bd] sn bd sn" $ every 4 rev $ every 3 (fast 1.5)
~chaos_filtered $ ~chaos_drums # hpf 100 0.8 # distortion 2.0


-- Example C: Atmospheric Jungle
~atmo_break $ s "bd ~ bd ~ ~ ~ bd ~, ~ sn ~ sn" # reverb 0.6 0.5 0.4
~atmo_hats $ s "hh*8" $ degradeBy 0.3 # delay 0.25 0.5 0.3 * 0.4
~atmo_pad $ sine "[55 82.5 110]" * 0.08


-- ============================================
-- QUICK REFERENCE
-- ============================================

-- Tempos:
-- tempo: 2.28  -- 137 BPM (original Amen)
-- tempo: 2.67  -- 160 BPM (slow jungle)
-- tempo: 2.75  -- 165 BPM (classic jungle)
-- tempo: 2.8   -- 168 BPM (standard jungle)
-- tempo: 2.9   -- 174 BPM (DnB crossover)
-- tempo: 3.0   -- 180 BPM (fast jungle)
-- tempo: 3.3+  -- 200+ BPM (breakcore)

-- Essential pattern elements:
-- Kicks: Syncopated, often on 1, 3-and, 6th 8th note
-- Snares: Usually beats 2 & 4, with ghost notes
-- Hats: 8th or 16th notes, often swung
-- Breaks: Chop and rearrange!

-- Output one of the patterns to test:
out $ ~jungle_basic # lpf 8000 0.7
