// 🏠 HOUSE MUSIC - 128 BPM
// Classic Chicago/Detroit house pattern

// Simple version - 4 bar pattern
"bd ~ bd ~ bd ~ bd ~ bd ~ bd ~ bd ~ bd ~, ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~ ~ ~, hh hh oh hh hh hh oh hh hh hh oh hh hh hh oh hh"

// Full Strudel version (needs complete parser):
/*
stack(
  // Four-on-floor kick drum 
  "bd*4",
  
  // Clap on 2 and 4
  "~ cp ~ cp",
  
  // Hi-hat pattern with variation every 2 bars
  "<hh*8 [hh*6 oh oh]>",
  
  // Bassline - 8 bar progression
  "<c2 c2 eb2 g2 c2 c2 f2 g2>".slow(2),
  
  // Piano stabs - come in every 4 bars
  "~ ~ [c4,e4,g4,bb4] ~ ~ ~ [f3,a3,c4,e4] ~".slow(4),
  
  // Percussion layer - builds over 16 bars
  "<~ ~ perc*2 perc*4>".slow(4),
  
  // Open hat accent - every 2 bars
  "~ ~ ~ oh ~ ~ ~ ~".slow(2)
).slow(2)  // Entire pattern over 2 cycles = 8 beats
*/

// Variations to try:
// "bd*4, ~ cp ~ cp, hh(7,8), ~ oh ~ oh ~ oh ~ oh"  // Euclidean hats
// "bd ~ bd ~ bd ~ bd ~, ~ ~ cp ~, hh*16"            // Minimal house
// "bd*4, [~ cp]*2, hh*8, bass:1 ~ bass:2 ~"         // With bass samples