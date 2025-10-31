mod audio_verification_enhanced;
use audio_verification_enhanced::*;

#[test]
#[ignore] // Debug test - requires specific file at /tmp/test_sample_bd_only.wav
fn debug_kick_onset() {
    let wav_path = "/tmp/test_sample_bd_only.wav";
    let analysis = analyze_wav_enhanced(wav_path).expect("Failed to analyze");

    eprintln!("=== Debug Kick Drum Analysis ===");
    eprintln!("RMS: {:.6}", analysis.rms);
    eprintln!("Peak: {:.6}", analysis.peak);
    eprintln!("Is empty: {}", analysis.is_empty);
    eprintln!("Onset count: {}", analysis.onset_count);
    eprintln!("Dominant frequency: {:.1} Hz", analysis.dominant_frequency);

    assert!(analysis.rms > 0.001, "RMS too low: {}", analysis.rms);
}
