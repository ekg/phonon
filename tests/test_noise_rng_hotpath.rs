//! wave3-noise-rng-hotpath: per-node seeded PRNG for noise generators.
//!
//! RT-safety hot-path polish (improvement-plan P4 / rt F-11). Before this change,
//! `SignalNode::WhiteNoise` / `PinkNoise` / `BrownNoise` called `rand::thread_rng()`
//! **per sample** on the audio render thread — a TLS lookup + periodic reseed check on
//! the hot path → timing jitter on noise-heavy patches. This suite verifies the fix:
//! each noise node owns a `NoiseRng` (xorshift32, splitmix64-seeded ONCE at construction)
//! advanced per sample, with no `thread_rng()` on the render loop.
//!
//! Three-level audio-testing methodology (CLAUDE.md):
//!   Level 1 — pattern/sample-count + finiteness across cycles.
//!   Level 2 — statistics/spectrum (white ≈ flat, pink ≈ -3dB/oct, zero-mean).
//!   Level 3 — audio characteristics (RMS in range, no NaN/Inf, no DC offset).
//! Plus: determinism when seeded, and independence across graph builds.

use phonon::unified_graph::{
    BrownNoiseState, NoiseRng, PinkNoiseState, SignalNode, UnifiedSignalGraph,
};
use std::f32::consts::PI;

const SR: f32 = 44100.0;

/// Render `n` raw samples of a single noise node (via `process_sample`, which — unlike
/// `render` — does not apply the master limiter, so we observe the raw generator).
fn render_noise(node: SignalNode, n: usize) -> Vec<f32> {
    let mut g = UnifiedSignalGraph::new(SR);
    let id = g.add_node(node);
    g.set_output(id);
    (0..n).map(|_| g.process_sample()).collect()
}

/// Render a white-noise node whose per-node PRNG seed is derived from an explicit
/// graph-level base seed, giving reproducible output for a fixed base.
fn render_white_seeded(base: u64, n: usize) -> Vec<f32> {
    let mut g = UnifiedSignalGraph::new(SR);
    g.set_noise_seed_base(base);
    let id = g.add_node(SignalNode::WhiteNoise);
    g.set_output(id);
    (0..n).map(|_| g.process_sample()).collect()
}

fn rms(buf: &[f32]) -> f32 {
    (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt()
}

fn mean(buf: &[f32]) -> f32 {
    buf.iter().sum::<f32>() / buf.len() as f32
}

/// Hann-windowed magnitude spectrum.
fn spectrum(buf: &[f32]) -> (Vec<f32>, Vec<f32>) {
    use rustfft::{num_complex::Complex, FftPlanner};
    let fft_size = 8192.min(buf.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut input: Vec<Complex<f32>> = buf[..fft_size]
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
            Complex::new(s * w, 0.0)
        })
        .collect();
    fft.process(&mut input);
    let mags: Vec<f32> = input[..fft_size / 2]
        .iter()
        .map(|c| (c.re * c.re + c.im * c.im).sqrt())
        .collect();
    let freqs: Vec<f32> = (0..fft_size / 2)
        .map(|i| i as f32 * SR / fft_size as f32)
        .collect();
    (freqs, mags)
}

fn band_energy(freqs: &[f32], mags: &[f32], lo: f32, hi: f32) -> f32 {
    freqs
        .iter()
        .zip(mags.iter())
        .filter(|(f, _)| **f >= lo && **f < hi)
        .map(|(_, m)| m * m)
        .sum()
}

/// Mean power **per FFT bin** in a band (spectral density). Unlike a raw sum, this is
/// comparable across bands of different widths, so it measures spectral *slope* correctly.
fn band_mean_power(freqs: &[f32], mags: &[f32], lo: f32, hi: f32) -> f32 {
    let bins: Vec<f32> = freqs
        .iter()
        .zip(mags.iter())
        .filter(|(f, _)| **f >= lo && **f < hi)
        .map(|(_, m)| m * m)
        .collect();
    if bins.is_empty() {
        0.0
    } else {
        bins.iter().sum::<f32>() / bins.len() as f32
    }
}

// ===================================================================================
// HEADLINE: no thread_rng() on the hot path (structural, proven via seeded determinism)
// ===================================================================================

#[test]
fn test_noise_rng_per_node_no_thread_rng_on_hotpath() {
    // If the render loop still called `thread_rng()` per sample, seeding a node's PRNG
    // could NOT make its output reproducible. Building two independent graphs, each with
    // an identically-seeded noise node, and getting bit-for-bit identical buffers is a
    // structural proof that the PRNG state lives on the node and is the sole entropy
    // source on the hot path.
    let n = 4096;

    // Pink noise via an explicitly-seeded state node — routed through the REAL eval hot path.
    let a = render_noise(
        SignalNode::PinkNoise {
            state: PinkNoiseState::with_seed(0xABCD_1234),
        },
        n,
    );
    let b = render_noise(
        SignalNode::PinkNoise {
            state: PinkNoiseState::with_seed(0xABCD_1234),
        },
        n,
    );
    assert_eq!(a, b, "seeded pink noise must be bit-for-bit reproducible");

    // Brown noise likewise.
    let c = render_noise(
        SignalNode::BrownNoise {
            state: BrownNoiseState::with_seed(777),
        },
        n,
    );
    let d = render_noise(
        SignalNode::BrownNoise {
            state: BrownNoiseState::with_seed(777),
        },
        n,
    );
    assert_eq!(c, d, "seeded brown noise must be bit-for-bit reproducible");

    // White noise via a graph-level base seed (its node PRNG is seeded from the base).
    let w1 = render_white_seeded(42, n);
    let w2 = render_white_seeded(42, n);
    assert_eq!(w1, w2, "seeded white noise must be bit-for-bit reproducible");

    // Sanity: all buffers actually contain varying signal (not a stuck constant).
    for buf in [&a, &c, &w1] {
        let distinct = buf.iter().map(|x| x.to_bits()).collect::<std::collections::HashSet<_>>();
        assert!(
            distinct.len() > buf.len() / 2,
            "noise must vary sample-to-sample, got {} distinct of {}",
            distinct.len(),
            buf.len()
        );
    }
}

// ===================================================================================
// Level 1 — sample count + finiteness across cycles
// ===================================================================================

#[test]
fn test_noise_level1_sample_count_and_finite() {
    let n = SR as usize; // 1 second = ~0.5 cycle at default cps, spans a cycle boundary
    for node in [
        SignalNode::WhiteNoise,
        SignalNode::PinkNoise {
            state: PinkNoiseState::new(),
        },
        SignalNode::BrownNoise {
            state: BrownNoiseState::new(),
        },
    ] {
        let buf = render_noise(node, n);
        assert_eq!(buf.len(), n, "must produce exactly n samples");
        assert!(
            buf.iter().all(|x| x.is_finite()),
            "all noise samples must be finite (no NaN/Inf)"
        );
    }
}

// ===================================================================================
// Level 2 — statistics / spectrum
// ===================================================================================

#[test]
fn test_white_noise_zero_mean_and_flat_spectrum() {
    let buf = render_white_seeded(12345, 44100);
    // Zero-mean (no DC).
    assert!(mean(&buf).abs() < 0.05, "white noise mean ~0, got {}", mean(&buf));

    let (freqs, mags) = spectrum(&buf);
    let low = band_energy(&freqs, &mags, 200.0, 2000.0);
    let high = band_energy(&freqs, &mags, 2000.0, 20000.0);
    // White noise is broadband: low and high octave-decade energy are within the same
    // order of magnitude (flat-ish). Allow a generous 4x window for a single-buffer FFT.
    let ratio = low / high.max(1e-9);
    assert!(
        ratio > 0.1 && ratio < 4.0,
        "white noise spectrum should be broadband/flat-ish, low/high={}",
        ratio
    );
}

#[test]
fn test_pink_noise_negative_slope_spectrum() {
    let buf = render_noise(
        SignalNode::PinkNoise {
            state: PinkNoiseState::with_seed(2024),
        },
        44100,
    );
    let (freqs, mags) = spectrum(&buf);
    // Compare spectral DENSITY (mean power per bin), which is band-width independent.
    // Pink noise (~-3dB/oct) has clearly higher density at low frequencies than high.
    let low = band_mean_power(&freqs, &mags, 100.0, 400.0);
    let high = band_mean_power(&freqs, &mags, 3200.0, 12800.0);
    assert!(
        low > high * 2.0,
        "pink noise density should slope down (low>>high), low={}, high={}",
        low,
        high
    );
}

#[test]
fn test_brown_noise_stronger_negative_slope_than_pink() {
    let pink = render_noise(
        SignalNode::PinkNoise {
            state: PinkNoiseState::with_seed(1),
        },
        44100,
    );
    let brown = render_noise(
        SignalNode::BrownNoise {
            state: BrownNoiseState::with_seed(1),
        },
        44100,
    );
    let (pf, pm) = spectrum(&pink);
    let (bf, bm) = spectrum(&brown);
    let pink_ratio =
        band_energy(&pf, &pm, 0.0, 500.0) / band_energy(&pf, &pm, 5000.0, 15000.0).max(1e-9);
    let brown_ratio =
        band_energy(&bf, &bm, 0.0, 500.0) / band_energy(&bf, &bm, 5000.0, 15000.0).max(1e-9);
    assert!(
        brown_ratio > pink_ratio,
        "brown noise should have steeper low-freq dominance than pink: brown={}, pink={}",
        brown_ratio,
        pink_ratio
    );
}

// ===================================================================================
// Level 3 — audio characteristics
// ===================================================================================

#[test]
fn test_noise_rms_in_sane_range_no_dc() {
    for (name, node) in [
        ("white", SignalNode::WhiteNoise),
        (
            "pink",
            SignalNode::PinkNoise {
                state: PinkNoiseState::new(),
            },
        ),
        (
            "brown",
            SignalNode::BrownNoise {
                state: BrownNoiseState::new(),
            },
        ),
    ] {
        let buf = render_noise(node, 44100);
        let r = rms(&buf);
        assert!(
            r > 0.01 && r < 1.0,
            "{} noise RMS should be in (0.01, 1.0), got {}",
            name,
            r
        );
        assert!(
            buf.iter().all(|x| x.is_finite()),
            "{} noise must have no NaN/Inf",
            name
        );
        assert!(
            mean(&buf).abs() < 0.15,
            "{} noise must not introduce a DC offset, mean={}",
            name,
            mean(&buf)
        );
    }
}

// ===================================================================================
// Determinism (same seed → same buffer) and independence (different builds → decorrelated)
// ===================================================================================

#[test]
fn test_noise_rng_determinism_and_decorrelation() {
    // Same seed → same stream.
    let mut r1 = NoiseRng::from_seed(0xDEAD_BEEF);
    let mut r2 = NoiseRng::from_seed(0xDEAD_BEEF);
    for _ in 0..10_000 {
        assert_eq!(r1.next_u32(), r2.next_u32());
    }
    // Different seeds → different streams (decorrelated).
    let mut a = NoiseRng::from_seed(1);
    let mut b = NoiseRng::from_seed(2);
    let mut diffs = 0;
    for _ in 0..10_000 {
        if a.next_u32() != b.next_u32() {
            diffs += 1;
        }
    }
    assert!(diffs > 9900, "distinct seeds should decorrelate, diffs={}", diffs);
}

#[test]
fn test_seeded_white_noise_different_seeds_differ() {
    let w42 = render_white_seeded(42, 4096);
    let w43 = render_white_seeded(43, 4096);
    assert_ne!(w42, w43, "different base seeds must yield different white noise");
}

#[test]
fn test_two_graph_builds_produce_independent_noise() {
    // Two graphs built from scratch with DEFAULT (unseeded) noise nodes must produce
    // INDEPENDENT streams — the per-node default seed advances a process-global counter,
    // so successive builds are decorrelated (not the same stuck sequence). This mirrors
    // the existing `test_brown_noise_consistent_output` guarantee.
    let a = render_noise(SignalNode::WhiteNoise, 4096);
    let b = render_noise(SignalNode::WhiteNoise, 4096);
    let same = a
        .iter()
        .zip(b.iter())
        .filter(|(x, y)| (**x - **y).abs() < 1e-9)
        .count();
    assert!(
        (same as f32) < 0.1 * a.len() as f32,
        "two default white-noise builds should be independent, {}/{} samples identical",
        same,
        a.len()
    );

    // Pink and brown likewise.
    let p1 = render_noise(SignalNode::PinkNoise { state: PinkNoiseState::new() }, 4096);
    let p2 = render_noise(SignalNode::PinkNoise { state: PinkNoiseState::new() }, 4096);
    assert_ne!(p1, p2, "two default pink builds should differ");
}
