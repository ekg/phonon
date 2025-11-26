# MIDI Polyrhythm Example
#
# Record two different patterns and layer them

tempo: 2.0

# Pattern 1: Bass line (record with Alt+R, insert with Alt+N)
# Example: 3 notes over 1 cycle
~bass_intervals: n "0 7 12"  # Root, fifth, octave
~bass_vel: "1.0 0.7 0.8"

# Pattern 2: Melody (record separately)
# Example: 4 notes over 1 cycle
~melody_intervals: n "0 4 7 4"  # Triad arpeggio
~melody_vel: "0.6 0.8 1.0 0.7"

# Apply velocities
~bass: ~bass_intervals # gain ~bass_vel
~melody: ~melody_intervals # gain ~melody_vel

# Add octave offset to melody
~melody_high: ~melody + 12

# Mix
out: ~bass * 0.6 + ~melody_high * 0.4
