//! `glitch_stress` — headless live-session stress harness runner.
//!
//! Simulates an interactive live-coding session end-to-end: load a DSL program,
//! render continuously, and perform scripted AND seeded-random sequences of
//! graph swaps / edits / tempo changes through the *real* modal-editor swap
//! path, analysing output for clicks, dropouts, NaN/Inf, DC offset, stuck
//! voices, unbounded RMS growth, and callback-budget overruns.
//!
//! # Single-command CI usage
//!
//! ```text
//! cargo run --release --bin glitch_stress -- --seed 42
//! ```
//!
//! Exit code is non-zero if any hard defect is detected, and the seed is always
//! printed so a failing session is reproducible.
//!
//! Modes (default: all of them):
//!   --scripted      run only the scripted audit-scenario set
//!   --random        run only the seeded randomised session
//!   --concurrent    run only the threaded (real synth-thread) session
//!   --self-test     run only the detector self-tests on injected defects

use clap::Parser;
use phonon::stress_harness::{
    self, known_good_pool, run_all_scenarios, run_concurrent_session, run_random_session,
    SessionConfig, Thresholds,
};

#[derive(Parser, Debug)]
#[command(
    name = "glitch_stress",
    about = "Headless live-coding stress harness with defect detection"
)]
struct Args {
    /// Seed for the deterministic randomised session (reproduces any failure).
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Target audio duration of the randomised session, in seconds.
    #[arg(long, default_value_t = 60.0)]
    seconds: f32,

    /// Minimum number of graph swaps in the randomised session.
    #[arg(long, default_value_t = 50)]
    swaps: usize,

    /// Number of swaps in the concurrent (threaded) session.
    #[arg(long, default_value_t = 40)]
    concurrent_swaps: usize,

    /// Run only the scripted audit-scenario set.
    #[arg(long, default_value_t = false)]
    scripted: bool,

    /// Run only the seeded randomised session.
    #[arg(long, default_value_t = false)]
    random: bool,

    /// Run only the threaded concurrent session.
    #[arg(long, default_value_t = false)]
    concurrent: bool,

    /// Run only the detector self-tests on injected defects.
    #[arg(long, default_value_t = false)]
    self_test: bool,

    /// Measure but do NOT hard-fail on the absolute wall-clock real-time budget
    /// (for local runs on a shared/loaded box). The relative per-block spike
    /// check stays active. By default the standalone binary enforces the
    /// real-time budget, auto-skipping only when its contention probe fires.
    /// `PHONON_STRESS_FORCE_RT_BUDGET=1` forces enforcement past the probe.
    #[arg(long, default_value_t = false)]
    report_budget_only: bool,

    /// Verbose per-block progress.
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    // If no explicit mode is chosen, run everything.
    let run_all = !(args.scripted || args.random || args.concurrent || args.self_test);

    let thr = Thresholds::default();
    let mut hard_failures: Vec<String> = Vec::new();

    println!("=== glitch_stress harness ===");
    println!("seed={}  seconds={}  swaps>={}", args.seed, args.seconds, args.swaps);
    println!();

    if args.self_test || run_all {
        println!("--- detector self-tests (injected defects) ---");
        match stress_harness::run_detector_self_tests() {
            Ok(n) => println!("  {n} detector self-tests passed\n"),
            Err(e) => {
                println!("  DETECTOR SELF-TEST FAILED: {e}\n");
                hard_failures.push(format!("detector self-test: {e}"));
            }
        }
    }

    if args.scripted || run_all {
        println!("--- scripted audit scenarios ---");
        let cfg = SessionConfig::ci(args.seed);
        let (results, failures) = run_all_scenarios(&cfg);
        for r in &results {
            let status = if !r.available {
                "SKIP".to_string()
            } else if r.passed() {
                "ok".to_string()
            } else {
                format!("FAIL: {:?}", r.failures)
            };
            println!(
                "  [{:<28}] {:<4} bnd_delta={:.3} pre_rms={:.4} post_rms={:.4} post_silent={} nan={} raw_nf={} raw_peak={:.2e} => {}",
                r.name, r.audit_ref, r.boundary_delta, r.pre_rms, r.post_rms, r.post_silent, r.nan,
                r.raw_nonfinite, r.raw_peak, status
            );
            if let Some(note) = &r.note {
                println!("        note: {note}");
            }
        }
        if failures.is_empty() {
            println!("  scripted scenarios: all clean/documented\n");
        } else {
            println!("  scripted scenario HARD FAILURES: {failures:?}\n");
            hard_failures.extend(failures);
        }
    }

    if args.random || run_all {
        println!("--- seeded randomised session ---");
        let mut cfg = SessionConfig::ci(args.seed);
        cfg.target_seconds = args.seconds;
        cfg.min_swaps = args.swaps;
        cfg.verbose = args.verbose;
        // The standalone binary is the real-time lane: enforce the absolute
        // wall-clock deadline (auto-skips under the contention probe) so a
        // genuinely over-budget program is caught. `--report-budget-only`
        // downgrades it to report-only for runs on a shared/loaded box.
        cfg.enforce_realtime_budget = !args.report_budget_only;
        let report = run_random_session(&cfg, &known_good_pool());
        println!("  {}", report.summary(&thr));
        if report.budget_check_skipped {
            println!(
                "  NOTE: absolute real-time budget check SKIPPED — host oversubscribed \
                 (probe {:.0}us vs deadline {:.0}us). Run on an idle box or set \
                 PHONON_STRESS_FORCE_RT_BUDGET=1 to enforce.",
                report.calibration_probe_us, report.deadline_us
            );
        }
        if let Some(d) = &report.first_defect {
            println!("  first defect: {d}");
            println!("  reproduce with: --seed {} --random", report.seed);
        }
        let defects = report.hard_defects(&thr);
        if defects.is_empty() {
            println!("  randomised session: CLEAN ({} swaps)\n", report.swaps);
        } else {
            println!("  randomised session HARD DEFECTS: {defects:?}\n");
            for d in defects {
                hard_failures.push(format!("random[seed={}]: {d}", report.seed));
            }
        }
    }

    if args.concurrent || run_all {
        println!("--- threaded concurrent session (real synth thread) ---");
        let cfg = SessionConfig::ci(args.seed);
        let report = run_concurrent_session(&cfg, &known_good_pool(), args.concurrent_swaps);
        println!(
            "  seed={} swaps={} synth_alive={} consumer_blocks={} silent_blocks={} underruns={} max_silent_run={} nonfinite={}",
            report.seed,
            report.swaps,
            report.synth_thread_alive,
            report.consumer_blocks,
            report.silent_consumer_blocks,
            report.underruns,
            report.max_consecutive_silent,
            report.nonfinite_in_output,
        );
        for n in &report.notes {
            if !n.contains("R1 window") {
                println!("        note: {n}");
            }
        }
        let defects = report.hard_defects();
        if defects.is_empty() {
            println!("  concurrent session: CLEAN\n");
        } else {
            println!("  concurrent session HARD DEFECTS: {defects:?}\n");
            for d in defects {
                hard_failures.push(format!("concurrent[seed={}]: {d}", report.seed));
            }
        }
    }

    println!("=== result ===");
    if hard_failures.is_empty() {
        println!("PASS — no hard defects (seed {})", args.seed);
    } else {
        println!("FAIL — {} hard defect(s) (seed {}):", hard_failures.len(), args.seed);
        for f in &hard_failures {
            println!("  - {f}");
        }
        std::process::exit(1);
    }
}
