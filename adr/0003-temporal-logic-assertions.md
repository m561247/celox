# ADR-0003: Temporal Logic Assertions

- **Date**: 2026-03-09
- **Status**: proposed

## Context

RTL 検証においてプロパティベースの検証（時相論理アサーション）は、コーナーケースの検出やプロトコルの正しさの保証に有効である。業界標準の SVA (SystemVerilog Assertions) は強力だが、仕様が膨大で、テストパターン生成と性質記述を同一言語内に抱え込んでいるため複雑になっている。

Celox ではテストパターン（stimulus）を既に TS/Rust で記述できる。アサーション記述を独立した DSL として設計する案も検討したが、以下の理由から TS/Rust API として提供する方が適切と判断した:

- テストパターンとアサーションが同一ファイルに共存でき、文脈が明確
- DSL パーサの実装・メンテナンスが不要
- TS の型システムによる信号名の補完・型チェックが活用できる
- 条件付きアサーションやパラメタライズが TS/Rust の制御構文でそのまま書ける

### なぜ STL か

当初 LTL (Linear Temporal Logic) をベースとする案を検討したが、RTL 検証では不十分と判断した:

- **信号は多ビット値**: LTL は命題（bool）レベルで動作するため、`count <= 8` のような信号値への述語を第一級に扱えない。毎回 bool 化のラッパーが必要になる。
- **時間区間は必須**: `eventually` にはバウンドが必要（無制限待ちは無意味）。STL の `F[a,b]` は時間区間を自然に表現できる。
- **定量的意味論**: STL の robustness semantics は「どれだけ余裕を持って性質を満たしているか」を数値で返す。デバッグ・回帰テストの感度分析に有用。
- **量化**: トランザクション ID やレジスタインデックスに対する全称・存在量化が RTL 検証では頻出する。

STL は LTL の上位互換であり、どうせ自前で構築するなら最初から STL ベースにする方が合理的である。

### 既存ライブラリ調査

TS/Rust の既存ライブラリを調査したが、推奨できるものがなかった。

**Rust:**

| ライブラリ | 概要 | 評価 |
|---|---|---|
| R2U2 (NASA) | MLTL ストリームベースモニタ。no_std Rust port あり | 組込み航空宇宙向け。アーキテクチャは参考になるが直接利用は困難 |
| telo | LTL 安全性モニタ→オートマトン | 成熟度不明。実質的に利用不可 |
| mcltl-rs | LTL モデルチェッカー | オフライン全状態探索用。ランタイムモニタではない |

**TypeScript:**

| ライブラリ | 概要 | 評価 |
|---|---|---|
| ltljs | `always`/`eventually` のみ実装 | トイレベル。演算子不足 |
| fast-check | プロパティベーステスト (QuickCheck 系) | 時相演算子なし |

**他言語 (アルゴリズムの参考):**

| ライブラリ | 言語 | 概要 |
|---|---|---|
| RTAMT | Python | STL ランタイムモニタ。オンライン/オフライン対応。定量的意味論サポート |
| LOLA | Haskell/native | 同期ストリーム RV。HDL シミュレーションと概念的に最も近い |

Rust/TS とも時相論理のランタイムモニタリングは空白地帯であり、自前実装が必要と判断した。RTAMT のアルゴリズムと LOLA のストリームモデルを参考にする。

### 検証スタックにおける位置づけ

時相論理アサーションは RTL 検証に必要だが十分ではない。検証スタック全体における位置づけを明確にする:

```
Constrained Random Stimulus  ← TS/Rust で既に可能
Immediate Assertions         ← 単純な if + assert (trivial)
Temporal Assertions (STL)    ← 本 ADR のスコープ
Scoreboard / Reference Model ← TS/Rust でユーザーが実装
Functional Coverage          ← 将来検討 (別 ADR)
```

本 ADR は Temporal Assertions 層のみを対象とする。

## Decision

STL (Signal Temporal Logic) ベースの時相論理アサーション機能を、量化付きで Rust コア + NAPI 経由の TS API として実装する。DSL は導入しない。

### 時相演算子

STL を基本とし、SVA のシーケンス演算子を簡略化して取り込む。

| 演算子 | 意味 | 形式的表記 |
|---|---|---|
| `always(p)` / `always([a,b], p)` | 常に（区間内で常に）p が成立 | G p / G[a,b] p |
| `eventually(p)` / `eventually([a,b], p)` | いつか（区間内で）p が成立 | F p / F[a,b] p |
| `next(p)` | 次のサイクルで p が成立 | X p |
| `until(p, q)` / `until([a,b], p, q)` | q が成立するまで p が成立 | p U q / p U[a,b] q |
| `when(p, q)` | p ならば q (含意) | p → q |
| `not(p)` | 否定 | ¬p |
| `and(p, q)` / `or(p, q)` | 論理結合 | p ∧ q / p ∨ q |
| `sequence(a, b, c)` | 連続サイクルで順に成立 | — (SVA 由来) |

### 信号述語

STL の核心。信号値に対する比較・算術を第一級で扱う。

| 述語 | 意味 |
|---|---|
| `sig(s).eq(v)` | s == v |
| `sig(s).neq(v)` | s != v |
| `sig(s).gt(v)` / `geq(v)` | s > v / s >= v |
| `sig(s).lt(v)` / `leq(v)` | s < v / s <= v |
| `sig(s).unchanged()` | s が前サイクルと同値 |
| `sig(s).changed()` | s が前サイクルから変化 |
| `sig(s).bits(hi, lo)` | ビットスライス |

### 量化

トランザクション ID・レジスタインデックス等に対する量化を第一級サポートする。

| 演算子 | 意味 |
|---|---|
| `forAll(domain, (v) => prop)` | ドメイン内のすべての v で prop が成立 |
| `exists(domain, (v) => prop)` | ドメイン内のある v で prop が成立 |

ドメインは `range(a, b)` (整数範囲) を初期サポートし、将来的に列挙型等に拡張可能。

量化を第一級にする利点:
- モニタの内部最適化（量化変数ごとのモニタ状態を共有構造で管理）
- 違反レポートに具体的な束縛値を含められる（「id=3, cycle=42 で違反」）
- 意図の明確化（ループで回すのと意味的に異なる）

### API 設計

#### TypeScript

```typescript
const sim = await Simulation.create(Top);
const dut = sim.dut;

// プロトコル: req が立ったら 1〜5 サイクル以内に ack
sim.assert("req_ack",
  always(when(sig(dut.req).eq(1), eventually([1, 5], sig(dut.ack).eq(1))))
);

// FIFO 深さ上限
sim.assert("fifo_depth", always(sig(dut.count).leq(8)));

// データ安定性: valid 中は data が変化しない
sim.assert("data_stable",
  always(when(sig(dut.valid).eq(1), sig(dut.data).unchanged()))
);

// 量化: 全トランザクション ID で応答が一致
sim.assert("id_match",
  forAll(range(0, 16), (id) =>
    always(when(
      sig(dut.req_id).eq(id).and(sig(dut.req).eq(1)),
      eventually([1, 10], sig(dut.resp_id).eq(id))
    ))
  )
);

// テストパターンは通常の TS で記述
for (let i = 0; i < 100; i++) {
    dut.req = randomBit();
    sim.step();
}

const results = sim.assertionResults();
```

#### Rust

```rust
let mut sim = Simulation::new(simulator);

sim.assert_prop("req_ack",
    always(when(sig("req").eq(1), eventually([1, 5], sig("ack").eq(1))))
);

sim.assert_prop("fifo_depth", always(sig("count").leq(8)));

sim.assert_prop("id_match",
    for_all(0..16, |id|
        always(when(
            sig("req_id").eq(id).and(sig("req").eq(1)),
            eventually([1, 10], sig("resp_id").eq(id))
        ))
    )
);

for _ in 0..100 {
    sim.modify("req", random_bit());
    sim.step();
}

let results = sim.assertion_results();
```

### アーキテクチャ

```
TS/Rust API (コンビネータ)
    ↓ ビルダーで組み立て
STL 式 (AST)
    ↓ コンパイル
モニタ (オートマトン / ストリーム評価器)
    ↓ 毎サイクル評価
違反検出 → レポート (サイクル番号, 信号値, 束縛変数)
```

1. **STL 式 → モニタ変換**: STL 式をオンラインモニタにコンパイルする。安全性プロパティは有限オートマトン、時間区間付き演算子はカウンタベースのモニタで実装。量化はモニタのインスタンスを展開するか、共有状態で管理する。
2. **毎サイクル評価**: `Simulation::step()` 内でモニタのステート遷移を評価する。JIT コンパイルされたシミュレーションループの後、モニタ評価を実行。
3. **違反レポート**: 違反検出時にサイクル番号・信号値・プロパティ名・量化変数の束縛値を記録。即座に panic するモードとシミュレーション終了後にまとめて報告するモードを選択可能。
4. **Robustness (オプション)**: STL の定量的意味論で robustness 値を計算。正の値は余裕あり、負の値は違反、0 は境界。初期実装では bool 判定のみとし、robustness は後から追加可能な設計にする。

### 実装場所

| コンポーネント | 場所 |
|---|---|
| STL AST・モニタ変換・評価エンジン | `crates/celox` |
| NAPI バインディング | `crates/celox-napi` |
| TS コンビネータ関数 (`always`, `sig`, `forAll`, ...) | `packages/celox` |
| TS 型生成への統合 | `crates/celox-ts-gen` |

### Non-goals

- 独立した DSL / パーサの導入
- CTL (分岐時相論理) のサポート
- Formal verification / bounded model checking（将来検討）
- 区間なし `eventually` の無制限待ち（バウンドなし活性プロパティ）
- Robustness semantics の初期実装（設計は考慮するが実装は後続）

## Consequences

- TS/Rust のテストコード内で STL ベースのプロパティ検証が可能になる
- 信号値に対する述語を第一級で扱えるため、RTL 検証で頻出するパターン（閾値チェック、データ一致、レイテンシ制約）が自然に記述できる
- 量化により、パラメタライズドな検証（ID ごと、レジスタごと）が簡潔に書ける
- DSL 不要のため導入コストが低く、既存のテストフローにそのまま統合できる
- 型付きの DUT オブジェクトと組み合わせることで、存在しない信号名を参照するアサーションをコンパイル時に検出できる（TS）
- モニタの毎サイクル評価はシミュレーション性能に影響するが、典型的なアサーション数（数十個程度）では誤差レベル
- Rust/TS 両エコシステムにおいて STL ランタイムモニタは空白地帯であり、Celox 発のライブラリとして独立公開する可能性もある

### Open Questions

- `sequence` でサイクル間ギャップ (`##[1:3]`) をどう表現するか — `delay([1,3], p)` のような演算子を追加するか、`sequence` のオプションにするか
- アサーション違反時のデフォルト動作（即 panic vs 収集して最後に報告）
- カバレッジ収集（アサーションの成功回数・到達回数）を初期実装に含めるか
- 量化のドメインを動的に変更可能にするか（シミュレーション中にドメインが変わるケース）
- Robustness semantics の追加タイミングと API 設計
