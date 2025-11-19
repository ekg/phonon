-- Phaser Effect Demo
-- Demonstrates various phaser configurations

tempo: 2.0

-- Classic phaser on saw wave
~classic: saw 110 # phaser 0.4 0.7 0.5 4

-- Deep dramatic phaser on pad
~pad: sine 220
~deep: ~pad # phaser 0.2 0.9 0.6 8

-- Fast phaser on higher frequency
~fast: saw 440 # phaser 2.0 0.6 0.4 4

-- Pattern-modulated phaser
~rate_lfo: sine 0.1 * 1.0 + 1.0
~depth_lfo: sine 0.2 * 0.3 + 0.5
~modulated: saw 220 # phaser ~rate_lfo ~depth_lfo 0.4 6

-- Mix all outputs
out: (~classic * 0.2 + ~deep * 0.2 + ~fast * 0.2 + ~modulated * 0.2) * 0.5
