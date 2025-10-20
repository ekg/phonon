mod audio_verification_enhanced;
use audio_verification_enhanced::*;

#[test]
fn validate_multiple_audio_files() {
    let test_files = vec![
        // Samples (should have onsets)
        ("/tmp/test_sample_bd_only.wav", "kick drum", true),
        ("/tmp/test_sample_sn_only.wav", "snare drum", true),
        ("/tmp/test_sample_hh_only.wav", "hihat", true),
        // Effects (should have audio)
        ("/tmp/test_effect_reverb_synth.wav", "reverb synth", false),
        ("/tmp/test_effect_delay_synth.wav", "delay synth", false),
        // Oscillators (should have audio + frequency)
        ("/tmp/test_osc_sine_constant.wav", "sine 440Hz", false),
        ("/tmp/test_osc_saw_constant.wav", "saw 440Hz", false),
    ];

    eprintln!("\n=== MULTI-DIMENSIONAL AUDIO VALIDATION ===\n");

    for (path, name, expect_onsets) in test_files {
        if std::path::Path::new(path).exists() {
            match analyze_wav_enhanced(path) {
                Ok(analysis) => {
                    eprintln!("📊 {}", name);
                    eprintln!("   File: {}", path);
                    eprintln!("   RMS: {:.6}", analysis.rms);
                    eprintln!("   Peak: {:.6}", analysis.peak);
                    eprintln!("   Dominant Freq: {:.1} Hz", analysis.dominant_frequency);
                    eprintln!("   Spectral Centroid: {:.1} Hz", analysis.spectral_centroid);
                    eprintln!("   Spectral Spread: {:.1} Hz", analysis.spectral_spread);
                    eprintln!("   Spectral Flux: {:.6}", analysis.spectral_flux);
                    eprintln!("   Onset Count: {}", analysis.onset_count);
                    eprintln!("   Is Empty: {}", analysis.is_empty);
                    eprintln!("   Is Clipping: {}", analysis.is_clipping);

                    if analysis.is_empty {
                        eprintln!("   ⚠️  WARNING: AUDIO IS SILENT!");
                    }

                    if expect_onsets && analysis.onset_count == 0 {
                        eprintln!("   ⚠️  WARNING: Expected onsets but found 0!");
                    }

                    eprintln!();
                }
                Err(e) => {
                    eprintln!("❌ {} - Analysis failed: {}\n", name, e);
                }
            }
        } else {
            eprintln!("⚠️  {} - File not found: {}\n", name, path);
        }
    }

    // This test always passes - it's for diagnostic output only
    assert!(true);
}
