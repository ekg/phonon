mod audio_verification_enhanced;
use audio_verification_enhanced::*;

#[test]
fn debug_tempo_amplitude_bug() {
    eprintln!("\n=== DEBUGGING TEMPO AMPLITUDE BUG ===\n");

    let files = vec![
        ("/tmp/debug_tempo_slow.wav", "Tempo 0.5 (1 cycle = 2s)"),
        ("/tmp/debug_tempo_fast.wav", "Tempo 2.0 (1 cycle = 0.5s)"),
    ];

    for (path, label) in files {
        if let Ok(analysis) = analyze_wav_enhanced(path) {
            eprintln!("📊 {}", label);
            eprintln!("   RMS: {:.6}", analysis.rms);
            eprintln!("   Peak: {:.6}", analysis.peak);
            eprintln!("   Onset Count: {}", analysis.onset_count);
            eprintln!("   Is Empty: {}", analysis.is_empty);
            eprintln!();
        }
    }

    assert!(true);
}
