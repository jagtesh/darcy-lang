# Fractal Generator

A complex fractal generator written in Darcy (a Lisp-like DSL that compiles to Rust) that generates three types of fractals: Mandelbrot, Julia, and Sierpinski triangle.

## Features

- **Mandelbrot Set**: Classic escape-time fractal algorithm
- **Julia Set**: Parameterized fractal with configurable constants
- **Sierpinski Triangle**: Geometric recursive fractal
- **Color Mapping**: Iteration-based color gradients
- **Configurable Output**: Custom dimensions and iteration counts

## Usage

Run with default settings (800x600, 100 iterations):
```bash
cargo run --release
```

Specify custom dimensions and iterations:
```bash
cargo run --release -- <width> <height> <max_iterations>
```

Examples:
```bash
# Small quick test
cargo run --release -- 200 200 20

# High quality output
cargo run --release -- 1920 1080 500
```

## Output

The program generates three PNG images:
- `mandelbrot.png` - Mandelbrot set visualization
- `julia.png` - Julia set (c = -0.7 + 0.27015i)
- `sierpinski.png` - Sierpinski triangle

## Darcy Language Features Used

- **Records**: `pixel`, `complex`, `fractal-config`
- **Enums**: `fractal-type`
- **Functions**: exported functions with type annotations
- **Pattern Matching**: `case` expressions for fractal types
- **Loops**: `while` for pixel-by-pixel iteration
- **Mutability**: `let!` for state updates
- **Math Operations**: Complex arithmetic with custom inline functions
- **Vector Operations**: `darcy.vec/new`, `darcy.vec/push`
- **Module Imports**: `darcy.math`, `darcy.vec`, `darcy.core`

## Architecture

### Darcy Layer (`darcy/main.dsl`)
- Implements fractal algorithms using complex number arithmetic
- Maps pixel coordinates to complex plane
- Iterates until escape or max iterations
- Colors pixels based on iteration count

### Rust Layer (`src/main.rs`)
- Calls generated Darcy functions
- Converts pixel data to image format using the `image` crate
- Handles CLI arguments for dimensions and iterations

## Project Structure

```
fractal-generator/
├── darcy/
│   └── main.dsl          # Darcy fractal algorithms
├── src/
│   ├── main.rs           # Rust binary entry point
│   └── lib.rs           # Generated Darcy module
├── Cargo.toml           # Rust dependencies
├── build.rs             # Darcy build integration
└── README.md            # This file
```

## Building

The project uses `darcy-build` to compile Darcy sources during the Cargo build:

```bash
cargo build --release
```

This automatically:
1. Compiles `darcy/main.dsl` to Rust
2. Generates code in `target/release/build/.../out/darcy_gen.rs`
3. Links with the Rust main program
