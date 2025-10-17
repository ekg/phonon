//! Test if "1" and "0" are being parsed as note names

use phonon::pattern_tonal::{midi_to_freq, note_to_midi};

#[test]
#[ignore = "Note parsing incorrectly interprets '0' and '1' as notes - needs investigation"]
fn test_numeric_strings_as_notes() {
    let result_1 = note_to_midi("1");
    let result_0 = note_to_midi("0");

    println!("note_to_midi('1') = {:?}", result_1);
    println!("note_to_midi('0') = {:?}", result_0);

    if let Some(midi) = result_1 {
        let freq = midi_to_freq(midi);
        println!("If '1' is a note, frequency would be: {}", freq);
    }

    if let Some(midi) = result_0 {
        let freq = midi_to_freq(midi);
        println!("If '0' is a note, frequency would be: {}", freq);
    }

    assert!(
        result_1.is_none(),
        "'1' should NOT be interpreted as a note name"
    );
    assert!(
        result_0.is_none(),
        "'0' should NOT be interpreted as a note name"
    );
}
