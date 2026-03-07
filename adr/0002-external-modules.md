# ADR-0002: External Modules

- **Date**: 2026-03-07
- **Status**: proposed

## Context

Celox は現在 Veryl ソースコードのみをシミュレーション対象としている。全モジュールが Veryl で記述されている必要があり、外部 IP コア（メモリモデル、アナログモデル、プロプライエタリ IP のスタブ等）をシミュレーションに組み込む手段がない。

将来の SystemVerilog フロントエンド対応の布石としても、フロントエンド非依存なモジュール抽象が必要。

### 現状の制約

- `Component::SystemVerilog` は `ParserError::unsupported()` で拒否される
- 全モジュールが `Veryl Source → Analyzer → SIR → JIT` パイプラインを通る前提
- 外部/スタブモジュールの概念がない

## Decision

Rust trait で振る舞いを実装したモジュールを、Veryl 設計内で透過的にインスタンスできる仕組みを導入する。DLL/SO からの動的ロードもサポートする。

### Architecture

```
Veryl Source (stub module + design)
    │
    ▼
Veryl Analyzer ── ポート情報を正常に解析
    │
    ▼
Celox Parser
    ├── 通常モジュール → SimModule → SIR → JIT
    └── 外部モジュール → ExternalSimModule (ポートのみ、ロジックなし)
    │
    ▼
Flatten (hierarchy expansion)
    ├── 通常インスタンス: glue blocks + relocate
    └── 外部インスタンス: glue blocks のみ (内部ロジックなし)
    │
    ▼
JIT Compilation
    └── eval_comb 内で外部モジュールは call_indirect で呼出し
    │
    ▼
Runtime
    ├── JIT コード: 通常の組合せ/順序ロジック
    └── 外部モジュール: trait method 呼出し (eval / clock_edge)
```

### 1. Rust Trait API

Factory と Instance の 2 層構造:

```rust
/// 外部モジュールの定義。ポート宣言とインスタンス生成を担う。
pub trait ExternalModule: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn ports(&self) -> &[PortDef];
    fn instantiate(&self) -> Box<dyn ExternalModuleInstance>;
}

/// 各インスタンスの振る舞い。
pub trait ExternalModuleInstance: Send + 'static {
    fn eval(&mut self, io: &mut PortIO);
    fn on_rising_edge(&mut self, _clock: &str, _io: &mut PortIO) {}
    fn on_falling_edge(&mut self, _clock: &str, _io: &mut PortIO) {}
}
```

Port I/O は JIT メモリバッファへの直接アクセス:

```rust
pub struct PortIO<'a> {
    memory: &'a mut [u8],
    ports: &'a [PortSlot],
}

impl<'a> PortIO<'a> {
    pub fn get_u64(&self, name: &str) -> u64;
    pub fn get_bool(&self, name: &str) -> bool;
    pub fn get_wide(&self, name: &str) -> BigUint;
    pub fn set_u64(&mut self, name: &str, val: u64);
    pub fn set_bool(&mut self, name: &str, val: bool);
    pub fn set_wide(&mut self, name: &str, val: BigUint);
}
```

### 2. Veryl Stub

Analyzer 変更不要。stub をヘッダファイルとして使う:

```veryl
module Memory (
    clk:   input  '_ clock,
    addr:  input  logic<16>,
    wdata: input  logic<8>,
    we:    input  logic,
    rdata: output logic<8>,
) {}
```

- Analyzer は普通のモジュールとして処理する
- ポート型チェックが Veryl 側で完結する
- `inst` のポート接続構文がそのまま使える

### 3. Builder API

```rust
let sim = Simulator::builder(code, "Top")
    .external_module(MemoryModel::descriptor())       // 同一バイナリ
    .load_external_module("path/to/libcustom.so")     // DLL/SO
    .build()?;
```

Builder 内部処理:
1. `parse_ir()` で Veryl を解析
2. 外部モジュール名と `SimModule` 名をマッチング
3. マッチしたモジュールを `ExternalSimModule` に置換（ポート Variables 保持、ロジック空）
4. `flatten()` で外部インスタンスも通常通り `InstanceId` を割当て
5. JIT コンパイル時、外部モジュールの `eval` を組込み

### 4. Parser Integration

`Component::SystemVerilog` と stub（ロジック空の `Component::Module`）の両方を外部モジュール名でマッチング。マッチしたら `ExternalSimModule`（ポートのみ）として扱う。

### 5. JIT Integration (eval_comb)

Cranelift の外部関数呼出しで eval_comb 内から直接コールバック:

```
eval_comb JIT function:
    ... (通常の comb ロジック) ...
    ┌─ glue: parent → external module input ports (JIT)
    │  call external_module_eval(instance_ptr, memory_ptr)
    └─ glue: external module output ports → parent (JIT)
    ... (通常の comb ロジック 続き) ...
```

トポロジカルソートには合成 `LogicPath` で参加（保守的に全出力が全入力に依存）。

### 6. Clock Edge Handling

`tick(event)` 時:
1. `eval_comb()` — 外部モジュール `eval` 含む
2. 通常の FF apply
3. 外部モジュールの `on_rising_edge()` / `on_falling_edge()` 呼出し
4. `eval_comb()` — 再評価

クロックエッジ処理は JIT 外で実行。出力ポートへの反映は次の `eval_comb` で。

### 7. DLL/SO Loading

```rust
pub type RegisterFn = unsafe extern "C" fn(registry: &mut dyn ModuleRegistry);

pub trait ModuleRegistry {
    fn register(&mut self, module: Box<dyn ExternalModule>);
}
```

- `celox_register` 関数のシグネチャは C ABI (`extern "C"`)
- trait は Rust ABI — 同一 rustc バージョン制約
- 将来、完全な C ABI vtable (`abi_stable` 等) に移行する可能性あり
- `libloading` で動的ロード

### Implementation Phases

| Phase | Scope |
|-------|-------|
| 1 | 基盤: trait 定義、builder API、parser 統合、eval_comb 組込み（同一バイナリ、comb のみ） |
| 2 | 順序ロジック: `on_rising_edge` / `on_falling_edge` の tick() 統合 |
| 3 | DLL/SO ロード: `celox-module-abi` crate、`libloading` |
| 4 | 拡張: TS/JS コールバック (NAPI ラッパー)、proc macro、stub 自動生成 |

## Consequences

### Positive

- メモリモデル、IP スタブ等をシミュレーションに組み込める
- Veryl 側は `inst` 構文がそのまま使え、外部モジュールを意識しない
- DLL/SO で配布可能 — ソース非公開の IP モデルに対応
- TS/JS コールバックは Rust trait の NAPI ラッパーとして自然に拡張可能

### Negative

- eval_comb 内の外部関数呼出しが JIT 最適化の境界になる
- DLL は同一 rustc バージョン制約（v1）
- Veryl stub と trait の `ports()` の整合性を build 時に検証する必要がある

### Open Questions

1. **eval_comb 分割戦略**: JIT 関数分割 vs Cranelift `call_indirect`。後者が実装は楽だが最適化境界になる。
2. **4-state サポート**: 外部モジュールポートでの X/Z 値の扱い。v1 では 2-state のみで十分か。
3. **配列インスタンス**: `inst mem: Memory[4]` — `instantiate()` が 4 回呼ばれ独立した状態を持つ。
4. **パラメトリック外部モジュール**: `instantiate()` にパラメータを渡す仕組みが必要。
