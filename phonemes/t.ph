~q: sine "<0.3 0.1 0.9 0.01>" * -(square "<223 221 219>")
~a: s "~q" # adsr :attack 0.01 :decay 0.1 :sustain 0.1 :release 3 
~b: ~a # reverb :room_size 1.0 :damping 0.1 :mix 0.9
~c: ~a # plate 1.0 1.0 0.7 :damping 0.3 :mod_depth 0.3 :mix 0.5