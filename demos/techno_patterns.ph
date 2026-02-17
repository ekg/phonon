-- Techno Pattern Library for Phonon
-- Comprehensive collection covering Detroit, Berlin, Dub, Industrial, Melodic, and Hard Techno
-- 15+ patterns demonstrating the breadth of techno sub-genres

-- =============================================================================
-- PATTERN 1: Classic Detroit - Juan Atkins/Derrick May Style
-- Futuristic, melodic, the original techno sound
-- =============================================================================

cps: 2.1  -- ~126 BPM (classic Detroit tempo)

~kick $ s "bd*4"

-- Claps on 2 and 4 with space
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7 # gain 0.65

-- Sparse open hats
~oh $ s "oh(3,8)" # gain 0.4

-- Rimshot pattern
~rim $ s "rim(5,16)" # gain 0.45

-- Futuristic strings
~strings $ saw 130.81 + saw 196 + saw 261.63
~strings_filtered $ ~strings # lpf 3000 0.3 # reverb 0.6 0.9 * 0.1

-- Funky bass
~bass $ saw "55 ~ 73.42 ~ 55 82.5 ~ 55"
~bass_filtered $ ~bass # lpf 600 1.2 * 0.3

out $ ~kick + ~clap + ~oh + ~rim + ~strings_filtered + ~bass_filtered


-- =============================================================================
-- PATTERN 2: Berlin Industrial - Berghain/Tresor Style
-- Raw, punishing, dark
-- =============================================================================

cps: 2.25  -- ~135 BPM (hard Berlin tempo)

-- Heavy distorted kick
~kick $ s "bd:3*4" # distortion 0.6 # gain 1.1

-- Sparse clap with short reverb
~clap $ s "~ ~ cp ~" # reverb 0.15 0.35 # gain 0.7

-- Minimal hats
~hats $ s "hh(3,8)" # gain 0.5

-- Industrial noise texture
~noise_hit $ noise * "~ 1 ~ ~ ~ 0.6 ~ ~" # hpf 1500 # distortion 1.5 * 0.12

-- Dark rumbling bass
~bass $ saw 41.2 # lpf 100 2.0 # distortion 1.2 * 0.35

out $ ~kick + ~clap + ~hats + ~noise_hit + ~bass


-- =============================================================================
-- PATTERN 3: Dub Techno - Basic Channel/Deepchord Style
-- Spacious, echoing, hypnotic
-- =============================================================================

cps: 2.0  -- 120 BPM (dubbed-out tempo)

~kick $ s "bd*4" # reverb 0.1 0.35 # gain 0.85

-- Dub-delayed rimshot - the signature sound
~rim $ s "~ rim ~ ~" # delay 0.375 0.65 0.45 # reverb 0.55 0.85 # gain 0.5

-- Sparse hats with delay tails
~hats $ s "hh(5,16)" # gain 0.35 # delay 0.25 0.45 0.35

-- Dub chord stabs - heavy reverb and delay
~chord $ sine 130.81 + sine 164.81 + sine 196
~stab $ ~chord * "~ 1 ~ ~ ~ 0.5 ~ ~" # delay 0.375 0.55 0.4 # reverb 0.65 0.92 * 0.15

-- Deep sub pulse
~sub $ sine 55 # lpf 100 0.8
~sub_pulse $ ~sub * "1 ~ 0.7 ~" * 0.4

out $ ~kick + ~rim + ~hats + ~stab + ~sub_pulse


-- =============================================================================
-- PATTERN 4: Hard/Rave Techno - Dave Clarke/Green Velvet Style
-- Driving, aggressive, rave-ready
-- =============================================================================

cps: 2.33  -- ~140 BPM

~kick $ s "bd*4" # gain 1.15

~clap $ s "~ cp ~ cp" # gain 0.7

-- Fast 16th hats
~hats $ s "hh*16" # gain 0.4

-- Aggressive stab
~stab $ saw 110 # lpf 2000 2.5 # distortion 1.8
~stab_pattern $ ~stab * "~ 1 ~ 0.5 ~ 1 ~ ~" * 0.2

-- Hoover-style bass
~bass $ saw 55 + saw 55.5 + saw 54.5  -- Slight detuning for width
~bass_filtered $ ~bass # lpf 300 1.5 # distortion 0.8 * 0.3

out $ ~kick + ~clap + ~hats + ~stab_pattern + ~bass_filtered


-- =============================================================================
-- PATTERN 5: Melodic Techno - Tale of Us/Stephan Bodzin Style
-- Emotional, hypnotic, progressive elements
-- =============================================================================

cps: 2.083  -- ~125 BPM

~kick $ s "bd*4"

-- Minimal percussion
~clap $ s "~ ~ cp ~" # reverb 0.4 0.75 # gain 0.6
~hats $ s "hh(5,8)" # gain 0.4

-- Slow evolving arpeggio
~arp $ sine "130.81 164.81 196 261.63 196 164.81"
~arp_filtered $ ~arp # lpf 3500 0.5 # delay 0.333 0.4 0.35 # reverb 0.5 0.85 * 0.12

-- Massive slow-evolving pad
~lfo_very_slow # sine 0.0208  -- 48-beat cycle
~pad $ saw 65.41 + saw 98.0 + saw 130.81
~pad_evolving $ ~pad # lpf (~lfo_very_slow * 2000 + 400) 0.6 # reverb 0.75 0.95 * 0.1

-- Pulsing bass
~bass $ sine 55 # lpf 150 0.9
~bass_pulse $ ~bass * "1 0 0.7 0" * 0.4

out $ ~kick + ~clap + ~hats + ~arp_filtered + ~pad_evolving + ~bass_pulse


-- =============================================================================
-- PATTERN 6: Peak Time Techno - Charlotte de Witte/Amelie Lens Style
-- High energy, relentless, festival-ready
-- =============================================================================

cps: 2.33  -- ~140 BPM

~kick $ s "bd*4" # distortion 0.3 # gain 1.1

~clap $ s "~ cp ~ cp" # reverb 0.2 0.4 # gain 0.7

-- Driving hats with variation
~hats $ s "hh*16" $ every 4 (fast 2) # gain 0.45

-- Percussive ride
~ride $ s "~ ride ~ ride" # gain 0.4

-- Pounding bass locked to kick
~bass $ saw 55 # lpf 200 1.8 # distortion 0.6
~bass_kick $ ~bass * "1 0 0 0 1 0 0 0 1 0 0 0 1 0 0 0" * 0.35

-- Riser for energy
~lfo_build # saw 0.0625
~build_element $ saw 110 + saw 220 # lpf (~lfo_build * 5000 + 500) 2.0 * 0.08

out $ ~kick + ~clap + ~hats + ~ride + ~bass_kick + ~build_element


-- =============================================================================
-- PATTERN 7: Hypnotic Techno - Jeff Mills/Robert Hood Style
-- Stripped down, cyclical, trance-inducing
-- =============================================================================

cps: 2.166  -- ~130 BPM (the hypnotic sweet spot)

~kick $ s "bd*4"

-- Very minimal percussion
~hats $ s "~ hh ~ hh" $ swing 0.06 # gain 0.4

-- Clap only on 4
~clap $ s "~ ~ ~ cp" # gain 0.55

-- Single note hypnotic bass - pure repetition
~bass $ saw 55 # lpf 400 1.2
~bass_hypnotic $ ~bass * "1 0.6 0.8 0.6" * 0.35

-- Subtle modulation over long cycles
~lfo_long # sine 0.0416  -- 24-beat cycle
~texture $ saw 110 # lpf (~lfo_long * 800 + 200) 0.8 * 0.05

out $ ~kick + ~hats + ~clap + ~bass_hypnotic + ~texture


-- =============================================================================
-- PATTERN 8: Acid Techno - Hardfloor/Dave Angel Style
-- 303 squelch meets techno energy
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

~clap $ s "~ cp ~ ~" # gain 0.6

~hats $ s "hh*8" $ swing 0.08 # gain 0.45

-- THE 303 ACID LINE
~acid_notes $ saw "55 ~ 55 110 55 ~ 82.5 55 55 ~ 55 73.42 110 ~ 55 ~"
~accent # "1 0 0.6 1 0.8 0 1 0.5 1 0 0.7 1 0.9 0 0.6 0"
~acid $ ~acid_notes # lpf (~accent * 3500 + 150) 4.0 # distortion 1.5 * 0.22

out $ ~kick + ~clap + ~hats + ~acid


-- =============================================================================
-- PATTERN 9: Techno Breakdown - Tension Builder
-- No kick, atmospheric, building energy
-- =============================================================================

cps: 2.166

-- Ghost percussion
~ghost_hats $ s "hh*8" $ degradeBy 0.6 # reverb 0.7 0.85 # gain 0.2
~ghost_rim $ s "~ rim ~ ~" $ slow 2 # delay 0.25 0.5 0.4 # reverb 0.6 0.9 # gain 0.25

-- Rising filter tension
~lfo_rise # saw 0.0625  -- 16-beat rise
~tension_pad $ saw 82.5 + saw 123.47 + saw 164.81
~rising_pad $ ~tension_pad # lpf (~lfo_rise * 3000 + 200) 0.7 # reverb 0.7 0.95 * 0.12

-- Sub rumble
~sub_rumble $ sine 41.2 # lpf 60 0.7 * 0.3

out $ ~ghost_hats + ~ghost_rim + ~rising_pad + ~sub_rumble


-- =============================================================================
-- PATTERN 10: Schranz/Hard Industrial - Chris Liebing Style
-- Extremely hard, distorted, relentless
-- =============================================================================

cps: 2.42  -- ~145 BPM

-- Punishing kick
~kick $ s "bd*4" # distortion 1.0 # gain 1.2

-- Distorted snare hits
~snare $ s "~ sn ~ sn" # distortion 0.8 # gain 0.65

-- Industrial hats
~hats $ s "hh*8" # bitcrush 6 1.0 # gain 0.45

-- Noise bursts
~noise $ noise * "1 ~ ~ 0.5 ~ ~ 1 ~" # hpf 3000 # distortion 2.5 * 0.15

-- Grinding bass
~bass $ saw 36.71 # lpf 80 2.5 # distortion 2.0 * 0.35

out $ ~kick + ~snare + ~hats + ~noise + ~bass


-- =============================================================================
-- PATTERN 11: Atmospheric/Ambient Techno - Recondite Style
-- Dreamy, emotional, beautiful textures
-- =============================================================================

cps: 2.0  -- 120 BPM

~kick $ s "bd*4" # reverb 0.15 0.4 # gain 0.75

-- Very sparse clap
~clap $ s "~ ~ ~ cp" $ slow 2 # reverb 0.6 0.9 # gain 0.5

-- Light hats
~hats $ s "hh(3,8)" # gain 0.3 # reverb 0.4 0.7

-- Beautiful pad
~pad $ sine 130.81 + sine 196 + sine 329.63
~pad_lush $ ~pad # lpf 2000 0.4 # reverb 0.8 0.97 * 0.15

-- Melodic element
~melody $ sine "261.63 ~ 329.63 ~ 392 ~ 329.63 ~"
~melody_delayed $ ~melody # delay 0.5 0.5 0.4 # reverb 0.6 0.9 * 0.08

-- Warm bass
~bass $ sine 55 # lpf 200 0.8 * 0.35

out $ ~kick + ~clap + ~hats + ~pad_lush + ~melody_delayed + ~bass


-- =============================================================================
-- PATTERN 12: EBM/Industrial Crossover - Front 242/Nitzer Ebb Influence
-- Militant, aggressive, synth-driven
-- =============================================================================

cps: 2.25  -- ~135 BPM

~kick $ s "bd*4" # distortion 0.5

-- Snare on 3 only (militant feel)
~snare $ s "~ ~ sn ~" # gain 0.7

-- Marching hats
~hats $ s "hh*8" # gain 0.5

-- Aggressive bass sequence
~bass $ saw "55 55 55 41.2 55 55 73.42 55"
~bass_dist $ ~bass # lpf 400 2.0 # distortion 1.5 * 0.3

-- Synth stab
~stab $ saw 220 + saw 330
~stab_pattern $ ~stab * "1 ~ ~ 1 ~ 1 ~ ~" # lpf 2500 1.5 # distortion 0.8 * 0.15

out $ ~kick + ~snare + ~hats + ~bass_dist + ~stab_pattern


-- =============================================================================
-- PATTERN 13: Trance-Techno Hybrid - Early 90s Style
-- Uplifting, driving, melodic
-- =============================================================================

cps: 2.25

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.3 0.6

-- Rolling hats
~hats $ s "hh*16" # gain "0.3 0.5 0.4 0.6"

-- Classic trance arp
~arp $ sine "130.81 164.81 196 261.63"
~arp_fast $ ~arp $ fast 2 # delay 0.167 0.4 0.35 # reverb 0.4 0.8 * 0.1

-- Supersaw-style pad (layered saws)
~pad $ saw 130.81 + saw 131.5 + saw 130.1 + saw 196 + saw 196.7 + saw 195.3
~pad_filtered $ ~pad # lpf 4000 0.4 # reverb 0.5 0.85 * 0.08

-- Driving bass
~bass $ saw 65.41 # lpf 300 1.3 * 0.35

out $ ~kick + ~clap + ~hats + ~arp_fast + ~pad_filtered + ~bass


-- =============================================================================
-- PATTERN 14: Polyrhythmic Techno - Euclidean Exploration
-- Complex interlocking rhythms
-- =============================================================================

cps: 2.166

~kick $ s "bd*4"

-- Multiple Euclidean layers
~hats $ s "hh(7,16)" # gain 0.4
~rim $ s "rim(5,8,1)" # gain 0.5  -- Rotated by 1
~clap $ s "cp(3,8,2)" # reverb 0.3 0.5 # gain 0.55  -- Rotated by 2
~perc $ s "perc(9,16)" # gain 0.35

-- Bass locked to kick
~bass $ saw 55 # lpf 250 1.3 * 0.35

out $ ~kick + ~hats + ~rim + ~clap + ~perc + ~bass


-- =============================================================================
-- PATTERN 15: Full Drop - Maximum Energy
-- Everything combined for peak moment
-- =============================================================================

cps: 2.166

~kick $ s "bd*4" # gain 1.05

-- Full percussion stack
~clap $ s "~ cp ~ cp" # reverb 0.2 0.45 # gain 0.7
~hats $ s "hh*16" $ degradeBy 0.15 $ swing 0.05 # gain 0.45
~ride $ s "~ ~ ride ~" # gain 0.4
~rim $ s "~ rim ~ [rim rim]" $ swing 0.08 # gain 0.45

-- Driving bass with filter motion
~lfo_bass # sine 0.5
~bass $ saw 55 # lpf (~lfo_bass * 250 + 200) 1.5 * 0.35

-- Energy synth
~synth $ saw 110 + saw 165
~synth_pattern $ ~synth * "~ 1 ~ 0.7 ~ 1 ~ 0.5" # lpf 3000 1.0 * 0.1

out $ ~kick + ~clap + ~hats + ~ride + ~rim + ~bass + ~synth_pattern # reverb 0.15 0.5 0.1


-- =============================================================================
-- Quick Reference - Techno Subgenres BPM/CPS
-- =============================================================================
-- Dub Techno:        118-125 BPM = cps: 1.97-2.08
-- Detroit Techno:    125-135 BPM = cps: 2.08-2.25
-- Melodic Techno:    120-130 BPM = cps: 2.0-2.17
-- Berlin Techno:     128-138 BPM = cps: 2.13-2.3
-- Peak Time:         135-145 BPM = cps: 2.25-2.42
-- Hard/Industrial:   140-150 BPM = cps: 2.33-2.5

-- =============================================================================
-- Key Frequencies for Dark Techno Keys
-- =============================================================================
-- E1 = 41.2   F1 = 43.65   G1 = 49.0    A1 = 55
-- E2 = 82.5   F2 = 87.31   G2 = 98.0    A2 = 110
-- E3 = 164.81 F3 = 174.61  G3 = 196     A3 = 220
-- Minor chords and tritones are common in techno

-- =============================================================================
-- Filter Resonance Guide for 303-Style Acid
-- =============================================================================
-- 0.7-1.0  = Subtle warmth
-- 1.5-2.0  = Noticeable resonance
-- 2.5-3.5  = Classic acid squelch
-- 4.0+     = Self-oscillation territory
