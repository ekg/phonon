-- Dub Delay Example
-- Classic dub techno delay with HPF in feedback loop
-- The high-pass filter prevents low-frequency buildup in the echoes

tempo: 2.0

-- Source signal (kick-like bass tone)
~kick: sine 55 * 0.6

-- Dub delay chain: delay with HPF removes lows from feedback
~dub: ~kick # delay 0.375 0.75 # hpf 800 0.7

-- Mix dry and wet signals
out: ~kick * 0.5 + ~dub * 0.5

-- Try adjusting:
-- - delay time (0.375) for different rhythmic feels
-- - feedback amount (0.75) for more/less echoes
-- - HPF cutoff (800) to control echo tone
