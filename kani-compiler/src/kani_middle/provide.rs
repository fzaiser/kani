// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains an interface for setting custom query implementations
//! to run during code generation. For example, this can be used to hook up
//! custom MIR transformations.

use crate::kani_middle::reachability::{collect_reachable_items, filter_crate_items};
use crate::kani_middle::stubbing;
use kani_queries::{QueryDb, UserInput};
use rustc_hir::def_id::DefId;
use rustc_interface;
use rustc_middle::ty::query::query_stored::collect_and_partition_mono_items;
use rustc_middle::{
    mir::Body,
    ty::{query::ExternProviders, query::Providers, TyCtxt},
};

/// Sets up rustc's query mechanism to apply Kani's custom queries to code from
/// the present crate.
pub fn provide(providers: &mut Providers, queries: &QueryDb) {
    providers.optimized_mir = run_mir_passes::<false>;
    if queries.get_stubbing_enabled() {
        providers.collect_and_partition_mono_items = collect_and_partition_mono_items;
    }
}

/// Sets up rustc's query mechanism to apply Kani's custom queries to code from
/// external crates.
pub fn provide_extern(providers: &mut ExternProviders) {
    providers.optimized_mir = run_mir_passes::<true>;
}

/// Returns the optimized code for the function associated with `def_id` by
/// running rustc's optimization passes followed by Kani-specific passes.
fn run_mir_passes<'tcx, const EXTERN: bool>(tcx: TyCtxt<'tcx>, def_id: DefId) -> &Body<'tcx> {
    tracing::debug!(?def_id, "Run rustc transformation passes");
    let optimized_mir = if EXTERN {
        rustc_interface::DEFAULT_EXTERN_QUERY_PROVIDERS.optimized_mir
    } else {
        rustc_interface::DEFAULT_QUERY_PROVIDERS.optimized_mir
    };
    let body = optimized_mir(tcx, def_id);

    run_kani_mir_passes(tcx, def_id, body)
}

/// Returns the optimized code for the function associated with `def_id` by
/// running Kani-specific passes. The argument `body` should be the optimized
/// code rustc generates for this function.
fn run_kani_mir_passes<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    body: &'tcx Body<'tcx>,
) -> &'tcx Body<'tcx> {
    tracing::debug!(?def_id, "Run Kani transformation passes");
    stubbing::transform(tcx, def_id, body)
}

/// Runs a reachability analysis before running the default
/// `collect_and_partition_mono_items` query. The reachability analysis finds
/// trait mismatches introduced by stubbing and performs a graceful exit in
/// these cases. Left to its own devices, the default query panics.
/// This is an issue when compiling a library, since the crate metadata is
/// generated (using this query) before code generation begins (which is
/// when we normally run the reachability analysis).
fn collect_and_partition_mono_items(tcx: TyCtxt, key: ()) -> collect_and_partition_mono_items {
    let entry_fn = tcx.entry_fn(()).map(|(id, _)| id);
    let local_reachable = filter_crate_items(tcx, |_, def_id| {
        tcx.is_reachable_non_generic(def_id) || entry_fn == Some(def_id)
    });
    // We do not actually need the value returned here.
    collect_reachable_items(tcx, &local_reachable);
    (rustc_interface::DEFAULT_QUERY_PROVIDERS.collect_and_partition_mono_items)(tcx, key)
}
