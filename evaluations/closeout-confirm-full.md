# Closeout: confirm full suite after fix-pre-existing-8

Task: closeout-confirm-full
Date: 2026-07-04T14:24:42Z

## Tested Context

- Worktree branch: `wg/agent-310/closeout-confirm-full`
- Tested HEAD: `4456191`
- Dependency fix commit: `aada6cd` (`fix: allow one-shot sample overlap`)
- Inclusion check: `git merge-base --is-ancestor aada6cd HEAD; echo ancestor_exit=$?`
- Inclusion result: `ancestor_exit=0`

The dependency fix was not initially an ancestor of the checkout. The branch was updated with a non-fast-forward merge of `aada6cd` so the closeout validation tested a context that explicitly includes `fix-pre-existing-8`.

## Commands and Results

1. `git merge-base --is-ancestor aada6cd HEAD; echo ancestor_exit=$?`
   - Result: passed, `ancestor_exit=0`

2. `cargo build`
   - Result: passed
   - Notes: emitted existing warnings from `src/bin/profile_refcell.rs` about unused `BORROW_COUNT` and `BORROW_MUT_COUNT`.

3. `cargo test`
   - Result: failed after extensive progress through the suite.
   - Cut-group/sample validation observed during the run:
     - `tests/test_cut_groups.rs`: passed, 5 passed / 0 failed.
     - `tests/test_sample_cut_groups.rs`: passed, 6 passed / 0 failed.
     - `tests/test_sample_parameters_verification.rs`: passed, 32 passed / 0 failed.
     - `tests/test_sample_trigger_timing.rs`: passed, 24 passed / 0 failed.
   - Failing test:
     - `tests/test_slice_pattern.rs::test_slice_level3_reverse_mirrors_halves`
     - Failure: `reversed first half 0.0530 should mirror base second half 0.0951`

4. `cargo test --test test_slice_pattern test_slice_level3_reverse_mirrors_halves -- --exact --nocapture`
   - Result on tested HEAD `4456191`: failed with the same assertion.

5. `git worktree add -q /tmp/phonon-closeout-main-check edea51f && cargo test --test test_slice_pattern test_slice_level3_reverse_mirrors_halves -- --exact --nocapture`
   - Result on `main`/`edea51f`: failed with the same assertion.
   - Cleanup: `git worktree remove -f /tmp/phonon-closeout-main-check`

## Classification

The full suite did not complete green because of `tests/test_slice_pattern.rs::test_slice_level3_reverse_mirrors_halves`.

Classification: unrelated/pre-existing.

Rationale:

- The failure is in slice pattern reverse-mirror validation, not sample cut-group voice overlap.
- The same single test fails on `main`/`edea51f` with the same assertion values.
- The cut-group-specific tests that were previously targeted by `fix-pre-existing-8` passed inside the full run, including `tests/test_sample_cut_groups.rs` with 6 passed / 0 failed.

## Follow-Up

Created focused follow-up task:

- `fix-pre-existing-10`: Fix pre-existing: slice reverse mirror validation fails

No new wave was started. The failure appears isolated and pre-existing, not a systemic new surface from `fix-pre-existing-8`.
