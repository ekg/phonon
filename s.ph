bpm: 120
~a: s "bd(3,8)"
~b: s "~ cp" $ fast 2
~s: sine 440
~c: s "~s(<7 7 6 10>,11,2)" # note "c3'maj" # gain 0.2 # adsr 0.01 0.9 0.2 0.1
out: ~a + ~b * ~a * ~c # reverb 0.9 0.9
