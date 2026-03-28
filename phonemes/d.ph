-- Main thing with filter sweep
~drums $ s "gtr(<3 5>,8)" # gain 0.4 # note "c a d g" # lpf (fast 9 $ "<300 990 82 389 2000>") # delay 0.9 "<0.9 0.3>"
-- Secondary 808 pattern
~kicks $ struct "t(<5 3>,8)" $ s "808bd cp" # n 3 # speed "0.5 0.66"
~hh $ s "hh(11,16,2)"
-- Mix all
out $ ~drums + ~kicks + ~hh