use celox::Simulator;

fn main() {
    let sorter_item = include_str!("../../../repro-celox-slow/src/sorter_item.veryl");
    let dist_entry = include_str!("../../../repro-celox-slow/src/dist_entry.veryl");
    let linear_sorter = include_str!("../../../repro-celox-slow/src/linear_sorter.veryl");
    let linear_sorter_pull = include_str!("../../../repro-celox-slow/src/linear_sorter_pull.veryl");
    let min_reduction_tree = include_str!("../../../repro-celox-slow/src/min_reduction_tree.veryl");
    let sorter_tree = include_str!("../../../repro-celox-slow/src/sorter_tree.veryl");

    let code = format!(
        "{sorter_item}\n{dist_entry}\n{linear_sorter}\n{linear_sorter_pull}\n{min_reduction_tree}\n{sorter_tree}"
    );

    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    eprintln!("=== Building SorterTreeDistEntry N={n} ===");
    let start = std::time::Instant::now();
    let _sim = Simulator::builder(&code, "SorterTreeDistEntry")
        .param("N", n)
        .param("LEAF_DEPTH", 8)
        .param("OUT_DEPTH", 16)
        .build()
        .unwrap();
    eprintln!("=== Total: {:?} ===", start.elapsed());
}
