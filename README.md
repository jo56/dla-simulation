# DLA Simulator TUI

A terminal-based Diffusion-Limited Aggregation (DLA) simulation using high-resolution Braille character rendering.

![Rust](https://img.shields.io/badge/rust-stable-orange)

## What is DLA?

Diffusion-Limited Aggregation is a process where particles undergo random walks and stick to a growing structure upon contact. This creates beautiful fractal patterns similar to:
- Snowflake formation
- Lightning branching
- Mineral dendrites
- Coral growth
- River delta patterns

## Features

- **High-resolution Braille rendering** - Each terminal character displays a 2x4 dot pattern
- **Real-time simulation** - Watch the fractal structure grow
- **Interactive controls** - Adjust parameters while running
- **Multiple seed patterns** - Point, Line, Cross, Circle
- **8 color schemes** - Ice, Fire, Plasma, Viridis, Rainbow, Grayscale, Ocean, Neon
- **Speed control** - Adjust simulation steps per frame
- **Fullscreen mode** - Hide sidebar for maximum canvas size

## Installation

### From GitHub (recommended)

```bash
cargo install --git https://github.com/jo56/dla-simulator
```

### From source

```bash
# Clone the repository
git clone https://github.com/jo56/dla-simulator.git
cd dla-simulator

# Build and run
cargo run --release
```

## Usage

```bash
# Run with defaults
cargo run --release

# Custom parameters
cargo run --release -- --particles 3000 --stickiness 0.5 --seed circle --speed 10
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --particles` | Number of particles (100-10000) | 5000 |
| `-s, --stickiness` | Adhesion probability (0.1-1.0) | 1.0 |
| `--seed` | Seed pattern (point, line, cross, circle) | point |
| `--speed` | Steps per frame (1-20) | 5 |

## Controls

| Key | Action |
|-----|--------|
| `Space` | Pause/Resume simulation |
| `R` | Reset simulation |
| `1-4` | Quick select seed pattern |
| `C` | Cycle color scheme |
| `A` | Toggle color-by-age |
| `Tab` | Next parameter focus |
| `Shift+Tab` | Previous parameter focus |
| `Up/Down` | Adjust focused parameter |
| `+/-` | Adjust speed |
| `V` | Toggle fullscreen |
| `H` or `?` | Show help |
| `Q` | Quit |

## Parameters

### Stickiness (0.1 - 1.0)
Controls the probability that a particle will stick when it touches the structure.
- **High stickiness (1.0)**: Dense, compact clusters
- **Low stickiness (0.1-0.3)**: Thin, dendritic branches with more fractal detail

### Seed Patterns
- **Point**: Single center particle - classic DLA growth
- **Line**: Horizontal line - creates symmetrical branching
- **Cross**: Cross pattern - four-way growth
- **Circle**: Ring of particles - inward/outward growth

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [rand](https://github.com/rust-random/rand) - Random number generation
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing

## License

MIT
