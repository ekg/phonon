bpm: 120
~a: s "bd(3,8)"
~b: s "~ cp" $ fast 2
~s: stack [(saw 112) + (sine 110), (saw 109) + (sine 108)]
~c: slow 8 $ s "~s(<7 7 6 10>,11,2)" # note "c3'maj" # gain 1
--~q: s "~"
~q: s "[~s ~c](3,8)" # lpf 900 # delay (3/7) 0.8
~a: s "808bd(3,8)" # n 3 # gain 2
out: ~q + ~a
--out: ~a + ~b * ~a * ~c # reverb 0.9 0.9
