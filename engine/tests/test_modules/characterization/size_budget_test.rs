use core::mem::size_of;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::ability_queue::AbilityQueueEntry;

/// Hot-struct size invariant.
///
/// This file used to carry a table of per-struct size ceilings alongside this
/// test. It asserted nothing — the loop body was empty and the table was never
/// read — and it was removed rather than restored, because:
///   * the numbers were guesses, not measurements, and had drifted (the
///     `GameModifiers` row claimed 1200 B against a struct well past that);
///   * per-struct ceilings are not a memory constraint. The real one is total
///     heap, and nothing here measured it;
///   * it had already been demoted from a hard failure to diagnostic-only, for
///     the recorded reason that it fought legitimate feature work.
///
/// `AbilityResolver` was the row that tripped when the assertion was briefly
/// put back: 2392 B against a 2200 B "ceiling". That is not a real finding. The
/// resolver is boxed inside the queue entry — which is what the surviving
/// assertion below guarantees — so its size is only paid while a resolution is
/// actually in flight, and it is paid on the heap, once, a handful of times.
///
/// What remains is the one invariant that is architectural rather than
/// numeric, and that should fail: an idle queue entry must not carry a copy of
/// the resolver. That is load-bearing for queue memory, and a regression here
/// is invisible to every behaviour test.
#[test]
fn queue_entry_does_not_inline_the_resolver() {
    let entry = size_of::<AbilityQueueEntry>();
    let resolver = size_of::<AbilityResolver>();

    assert!(
        entry < resolver,
        "AbilityQueueEntry ({entry} B) is not smaller than an inlined \
         AbilityResolver ({resolver} B) — the resolver must be boxed so idle \
         queue entries pay pointer-sized cost rather than the full struct."
    );
}
