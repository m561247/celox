# Parameter Overrides

Celox lets you override top-level module parameters at simulation time without modifying the Veryl source. This is useful for testing a design across multiple configurations — for example, running the same test suite at different data widths.

## Basic Usage

Pass a `parameters` array in the options object. Each entry has a `name` and a `value`:

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

// Default: WIDTH = 8
const sim8 = Simulator.fromSource(SOURCE, "ParamWidth");

// Override: WIDTH = 16
const sim16 = Simulator.fromSource(SOURCE, "ParamWidth", {
  parameters: [{ name: "WIDTH", value: 16 }],
});

sim16.dut.a = 0xABCDn;
expect(sim16.dut.b).toBe(0xABCDn); // full 16-bit value preserved

sim16.dispose();
```

The same option works with `Simulation`:

```typescript
const sim = Simulation.fromSource(SOURCE, "ParamWidth", {
  parameters: [{ name: "WIDTH", value: 16 }],
});
```

## Overriding Multiple Parameters

List as many entries as you need:

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  parameters: [
    { name: "WIDTH", value: 32 },
    { name: "DEPTH", value: 512 },
  ],
});
```

## How It Works

The override values are injected into the Veryl analyzer before compilation. All downstream elaboration — including child module instantiation — sees the overridden constant. The DUT's port layout is derived from the actual compiled result, so it correctly reflects the overridden widths even if the `ModuleDefinition` type was generated for different parameter values.

## Parameter Values

Parameter values must be integers (number or bigint). Passing a non-integer value (for example, a float or a string) is not supported.

## Further Reading

- [Writing Tests](./writing-tests.md) -- Event-based and time-based simulation patterns.
- [API Reference](/api/) -- Full TypeScript API documentation.
