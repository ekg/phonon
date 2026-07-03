//! Scale quantization + note-name parsing for the pattern DSL.
//!
//! This module exposes the scale/note machinery that already lives in
//! [`crate::midi_input::Scale`] to the compositional DSL, so a live coder can
//! write melodies with scale degrees and note names instead of raw numbers:
//!
//! ```text
//! ~mel $ n "0 2 4 7" # scale "minor"   -- scale-degree indices -> semitone offsets
//! note "c e g"                         -- note names -> semitone offsets (c = 0)
//! ```
//!
//! Two mapping primitives:
//!
//! * [`degree_to_semitone`] maps a scale-degree *index* to a semitone offset,
//!   wrapping indices beyond the scale length into higher/lower octaves. This is
//!   what `# scale "minor"` does to the numbers coming out of `n "..."`.
//! * [`note_name_to_semitone`] maps a note *name* to a semitone offset. Bare
//!   names (no octave digit) return their pitch class `0..=11` (so `c e g`
//!   yields `[0, 4, 7]`), while octave-qualified names (`c4`) return the
//!   absolute MIDI number (so `n "c4 e4 g4"` yields `[60, 64, 67]`).
//!
//! Both are exposed as pattern combinators ([`quantize_degree_pattern`] and
//! [`note_names_to_semitone_pattern`]) so the compiler can wire them onto
//! `Pattern<String>` values and, per the architectural rule, `scale` itself
//! accepts a *pattern* of scale names.

use crate::midi_input::Scale;
use crate::pattern::{Hap, Pattern, State};
use crate::pattern_tonal::{note_to_midi, CHORD_INTERVALS};

/// Parse a scale name into a [`Scale`], reusing [`Scale::from_str`] and adding
/// a couple of DSL-friendly aliases that the underlying table lacks
/// (`"pentatonic"`/`"pent"` -> major pentatonic, `"minor_pentatonic"` variants).
///
/// Returns `None` for unknown names so callers can degrade gracefully instead
/// of panicking (validation requirement: unknown scale name must not panic).
pub fn scale_from_name(name: &str) -> Option<Scale> {
    let key = name.trim().to_lowercase();
    // Extra aliases beyond Scale::from_str, kept here so the shared midi_input
    // table stays focused on MIDI use.
    match key.as_str() {
        "major_pentatonic" | "majpent" => return Some(Scale::Pentatonic),
        "min_pentatonic" | "minor_pent" | "minorpentatonic" => {
            return Some(Scale::MinorPentatonic)
        }
        _ => {}
    }
    Scale::from_str(&key)
}

/// Map a scale-degree *index* to a semitone offset relative to the tonic.
///
/// Degrees beyond the length of the scale wrap into the next octave, and
/// negative degrees wrap downward, using Euclidean division so the mapping is
/// continuous in both directions:
///
/// ```text
/// minor = [0, 2, 3, 5, 7, 8, 10]  (7 degrees)
/// degree  0 ->  0
/// degree  2 ->  3
/// degree  4 ->  7
/// degree  7 -> 12   (octave up, index 0)
/// degree -1 -> -2   (octave down, index 6: -12 + 10)
/// ```
pub fn degree_to_semitone(degree: i32, scale: Scale) -> i32 {
    let intervals = scale.intervals();
    let len = intervals.len() as i32;
    if len == 0 {
        return degree; // defensive: empty scale acts as chromatic passthrough
    }
    let octave = degree.div_euclid(len);
    let idx = degree.rem_euclid(len) as usize;
    octave * 12 + intervals[idx] as i32
}

/// Parse a note *name* into a semitone offset.
///
/// * Pure integers pass through unchanged (`"5"` -> `5`) so numeric patterns
///   are transparent.
/// * Bare names with no octave digit return their pitch class `0..=11`
///   (`c` -> 0, `e` -> 4, `g` -> 7, `cs`/`c#` -> 1, `df`/`db` -> 1).
/// * Octave-qualified names return the absolute MIDI note number
///   (`c4` -> 60, `a4` -> 69), matching [`note_to_midi`].
///
/// Returns `None` for anything unparseable (callers pass the original value
/// through unchanged rather than panicking).
pub fn note_name_to_semitone(name: &str) -> Option<i32> {
    let s = name.trim();
    if s.is_empty() {
        return None;
    }

    // Numeric passthrough (degrees / explicit semitones already numeric).
    if let Ok(n) = s.parse::<i32>() {
        return Some(n);
    }

    let lower = s.to_lowercase();

    // Octave-qualified (contains a digit) -> absolute MIDI via the shared table.
    if lower.chars().any(|c| c.is_ascii_digit()) {
        return note_to_midi(&lower).map(|m| m as i32);
    }

    // Bare name -> pitch class 0..=11.
    let chars: Vec<char> = lower.chars().collect();
    let base: i32 = match chars[0] {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };
    let mut semis = base;
    for &c in &chars[1..] {
        match c {
            's' | '#' => semis += 1,
            'f' | 'b' => semis -= 1,
            _ => return None,
        }
    }
    Some(semis.rem_euclid(12))
}

/// Quantize a `Pattern<String>` of scale-degree numbers into a
/// `Pattern<String>` of semitone offsets, using a *pattern* of scale names
/// (so `scale "minor"` and `scale "minor major"` both work — architectural
/// rule: every parameter is a pattern).
///
/// Each degree event is quantized by whichever scale is active at that event's
/// start time. Non-numeric values (e.g. the rest `~`) pass through untouched.
/// Unknown scale names fall back to chromatic (identity), never panicking.
pub fn quantize_degree_pattern(
    degrees: Pattern<String>,
    scale_names: Pattern<String>,
) -> Pattern<String> {
    Pattern::new(move |state: &State| {
        let scale_haps = scale_names.query(state);
        degrees
            .query(state)
            .into_iter()
            .map(|hap| {
                let begin = hap.part.begin.to_float();
                // Scale active at this event's start; fall back to the first
                // scale in the pattern, then to chromatic (identity).
                let scale = scale_haps
                    .iter()
                    .find(|s| {
                        let sb = s.part.begin.to_float();
                        let se = s.part.end.to_float();
                        begin >= sb && begin < se
                    })
                    .or_else(|| scale_haps.first())
                    .and_then(|s| scale_from_name(&s.value))
                    .unwrap_or(Scale::Chromatic);

                let out = match hap.value.trim().parse::<f64>() {
                    Ok(deg) => degree_to_semitone(deg.round() as i32, scale).to_string(),
                    Err(_) => hap.value.clone(),
                };
                Hap::new(hap.whole, hap.part, out)
            })
            .collect()
    })
}

/// Map a `Pattern<String>` of note names into a `Pattern<String>` of semitone
/// offsets via [`note_name_to_semitone`]. Unparseable values pass through
/// unchanged (graceful degradation).
pub fn note_names_to_semitone_pattern(names: Pattern<String>) -> Pattern<String> {
    Pattern::new(move |state: &State| {
        names
            .query(state)
            .into_iter()
            .map(|hap| {
                let out = match note_name_to_semitone(&hap.value) {
                    Some(s) => s.to_string(),
                    None => hap.value.clone(),
                };
                Hap::new(hap.whole, hap.part, out)
            })
            .collect()
    })
}

/// Look up chord intervals for a quality name, e.g. `"maj" -> [0, 4, 7]`.
///
/// Tries an exact-case lookup first (so both the uppercase `M7` = major-7th and
/// the lowercase `m7` = minor-7th resolve to their distinct entries) and falls
/// back to a lowercased lookup (so `MAJ`/`Min7` still work). Returns `None` for
/// unknown qualities so callers can degrade gracefully (root-only), never
/// panicking. Reuses the shared [`CHORD_INTERVALS`] table so the DSL and the
/// voice-manager chord expansion stay in lock-step.
pub fn chord_quality_intervals(quality: &str) -> Option<Vec<i32>> {
    let q = quality.trim();
    if q.is_empty() {
        return None;
    }
    if let Some(iv) = CHORD_INTERVALS.get(q) {
        return Some(iv.clone());
    }
    let lower = q.to_lowercase();
    CHORD_INTERVALS.get(lower.as_str()).cloned()
}

/// Expand a single chord token `root'quality` into a stack of semitone offsets.
///
/// The root contributes its semitone value via [`note_name_to_semitone`] — a
/// *bare* root name yields its pitch class (`c` -> 0), so `c'maj` -> `[0, 4, 7]`
/// and `e'min7` -> `[4, 7, 11, 14]` (relative to the tonic), while an
/// *octave-qualified* root yields the absolute MIDI number (`c4` -> 60), so
/// `c4'maj` -> `[60, 64, 67]`. The quality contributes its intervals from
/// [`chord_quality_intervals`].
///
/// Tokens with no `'` fall back to the single semitone value of the token (note
/// name or number). An unknown quality degrades to the root alone. Returns
/// `None` only when the token is entirely unparseable (e.g. the rest `~`), so
/// callers can pass such tokens through unchanged.
pub fn chord_token_to_semitones(token: &str) -> Option<Vec<i32>> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    match t.find('\'') {
        Some(pos) => {
            let root = &t[..pos];
            let quality = &t[pos + 1..];
            let root_semi = note_name_to_semitone(root)?;
            match chord_quality_intervals(quality) {
                Some(intervals) => Some(intervals.iter().map(|i| root_semi + i).collect()),
                // Unknown quality: sound the root alone rather than dropping it.
                None => Some(vec![root_semi]),
            }
        }
        None => note_name_to_semitone(t).map(|s| vec![s]),
    }
}

/// Expand a `Pattern<String>` of chord/note tokens into a `Pattern<String>`
/// where each chord token becomes a **stack** of simultaneous semitone events
/// (all sharing the source event's time span).
///
/// This is what standalone `n "c'maj e'min7"` compiles to: `c'maj` becomes the
/// stack `[0, 4, 7]` and `e'min7` becomes `[4, 7, 11, 14]`, both relative to the
/// tonic. Plain numbers/note names pass through as single events; unparseable
/// tokens (rests) are preserved untouched (graceful degradation).
pub fn chord_expand_pattern(names: Pattern<String>) -> Pattern<String> {
    Pattern::new(move |state: &State| {
        names
            .query(state)
            .into_iter()
            .flat_map(|hap| match chord_token_to_semitones(&hap.value) {
                Some(semis) => semis
                    .into_iter()
                    .map(|s| Hap::new(hap.whole, hap.part, s.to_string()))
                    .collect::<Vec<_>>(),
                None => vec![hap.clone()],
            })
            .collect()
    })
}

/// Combine a `Pattern<String>` of roots (numeric degrees or note names) with a
/// `Pattern<String>` of chord qualities, producing a `Pattern<String>` where
/// each root event becomes a **stack** of semitone offsets for whichever quality
/// is active at that event's start time.
///
/// This backs the `chord` modifier: `n "c e" # chord "maj min7"` stacks a major
/// triad on `c` (-> `[0, 4, 7]`) and a minor-7th on `e` (-> `[4, 7, 11, 14]`).
/// Because every parameter is a pattern (architectural rule), the quality is a
/// pattern too, so `chord "maj min7"` alternates qualities across the cycle.
/// Roots that are octave-qualified note names yield absolute-MIDI stacks (`c4`
/// -> `[60, 64, 67]`). Unknown qualities degrade to the root alone; unparseable
/// roots (rests) pass through untouched.
pub fn chord_from_root_quality_pattern(
    roots: Pattern<String>,
    qualities: Pattern<String>,
) -> Pattern<String> {
    Pattern::new(move |state: &State| {
        let quality_haps = qualities.query(state);
        roots
            .query(state)
            .into_iter()
            .flat_map(|hap| {
                let begin = hap.part.begin.to_float();
                // Quality active at this root's start; fall back to the first
                // quality in the pattern, then to a plain major triad.
                let quality = quality_haps
                    .iter()
                    .find(|q| {
                        let qb = q.part.begin.to_float();
                        let qe = q.part.end.to_float();
                        begin >= qb && begin < qe
                    })
                    .or_else(|| quality_haps.first())
                    .map(|q| q.value.clone())
                    .unwrap_or_else(|| "maj".to_string());

                let root_semi = match note_name_to_semitone(&hap.value) {
                    Some(s) => s,
                    None => return vec![hap.clone()], // preserve rests / unparseable
                };

                match chord_quality_intervals(&quality) {
                    Some(intervals) => intervals
                        .iter()
                        .map(|i| Hap::new(hap.whole, hap.part, (root_semi + i).to_string()))
                        .collect(),
                    None => vec![Hap::new(hap.whole, hap.part, root_semi.to_string())],
                }
            })
            .collect()
    })
}

/// Expand a `Pattern<String>` of chord *qualities* into a `Pattern<String>` of
/// semitone stacks rooted on the tonic (root 0). Each quality event becomes a
/// stack of its intervals, so `chord "maj min7"` yields `[0, 4, 7]` in the first
/// half of the cycle and `[0, 3, 7, 10]` in the second.
///
/// This backs the standalone `chord "..."` form (no explicit root). Unknown
/// qualities degrade to a single root-0 event; the rest `~` passes through.
pub fn chord_quality_stack_pattern(qualities: Pattern<String>) -> Pattern<String> {
    Pattern::new(move |state: &State| {
        qualities
            .query(state)
            .into_iter()
            .flat_map(|hap| {
                let q = hap.value.trim();
                if q == "~" || q.is_empty() {
                    return vec![hap.clone()];
                }
                match chord_quality_intervals(q) {
                    Some(intervals) => intervals
                        .iter()
                        .map(|i| Hap::new(hap.whole, hap.part, i.to_string()))
                        .collect(),
                    None => vec![Hap::new(hap.whole, hap.part, "0".to_string())],
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_notation_v3::parse_mini_notation;
    use crate::pattern::{Fraction, State, TimeSpan};
    use std::collections::HashMap;

    /// Query a `Pattern<String>` over one cycle and return its numeric values in
    /// time order (Level-1 pattern-query verification).
    fn query_values(pattern: &Pattern<String>) -> Vec<f64> {
        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };
        let mut haps = pattern.query(&state);
        haps.sort_by(|a, b| {
            a.part
                .begin
                .to_float()
                .partial_cmp(&b.part.begin.to_float())
                .unwrap()
        });
        haps.iter()
            .map(|h| h.value.trim().parse::<f64>().unwrap())
            .collect()
    }

    #[test]
    fn test_degree_to_semitone_minor() {
        // Validation (Level 1): n "0 2 4" # scale "minor" -> [0, 3, 7]
        assert_eq!(degree_to_semitone(0, Scale::Minor), 0);
        assert_eq!(degree_to_semitone(2, Scale::Minor), 3);
        assert_eq!(degree_to_semitone(4, Scale::Minor), 7);
    }

    #[test]
    fn test_degree_to_semitone_octave_wrap() {
        // Minor has 7 degrees; degree 7 wraps to the octave (index 0 + 12).
        assert_eq!(degree_to_semitone(7, Scale::Minor), 12);
        assert_eq!(degree_to_semitone(9, Scale::Minor), 12 + 3);
        // Negative degrees wrap downward (Euclidean).
        assert_eq!(degree_to_semitone(-1, Scale::Minor), -2); // -12 + 10
    }

    #[test]
    fn test_degree_to_semitone_major_dorian_mixolydian_pentatonic() {
        // Validation: at least major, minor, dorian, mixolydian, pentatonic.
        assert_eq!(degree_to_semitone(4, Scale::Major), 7); // [0,2,4,5,7,..][4]
        assert_eq!(degree_to_semitone(2, Scale::Dorian), 3); // [0,2,3,..][2]
        assert_eq!(degree_to_semitone(3, Scale::Mixolydian), 5); // [0,2,4,5,..][3]
        // Pentatonic has 5 degrees: [0,2,4,7,9]
        assert_eq!(degree_to_semitone(3, Scale::Pentatonic), 7);
        assert_eq!(degree_to_semitone(5, Scale::Pentatonic), 12); // wrap
    }

    #[test]
    fn test_scale_from_name() {
        assert_eq!(scale_from_name("minor"), Some(Scale::Minor));
        assert_eq!(scale_from_name("MAJOR"), Some(Scale::Major));
        assert_eq!(scale_from_name("dorian"), Some(Scale::Dorian));
        assert_eq!(scale_from_name("mixolydian"), Some(Scale::Mixolydian));
        assert_eq!(scale_from_name("pentatonic"), Some(Scale::Pentatonic));
        assert_eq!(scale_from_name("pent"), Some(Scale::Pentatonic));
        // Unknown name degrades gracefully (no panic).
        assert_eq!(scale_from_name("not_a_scale"), None);
    }

    #[test]
    fn test_note_name_to_semitone_bare() {
        // Validation (Level 1): note "c e g" -> [0, 4, 7]
        assert_eq!(note_name_to_semitone("c"), Some(0));
        assert_eq!(note_name_to_semitone("e"), Some(4));
        assert_eq!(note_name_to_semitone("g"), Some(7));
        // Accidentals.
        assert_eq!(note_name_to_semitone("cs"), Some(1));
        assert_eq!(note_name_to_semitone("c#"), Some(1));
        assert_eq!(note_name_to_semitone("df"), Some(1));
        assert_eq!(note_name_to_semitone("bb"), Some(10));
    }

    #[test]
    fn test_note_name_to_semitone_octave_qualified() {
        // Octave-qualified -> absolute MIDI (n "c4 e4 g4" -> [60, 64, 67]).
        assert_eq!(note_name_to_semitone("c4"), Some(60));
        assert_eq!(note_name_to_semitone("e4"), Some(64));
        assert_eq!(note_name_to_semitone("g4"), Some(67));
        assert_eq!(note_name_to_semitone("a4"), Some(69));
    }

    #[test]
    fn test_note_name_numeric_passthrough_and_unknown() {
        assert_eq!(note_name_to_semitone("5"), Some(5));
        assert_eq!(note_name_to_semitone("-3"), Some(-3));
        // Unknown / unparseable -> None (graceful).
        assert_eq!(note_name_to_semitone("zonk"), None);
        assert_eq!(note_name_to_semitone(""), None);
    }

    #[test]
    fn test_quantize_degree_pattern_minor() {
        // Validation (Level 1): n "0 2 4" # scale "minor" yields [0, 3, 7].
        let degrees = parse_mini_notation("0 2 4");
        let scale = parse_mini_notation("minor");
        let quantized = quantize_degree_pattern(degrees, scale);
        assert_eq!(query_values(&quantized), vec![0.0, 3.0, 7.0]);
    }

    #[test]
    fn test_quantize_degree_pattern_major_and_pentatonic() {
        let degrees = parse_mini_notation("0 1 2 3 4");
        let major = quantize_degree_pattern(degrees.clone(), parse_mini_notation("major"));
        assert_eq!(query_values(&major), vec![0.0, 2.0, 4.0, 5.0, 7.0]);

        let pent = quantize_degree_pattern(degrees, parse_mini_notation("pentatonic"));
        assert_eq!(query_values(&pent), vec![0.0, 2.0, 4.0, 7.0, 9.0]);
    }

    #[test]
    fn test_quantize_degree_pattern_unknown_scale_graceful() {
        // Unknown scale -> chromatic identity (degree == semitone), no panic.
        let degrees = parse_mini_notation("0 3 5");
        let quantized = quantize_degree_pattern(degrees, parse_mini_notation("bogus"));
        assert_eq!(query_values(&quantized), vec![0.0, 3.0, 5.0]);
    }

    #[test]
    fn test_note_names_to_semitone_pattern() {
        // Validation (Level 1): note "c e g" -> [0, 4, 7].
        let names = parse_mini_notation("c e g");
        let semis = note_names_to_semitone_pattern(names);
        assert_eq!(query_values(&semis), vec![0.0, 4.0, 7.0]);
    }

    // ------------------------------------------------------------------
    // Chord expansion (feat-chord-support)
    // ------------------------------------------------------------------

    /// Query a `Pattern<String>` over one cycle and return numeric values sorted
    /// by (start-time, value). Chords are *stacks* (several haps share a start
    /// time), so sorting by value gives a deterministic order for assertions.
    fn query_stack_values(pattern: &Pattern<String>) -> Vec<f64> {
        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };
        let mut haps = pattern.query(&state);
        haps.sort_by(|a, b| {
            let ka = (a.part.begin.to_float(), a.value.trim().parse::<f64>().unwrap_or(f64::MAX));
            let kb = (b.part.begin.to_float(), b.value.trim().parse::<f64>().unwrap_or(f64::MAX));
            ka.partial_cmp(&kb).unwrap()
        });
        haps.iter()
            .map(|h| h.value.trim().parse::<f64>().unwrap())
            .collect()
    }

    #[test]
    fn test_chord_quality_intervals_required_set() {
        // Validation (Level 1): at least maj, min, dom7, maj7, min7, dim, aug,
        // sus2, sus4 must all resolve.
        assert_eq!(chord_quality_intervals("maj"), Some(vec![0, 4, 7]));
        assert_eq!(chord_quality_intervals("min"), Some(vec![0, 3, 7]));
        assert_eq!(chord_quality_intervals("dom7"), Some(vec![0, 4, 7, 10]));
        assert_eq!(chord_quality_intervals("maj7"), Some(vec![0, 4, 7, 11]));
        assert_eq!(chord_quality_intervals("min7"), Some(vec![0, 3, 7, 10]));
        assert_eq!(chord_quality_intervals("dim"), Some(vec![0, 3, 6]));
        assert_eq!(chord_quality_intervals("aug"), Some(vec![0, 4, 8]));
        assert_eq!(chord_quality_intervals("sus2"), Some(vec![0, 2, 7]));
        assert_eq!(chord_quality_intervals("sus4"), Some(vec![0, 5, 7]));
        // Case-insensitive fallback, and unknown -> None (graceful).
        assert_eq!(chord_quality_intervals("MAJ"), Some(vec![0, 4, 7]));
        assert_eq!(chord_quality_intervals("bogus"), None);
    }

    #[test]
    fn test_chord_token_to_semitones_relative() {
        // Validation (Level 1): n "c'maj" -> [0, 4, 7]; e'min7 -> [4, 7, 11, 14].
        assert_eq!(chord_token_to_semitones("c'maj"), Some(vec![0, 4, 7]));
        assert_eq!(chord_token_to_semitones("e'min7"), Some(vec![4, 7, 11, 14]));
        // Other required qualities relative to their roots.
        assert_eq!(chord_token_to_semitones("c'dom7"), Some(vec![0, 4, 7, 10]));
        assert_eq!(chord_token_to_semitones("c'maj7"), Some(vec![0, 4, 7, 11]));
        assert_eq!(chord_token_to_semitones("c'dim"), Some(vec![0, 3, 6]));
        assert_eq!(chord_token_to_semitones("c'aug"), Some(vec![0, 4, 8]));
        assert_eq!(chord_token_to_semitones("c'sus2"), Some(vec![0, 2, 7]));
        assert_eq!(chord_token_to_semitones("c'sus4"), Some(vec![0, 5, 7]));
        assert_eq!(chord_token_to_semitones("g'min"), Some(vec![7, 10, 14]));
    }

    #[test]
    fn test_chord_token_to_semitones_absolute_and_plain() {
        // Octave-qualified root -> absolute MIDI stack (c4 = 60).
        assert_eq!(chord_token_to_semitones("c4'maj"), Some(vec![60, 64, 67]));
        // Plain note / number tokens -> single-element stacks.
        assert_eq!(chord_token_to_semitones("c"), Some(vec![0]));
        assert_eq!(chord_token_to_semitones("5"), Some(vec![5]));
        // Unknown quality -> root alone (graceful), not a panic.
        assert_eq!(chord_token_to_semitones("c'zonk"), Some(vec![0]));
        // Fully unparseable -> None (rest passes through).
        assert_eq!(chord_token_to_semitones("~"), None);
    }

    #[test]
    fn test_chord_expand_pattern_stack() {
        // Validation (Level 1): n "c'maj" expands to a stack [0, 4, 7].
        let maj = chord_expand_pattern(parse_mini_notation("c'maj"));
        assert_eq!(query_stack_values(&maj), vec![0.0, 4.0, 7.0]);

        // A two-chord progression: c'maj then e'min7 in successive slots.
        let prog = chord_expand_pattern(parse_mini_notation("c'maj e'min7"));
        assert_eq!(
            query_stack_values(&prog),
            vec![0.0, 4.0, 7.0, 4.0, 7.0, 11.0, 14.0]
        );
    }

    #[test]
    fn test_chord_expand_pattern_all_simultaneous() {
        // The three notes of a chord must share ONE time span (a real stack, not
        // an arpeggio) so the render triggers simultaneous voices.
        let maj = chord_expand_pattern(parse_mini_notation("c'maj"));
        let state = State {
            span: TimeSpan::new(Fraction::from_float(0.0), Fraction::from_float(1.0)),
            controls: HashMap::new(),
        };
        let haps = maj.query(&state);
        assert_eq!(haps.len(), 3, "c'maj must produce 3 simultaneous events");
        let first_begin = haps[0].part.begin.to_float();
        let first_end = haps[0].part.end.to_float();
        for h in &haps {
            assert_eq!(h.part.begin.to_float(), first_begin);
            assert_eq!(h.part.end.to_float(), first_end);
        }
    }

    #[test]
    fn test_chord_from_root_quality_pattern() {
        // Validation (Level 1): n "c e" # chord "maj min7" stacks per-root.
        let roots = parse_mini_notation("c e");
        let qualities = parse_mini_notation("maj min7");
        let stacked = chord_from_root_quality_pattern(roots, qualities);
        assert_eq!(
            query_stack_values(&stacked),
            vec![0.0, 4.0, 7.0, 4.0, 7.0, 11.0, 14.0]
        );

        // Numeric roots (scale degrees / semitones) also work.
        let roots = parse_mini_notation("0 7");
        let qualities = parse_mini_notation("maj");
        let stacked = chord_from_root_quality_pattern(roots, qualities);
        assert_eq!(
            query_stack_values(&stacked),
            vec![0.0, 4.0, 7.0, 7.0, 11.0, 14.0]
        );
    }

    #[test]
    fn test_chord_quality_stack_pattern() {
        // Standalone chord "maj min7" -> root-0 stacks per quality slot.
        let stacked = chord_quality_stack_pattern(parse_mini_notation("maj min7"));
        assert_eq!(
            query_stack_values(&stacked),
            vec![0.0, 4.0, 7.0, 0.0, 3.0, 7.0, 10.0]
        );
        // Single quality -> single stack.
        let one = chord_quality_stack_pattern(parse_mini_notation("aug"));
        assert_eq!(query_stack_values(&one), vec![0.0, 4.0, 8.0]);
    }
}
