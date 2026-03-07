# ADR-0001: Value Change Callback / GPI / VPI

- **Date**: 2026-03-07
- **Status**: proposed

## Context

Simulation に値変化コールバック (VCC) がなく、cocotb 連携や JS/TS からのリアクティブな信号監視ができない。

cocotb は GPI (Generic Programming Interface) を通じてシミュレータと通信する。Celox をcocotb バックエンドとして使うには GPI 互換レイヤーが必要。また JS/TS テストベンチでも同等の機能が求められる。

## Decision

4 段階のフェーズで VCC → JS/TS API → GPI → VPI を積み上げる。

### Phase 1: Value Change Callback in Simulation

Snapshot Compare 方式（JIT ノータッチ）で実装する。

- `step()` 前後で watched signal のバイト列を比較し変化を検出
- JIT コード変更不要
- watched signal 数は cocotb 典型で 10–20 個程度、スケジューラ処理に比べてオーバーヘッドは誤差

#### API (Rust)

```rust
pub type WatchId = u32;

impl Simulation {
    fn watch(&mut self, signal: SignalRef, callback: WatchCallback) -> WatchId;
    fn unwatch(&mut self, id: WatchId);
}
```

#### Implementation

- `step()` 前: watched signal の値を snapshot (`Vec<u8>` バッファ)
- `step()` 実行
- `step()` 後: snapshot と現在値を比較、変化していたら callback 発火
- バッファは watch リスト変更時のみ再確保

### Phase 2: JS/TS API (NAPI)

Simulation の VCC を NAPI 経由で JS に expose する。

```typescript
const watchId = simulation.watch(signal, (newValue) => {
  console.log(`signal changed: ${newValue}`);
});
simulation.unwatch(watchId);

// cocotb-like sugar
await simulation.risingEdge(clockSignal);
await simulation.valueChange(dataSignal);
```

JS callback は `step()` の同期コンテキスト内で呼ばれる（napi-rs の threadsafe function は不要、single-threaded）。

### Phase 3: cocotb GPI Backend

cocotb の `GpiImplInterface` を C ABI で実装する。

| GPI operation | Celox mapping |
|---|---|
| `gpi_get_root_handle` | `Simulation::named_hierarchy()` |
| `gpi_iterate` / `gpi_next` | hierarchy traversal |
| `gpi_get_handle_by_name` | `Simulation::signal()` |
| `gpi_get_signal_val_*` | `Simulation::get()` |
| `gpi_set_signal_val_*` | `Simulation::modify()` |
| `gpi_register_value_change_callback` | `Simulation::watch()` |
| `gpi_register_timed_callback` | `Simulation::schedule()` |
| `gpi_deregister_callback` | `Simulation::unwatch()` |

Build:
- Rust crate (`celox-gpi`) exporting `extern "C"` functions
- cocotb discovers it via `GPI_EXTRA` / `MODULE` env vars
- Shared library (.so) that cocotb loads at startup

### Phase 4: VPI Compatibility Layer

GPI の上に thin VPI wrapper を載せる。

- `vpi_get_value` / `vpi_put_value`
- `vpi_register_cb` (cbValueChange, cbAfterDelay, cbNextSimTime)
- `vpi_iterate` / `vpi_scan`
- `vpi_handle_by_name`

初期化は VPI 標準の `vlog_startup_routines` ではなく Celox 独自のロード機構で良い。

### Non-goals

- JIT コードへの dirty bit 埋め込み（不要）
- VPI 完全準拠（cocotb + 基本操作のみ）
- マルチスレッド対応（single-threaded で十分）

## Consequences

- cocotb の Python テストベンチがそのまま Celox 上で動くようになる
- JS/TS テストベンチでリアクティブな信号監視が可能になる
- Snapshot Compare はシンプルだが、watched signal が大量（数百以上）になると性能懸念あり（その場合は JIT 内 dirty bit を検討）

### Open Questions

- cocotb の GPI C ABI は安定しているか？バージョン間で互換性は？
- `cbReadWriteSynch` / `cbReadOnlySynch` の region scheduling をどこまでサポートするか
- Python ↔ Rust の FFI: PyO3 vs 生 C ABI
