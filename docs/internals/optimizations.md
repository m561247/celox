# Simulator Optimization Algorithms

Celox applies optimizations across multiple layers from compile time to runtime to accelerate simulation.

## 1. Logic Layer (SLT) Optimizations

These optimizations are applied during the stage where RTL logic expressions are analyzed.

### 1.1 Global Hash Consing
Expressions (`SLTNode`) with identical logical structure are deduplicated into shared instances across all modules and all `always_comb` blocks. This reduces memory usage and improves the efficiency of subsequent hoisting.

### 1.2 Topological Hoisting
Shared subexpressions referenced multiple times are moved forward in the instruction sequence so they are evaluated only once at the beginning of the simulation cycle. This significantly reduces redundant `Load` instructions and the number of operations.

## 2. Structural Layer (SIR) Optimizations

These are pass-based optimizations applied to the generated instruction sequence (SIR).

### 2.1 Load/Store Coalescing
-   **Load Coalescing**: Merges multiple `Load` operations to adjacent bit ranges at the same address into a single wider `Load`.
-   **Store Coalescing**: Combines writes to consecutive bit ranges at the same address using a `Concat` instruction, then executes them as a single `Store`.

### 2.2 Redundant Load Elimination (RLE / Forwarding)
Tracks values that have been loaded into registers or stored, and eliminates reloads to the same address by reusing the existing register value.

### 2.3 Commit Optimization
-   **Commit Sinking**: Pushes `Commit` instructions in merge blocks down into preceding `Store` instructions to combine them.
-   **Inline Forwarding**: Replaces generated `Commit` instructions with direct `Store` instructions where possible, eliminating unnecessary copies between buffers.

### 2.4 Dead Store Elimination
Detects and removes writes to the Working region that are never referenced.

### 2.5 Instruction Scheduling
Reorders instructions while preserving inter-instruction dependencies (RAW/WAR/WAW), taking into account processor execution ports and memory latency.

### 2.6 Tail-Call Splitting
Cranelift uses a 24-bit instruction index internally, limiting a single function to approximately 16M CLIF instructions. Large combinational designs (e.g., wide-bus arithmetic, many coalesced execution units) can exceed this limit.

When the estimated CLIF instruction count for `eval_comb` exceeds the threshold (currently 8M, a 50% safety margin), the optimizer splits it into a chain of smaller functions connected by Cranelift's `return_call` (tail-call) instruction, which avoids stack growth.

Three strategies are applied in order of increasing cost:

1.  **EU-boundary splitting**: Splits between execution units. Since `RegisterId`s are EU-scoped, no live registers need to be forwarded across the split boundary (zero overhead).
2.  **Intra-EU single-block splitting**: For a single-block EU that exceeds the threshold, splits at `Store` instruction boundaries. A dynamic programming pass minimizes the number of live registers that must be forwarded as tail-call arguments. A cost model (`cost_model.rs`) estimates per-instruction CLIF cost, calibrated against the actual translator (including quadratic costs for wide shifts, multiplication, and division).
3.  **Memory-spilled multi-block splitting**: For multi-block EUs (containing branches and loops), splits the CFG into chunks with a single-entry-point guarantee. Inter-chunk live registers are passed through a scratch memory region appended to the unified memory buffer, rather than as function arguments. Each chunk is compiled with signature `(mem_ptr) -> i64`, and cross-chunk edges emit spill stores followed by a tail-call.

This pass runs even when `optimize=false` to prevent compilation failures.

## 3. Execution Layer (Behavioral) Optimizations

These are dynamic optimizations applied in the simulator's execution loop.

### 3.1 Silent Edge Skipping
When events such as clock signals occur but the signal value has not changed, or the trigger condition (rising/falling edge) is not met, evaluation of dependent flip-flops and re-evaluation of associated combinational logic are skipped.

### 3.2 Multi-Phase Evaluation (Separation of Evaluation and Update)
When multiple events are triggered simultaneously, all evaluations are performed based on the current values in the Stable region, followed by a bulk update. This guarantees consistent simulation results independent of the order in which events occur.
