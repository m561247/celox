# パラメータオーバーライド

Celox では、Veryl ソースを変更せずにシミュレーション時にトップレベルモジュールのパラメータを上書きできます。たとえば、データ幅を変えながら同じテストスイートを実行するといった、複数の設定を横断したテストに役立ちます。

## 基本的な使い方

オプションオブジェクトに `parameters` 配列を渡します。各エントリは `name` と `value` を持ちます：

```typescript
import { Simulator } from "@celox-sim/celox";

const SOURCE = `
module ParamWidth #(
    param WIDTH: u32 = 8,
) (
    a: input  logic<WIDTH>,
    b: output logic<WIDTH>,
) {
    always_comb {
        b = a;
    }
}
`;

// デフォルト: WIDTH = 8
const sim8 = Simulator.fromSource(SOURCE, "ParamWidth");

// オーバーライド: WIDTH = 16
const sim16 = Simulator.fromSource(SOURCE, "ParamWidth", {
  parameters: [{ name: "WIDTH", value: 16 }],
});

sim16.dut.a = 0xABCDn;
expect(sim16.dut.b).toBe(0xABCDn); // 16 ビット値がそのまま保持される

sim16.dispose();
```

`Simulation` でも同様に使えます：

```typescript
const sim = Simulation.fromSource(SOURCE, "ParamWidth", {
  parameters: [{ name: "WIDTH", value: 16 }],
});
```

## 複数パラメータのオーバーライド

必要な数だけエントリを並べます：

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  parameters: [
    { name: "WIDTH", value: 32 },
    { name: "DEPTH", value: 512 },
  ],
});
```

## 動作の仕組み

オーバーライド値はコンパイル前に Veryl アナライザへ注入されます。子モジュールのインスタンス化を含む、すべての下流のエラボレーションが上書き後の定数を参照します。DUT のポートレイアウトは実際のコンパイル結果から導出されるため、`ModuleDefinition` 型が別のパラメータ値で生成されていた場合でも、オーバーライド後のビット幅が正しく反映されます。

## パラメータの値

パラメータの値は整数（`number` または `bigint`）でなければなりません。浮動小数点数や文字列を渡すことはサポートされていません。

## 関連資料

- [テストの書き方](./writing-tests.md) -- イベントベースとタイムベースのシミュレーションパターン。
- [API リファレンス](/api/) -- TypeScript API の完全なドキュメント。
