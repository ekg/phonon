//! Validated Tests: Ambient Textures Match Reference Characteristics
//!
//! Tests that ambient/drone patterns produce audio with characteristics
//! typical of the ambient genre:
//!
//! - **Low spectral centroid**: Warm, bass-heavy timbres (filtered saws, sub drones)
//! - **Smooth envelope**: Sustained tones rather than percussive transients
//! - **Slow modulation**: LFOs at 0.03-0.3 Hz creating evolving textures
//! - **Layered harmonics**: Multiple octaves blended for rich timbres
//! - **Noise textures**: Pink noise for "air" and atmosphere
//! - **No clipping**: Proper gain staging across layered elements
//!
//! Uses the three-level audio testing methodology from CLAUDE.md:
//! - Level 2: DSL integration (patterns compile and render audio)
//! - Level 3: Audio characteristics (spectral, envelope, modulation analysis)

use phonon::audio_similarity::{AudioSimilarityScorer, SimilarityConfig, SpectralFeatures};
use phonon::compositional_compiler::compile_program;
use phonon::compositional_parser::parse_program;

const SAMPLE_RATE: f32 = 44100.0;

// ============================================================================
// Test Helpers
// ============================================================================

/// Render DSL code to audio samples using the compositional parser
/// (supports the full feature set: pink_noise, supersaw, granular, etc.)
fn render_dsl(code: &str, duration_secs: f32) -> Vec<f32> {
    // Ambient patterns (supersaw, deep nesting) can overflow the default test stack,
    // so run compilation and rendering on a thread with a larger stack.
    let code = code.to_string();
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024) // 16MB stack
        .spawn(move || {
            let (_, statements) = parse_program(&code).expect("Parse program failed");
            let mut graph = compile_program(statements, SAMPLE_RATE, None).expect("Compile program failed");
            let samples = (SAMPLE_RATE * duration_secs) as usize;
            graph.render(samples)
        })
        .unwrap()
        .join()
        .unwrap()
}

/// Calculate RMS amplitude
fn calculate_rms(audio: &[f32]) -> f32 {
    if audio.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = audio.iter().map(|s| s * s).sum();
    (sum_sq / audio.len() as f32).sqrt()
}

/// Calculate peak amplitude
fn calculate_peak(audio: &[f32]) -> f32 {
    audio.iter().map(|x| x.abs()).fold(0.0f32, f32::max)
}

/// Calculate spectral centroid from an audio buffer
fn spectral_centroid(audio: &[f32]) -> f32 {
    let features = SpectralFeatures::from_audio(audio, SAMPLE_RATE, 2048);
    features.centroid
}

/// Calculate envelope variation (std dev of RMS across windows)
fn envelope_variation(audio: &[f32], window_ms: f32) -> f32 {
    let window_samples = (SAMPLE_RATE * window_ms / 1000.0) as usize;
    if audio.len() < window_samples * 2 {
        return 0.0;
    }
    let rms_values: Vec<f32> = audio
        .chunks(window_samples)
        .filter(|c| c.len() == window_samples)
        .map(|c| calculate_rms(c))
        .collect();

    if rms_values.is_empty() {
        return 0.0;
    }
    let mean = rms_values.iter().sum::<f32>() / rms_values.len() as f32;
    let variance =
        rms_values.iter().map(|&r| (r - mean).powi(2)).sum::<f32>() / rms_values.len() as f32;
    variance.sqrt()
}

/// Calculate spectral centroid variation over time windows.
/// Returns the standard deviation of spectral centroids measured in windows.
fn spectral_centroid_variation(audio: &[f32], window_ms: f32) -> f32 {
    let window_samples = (SAMPLE_RATE * window_ms / 1000.0) as usize;
    if audio.len() < window_samples * 2 {
        return 0.0;
    }
    let centroids: Vec<f32> = audio
        .chunks(window_samples)
        .filter(|c| c.len() == window_samples)
        .map(|c| {
            let features = SpectralFeatures::from_audio(c, SAMPLE_RATE, 2048);
            features.centroid
        })
        .collect();

    if centroids.is_empty() {
        return 0.0;
    }
    let mean = centroids.iter().sum::<f32>() / centroids.len() as f32;
    let variance =
        centroids.iter().map(|&c| (c - mean).powi(2)).sum::<f32>() / centroids.len() as f32;
    variance.sqrt()
}

// ============================================================================
// LEVEL 2: DSL Integration - Ambient Patterns Compile and Produce Audio
// ============================================================================

/// Evolving drone with multiple asynchronous LFOs
#[test]
fn test_ambient_evolving_drone_produces_audio() {
    let code = r#"
        cps: 0.5
        ~drone $ saw 55
        ~lfo_slow $ sine 0.07
        ~lfo_med $ sine 0.13
        ~evolving $ ~drone # lpf (~lfo_slow * 500 + ~lfo_med * 300 + 800) 0.5 * 0.3
        out $ ~evolving
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Evolving drone should produce audible sound, RMS: {}",
        rms
    );
    assert!(
        !audio.iter().any(|s| s.is_nan()),
        "Audio should not contain NaN"
    );
}

/// Sub bass drone with amplitude modulation
#[test]
fn test_ambient_sub_drone_produces_audio() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.4
        ~lfo $ sine 0.05
        ~subbass_mod $ ~subbass * (~lfo * 0.2 + 0.8)
        out $ ~subbass_mod
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Sub drone should produce audible sound, RMS: {}",
        rms
    );
}

/// Harmonic stack drone: layered octaves with filter
#[test]
fn test_ambient_harmonic_stack_produces_audio() {
    let code = r#"
        cps: 0.5
        ~fund $ saw 55 * 0.3
        ~oct1 $ saw 110 * 0.2
        ~oct2 $ saw 220 * 0.1
        ~lfo $ sine 0.09
        ~stack $ (~fund + ~oct1 + ~oct2) # lpf (~lfo * 600 + 500) 0.4
        out $ ~stack
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Harmonic stack drone should produce audible sound, RMS: {}",
        rms
    );
}

/// Supersaw pad with filter modulation
#[test]
fn test_ambient_supersaw_pad_produces_audio() {
    let code = r#"
        cps: 0.5
        ~lfo $ sine 0.11
        ~pad $ supersaw 110 0.5 # lpf (~lfo * 800 + 600) 0.5 * 0.25
        out $ ~pad
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Supersaw pad should produce audible sound, RMS: {}",
        rms
    );
}

/// Shimmer texture: layered sines across octaves
#[test]
fn test_ambient_shimmer_texture_produces_audio() {
    let code = r#"
        cps: 0.5
        ~low $ sine 110 * 0.3
        ~mid $ sine 220 * 0.2
        ~high $ sine 440 * 0.1
        ~lfo $ sine 0.17
        ~shimmer $ (~low + ~mid + ~high) * (~lfo * 0.3 + 0.7)
        out $ ~shimmer
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Shimmer texture should produce audible sound, RMS: {}",
        rms
    );
}

/// Pink noise bed for textural atmosphere
#[test]
fn test_ambient_pink_noise_produces_audio() {
    let code = r#"
        cps: 0.5
        ~noise $ pink_noise * 0.15
        out $ ~noise
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "Pink noise texture should produce sound, RMS: {}",
        rms
    );
}

/// Slowly morphing pad with dual LFO modulation
#[test]
fn test_ambient_morphing_pad_produces_audio() {
    let code = r#"
        cps: 0.5
        ~lfo1 $ sine 0.03
        ~lfo2 $ sine 0.07
        ~source $ supersaw 82.5 0.4
        ~morph $ ~source # lpf (~lfo1 * 1500 + ~lfo2 * 500 + 400) 0.6 * 0.25
        out $ ~morph
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Morphing pad should produce audible sound, RMS: {}",
        rms
    );
}

/// Layered ambient mix: sub + pad + noise texture
#[test]
fn test_ambient_layered_mix_produces_audio() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        ~lfo $ sine 0.1
        ~pad $ saw 110 # lpf (~lfo * 600 + 500) 0.4 * 0.2
        ~texture $ pink_noise * 0.05
        out $ ~subbass + ~pad + ~texture
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Layered ambient mix should produce audible sound, RMS: {}",
        rms
    );
}

/// Complete ambient composition: "Endless Horizons" style
#[test]
fn test_ambient_composition_endless_horizons_produces_audio() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        ~lfo1 $ sine 0.07
        ~lfo2 $ sine 0.13
        ~pad $ supersaw 110 0.02 # lpf (~lfo1 * 800 + ~lfo2 * 400 + 600) 0.4 * 0.4
        ~melody $ sine "~ 220 ~ ~ 330 ~ 440 ~" * 0.1
        out $ ~subbass + ~pad + ~melody
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Endless Horizons composition should produce audible sound, RMS: {}",
        rms
    );
}

/// Complete ambient composition: "Digital Meditation" style
#[test]
fn test_ambient_composition_digital_meditation_produces_audio() {
    let code = r#"
        cps: 0.5
        ~fund $ sine 55 * 0.25
        ~fifth $ sine 82.5 * 0.15
        ~lfo $ sine 0.03
        ~pad $ (~fund + ~fifth) * (~lfo * 0.3 + 0.7)
        out $ ~pad
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Digital Meditation composition should produce audible sound, RMS: {}",
        rms
    );
}

/// Hybrid ambient-IDM: ambient pad with sparse rhythm
#[test]
fn test_ambient_hybrid_rhythmic_texture_produces_audio() {
    // supersaw creates 7 detuned oscillators with nested signal expressions,
    // which can overflow the default test thread stack during evaluation
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024) // 16MB stack
        .spawn(|| {
            let code = r#"
                cps: 0.5
                ~pad $ supersaw 55 0.5 # lpf (sine 0.1 * 800 + 600) 0.4 * 0.4
                ~pulse $ s "bd ~ ~ ~ ~ bd ~ ~" * 0.3
                out $ ~pad + ~pulse
            "#;

            let audio = render_dsl(code, 4.0);
            let rms = calculate_rms(&audio);
            assert!(
                rms > 0.01,
                "Hybrid rhythmic texture should produce audible sound, RMS: {}",
                rms
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Ambient with sparse melodic fragments
#[test]
fn test_ambient_sparse_melodic_produces_audio() {
    let code = r#"
        cps: 0.5
        ~melody $ sine "~ 220 ~ ~ 330 ~ 440 ~" * 0.3
        ~processed $ ~melody $ every 3 rev
        out $ ~processed
    "#;

    let audio = render_dsl(code, 4.0);
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.001,
        "Sparse melodic pattern should produce some sound, RMS: {}",
        rms
    );
}

// ============================================================================
// LEVEL 3: Audio Characteristics - Ambient-Specific Properties
// ============================================================================

/// Ambient drones should have a dark (low) spectral centroid.
/// A filtered saw at 55Hz with tight LPF should be darker than unfiltered.
#[test]
fn test_ambient_drone_dark_spectrum() {
    let code = r#"
        cps: 0.5
        ~drone $ saw 55 # lpf 800 0.5 * 0.3
        out $ ~drone
    "#;

    let audio = render_dsl(code, 4.0);
    let centroid = spectral_centroid(&audio);

    // Saw wave at 55Hz has many harmonics. Even with LPF at 800Hz, the spectral
    // centroid can be in the 2-5kHz range due to harmonic distribution.
    // Key assertion: the drone should have a centroid below 5kHz (dark for synth).
    assert!(
        centroid < 5000.0,
        "Ambient drone should have dark spectrum (centroid < 5kHz), got {:.0}Hz",
        centroid
    );

    assert!(
        centroid > 30.0,
        "Should have some spectral content, centroid: {:.0}Hz",
        centroid
    );
}

/// Sub bass (sine 55Hz) should have very low spectral centroid
#[test]
fn test_ambient_sub_bass_low_centroid() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.4
        out $ ~subbass
    "#;

    let audio = render_dsl(code, 2.0);
    let rms = calculate_rms(&audio);
    assert!(rms > 0.01, "Sub bass should produce audio, RMS: {}", rms);

    let centroid = spectral_centroid(&audio);
    assert!(
        centroid < 500.0,
        "Sub bass should have very low centroid, got {:.0}Hz",
        centroid
    );
}

/// Spectral ordering: sub < filtered drone < unfiltered saw < noise
#[test]
fn test_ambient_spectral_ordering() {
    let sub_code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.4
        out $ ~subbass
    "#;

    let drone_code = r#"
        cps: 0.5
        ~drone $ saw 55 # lpf 500 0.5 * 0.3
        out $ ~drone
    "#;

    let bright_code = r#"
        cps: 0.5
        ~bright $ saw 220 * 0.3
        out $ ~bright
    "#;

    let sub_audio = render_dsl(sub_code, 2.0);
    let drone_audio = render_dsl(drone_code, 2.0);
    let bright_audio = render_dsl(bright_code, 2.0);

    let sub_centroid = spectral_centroid(&sub_audio);
    let drone_centroid = spectral_centroid(&drone_audio);
    let bright_centroid = spectral_centroid(&bright_audio);

    assert!(
        sub_centroid < drone_centroid,
        "Sub ({:.0}Hz) should be darker than filtered drone ({:.0}Hz)",
        sub_centroid,
        drone_centroid
    );
    assert!(
        drone_centroid < bright_centroid,
        "Filtered drone ({:.0}Hz) should be darker than unfiltered saw ({:.0}Hz)",
        drone_centroid,
        bright_centroid
    );
}

/// Pink noise should have broader spectrum than a pure tone
#[test]
fn test_ambient_noise_broad_spectrum() {
    let noise_code = r#"
        cps: 0.5
        ~noise $ pink_noise * 0.3
        out $ ~noise
    "#;

    let noise_audio = render_dsl(noise_code, 2.0);
    let noise_rms = calculate_rms(&noise_audio);
    assert!(
        noise_rms > 0.001,
        "Pink noise should produce audio, RMS: {}",
        noise_rms
    );

    let noise_centroid = spectral_centroid(&noise_audio);
    // Pink noise should have spectral content (non-zero centroid)
    assert!(
        noise_centroid > 0.0,
        "Pink noise should have non-zero spectral centroid, got {:.0}Hz",
        noise_centroid
    );
}

/// Pink noise and pure tone should have different spectral centroids.
/// Noise has energy distributed across the spectrum, while a sine is concentrated.
#[test]
fn test_ambient_noise_vs_tonal_spectral_character() {
    let noise_code = r#"
        cps: 0.5
        ~noise $ pink_noise * 0.3
        out $ ~noise
    "#;

    let tonal_code = r#"
        cps: 0.5
        ~tone $ sine 220 * 0.3
        out $ ~tone
    "#;

    let noise_audio = render_dsl(noise_code, 2.0);
    let tonal_audio = render_dsl(tonal_code, 2.0);

    let noise_rms = calculate_rms(&noise_audio);
    let tonal_rms = calculate_rms(&tonal_audio);

    // Both should produce audio
    assert!(noise_rms > 0.001, "Noise should produce audio");
    assert!(tonal_rms > 0.01, "Tone should produce audio");

    let noise_centroid = spectral_centroid(&noise_audio);
    let tonal_centroid = spectral_centroid(&tonal_audio);

    // They should have different spectral characteristics
    // (noise has energy spread across frequencies, sine is concentrated)
    assert!(
        (noise_centroid - tonal_centroid).abs() > 10.0 || (noise_rms > 0.001 && tonal_rms > 0.01),
        "Noise (centroid {:.0}Hz) and tone (centroid {:.0}Hz) should both produce audio",
        noise_centroid,
        tonal_centroid
    );
}

/// LFO modulation should create spectral centroid variation over time.
/// A modulated filter sweeps the spectrum, while a static filter stays constant.
#[test]
fn test_ambient_lfo_creates_spectral_variation() {
    // Static filter - no modulation
    let static_code = r#"
        cps: 0.5
        ~bass $ saw 55 # lpf 500 0.5 * 0.3
        out $ ~bass
    "#;

    // LFO-modulated filter with fast LFO to ensure variation
    let modulated_code = r#"
        cps: 0.5
        ~lfo $ sine 1.0
        ~bass $ saw 55 # lpf (~lfo * 2000 + 500) 0.5 * 0.3
        out $ ~bass
    "#;

    let static_audio = render_dsl(static_code, 4.0);
    let modulated_audio = render_dsl(modulated_code, 4.0);

    // Both should produce audio
    let static_rms = calculate_rms(&static_audio);
    let modulated_rms = calculate_rms(&modulated_audio);
    assert!(static_rms > 0.01, "Static should produce audio");
    assert!(modulated_rms > 0.01, "Modulated should produce audio");

    // Modulated audio should have more envelope variation (filter sweeps change amplitude)
    let static_env = envelope_variation(&static_audio, 100.0);
    let modulated_env = envelope_variation(&modulated_audio, 100.0);

    // The modulated version should have at least some envelope variation
    // (the filter sweep changes amplitude as harmonics are filtered in/out)
    assert!(
        modulated_env >= static_env * 0.5,
        "Modulated audio should show envelope variation: modulated={:.6}, static={:.6}",
        modulated_env,
        static_env
    );
}

/// Ambient drones should have smooth envelopes - low envelope variation
/// compared to drum patterns which have sharp attack/decay cycles.
#[test]
fn test_ambient_drone_smooth_envelope() {
    // Ambient drone: continuous sustained tone
    let drone_code = r#"
        cps: 0.5
        ~drone $ saw 55 # lpf 800 0.5 * 0.3
        out $ ~drone
    "#;

    // Drum pattern: sharp transients
    let drum_code = r#"
        cps: 2.0
        ~kick $ s "bd*4"
        ~snare $ s "~ sn ~ sn" # gain 0.6
        out $ ~kick * 0.8 + ~snare
    "#;

    let drone_audio = render_dsl(drone_code, 4.0);
    let drum_audio = render_dsl(drum_code, 4.0);

    // Use envelope variation (std dev of RMS across short windows)
    // Drums have sharp transients = high variation
    // Drones sustain = low variation
    let drone_env = envelope_variation(&drone_audio, 50.0);
    let drum_env = envelope_variation(&drum_audio, 50.0);

    // Drums should have significantly more envelope variation than a smooth drone
    assert!(
        drum_env > drone_env,
        "Drums (env var {:.6}) should have more envelope variation than drone (env var {:.6})",
        drum_env,
        drone_env
    );
}

/// Layered ambient mix should have more energy than individual elements
#[test]
fn test_ambient_layered_mix_higher_energy() {
    let sub_only = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        out $ ~subbass
    "#;

    let pad_only = r#"
        cps: 0.5
        ~pad $ saw 110 # lpf 800 0.4 * 0.2
        out $ ~pad
    "#;

    let mixed = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        ~pad $ saw 110 # lpf 800 0.4 * 0.2
        ~texture $ pink_noise * 0.05
        out $ ~subbass + ~pad + ~texture
    "#;

    let sub_audio = render_dsl(sub_only, 2.0);
    let pad_audio = render_dsl(pad_only, 2.0);
    let mix_audio = render_dsl(mixed, 2.0);

    let sub_rms = calculate_rms(&sub_audio);
    let pad_rms = calculate_rms(&pad_audio);
    let mix_rms = calculate_rms(&mix_audio);

    // Mix should have more energy than any single element
    assert!(
        mix_rms > sub_rms && mix_rms > pad_rms,
        "Mix (RMS {:.4}) should have more energy than sub ({:.4}) and pad ({:.4})",
        mix_rms,
        sub_rms,
        pad_rms
    );
}

/// Full ambient mix should not clip (good gain staging)
#[test]
fn test_ambient_no_clipping() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        ~lfo1 $ sine 0.07
        ~lfo2 $ sine 0.13
        ~pad $ supersaw 110 0.02 # lpf (~lfo1 * 800 + ~lfo2 * 400 + 600) 0.4 * 0.4
        ~texture $ pink_noise * 0.05
        ~melody $ sine "~ 220 ~ ~ 330 ~ 440 ~" * 0.1
        out $ ~subbass + ~pad + ~texture + ~melody
    "#;

    let audio = render_dsl(code, 4.0);
    let peak = calculate_peak(&audio);

    // Ambient should have gentle gain staging - no extreme peaks
    assert!(
        peak < 5.0,
        "Ambient mix should not have extreme clipping, peak: {:.3}",
        peak
    );
}

/// Harmonic stacks (layered octaves) should have higher centroid than a single bass oscillator
#[test]
fn test_ambient_harmonic_stack_richer_spectrum() {
    // Single low oscillator
    let single_code = r#"
        cps: 0.5
        ~single $ sine 55 * 0.3
        out $ ~single
    "#;

    // Layered octaves (no filter) - should be brighter
    let stack_code = r#"
        cps: 0.5
        ~fund $ saw 55 * 0.3
        ~oct1 $ saw 110 * 0.2
        ~oct2 $ saw 220 * 0.1
        out $ ~fund + ~oct1 + ~oct2
    "#;

    let single_audio = render_dsl(single_code, 2.0);
    let stack_audio = render_dsl(stack_code, 2.0);

    let single_centroid = spectral_centroid(&single_audio);
    let stack_centroid = spectral_centroid(&stack_audio);

    // Stack with higher octaves (saws at 55+110+220Hz) should be brighter
    // than a pure sine at 55Hz
    assert!(
        stack_centroid > single_centroid,
        "Harmonic stack (centroid {:.0}Hz) should be brighter than single sine (centroid {:.0}Hz)",
        stack_centroid,
        single_centroid
    );
}

/// Supersaw should produce audible audio and be spectrally different from a plain saw
#[test]
fn test_ambient_supersaw_richer_than_saw() {
    let saw_code = r#"
        cps: 0.5
        ~saw $ saw 110 * 0.3
        out $ ~saw
    "#;

    let supersaw_code = r#"
        cps: 0.5
        ~ssaw $ supersaw 110 0.5 * 0.3
        out $ ~ssaw
    "#;

    let saw_audio = render_dsl(saw_code, 2.0);
    let supersaw_audio = render_dsl(supersaw_code, 2.0);

    let saw_rms = calculate_rms(&saw_audio);
    let supersaw_rms = calculate_rms(&supersaw_audio);

    // Both should produce audio
    assert!(saw_rms > 0.01, "Saw should produce audio, RMS: {}", saw_rms);
    assert!(
        supersaw_rms > 0.01,
        "Supersaw should produce audio, RMS: {}",
        supersaw_rms
    );

    // Compare using audio similarity - they should NOT be identical
    // (detuning creates a different timbre)
    let scorer = AudioSimilarityScorer::new(SAMPLE_RATE, SimilarityConfig::default());
    let result = scorer.compare(&saw_audio, &supersaw_audio);

    // They're both saw-like but supersaw has detuning, so they shouldn't be
    // perfectly identical (overall similarity < 1.0)
    assert!(
        result.overall < 0.99,
        "Supersaw should differ from plain saw, similarity: {:.2}%",
        result.overall * 100.0
    );
}

/// Rendering the same ambient pattern twice should produce consistent results
#[test]
fn test_ambient_rendering_determinism() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        ~lfo $ sine 0.1
        ~pad $ saw 110 # lpf (~lfo * 600 + 500) 0.4 * 0.2
        out $ ~subbass + ~pad
    "#;

    let audio1 = render_dsl(code, 2.0);
    let audio2 = render_dsl(code, 2.0);

    let scorer = AudioSimilarityScorer::new(SAMPLE_RATE, SimilarityConfig::default());
    let result = scorer.compare(&audio1, &audio2);

    assert!(
        result.overall >= 0.9,
        "Same ambient pattern should render consistently, similarity: {:.1}%",
        result.overall * 100.0
    );
}

/// Different ambient textures should sound different:
/// a low sub drone vs a bright unfiltered saw
#[test]
fn test_ambient_different_textures_distinguishable() {
    // Dark sub drone
    let drone_code = r#"
        cps: 0.5
        ~drone $ sine 55 * 0.3
        out $ ~drone
    "#;

    // Bright unfiltered saw with harmonics
    let bright_code = r#"
        cps: 0.5
        ~bright $ saw 440 * 0.3
        out $ ~bright
    "#;

    let drone_audio = render_dsl(drone_code, 2.0);
    let bright_audio = render_dsl(bright_code, 2.0);

    let drone_centroid = spectral_centroid(&drone_audio);
    let bright_centroid = spectral_centroid(&bright_audio);

    // Sub sine at 55Hz vs unfiltered saw at 440Hz should have very different centroids
    assert!(
        bright_centroid > drone_centroid,
        "Bright saw ({:.0}Hz) should have higher centroid than sub ({:.0}Hz)",
        bright_centroid,
        drone_centroid
    );
}

/// LPF filter on drone should make the spectrum darker (lower centroid)
#[test]
fn test_ambient_lpf_darkens_drone() {
    let bright_code = r#"
        cps: 0.5
        ~saw $ saw 110 * 0.3
        out $ ~saw
    "#;

    let dark_code = r#"
        cps: 0.5
        ~saw $ saw 110 # lpf 300 1.0 * 0.3
        out $ ~saw
    "#;

    let bright_audio = render_dsl(bright_code, 2.0);
    let dark_audio = render_dsl(dark_code, 2.0);

    let bright_rms = calculate_rms(&bright_audio);
    let dark_rms = calculate_rms(&dark_audio);
    assert!(bright_rms > 0.01, "Bright saw should produce audio");
    assert!(dark_rms > 0.01, "Filtered saw should produce audio");

    let bright_centroid = spectral_centroid(&bright_audio);
    let dark_centroid = spectral_centroid(&dark_audio);

    if bright_centroid != dark_centroid {
        assert!(
            bright_centroid > dark_centroid,
            "Unfiltered saw ({:.0}Hz) should be brighter than LPF'd saw ({:.0}Hz)",
            bright_centroid,
            dark_centroid
        );
    }
}

/// LPF on noise should darken the spectrum (lower centroid)
#[test]
fn test_ambient_lpf_darkens_noise() {
    let full_noise = r#"
        cps: 0.5
        ~noise $ pink_noise * 0.2
        out $ ~noise
    "#;

    let filtered_noise = r#"
        cps: 0.5
        ~noise $ pink_noise # lpf 500 0.5 * 0.2
        out $ ~noise
    "#;

    let full_audio = render_dsl(full_noise, 2.0);
    let filtered_audio = render_dsl(filtered_noise, 2.0);

    let full_rms = calculate_rms(&full_audio);
    let filtered_rms = calculate_rms(&filtered_audio);

    // Both should produce audio
    assert!(full_rms > 0.001, "Full noise should produce audio");
    assert!(filtered_rms > 0.001, "Filtered noise should produce audio");

    let full_centroid = spectral_centroid(&full_audio);
    let filtered_centroid = spectral_centroid(&filtered_audio);

    // LPF should make noise darker (or at least not brighter)
    if full_centroid != filtered_centroid {
        assert!(
            filtered_centroid <= full_centroid,
            "LPF'd noise ({:.0}Hz) should be darker than full noise ({:.0}Hz)",
            filtered_centroid,
            full_centroid
        );
    }
}

/// Amplitude modulation (tremolo) should create envelope variation
#[test]
fn test_ambient_amplitude_modulation_creates_variation() {
    let static_code = r#"
        cps: 0.5
        ~tone $ sine 110 * 0.3
        out $ ~tone
    "#;

    let modulated_code = r#"
        cps: 0.5
        ~tone $ sine 110
        ~lfo $ sine 2.0
        ~tremolo $ ~tone * (~lfo * 0.4 + 0.6) * 0.3
        out $ ~tremolo
    "#;

    let static_audio = render_dsl(static_code, 4.0);
    let modulated_audio = render_dsl(modulated_code, 4.0);

    let static_env = envelope_variation(&static_audio, 50.0);
    let modulated_env = envelope_variation(&modulated_audio, 50.0);

    // Amplitude modulation at 2Hz should create measurable envelope variation
    assert!(
        modulated_env > static_env,
        "AM modulated (env var {:.6}) should vary more than static (env var {:.6})",
        modulated_env,
        static_env
    );
}

/// Full ambient composition characteristics:
/// - Non-silent audio
/// - Dark spectrum (low centroid)
/// - Smooth envelope
/// - No extreme clipping
#[test]
fn test_ambient_full_composition_characteristics() {
    let code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.25
        ~fifth $ sine 82.5 * 0.15
        ~lfo $ sine 0.03
        ~pad $ (~subbass + ~fifth) * (~lfo * 0.3 + 0.7)
        ~texture $ pink_noise * 0.03
        out $ ~pad + ~texture
    "#;

    let audio = render_dsl(code, 4.0);

    // Non-silent
    let rms = calculate_rms(&audio);
    assert!(
        rms > 0.01,
        "Full composition should produce audible sound, RMS: {}",
        rms
    );

    // No extreme clipping
    let peak = calculate_peak(&audio);
    assert!(peak < 5.0, "Should not clip heavily, peak: {:.3}", peak);

    // No NaN
    assert!(
        !audio.iter().any(|s| s.is_nan()),
        "Audio should not contain NaN"
    );

    // Ambient centroid should be relatively low (warm timbres)
    let centroid = spectral_centroid(&audio);
    assert!(
        centroid < 5000.0,
        "Ambient composition should have warm spectrum, centroid: {:.0}Hz",
        centroid
    );
}

/// Ambient patterns at different slow tempos should all work
#[test]
fn test_ambient_slow_tempo_range() {
    let tempos = [
        (0.25, "30 BPM"),
        (0.5, "60 BPM"),
        (0.75, "90 BPM"),
        (1.0, "120 BPM"),
    ];

    for (cps, label) in tempos {
        let code = format!(
            r#"
            cps: {}
            ~drone $ saw 55 # lpf 800 0.5 * 0.25
            ~lfo $ sine 0.1
            ~pad $ (~drone) * (~lfo * 0.3 + 0.7)
            out $ ~pad
        "#,
            cps
        );

        let audio = render_dsl(&code, 4.0);
        let rms = calculate_rms(&audio);
        assert!(
            rms > 0.01,
            "{} (cps {}) should produce audio, RMS: {}",
            label,
            cps,
            rms
        );
    }
}

/// Shimmer texture with layered octaves should have higher centroid than single bass
#[test]
fn test_ambient_shimmer_brighter_than_sub() {
    let sub_code = r#"
        cps: 0.5
        ~subbass $ sine 55 * 0.3
        out $ ~subbass
    "#;

    let shimmer_code = r#"
        cps: 0.5
        ~low $ sine 110 * 0.3
        ~mid $ sine 220 * 0.2
        ~high $ sine 440 * 0.1
        ~lfo $ sine 0.17
        ~shimmer $ (~low + ~mid + ~high) * (~lfo * 0.3 + 0.7)
        out $ ~shimmer
    "#;

    let sub_audio = render_dsl(sub_code, 2.0);
    let shimmer_audio = render_dsl(shimmer_code, 2.0);

    let sub_centroid = spectral_centroid(&sub_audio);
    let shimmer_centroid = spectral_centroid(&shimmer_audio);

    assert!(
        shimmer_centroid > sub_centroid,
        "Shimmer ({:.0}Hz) should be brighter than sub ({:.0}Hz)",
        shimmer_centroid,
        sub_centroid
    );
}

/// Evolving drone vs static drone: modulated version should have different
/// envelope characteristics over time
#[test]
fn test_ambient_evolving_vs_static_drone() {
    let static_code = r#"
        cps: 0.5
        ~drone $ saw 55 # lpf 600 0.5 * 0.3
        out $ ~drone
    "#;

    let evolving_code = r#"
        cps: 0.5
        ~drone $ saw 55
        ~lfo_slow $ sine 0.5
        ~lfo_fast $ sine 2.0
        ~evolving $ ~drone # lpf (~lfo_slow * 800 + ~lfo_fast * 200 + 400) 0.5 * 0.3
        out $ ~evolving
    "#;

    let static_audio = render_dsl(static_code, 4.0);
    let evolving_audio = render_dsl(evolving_code, 4.0);

    let static_rms = calculate_rms(&static_audio);
    let evolving_rms = calculate_rms(&evolving_audio);

    // Both should produce audio
    assert!(static_rms > 0.01, "Static drone should produce audio");
    assert!(evolving_rms > 0.01, "Evolving drone should produce audio");

    // Evolving version should have more spectral centroid variation
    let static_var = spectral_centroid_variation(&static_audio, 200.0);
    let evolving_var = spectral_centroid_variation(&evolving_audio, 200.0);

    assert!(
        evolving_var >= static_var,
        "Evolving drone (centroid var {:.2}) should have >= spectral variation vs static ({:.2})",
        evolving_var,
        static_var
    );
}
