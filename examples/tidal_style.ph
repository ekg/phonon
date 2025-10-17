# Tidal-style sample patterns using 's' command
# This demonstrates the integration of mini-notation parser with sample playback

# Simple four-on-the-floor kick pattern
~kick_pattern = s "bd bd bd bd"

# Clap on beats 2 and 4 with some rests
~clap_pattern = s "~ cp ~ cp"

# Fast hihat pattern with grouping
~hihat_pattern = s "hh [hh hh] hh hh"

# Mix all the patterns together
out = ~kick_pattern + ~clap_pattern + ~hihat_pattern # mul 0.6