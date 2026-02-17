-- Groove Showcase
-- Demonstrates groove templates for humanized timing

cps: 0.5

-- Basic pattern without groove (quantized/robotic)
-- ~straight $ s "bd sn hh*4 cp"

-- MPC-style swing: delays off-beat 16th notes
-- Classic hip-hop / house groove
~mpc $ s "bd sn hh*4 cp" $ groove "mpc"

-- Hip-hop lazy feel: drags beats 2 and 4
-- ~hiphop $ s "bd ~ sn ~ bd bd sn ~" $ groove "hiphop"

-- Jazz swing: triplet-feel on 8th notes
-- ~jazz $ s "hh*8" $ groove "jazz" 0.8

-- Reggae one-drop: pushes the backbeat
-- ~reggae $ s "~ ~ sn ~ ~ ~ sn ~" $ groove "reggae"

-- Drunken drummer: pseudo-random humanization
-- ~drunk $ s "bd sn hh cp" $ groove "drunken" 0.3

-- Groove with pattern-controlled amount
-- Fades in/out the groove feel across 4 cycles
-- ~morphing $ s "bd sn hh*4 cp" $ groove "mpc" "0 0.3 0.7 1.0"

-- Compose groove with other transforms
-- ~groovy_fast $ s "bd sn hh cp" $ fast 2 $ groove "jazz"

-- Apply groove conditionally every 2 cycles
-- ~conditional $ s "bd sn hh*4 cp" $ every 2 (groove "mpc" 0.6)

out $ ~mpc
