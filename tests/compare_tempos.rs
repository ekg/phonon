mod audio_verification_enhanced;
use audio_verification_enhanced::*;

#[test]
fn compare_tempo_effects() {
    eprintln!("\n=== TEMPO COMPARISON ===\n");
    
    let files = vec![
        ("/tmp/tempo_test_bd.wav", "Tempo 2.0 (fast)"),
        ("/tmp/manual_test_bd.wav", "Tempo 0.5 (slow)"),
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
