//! Capstone test: Musical quality metrics above threshold for all patterns.
//!
//! Verifies that every genre pattern and common Phonon program scores above
//! the minimum quality threshold (0.7) when evaluated against its matching
//! genre profile using the three-level scoring methodology:
//!   1. Rhythm (pattern density, syncopation, evenness, entropy)
//!   2. Audio  (RMS, peak, spectral content, onset count)
//!   3. Timing (expected vs detected events)

use phonon::musical_quality::{GenreProfile, MusicalQualityScorer};

const QUALITY_THRESHOLD: f32 = 0.7;

fn assert_quality(name: &str, code: &str, profile: GenreProfile) {
    let score = MusicalQualityScorer::new()
        .genre(profile)
        .duration(2.0)
        .score_dsl(code);

    assert!(
        score.overall >= QUALITY_THRESHOLD,
        "{} scored {:.2} (below {:.2} threshold):\n{}",
        name,
        score.overall,
        QUALITY_THRESHOLD,
        score.report()
    );

    // Also verify sub-scores are non-zero (all three levels evaluated)
    assert!(
        score.audio.has_audio,
        "{} should produce audio",
        name
    );
}

// ============================================================================
// Hip-Hop Genre Family
// ============================================================================

#[test]
fn capstone_quality_boom_bap() {
    assert_quality(
        "boom-bap",
        r#"
            tempo: 1.5
            out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
        "#,
        GenreProfile::boom_bap(),
    );
}

#[test]
fn capstone_quality_trap() {
    assert_quality(
        "trap",
        r#"
            tempo: 2.33
            out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
        "#,
        GenreProfile::trap(),
    );
}

#[test]
fn capstone_quality_lofi() {
    assert_quality(
        "lo-fi",
        r#"
            tempo: 1.25
            out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8" * 0.4
        "#,
        GenreProfile::lofi(),
    );
}

#[test]
fn capstone_quality_drill() {
    assert_quality(
        "drill",
        r#"
            tempo: 2.33
            out $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~" + s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*3] hh hh hh hh"
        "#,
        GenreProfile::drill(),
    );
}

#[test]
fn capstone_quality_phonk() {
    assert_quality(
        "phonk",
        r#"
            tempo: 2.33
            out $ s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*8" * 0.5
        "#,
        GenreProfile::phonk(),
    );
}

// ============================================================================
// Electronic Dance Music Family
// ============================================================================

#[test]
fn capstone_quality_house() {
    assert_quality(
        "house",
        r#"
            tempo: 2.0
            out $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~" + s "hh*8"
        "#,
        GenreProfile::house(),
    );
}

#[test]
fn capstone_quality_techno() {
    assert_quality(
        "techno",
        r#"
            tempo: 2.17
            out $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~" + s "hh*16" * 0.5
        "#,
        GenreProfile::techno(),
    );
}

#[test]
fn capstone_quality_dnb() {
    assert_quality(
        "dnb",
        r#"
            tempo: 2.83
            out $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh*8"
        "#,
        GenreProfile::dnb(),
    );
}

#[test]
fn capstone_quality_uk_garage() {
    assert_quality(
        "uk-garage",
        r#"
            tempo: 2.17
            out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~" + s "hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~"
        "#,
        GenreProfile::uk_garage(),
    );
}

// ============================================================================
// Synthesis Patterns
// ============================================================================

#[test]
fn capstone_quality_sine_oscillator() {
    assert_quality(
        "sine-oscillator",
        "out $ sine 440",
        GenreProfile::default(),
    );
}

#[test]
fn capstone_quality_saw_bass_with_filter() {
    assert_quality(
        "saw-bass-filtered",
        r#"out $ saw "55 82.5" # lpf 600 0.7 * 0.3"#,
        GenreProfile::default(),
    );
}

// ============================================================================
// Multi-Layer Production
// ============================================================================

#[test]
fn capstone_quality_multi_layer_production() {
    assert_quality(
        "multi-layer-production",
        r#"
            tempo: 1.5
            ~kick $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~"
            ~snare $ s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~"
            ~hats $ s "hh hh oh hh hh hh hh oh"
            ~bass $ saw "55 55 82.5 73.4" # lpf 600 0.7 * 0.3
            out $ ~kick + ~snare + ~hats + ~bass
        "#,
        GenreProfile::boom_bap(),
    );
}

// ============================================================================
// Summary: All Genres Above Threshold
// ============================================================================

#[test]
fn capstone_all_genres_above_threshold() {
    let patterns: Vec<(&str, &str, GenreProfile)> = vec![
        (
            "boom-bap",
            r#"
                tempo: 1.5
                out $ s "bd ~ ~ ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8"
            "#,
            GenreProfile::boom_bap(),
        ),
        (
            "trap",
            r#"
                tempo: 2.33
                out $ s "808bd ~ ~ ~ ~ ~ ~ ~ 808bd ~ ~ ~ 808bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*16"
            "#,
            GenreProfile::trap(),
        ),
        (
            "lo-fi",
            r#"
                tempo: 1.25
                out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ bd ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~" + s "hh*8" * 0.4
            "#,
            GenreProfile::lofi(),
        ),
        (
            "drill",
            r#"
                tempo: 2.33
                out $ s "808bd ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~ ~ ~ 808bd ~ ~" + s "~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh hh hh hh [hh*3] hh hh hh hh hh hh [hh*3] hh hh hh hh"
            "#,
            GenreProfile::drill(),
        ),
        (
            "phonk",
            r#"
                tempo: 2.33
                out $ s "808bd ~ ~ ~ ~ ~ 808bd ~ 808bd ~ ~ ~ ~ ~ 808bd ~" + s "~ ~ ~ ~ ~ ~ cp ~ ~ ~ ~ ~ ~ ~ cp ~" + s "hh*8" * 0.5
            "#,
            GenreProfile::phonk(),
        ),
        (
            "house",
            r#"
                tempo: 2.0
                out $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~" + s "~ ~ ~ ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~" + s "hh*8"
            "#,
            GenreProfile::house(),
        ),
        (
            "techno",
            r#"
                tempo: 2.17
                out $ s "bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~ bd ~ ~ ~" + s "hh*16" * 0.5
            "#,
            GenreProfile::techno(),
        ),
        (
            "dnb",
            r#"
                tempo: 2.83
                out $ s "bd ~ ~ ~ ~ ~ ~ ~ ~ ~ bd ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ ~ ~ ~ ~ sn ~" + s "hh*8"
            "#,
            GenreProfile::dnb(),
        ),
        (
            "uk-garage",
            r#"
                tempo: 2.17
                out $ s "bd ~ ~ ~ ~ bd ~ ~ bd ~ ~ ~ ~ ~ ~ ~" + s "~ ~ ~ ~ sn ~ ~ ~ ~ ~ sn ~ ~ ~ ~ ~" + s "hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~ hh ~"
            "#,
            GenreProfile::uk_garage(),
        ),
    ];

    let mut all_pass = true;
    let mut report = String::from("Musical Quality Capstone Results:\n");

    for (name, code, profile) in &patterns {
        let score = MusicalQualityScorer::new()
            .genre(profile.clone())
            .duration(2.0)
            .score_dsl(code);

        let passed = score.overall >= QUALITY_THRESHOLD;
        let marker = if passed { "PASS" } else { "FAIL" };
        report.push_str(&format!(
            "  [{}] {}: {:.0}% (rhythm={:.0}%, audio={:.0}%, timing={:.0}%)\n",
            marker,
            name,
            score.overall * 100.0,
            score.rhythm.score * 100.0,
            score.audio.score * 100.0,
            score.timing.score * 100.0,
        ));

        if !passed {
            all_pass = false;
            report.push_str(&format!("        {}\n", score.report()));
        }
    }

    println!("{}", report);
    assert!(all_pass, "Some patterns failed quality threshold:\n{}", report);
}
