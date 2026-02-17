//! Integration tests for the automated musical quality scoring system.
//!
//! Tests that the quality scorer correctly evaluates patterns across:
//! - Genre-specific scoring profiles
//! - Three-level methodology (pattern, audio, timing)
//! - Batch scoring
//! - Edge cases

use phonon::musical_quality::{
    batch_genre_score, batch_score, genre_quality_score, quality_score, GenreProfile,
    MusicalQualityScorer,
};

// ============================================================================
// Genre Profile Scoring Tests
// ============================================================================

#[test]
fn test_boombap_pattern_scores_well_against_boombap_profile() {
    let code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::boom_bap())
        .duration(2.0)
        .score_dsl(code);

    println!("Boom-bap quality report:\n{}", score.report());

    assert!(score.audio.has_audio, "Should produce audio");
    assert!(!score.audio.clips, "Should not clip");
    assert!(
        score.rhythm.density >= 8.0,
        "Combined density should be >= 8 (3 kick + 2 snare + 8 hats), got {}",
        score.rhythm.density
    );
    assert!(
        score.overall >= 0.3,
        "Boom-bap pattern should score at least 0.3: {}",
        score.report()
    );
}

#[test]
fn test_trap_pattern_scores_well_against_trap_profile() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::trap())
        .duration(2.0)
        .score_dsl(code);

    println!("Trap quality report:\n{}", score.report());

    assert!(score.audio.has_audio, "Should produce audio");
    assert!(
        score.rhythm.density >= 12.0,
        "Trap density should be high (16 hats + kicks + claps), got {}",
        score.rhythm.density
    );
}

#[test]
fn test_lofi_pattern_scores_well_against_lofi_profile() {
    let code = r#"
        tempo: 1.25
        out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8" * 0.4
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::lofi())
        .duration(2.0)
        .score_dsl(code);

    println!("Lo-fi quality report:\n{}", score.report());

    assert!(score.audio.has_audio, "Should produce audio");
    assert!(!score.audio.clips, "Should not clip");
}

#[test]
fn test_drill_pattern_evaluation() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~" + s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*3] hh hh hh hh"
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::drill())
        .duration(2.0)
        .score_dsl(code);

    println!("Drill quality report:\n{}", score.report());

    assert!(score.audio.has_audio, "Drill pattern should produce audio");
}

#[test]
fn test_phonk_pattern_evaluation() {
    let code = r#"
        tempo: 2.33
        out $ s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*8" * 0.5
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::phonk())
        .duration(2.0)
        .score_dsl(code);

    println!("Phonk quality report:\n{}", score.report());

    assert!(score.audio.has_audio, "Phonk should produce audio");
}

// ============================================================================
// Cross-Genre Comparison Tests
// ============================================================================

#[test]
fn test_boombap_pattern_different_genre_profiles() {
    let boombap_code = r#"
        tempo: 1.5
        out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
    "#;

    let boombap_vs_boombap = MusicalQualityScorer::new()
        .genre(GenreProfile::boom_bap())
        .duration(2.0)
        .score_dsl(boombap_code);

    let boombap_vs_trap = MusicalQualityScorer::new()
        .genre(GenreProfile::trap())
        .duration(2.0)
        .score_dsl(boombap_code);

    println!(
        "Boom-bap vs boom-bap profile: {:.2}",
        boombap_vs_boombap.overall
    );
    println!("Boom-bap vs trap profile: {:.2}", boombap_vs_trap.overall);

    // Both should produce audio
    assert!(boombap_vs_boombap.audio.has_audio);
    assert!(boombap_vs_trap.audio.has_audio);
}

// ============================================================================
// Pattern-Only Scoring Tests
// ============================================================================

#[test]
fn test_pattern_only_scoring_basic() {
    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::boom_bap())
        .score_pattern("bd ~ sn ~ bd ~ sn ~");

    assert!(score.rhythm.density > 0.0, "Should detect events");
    assert!(score.rhythm.score > 0.0, "Should have rhythm score");
    assert!(
        score.checks.iter().any(|c| c.category == "rhythm"),
        "Should have rhythm checks"
    );
}

#[test]
fn test_pattern_only_high_density() {
    let sparse = MusicalQualityScorer::new().score_pattern("bd ~ ~ ~ ~ ~ ~ ~");

    let dense = MusicalQualityScorer::new().score_pattern("bd sn hh cp bd sn hh cp");

    assert!(
        dense.rhythm.density > sparse.rhythm.density,
        "Dense pattern ({}) should have higher density than sparse ({})",
        dense.rhythm.density,
        sparse.rhythm.density
    );
}

#[test]
fn test_pattern_only_syncopation() {
    let on_beat = MusicalQualityScorer::new().score_pattern("bd ~ bd ~ bd ~ bd ~");

    let off_beat = MusicalQualityScorer::new().score_pattern("~ bd ~ bd ~ bd ~ bd");

    assert!(
        off_beat.rhythm.syncopation > on_beat.rhythm.syncopation,
        "Off-beat ({}) should be more syncopated than on-beat ({})",
        off_beat.rhythm.syncopation,
        on_beat.rhythm.syncopation
    );
}

#[test]
fn test_pattern_only_evenness() {
    let even = MusicalQualityScorer::new().score_pattern("bd bd bd bd");

    assert!(
        even.rhythm.evenness > 0.9,
        "Evenly spaced pattern should have high evenness, got {}",
        even.rhythm.evenness
    );
}

// ============================================================================
// Audio-Only Scoring Tests
// ============================================================================

#[test]
fn test_audio_only_scoring_sine() {
    let signal: Vec<f32> = (0..88200)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5)
        .collect();

    let score = MusicalQualityScorer::new().score_audio(&signal);

    assert!(score.audio.has_audio, "Should detect audio");
    assert!(!score.audio.clips, "Should not clip");
    assert!(score.audio.rms > 0.1, "Should have significant RMS");
}

#[test]
fn test_audio_only_scoring_silence() {
    let silence = vec![0.0f32; 44100];
    let score = MusicalQualityScorer::new().score_audio(&silence);

    assert!(!score.audio.has_audio, "Should detect silence");
    assert_eq!(score.audio.rms, 0.0);
    assert!(!score.audio.clips);
}

// ============================================================================
// Batch Scoring Tests
// ============================================================================

#[test]
fn test_batch_score_multiple_patterns() {
    let programs = vec![
        (
            "boom-bap",
            r#"
                tempo: 1.5
                out $ s "bd ~ sn ~" + s "hh*8"
            "#,
        ),
        (
            "trap",
            r#"
                tempo: 2.33
                out $ s "808bd ~ ~ ~ 808bd ~ ~ ~" + s "hh*16"
            "#,
        ),
        ("sine", "out $ sine 440"),
    ];

    let results = batch_score(&programs);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].0, "boom-bap");
    assert_eq!(results[1].0, "trap");
    assert_eq!(results[2].0, "sine");

    // All should produce some score
    for (name, score) in &results {
        assert!(score.audio.has_audio, "{} should produce audio", name);
        println!("{}: overall={:.2}", name, score.overall);
    }
}

#[test]
fn test_batch_genre_score() {
    let programs = vec![
        (
            "basic beat",
            r#"
                tempo: 1.5
                out $ s "bd ~ sn ~" + s "hh*8"
            "#,
        ),
        (
            "full beat",
            r#"
                tempo: 1.5
                out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
            "#,
        ),
    ];

    let results = batch_genre_score(&programs, GenreProfile::boom_bap());

    assert_eq!(results.len(), 2);
    for (name, score) in &results {
        assert!(score.audio.has_audio, "{} should produce audio", name);
    }
}

// ============================================================================
// Convenience Function Tests
// ============================================================================

#[test]
fn test_quality_score_convenience_function() {
    let score = quality_score(
        r#"
        tempo: 1.5
        out $ s "bd sn" + s "hh*8"
    "#,
    );

    assert!(score > 0.0, "Should return a positive score, got {}", score);
}

#[test]
fn test_genre_quality_score_convenience_function() {
    let score = genre_quality_score(
        r#"
            tempo: 1.5
            out $ s "bd ~ sn ~" + s "hh*8"
        "#,
        GenreProfile::boom_bap(),
    );

    assert!(score > 0.0, "Should score > 0, got {}", score);
}

// ============================================================================
// Score Report Tests
// ============================================================================

#[test]
fn test_report_contains_all_sections() {
    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::boom_bap())
        .duration(2.0)
        .score_dsl(
            r#"
            tempo: 1.5
            out $ s "bd sn" + s "hh*8"
        "#,
        );

    let report = score.report();

    assert!(report.contains("Musical Quality Score"), "Missing header");
    assert!(report.contains("Rhythm:"), "Missing rhythm section");
    assert!(report.contains("Audio:"), "Missing audio section");
    assert!(report.contains("Timing:"), "Missing timing section");
}

#[test]
fn test_report_shows_failures() {
    // Score silence - should have failures
    let silence = vec![0.0f32; 44100];
    let score = MusicalQualityScorer::new().score_audio(&silence);

    let report = score.report();
    assert!(
        report.contains("Failed checks"),
        "Report should show failed checks for silence"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_audio() {
    let score = MusicalQualityScorer::new().score_audio(&[]);

    assert!(!score.audio.has_audio);
    assert_eq!(score.audio.rms, 0.0);
    assert_eq!(score.audio.peak, 0.0);
}

#[test]
fn test_single_sample() {
    let score = MusicalQualityScorer::new().score_audio(&[0.5]);

    assert!(score.audio.has_audio);
    assert!(score.audio.rms > 0.0);
}

#[test]
fn test_custom_genre_profile() {
    let profile = GenreProfile::custom("minimal")
        .with_density(2.0, 6.0)
        .with_syncopation(0.0, 0.3)
        .with_evenness(0.8, 1.0)
        .with_rms(0.001, 0.5)
        .with_min_onsets(1.0)
        .with_weights(0.5, 0.3, 0.2);

    let score = MusicalQualityScorer::new()
        .genre(profile)
        .score_pattern("bd ~ bd ~ bd ~ bd ~");

    assert_eq!(score.rhythm.density, 4.0);
    assert!(score.rhythm.evenness > 0.8);
}

#[test]
fn test_oscillator_pattern_scoring() {
    let score = MusicalQualityScorer::new()
        .duration(1.0)
        .score_dsl("out $ sine 440");

    assert!(
        score.audio.has_audio,
        "Oscillator should produce audio: {}",
        score.report()
    );
    assert!(
        score.audio.rms > 0.01,
        "Oscillator should have audible RMS: {}",
        score.audio.rms
    );
}

#[test]
fn test_multi_layer_production_quality() {
    let code = r#"
        tempo: 1.5
        ~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
        ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
        ~hats $ s "hh hh oh hh hh hh hh oh"
        ~bass $ saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
        out $ ~kick + ~snare + ~hats + ~bass
    "#;

    let score = MusicalQualityScorer::new()
        .genre(GenreProfile::boom_bap())
        .duration(3.0)
        .score_dsl(code);

    println!("Multi-layer production:\n{}", score.report());

    assert!(score.audio.has_audio, "Production should produce audio");
    assert!(!score.audio.clips, "Production should not clip");
    assert!(
        score.audio.onset_count >= 5,
        "Production should have many onsets, got {}",
        score.audio.onset_count
    );
}

// ============================================================================
// Three-Level Methodology Validation
// ============================================================================

#[test]
fn test_three_levels_all_evaluated_for_dsl() {
    let score = MusicalQualityScorer::new().duration(2.0).score_dsl(
        r#"
            tempo: 1.5
            out $ s "bd ~ sn ~" + s "hh*8"
        "#,
    );

    // Level 1: Rhythm (pattern metrics)
    assert!(
        score.checks.iter().any(|c| c.category == "rhythm"),
        "Should have rhythm checks (Level 1)"
    );
    assert!(score.rhythm.density > 0.0, "Should compute density");

    // Level 2: Audio (signal analysis)
    assert!(
        score.checks.iter().any(|c| c.category == "audio"),
        "Should have audio checks (Level 2)"
    );

    // Level 3: Timing (onset alignment)
    assert!(
        score.checks.iter().any(|c| c.category == "timing"),
        "Should have timing checks (Level 3)"
    );
}

#[test]
fn test_pass_rate_calculation() {
    let score = MusicalQualityScorer::new().duration(2.0).score_dsl(
        r#"
            tempo: 1.5
            out $ s "bd sn" + s "hh*8"
        "#,
    );

    let rate = score.pass_rate();
    assert!(
        rate >= 0.0 && rate <= 1.0,
        "Pass rate should be 0-1, got {}",
        rate
    );
    assert!(rate > 0.0, "Valid pattern should pass at least some checks");
}
