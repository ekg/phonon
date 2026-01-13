-- VST3 Multi-Synth Demo with Sick Reverb
-- Press C-x to evaluate, Alt+G to open all plugin GUIs
-- Tweak sounds in the GUIs, changes reflect immediately!

cps: 0.4

-- Surge XT - ethereal pad with chord progression
-- (Open GUI with Alt+G, try different patches!)
~pad $ vst "Surge XT" # note "<c4 e4 g4 b4> <d4 f4 a4 c5> <e4 g4 b4 d5> <c4 e4 g4 b4>"

-- OB-Xd - fat analog bass
~bass $ vst "OB-Xd" # note "c2 ~ c2 g2 ~ g2 e2 ~"

-- Dexed - sparkly FM bells
~bells $ vst "Dexed" # note "~ c6 ~ e6 ~ g6 ~ c7"

-- Mix everything with sick built-in reverb
-- (VST effect chaining coming soon!)
out $ (~pad * 0.4 + ~bass * 0.5 + ~bells * 0.3) # reverb 0.85 0.6
