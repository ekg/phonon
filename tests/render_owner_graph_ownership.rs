//! Regression guard for the **render-owner** graph-swap model
//! (`verify-render-owner-swap`, design `docs/audits/design-render-owner-swap-2026-07.md`).
//!
//! The C1-root data race lived in `unsafe impl Sync for UnifiedSignalGraph`
//! (design §2.1, site `src/unified_graph.rs:5327-5328`): it let two threads
//! alias a `&UnifiedSignalGraph` and mutate its interior `RefCell`s
//! concurrently. The render-owner migration transfers graph *ownership* through
//! an SPSC swap channel (`src/render_swap.rs`) instead of sharing it, so the
//! graph must be:
//!
//! - **`Send`**  — it is *moved* from the control thread to the render thread;
//! - **NOT `Sync`** — it is never `&`-shared across threads; re-adding
//!   `unsafe impl Sync` would reintroduce the exact data race the migration
//!   deleted.
//!
//! Design §7 requires machine-confirming that "no `unsafe impl Sync` remains".
//! This test is that confirmation, locked in permanently.

use phonon::unified_graph::UnifiedSignalGraph;
use std::marker::PhantomData;
use std::rc::Rc;

/// Autoref specialization probe: reports whether `T: Sync` at compile time
/// without requiring the bound (so it compiles for `!Sync` types too).
///
/// The trait method is the fallback (`false`); the inherent method — valid only
/// when `T: Sync` — shadows it (`true`). Method resolution prefers the inherent
/// method when its `where` bound holds, and silently falls back to the trait
/// method otherwise.
struct SyncProbe<T>(PhantomData<T>);

trait SyncFallback {
    fn probe_is_sync(&self) -> bool {
        false
    }
}
impl<T> SyncFallback for SyncProbe<T> {}

impl<T: Sync> SyncProbe<T> {
    fn probe_is_sync(&self) -> bool {
        true
    }
}

// NOTE: the probe MUST be invoked with the concrete type visible at the call
// site. Wrapping it in a generic `fn is_sync<T>()` would defeat the trick — the
// inherent method's `T: Sync` bound is unknown inside a generic body, so
// resolution would always pick the fallback (`false`) regardless of the type.

fn assert_send<T: Send>() {}

#[test]
fn render_owner_graph_is_send_but_not_sync() {
    // First, prove the probe actually discriminates — otherwise a broken probe
    // that always returned `false` would make the `!Sync` assertion vacuous.
    assert!(
        SyncProbe::<i32>(PhantomData).probe_is_sync(),
        "probe broken: i32 is Sync but probe said !Sync"
    );
    assert!(
        !SyncProbe::<Rc<i32>>(PhantomData).probe_is_sync(),
        "probe broken: Rc<i32> is !Sync but probe said Sync"
    );

    // The graph must stay `Send`: it is moved control-thread -> render-thread
    // through the render_swap channel. (Compile-time bound check.)
    assert_send::<UnifiedSignalGraph>();

    // The C1-root invariant: the graph must NOT be `Sync`. If someone re-adds
    // `unsafe impl Sync for UnifiedSignalGraph`, this flips to `true` and fails.
    assert!(
        !SyncProbe::<UnifiedSignalGraph>(PhantomData).probe_is_sync(),
        "UnifiedSignalGraph is Sync — the C1-root `unsafe impl Sync` \
         (unified_graph.rs:5327-5328) has been reintroduced. The render-owner \
         model transfers graph ownership, never shares it: the graph must be \
         Send but NOT Sync (design-render-owner-swap-2026-07.md §2.1/§4/§7)."
    );
}
