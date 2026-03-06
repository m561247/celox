/// Regression tests for sorter tree compilation scaling.
///
/// The SorterTreeDistEntry design creates deeply nested mux trees from
/// MinReductionTree's binary merger structure.  Before the select-based mux
/// lowering fix, branch-based mux lowering created 3 blocks per mux,
/// causing exponential block count growth (N=16 took 24 minutes).
///
/// These tests verify that compilation time scales roughly linearly with N.
use celox::SimulatorBuilder;
use std::time::Instant;

fn load_sorter_sources() -> String {
    [
        include_str!("fixtures/sorter_tree/sorter_item.veryl"),
        include_str!("fixtures/sorter_tree/dist_entry.veryl"),
        include_str!("fixtures/sorter_tree/min_reduction_tree.veryl"),
        include_str!("fixtures/sorter_tree/linear_sorter_pull.veryl"),
        include_str!("fixtures/sorter_tree/linear_sorter.veryl"),
        include_str!("fixtures/sorter_tree/sorter_tree.veryl"),
    ]
    .join("\n")
}

fn build_sorter(n: u64) -> std::time::Duration {
    let code = load_sorter_sources();
    let start = Instant::now();
    SimulatorBuilder::new(&code, "SorterTreeDistEntry")
        .param("N", n)
        .param("LEAF_DEPTH", 4)
        .param("OUT_DEPTH", 16)
        .build()
        .unwrap();
    start.elapsed()
}

/// Baseline: N=4 compiles successfully.
#[test]
fn sorter_tree_n4_compiles() {
    let elapsed = build_sorter(4);
    eprintln!("SorterTreeDistEntry N=4: {elapsed:?}");
    assert!(
        elapsed.as_secs() < 30,
        "N=4 took {elapsed:?}, expected < 30s"
    );
}

/// N=16 compiles in reasonable time.
/// Before the fix this took ~24 minutes; now it should be a few seconds.
#[test]
fn sorter_tree_n16_compiles() {
    let elapsed = build_sorter(16);
    eprintln!("SorterTreeDistEntry N=16: {elapsed:?}");
    assert!(
        elapsed.as_secs() < 60,
        "N=16 took {elapsed:?}, expected < 60s"
    );
}

/// Scaling regression: N=8 should take at most 4x longer than N=4.
/// Linear scaling gives ~2x; exponential would give >100x.
#[test]
fn sorter_tree_scaling_n4_n8() {
    let t4 = build_sorter(4);
    let t8 = build_sorter(8);
    let ratio = t8.as_secs_f64() / t4.as_secs_f64();
    eprintln!("N=4: {t4:?}, N=8: {t8:?}, ratio: {ratio:.2}x");
    assert!(
        ratio < 4.0,
        "N=8/N=4 ratio is {ratio:.2}x, expected < 4.0x (linear scaling)"
    );
}
