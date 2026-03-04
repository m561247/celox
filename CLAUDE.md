# CLAUDE.md

## Project Overview

Celox is a JIT simulator for Veryl HDL. It compiles Veryl designs with Cranelift for high-speed simulation. Future plans include SystemVerilog/Verilog support.

## Build Commands

```bash
cargo build              # Build all crates
cargo test               # Run all tests
cargo test -p celox      # Run tests for the core crate only
cargo run -p celox-ts-gen -- --help  # TypeScript type generator CLI
```

### Snapshot Tests

```bash
cargo insta test         # Run snapshot tests
cargo insta accept       # Accept snapshot changes
```

## Workspace Structure

| Crate / Package | Description |
|---|---|
| `crates/celox` | Core simulator (IR, JIT compilation, runtime) |
| `crates/celox-macros` | Procedural macros |
| `crates/celox-napi` | N-API bindings for Node.js |
| `crates/celox-ts-gen` | CLI tool for TypeScript type generation |
| `packages/celox` | TypeScript runtime package |
| `packages/vite-plugin` | Vite plugin |

## Veryl Submodule

The `deps/veryl/` directory contains a fork of Veryl (`tignear/veryl`). The workspace depends on `veryl-analyzer`, `veryl-parser`, `veryl-metadata`, and `veryl-path` from this submodule.

- `default-features = false` is set on `veryl-parser` to suppress parser regeneration during builds.
- After updating the submodule, run `cargo test` to verify compatibility.

### Analyzer API

The Veryl analyzer pass functions (`analyze_pass1`, `analyze_post_pass1`, `analyze_pass2`, `analyze_post_pass2`) return `Vec<AnalyzerError>`. All 4 passes must be called and their errors checked. `SimulatorError::Analyzer` wraps these errors.

### Writing Veryl in Tests

The analyzer enforces strict checks. When writing Veryl source in integration tests:

- **Clock domain annotations**: Multi-clock designs require `'a`/`'b` (or `'_` for single-clock) on all ports and vars. Cross-domain access needs `unsafe (cdc) { ... }`.
- **Logical operators on multi-bit**: `a && b` / `a || b` / `!a` are rejected for operands wider than 1 bit. Use reduction: `(|a) && (|b)`, `!(|a)`.
- **logic → bit assignment**: Requires explicit cast `as u8`.
- **SV keywords as identifiers**: Forbidden (e.g. `reg`). Use alternatives like `r_val`.
- **Clock from logic**: A `var` of type `logic` cannot be used as a clock. Use an external `clock` input or `let gated: '_ clock = clk_input & en;` (first operand must be clock-typed).
- **Self-referential assign**: `assign v = f(v);` is rejected as `UnassignVariable`. Use `always_comb` with `if`/`else` branches if possible, or redesign the circuit.

## Dead Store Elimination (DSE)

DSE removes stores that are never read, improving JIT performance. Controlled via `DeadStorePolicy` (`crates/celox/src/simulator/builder.rs`):

| Policy | Behavior |
|---|---|
| `Off` (default) | No elimination — all stores preserved |
| `PreserveTopPorts` | Eliminate except top-module port stores |
| `PreserveAllPorts` | Eliminate except port stores of **all** instances |

### Rust API

```rust
Simulator::builder(code, "Top")
    .dead_store_policy(DeadStorePolicy::PreserveAllPorts)
    .build()
```

### NAPI / TypeScript

- **NAPI option**: `dead_store_policy: "off" | "preserve_top_ports" | "preserve_all_ports"` (`crates/celox-napi/src/lib.rs`)
- **TS option**: `deadStorePolicy: "off" | "preserveTopPorts" | "preserveAllPorts"` in `SimulatorOptions` (`packages/celox/src/types.ts`)
- **Mapping**: `buildNapiOpts()` in `napi-helpers.ts` converts camelCase → snake_case

### Hierarchy Filtering

DSE-enabled DUTs must not expose sub-instance signals that were eliminated. `filterHierarchyForDse()` (`napi-helpers.ts`) adjusts the hierarchy passed to `createDut()`:
- `preserveTopPorts` → `children = {}` (sub-instances stripped)
- `preserveAllPorts` → hierarchy intact (all instance ports survive)

### Vite Plugin `?dse=` Query

Import with `?dse=preserveAllPorts` or `?dse=preserveTopPorts` to bake `defaultOptions.deadStorePolicy` into the `ModuleDefinition`. `?dse` (no value) defaults to `preserveAllPorts`.

```ts
import { Adder } from './Adder.veryl?dse=preserveAllPorts';
```

`Simulator.create()` / `Simulation.create()` merge `module.defaultOptions` with caller-supplied options (caller wins).

## Rust Edition

This project uses Rust **edition 2024**.

## GitHub Project

- **URL**: https://github.com/orgs/celox-sim/projects/1
- **Owner**: celox-sim
- **Project number**: 1

タスク管理・ロードマップ管理に使用する。Issue を作成・更新する際は Project に紐づけること。

### フィールド

| フィールド | 種類 | 値 |
|---|---|---|
| Status | Single Select | `Backlog` / `In Progress` / `In Review` / `Done` |
| Priority | Single Select | `P0 Critical` / `P1 High` / `P2 Medium` / `P3 Low` |
| Milestone | 標準 | フェーズ・リリース単位で管理 |

### Issue 操作例

```bash
# Issue を Project に追加
gh project item-add 1 --owner celox-sim --url <issue-url>

# フィールド更新
gh project item-edit --project-id PVT_kwDOD8WmI84BQmif --id <item-id> --field-id <field-id> --single-select-option-id <option-id>
```
