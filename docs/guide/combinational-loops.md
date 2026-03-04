# Combinational Loops

Celox performs static dependency analysis on `always_comb` blocks and schedules them in topological order. When it detects a cycle in the dependency graph, compilation fails with a `CombinationalLoop` error.

There are two distinct situations that produce this error, and each has its own fix.

## False Loops

A **false loop** is a cycle that appears in the static dependency graph but can never actually loop at runtime. The most common cause is a mux whose two branches each depend on the opposite path:

```veryl
module Top (
    sel: input  logic,
    i:   input  logic<2>,
    o:   output logic<2>,
) {
    var v: logic<2>;
    always_comb {
        if sel {
            v[0] = v[1];  // reads v[1]
            v[1] = i[1];
        } else {
            v[0] = i[0];
            v[1] = v[0];  // reads v[0]
        }
    }
    assign o = v;
}
```

`v[0]` and `v[1]` appear to depend on each other, but `v[0]→v[1]` only happens when `sel=1` and `v[1]→v[0]` only when `sel=0` — they never loop simultaneously.

Without intervention, this fails to compile. Use `falseLoops` to declare the cycle safe:

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  falseLoops: [
    { from: "v", to: "v" },
  ],
});
```

The `from` and `to` fields identify the signals involved in the cycle. Celox will execute the SCC block multiple times (the exact count is derived from the structural depth of the cycle) to ensure all values propagate correctly regardless of execution order.


## True Loops

A **true loop** is a genuine combinational feedback path — the output feeds back into the input and the circuit must iterate until it reaches a stable state (fixed point).

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
The Veryl analyzer rejects bare self-referential assignments such as `assign v = {v[2:0], i}` with an `UnassignVariable` error. To write a true loop, use `always_comb` with `if`/`else` branches so that at least one branch assigns the variable without reading it. This satisfies the analyzer's data-flow check while still creating a combinational feedback path.
:::

By default, Celox allows only a small number of iterations (based on the structural loop depth). If the circuit needs more iterations to converge, declare a `trueLoop` with a sufficient `maxIter`:

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  trueLoops: [
    { from: "v", to: "v", maxIter: 10 },
  ],
});
```

If the loop does not converge within `maxIter` iterations, the simulation throws a `DetectedTrueLoop` error at runtime.

::: warning
A true loop that never converges (e.g., `assign y = ~y`) will always throw at runtime regardless of `maxIter`. This represents combinational oscillation, which is not a valid hardware state.
:::

## Signal Path Syntax

Both `from` and `to` accept a signal path string:

| Pattern | Meaning |
|---------|---------|
| `"v"` | Top-level variable `v` |
| `"u_sub:i_data"` | Port `i_data` of child instance `u_sub` |
| `"u_a.u_b:x"` | Port `x` of instance `u_b` inside `u_a` |

## Choosing Between falseLoops and trueLoops

| | `falseLoops` | `trueLoops` |
|---|---|---|
| Cycle is real at runtime | No — paths are mutually exclusive | Yes — output feeds back to input |
| Iteration behavior | Statically unrolled N times (N = structural depth) | Iterates until convergence or `maxIter` |
| Runtime error possible | No | Yes, if it fails to converge |

## Further Reading

- [Combinational Analysis](/internals/combinational-analysis) -- How the dependency graph is built and scheduled.
- [Writing Tests](./writing-tests.md) -- Simulator options overview.
