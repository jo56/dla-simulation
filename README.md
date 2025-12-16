# DLA Simulator

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
- **24 adjustable parameters** - Fine-tune movement, sticking behavior, spawning, and visuals
- **Multiple seed patterns** - Points, lines, rings, blocks, spokes, scatter/noise blobs and more
- **8 color schemes** - Ice, Fire, Plasma, Viridis, Rainbow, Grayscale, Ocean, Neon
- **Parameter popup** - Quick access to any parameter via Shift+letter
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
| `--seed` | Seed pattern (point, line, cross, circle, ring, block, noise, scatter, multipoint, starburst) | point |
| `--speed` | Steps per frame (1-50) | 5 |

## Controls

### Navigation & System

| Key | Action |
|-----|--------|
| `Space` | Pause/Resume simulation |
| `R` | Reset simulation |
| `Tab` | Next parameter |
| `Shift+Tab` | Previous parameter |
| `Up/Down` | Adjust focused parameter |
| `V` | Toggle fullscreen |
| `H` or `?` | Show help |
| `Q` | Quit |

### Quick Keys

| Key | Action |
|-----|--------|
| `1-0` | Quick select seed pattern (1=Point through 0=Scatter) |
| `+/-` | Adjust simulation speed |
| `[/]` | Adjust highlight count |

### Parameter Popup

| Key | Action |
|-----|--------|
| `Shift+?` | Open popup with all parameters |
| `Shift+letter` | Open popup filtered to parameters starting with that letter |
| `Up/Down` | Navigate popup options |
| `Enter` | Select and focus parameter |
| `Esc` | Close popup |

### Direct Parameter Keys

| Key | Action |
|-----|--------|
| `A` | Toggle color-by-age |
| `C` | Cycle color scheme |
| `M` | Cycle color mode |
| `N` | Cycle neighborhood type |
| `B` | Cycle boundary behavior |
| `S` | Cycle spawn mode |
| `W` | Increase walk step size |
| `E` | Decrease walk step size |
| `I` | Invert colors |

## Parameters

The simulation has 24 adjustable parameters organized into four categories.

### Movement Parameters

Control how particles move during their random walk.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| Walk Step Size | 0.5-5.0 | 2.0 | Distance moved per step. Larger = faster but coarser patterns |
| Direction | 0-360° | 0 | Bias angle for directional drift |
| Force | 0-0.5 | 0 | Strength of directional bias (0 = isotropic random walk) |
| Radial Bias | -0.3 to 0.3 | 0 | Negative = outward drift, Positive = inward drift |

### Sticking Parameters

Control when and how particles attach to the structure.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| Stickiness | 0.1-1.0 | 1.0 | Base probability of sticking on contact |
| Neighborhood | VonNeumann/Moore/Extended | Moore | How many neighbors are checked (4/8/24) |
| Multi-Contact | 1-4 | 1 | Minimum neighbors required to stick |
| Tip Stickiness | 0.1-1.0 | 1.0 | Stickiness at branch tips (few neighbors) |
| Side Stickiness | 0.1-1.0 | 1.0 | Stickiness on branch sides (many neighbors) |
| Stickiness Gradient | -0.5 to 0.5 | 0 | How stickiness changes with distance from center (per 100px) |

**Neighborhood Types:**
- **Von Neumann (4)**: Only orthogonal neighbors. Creates angular, cross-like patterns.
- **Moore (8)**: Orthogonal + diagonal. Natural fractal patterns (default).
- **Extended (24)**: 2-cell radius. Dense, blob-like growth.

### Spawn & Boundary Parameters

Control where particles appear and how edges are handled.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| Spawn Mode | 8 options | Circle | Where new particles originate |
| Boundary | 5 options | Clamp | What happens at grid edges |
| Spawn Offset | 5-50 | 10 | Buffer distance between structure and spawn circle |
| Escape Mult | 2.0-6.0 | 2.0 | Multiplier for escape/respawn distance |
| Min Radius | 20-100 | 50 | Minimum spawn radius |
| Max Iterations | 1000-50000 | 10000 | Steps before particle respawns |

**Spawn Modes:**
- **Circle**: Classic DLA - spawn on a circle around the structure
- **Edges**: Spawn from all four grid edges
- **Corners**: Spawn only from corners
- **Random**: Spawn at random positions outside structure
- **Top/Bottom/Left/Right**: Directional spawning from a single edge

**Boundary Behaviors:**
- **Clamp**: Particles stop at edges (default)
- **Wrap**: Particles wrap to opposite side (toroidal)
- **Bounce**: Particles reflect off edges
- **Stick**: Particles can stick to edges themselves
- **Absorb**: Particles are removed and respawned at edges

### Visual Parameters

Control how the simulation is displayed.

| Parameter | Range | Default | Description |
|-----------|-------|---------|-------------|
| Particles | 100-10000 | 5000 | Total number of particles |
| Speed | 1-50 | 5 | Simulation steps per frame |
| Color Scheme | 8 options | Ice | Color palette |
| Color Mode | Age/Distance/Density/Direction | Age | What property determines color |
| Color by Age | on/off | on | Enable color gradient |
| Invert | on/off | off | Invert color gradient |
| Highlight | 0-50 | 0 | Recent particles shown in white |

**Color Schemes:** Ice, Fire, Plasma, Viridis, Rainbow, Grayscale, Ocean, Neon

**Color Modes:**
- **Age**: Color based on attachment order (oldest to newest)
- **Distance**: Color based on distance from center
- **Density**: Color based on neighbor count when stuck
- **Direction**: Color based on approach angle when stuck

### Seed Patterns

Press number keys 1-0 to quickly select:

| Key | Pattern | Description |
|-----|---------|-------------|
| 1 | Point | Single center particle - classic DLA growth |
| 2 | Line | Horizontal line - symmetrical branching |
| 3 | Cross | Cross pattern - four-way growth |
| 4 | Circle | Ring of particles - inward/outward growth |
| 5 | Ring | Thick rim with hollow core - outside-in branching |
| 6 | Block | Solid square - surface roughening |
| 7 | Multi-Point | Multiple competing centers |
| 8 | Starburst | Radial spokes with rim - strong anisotropy |
| 9 | Noise Patch | Dense noisy blob - asymmetric drift |
| 0 | Scatter | Randomized small seeds near center |

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [rand](https://github.com/rust-random/rand) - Random number generation
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [serde](https://github.com/serde-rs/serde) - Serialization framework
- [dirs](https://github.com/dirs-dev/dirs-rs) - Platform directory paths

## License

MIT
