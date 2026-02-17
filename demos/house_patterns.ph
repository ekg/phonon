-- House Music Pattern Library for Phonon
-- Comprehensive collection covering Chicago, Deep, Acid, Progressive, and Tech House
-- 15+ patterns demonstrating four-on-the-floor grooves and sub-genre variations

-- =============================================================================
-- PATTERN 1: Chicago House - Classic Frankie Knuckles/Marshall Jefferson
-- The original - funky, soulful, uplifting
-- =============================================================================

cps: 2.0  -- 120 BPM (classic Chicago tempo)

-- Four-on-the-floor: the foundation
~kick $ s "bd*4"

-- Claps on 2 and 4 with reverb for that warehouse sound
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7 # gain 0.7

-- Swinging 16th hats - the Chicago shuffle
~hats $ s "hh*16" $ swing 0.12 # gain "0.4 0.6 0.5 0.7 0.4 0.6 0.5 0.8"

-- Off-beat open hats
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.5

-- Funky syncopated bass
~bass $ saw "55 ~ 55 82.5 ~ 55 ~ 110" # lpf 400 1.2 * 0.35

out $ ~kick + ~clap + ~hats + ~oh + ~bass


-- =============================================================================
-- PATTERN 2: Deep House - Larry Heard/Kerri Chandler Style
-- Warm, soulful, slower tempo
-- =============================================================================

cps: 1.97  -- ~118 BPM (deep house sweet spot)

~kick $ s "bd*4"

-- Sparse clap with lots of reverb
~clap $ s "~ cp ~ ~" # reverb 0.6 0.9 # gain 0.6

-- Gentle 8th note hats
~hats $ s "hh*8" # gain 0.35

-- Rim on beat 4 for groove
~rim $ s "~ ~ ~ rim" # gain 0.4

-- Warm sub bass - sine for purity
~bass $ sine 55 # lpf 200 0.8
~bass_pattern $ ~bass * "1 ~ 0.7 ~ 0.8 ~ ~ 0.6"

-- Lush pad for atmosphere
~pad $ sine 130.81 + sine 155.56 + sine 196
~pad_filtered $ ~pad # reverb 0.7 0.95 # lpf 1500 0.5 * 0.12

out $ ~kick * 0.85 + ~clap + ~hats + ~rim + ~bass_pattern * 0.4 + ~pad_filtered


-- =============================================================================
-- PATTERN 3: Acid House - Phuture/808 State Style
-- Squelchy 303 basslines with high resonance
-- =============================================================================

cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"
~hats $ s "hh*16" # gain 0.5

-- THE ACID LINE: resonant filter with accent pattern
~bass $ saw "55 55 110 55 82.5 55 110 55"
~accent # "1 0.5 1 0.7 1 0.5 0.8 1"
~acid $ ~bass # lpf (~accent * 2000 + 200) 3.5 # distortion 1.5 * 0.25

out $ ~kick + ~clap + ~hats + ~acid


-- =============================================================================
-- PATTERN 4: Progressive House - Sasha/Digweed Style
-- Longer builds, epic breakdowns, slower evolution
-- =============================================================================

cps: 2.1  -- ~126 BPM

~kick $ s "bd*4"

-- Clap only on beat 4 (more sparse)
~clap $ s "~ ~ ~ cp" # reverb 0.4 0.8 # gain 0.65

-- Driving 8th hats with ride layer
~hats $ s "hh*8" # gain 0.45
~ride $ s "~ ride*4" # gain 0.35

-- Slow arpeggiated synth
~arp $ sine "130.81 196 261.63 196" $ fast 2
~arp_filtered $ ~arp # lpf 4000 0.5 # delay 0.25 0.4 0.3 * 0.12

-- Very slow filter sweep on pad (32-beat cycle)
~lfo_slow # sine 0.03125
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_filtered $ ~pad # lpf (~lfo_slow * 3000 + 500) 0.7 # reverb 0.6 0.9 * 0.1

out $ ~kick + ~clap + ~hats + ~ride + ~arp_filtered + ~pad_filtered


-- =============================================================================
-- PATTERN 5: Tech House - Green Velvet/Carl Cox Style
-- Driving, hypnotic, blend of techno and house
-- =============================================================================

cps: 2.083  -- ~125 BPM

~kick $ s "bd*4"

~clap $ s "~ cp ~ cp" # gain 0.65
~rim $ s "~ ~ rim ~" # gain 0.5

-- Tight 16th hats with subtle swing
~hats $ s "hh*16" $ swing 0.06 # gain 0.4

-- Punchy bass that locks with kick
~bass $ saw 55 # lpf 300 1.3
~bass_pattern $ ~bass * "1 0 0.8 0 1 0 0.6 0"

-- Filter modulation for movement
~lfo # sine 0.25
~bass_filtered $ ~bass_pattern # lpf (~lfo * 200 + 300) 1.0 * 0.35

out $ ~kick + ~clap + ~rim + ~hats + ~bass_filtered


-- =============================================================================
-- PATTERN 6: Disco House - Daft Punk/Stardust Style
-- Filtered loops, funky grooves, chopped samples
-- =============================================================================

cps: 2.0  -- 120 BPM

~kick $ s "bd*4"

-- Layered clap/snare on 2 and 4
~clap $ s "~ cp ~ cp" # gain 0.6
~snare $ s "~ sn ~ sn" # gain 0.35

-- Funky hi-hat pattern with open hat accents
~hats $ s "hh hh oh hh hh hh oh hh" $ swing 0.08 # gain 0.5

-- Disco bass - octave jumps
~bass $ saw "55 110 55 82.5 110 55 82.5 55"
~bass_filtered $ ~bass # lpf 700 1.0 * 0.3

-- Classic filter sweep
~lfo # saw 0.125  -- 8-beat ramp
~filtered_element $ saw 220 # lpf (~lfo * 3000 + 200) 1.8 * 0.08

out $ ~kick + ~clap + ~snare + ~hats + ~bass_filtered + ~filtered_element


-- =============================================================================
-- PATTERN 7: French House - Cassius/Modjo Style
-- Phased samples, side-chain feel, compressed
-- =============================================================================

cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp"

-- Constant 16ths for that compressed feel
~hats $ s "hh*16" # gain 0.35

-- Pumping bass - dips on kick hits
~bass $ saw 55 # lpf 500 1.2
~pump_envelope # "0.3 0.8 1 1 0.3 0.8 1 1"  -- Simulates side-chain
~pumped_bass $ ~bass * ~pump_envelope * 0.35

-- Filtered chord stab
~chord $ saw 130.81 + saw 155.56 + saw 196
~chord_pump $ ~chord * ~pump_envelope # lpf 2500 0.7 * 0.15

out $ ~kick + ~clap + ~hats + ~pumped_bass + ~chord_pump


-- =============================================================================
-- PATTERN 8: Garage House (US Garage) - Todd Edwards/MAW Style
-- Chopped vocals, shuffled beats, organ stabs
-- =============================================================================

cps: 2.0  -- 120 BPM

~kick $ s "bd*4"

-- Classic 2-and-4 with reverb tail
~clap $ s "~ cp ~ cp" # reverb 0.35 0.6 # gain 0.7

-- Swung 16ths - essential for US garage feel
~hats $ s "hh*16" $ swing 0.15 # gain "0.35 0.5 0.4 0.55"

-- Shaker layer
~shaker $ s "shaker*8" # gain 0.25

-- Organ-style bass
~bass $ sine "55 ~ 55 ~ 82.5 ~ 55 ~"
~bass_filtered $ ~bass # lpf 300 0.9 * 0.4

-- Staccato chord hits
~chord $ sine 196 + sine 246.94 + sine 293.66
~stab $ ~chord * "~ 1 ~ 0.5 ~ 1 ~ ~" # reverb 0.3 0.5 * 0.12

out $ ~kick + ~clap + ~hats + ~shaker + ~bass_filtered + ~stab


-- =============================================================================
-- PATTERN 9: Vocal House / Handbag House
-- Big piano chords, diva vocals territory, uplifting
-- =============================================================================

cps: 2.083  -- ~125 BPM

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.4 0.7

-- 8th note hats with accented open hat
~hats $ s "hh*8" # gain 0.45
~oh $ s "~ ~ oh ~ ~ ~ oh ~" # gain 0.5 # reverb 0.3 0.6

-- Big piano chord pattern (C major 7)
~piano_c $ sine 130.81 + sine 164.81 + sine 196 + sine 246.94
~piano_f $ sine 174.61 + sine 220 + sine 261.63 + sine 329.63
~piano $ (~piano_c * "1 ~ ~ 0.7") + (~piano_f * "~ 0.8 ~ ~")
~piano_filtered $ ~piano # lpf 3500 0.5 # reverb 0.5 0.8 * 0.15

-- Classic octave bass
~bass $ saw "55 55 110 55" # lpf 350 1.1 * 0.35

out $ ~kick + ~clap + ~hats + ~oh + ~piano_filtered + ~bass


-- =============================================================================
-- PATTERN 10: Tribal House - Danny Tenaglia/Junior Vasquez Style
-- Heavy percussion, congas, ritualistic
-- =============================================================================

cps: 2.083

~kick $ s "bd*4"

-- Sparse clap
~clap $ s "~ ~ cp ~" # reverb 0.25 0.5 # gain 0.6

-- Layered percussion for tribal feel
~conga $ s "~ ~ [conga:0 ~] conga:1 ~ conga:0 ~ ~" # gain 0.5
~tom $ s "~ ~ ~ ~ ~ ~ tom:0 ~" $ slow 2 # reverb 0.3 0.4 # gain 0.5
~shaker $ s "shaker*8" $ swing 0.1 # gain 0.3

-- Minimal hats
~hats $ s "hh*8" # gain 0.35

-- Deep sub
~bass $ sine 55 # lpf 120 0.8 * 0.45

out $ ~kick + ~clap + ~hats + ~conga + ~tom + ~shaker + ~bass


-- =============================================================================
-- PATTERN 11: Minimal House - Ricardo Villalobos/Luciano Style
-- Stripped back, shuffled, micro-edits
-- =============================================================================

cps: 2.0  -- 120 BPM

~kick $ s "bd*4"

-- Heavy shuffle on minimal elements
~hats $ s "hh*8" $ swing 0.18 $ degradeBy 0.3 # gain 0.35
~rim $ s "~ rim [~ rim] ~" $ swing 0.12 # gain 0.45

-- Very minimal melodic element
~bass $ sine "55 ~ ~ 82.5 ~ 55 ~ ~" * 0.4

-- Subtle atmospheric texture
~texture $ noise # lpf 2000 0.5 # hpf 800 0.5 * 0.03

out $ ~kick * 0.85 + ~hats + ~rim + ~bass + ~texture


-- =============================================================================
-- PATTERN 12: Afro House - Black Coffee/Louie Vega Style
-- Organic percussion, African rhythms, spiritual
-- =============================================================================

cps: 2.0

~kick $ s "bd*4"

~clap $ s "~ cp ~ ~" # reverb 0.4 0.7 # gain 0.6

-- Complex polyrhythmic percussion
~djembe $ s "djembe:0 ~ [djembe:1 djembe:0] ~ djembe:1 ~ djembe:0 ~" # gain 0.5
~shaker $ s "shaker(5,8)" $ swing 0.1 # gain 0.35
~conga $ s "~ conga:0 ~ conga:1" # gain 0.4

-- Sparse hats
~hats $ s "~ hh ~ hh" # gain 0.4

-- Melodic bass in minor key
~bass $ sine "55 ~ 55 65.41 ~ 55 ~ 73.42"
~bass_filtered $ ~bass # lpf 250 0.9 * 0.4

out $ ~kick + ~clap + ~hats + ~djembe + ~shaker + ~conga + ~bass_filtered


-- =============================================================================
-- PATTERN 13: Soulful House - Louie Vega/Kenny Dope Style
-- Gospel chords, Hammond organ, uplifting progressions
-- =============================================================================

cps: 2.0

~kick $ s "bd*4"
~clap $ s "~ cp ~ cp" # reverb 0.35 0.65

~hats $ s "hh*16" $ swing 0.1 # gain 0.4
~oh $ s "~ oh ~ oh ~ oh ~ oh" # gain 0.45

-- Hammond-style organ pad
~organ_root $ sine 130.81 + sine 196 + sine 261.63
~organ_third $ sine 155.56 + sine 220 + sine 311.13
~organ $ ~organ_root * "1 ~ ~ 1" + ~organ_third * "~ 1 1 ~"
~organ_filtered $ ~organ # lpf 2000 0.6 # reverb 0.5 0.85 * 0.12

-- Walking bass
~bass $ saw "55 65.41 73.42 82.5 73.42 65.41 55 55"
~bass_filtered $ ~bass # lpf 400 1.0 * 0.3

out $ ~kick + ~clap + ~hats + ~oh + ~organ_filtered + ~bass_filtered


-- =============================================================================
-- PATTERN 14: Electro House - Benny Benassi/Deadmau5 Style
-- Heavy sidechained bass, aggressive filter sweeps
-- =============================================================================

cps: 2.133  -- 128 BPM

~kick $ s "bd*4" # gain 1.1

~clap $ s "~ cp ~ cp" # gain 0.7

-- Driving hats
~hats $ s "hh*16" # gain 0.4

-- Big sidechained saw bass
~bass $ saw 55 # lpf 400 1.5
~sidechain # "0.2 0.6 1 1 0.2 0.6 1 1"
~bass_pumped $ ~bass * ~sidechain * 0.4

-- Filter sweep for builds
~lfo_fast # saw 0.5
~lead $ saw 110 + saw 220
~lead_sweep $ ~lead * ~sidechain # lpf (~lfo_fast * 4000 + 500) 2.0 * 0.12

out $ ~kick + ~clap + ~hats + ~bass_pumped + ~lead_sweep


-- =============================================================================
-- PATTERN 15: Breakdown Section - No Kick
-- For transitional moments - atmospheric and building tension
-- =============================================================================

cps: 2.0

-- Reverbed percussion hints
~hats $ s "~ ~ hh? ~" $ swing 0.1 # reverb 0.7 0.8 # gain 0.25
~rim $ s "~ rim ~ ~" $ slow 2 # reverb 0.6 0.85 # delay 0.375 0.4 0.3 # gain 0.3

-- Building pad with slow filter
~lfo_tension # sine 0.0416  -- 24-beat cycle
~pad $ saw 130.81 + saw 164.81 + saw 196
~pad_building $ ~pad # lpf (~lfo_tension * 2000 + 300) 0.6 # reverb 0.7 0.95 * 0.15

-- Sub rumble
~sub $ sine 55 # lpf 80 0.7 * 0.25

out $ ~hats + ~rim + ~pad_building + ~sub


-- =============================================================================
-- Quick Reference - House Subgenres BPM/CPS
-- =============================================================================
-- Deep House:        118-122 BPM = cps: 1.97-2.03
-- Chicago House:     120-125 BPM = cps: 2.0-2.08
-- Tech House:        124-128 BPM = cps: 2.07-2.13
-- Progressive House: 126-132 BPM = cps: 2.1-2.2
-- Electro House:     126-132 BPM = cps: 2.1-2.2
-- Acid House:        120-130 BPM = cps: 2.0-2.17

-- =============================================================================
-- Swing Settings Guide
-- =============================================================================
-- 0.0      = Straight (robotic)
-- 0.06-0.08 = Subtle swing (tech house)
-- 0.10-0.12 = Medium swing (Chicago house)
-- 0.15-0.18 = Heavy shuffle (US garage, minimal)
-- 0.20+     = Extreme shuffle (rarely used)

-- =============================================================================
-- Common Chord Frequencies (Hz)
-- =============================================================================
-- C3 = 130.81   C#3 = 138.59   D3 = 146.83   Eb3 = 155.56
-- E3 = 164.81   F3 = 174.61    G3 = 196      Ab3 = 207.65
-- A3 = 220      Bb3 = 233.08   B3 = 246.94   C4 = 261.63
