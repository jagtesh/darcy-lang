# Darcy <> Rust interop example: Trading Signals

Darcy generates bars, computes indicators, and generates signals. Rust runs the loop
and prints the signal. Darcy also calls back into Rust to round prices.

## Layout

- `examples/trading/darcy/trading/types.dsl` - Rust Candle type + Darcy Signal enum
- `examples/trading/darcy/trading/indicators.dsl` - SMA
- `examples/trading/darcy/trading/signals.dsl` - crossover logic
- `examples/trading/darcy/trading/main.dsl` - public API for Rust + extern Rust call
- `examples/trading/engine/` - tiny Rust engine (tick loop mock)
- `crates/darcy-stdlib/darcy/darcy/astra/sim.dsl` - bar simulation helpers used by the example

## Build + run

From the repo root:

```bash
# Build runs Darcy codegen via build.rs
cargo run --manifest-path examples/trading/engine/Cargo.toml
```

## Notes

- The SMA uses the first `period` values; assume bars are newest-first.
- `Candle` lives in Rust and is referenced from Darcy as `crate::Candle`.
- The RNG is a pure Darcy implementation of a Park-Miller LCG.
