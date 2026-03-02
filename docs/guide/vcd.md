# VCD Waveform Output

Celox can write [Value Change Dump](https://en.wikipedia.org/wiki/Value_change_dump) (VCD) files for viewing waveforms in tools like [GTKWave](https://gtkwave.sourceforge.net/) or [Surfer](https://surfer-project.org/).

## Basic Usage

Pass `vcd` with the output file path when creating a simulator:

```typescript
const sim = Simulator.fromSource(SOURCE, "Top", {
  vcd: "./dump.vcd",
});
```

Then call `sim.dump(timestamp)` at each point in time you want recorded:

```typescript
sim.dut.a = 10n;
sim.dump(0);      // record initial state at t=0

sim.dut.a = 20n;
sim.tick();
sim.dump(10);     // record state at t=10

sim.dispose();    // flushes and closes the file
```

::: warning
The VCD file is not written until `dispose()` is called. Always call `dispose()` when you are done — or use a try/finally block.
:::

## With Time-Based Simulation

With `Simulation`, `sim.time()` gives you the current time to pass to `dump()`:

```typescript
const sim = Simulation.fromSource(SOURCE, "Top", {
  vcd: "./dump.vcd",
});
sim.addClock("clk", { period: 10 });

sim.reset("rst");
sim.dump(sim.time());

sim.dut.en = 1n;
sim.runUntil(100);
sim.dump(sim.time());

sim.dispose();
```

## Further Reading

- [Writing Tests](./writing-tests.md) -- Simulator and Simulation patterns.
