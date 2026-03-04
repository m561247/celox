# 組み合わせループ

Celox は `always_comb` ブロックの依存関係を静的に解析してトポロジカル順序でスケジューリングします。依存グラフにサイクルを検出するとコンパイルが `CombinationalLoop` エラーで失敗します。

このエラーが発生する状況は 2 種類あり、それぞれ異なる対処が必要です。

## 見かけ上のループ（`falseLoops`）

静的な依存グラフ上にサイクルが現れるものの、実行時には決してループしない場合です。最もよくある原因は、2 つのブランチが互いに逆側のパスに依存しているマルチプレクサです：

```veryl
module Top (
    sel: input  logic,
    i:   input  logic<2>,
    o:   output logic<2>,
) {
    var v: logic<2>;
    always_comb {
        if sel {
            v[0] = v[1];  // v[1] を読む
            v[1] = i[1];
        } else {
            v[0] = i[0];
            v[1] = v[0];  // v[0] を読む
        }
    }
    assign o = v;
}
```

`v[0]` と `v[1]` は互いに依存しているように見えますが、`v[0]→v[1]` は `sel=1` のとき、`v[1]→v[0]` は `sel=0` のときだけ起き、同時にループすることはありません。

そのままではコンパイルに失敗します。`falseLoops` でこのサイクルが安全であることを宣言します：

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  falseLoops: [
    { from: "v", to: "v" },
  ],
});
```

`from` と `to` にはサイクルに関係するシグナル名を指定します。Celox は SCC ブロックをサイクルの構造的な深さから算出した回数だけ実行して、実行順序によらずすべての値が正しく伝搬するようにします。

## 本物のフィードバックループ（`trueLoops`）

本物の組み合わせフィードバックパスです。出力が入力に戻り、安定状態（固定点）に達するまで反復が必要です。

```veryl
module Top (i: input logic, o: output logic<4>) {
    var v: logic<4>;
    always_comb {
        if i {
            v = (v << 1) | 4'd1;
        } else {
            v = 4'd0;
        }
    }
    assign o = v;
}
```

::: info
Veryl の analyzer は `assign v = {v[2:0], i}` のような自己参照的な代入を `UnassignVariable` エラーで拒否します。true loop を記述するには `always_comb` 内で `if`/`else` を使い、少なくとも一方のブランチで自己参照なしに変数を代入してください。これにより analyzer のデータフロー検査を通過しつつ、組み合わせフィードバックパスを構成できます。
:::

デフォルトでは Celox はごく少ない反復回数しか許可しません（構造的なループ深さを基準にした上限値）。回路の収束にさらに多くの反復が必要な場合は `trueLoops` に十分な `maxIter` を指定します：

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  trueLoops: [
    { from: "v", to: "v", maxIter: 10 },
  ],
});
```

`maxIter` 回以内に収束しない場合、シミュレーションは実行時に `DetectedTrueLoop` エラーをスローします。

::: warning
永遠に収束しないループ（例: `assign y = ~y`）は `maxIter` の値によらず常に実行時エラーになります。これは組み合わせ発振であり、有効なハードウェア状態ではありません。
:::

## シグナルパスの書き方

`from` と `to` にはシグナルパス文字列を指定します：

| パターン | 意味 |
|---------|------|
| `"v"` | トップレベルの変数 `v` |
| `"u_sub:i_data"` | 子インスタンス `u_sub` のポート `i_data` |
| `"u_a.u_b:x"` | `u_a` 内の `u_b` インスタンスのポート `x` |

## `falseLoops` と `trueLoops` の使い分け

| | `falseLoops` | `trueLoops` |
|---|---|---|
| 実行時に実際にループするか | しない（パスが排他的） | する（出力が入力にフィードバック） |
| 反復の動作 | 構造的な深さ N 回のスタティックアンロール | 収束または `maxIter` まで反復 |
| 実行時エラーの可能性 | なし | 収束しない場合にあり |

## 関連資料

- [組み合わせ回路解析](/internals/combinational-analysis) -- 依存グラフの構築とスケジューリングの詳細。
- [テストの書き方](./writing-tests.md) -- シミュレータオプションの概要。
