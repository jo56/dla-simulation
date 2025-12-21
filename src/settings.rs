use serde::{Deserialize, Serialize};

/// Neighborhood type for sticking checks
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum NeighborhoodType {
    /// 4 neighbors (orthogonal only) - creates angular, cross-like patterns (classic DLA)
    #[default]
    VonNeumann,
    /// 8 neighbors (orthogonal + diagonal) - natural fractal patterns
    Moore,
    /// 24 neighbors (2-cell radius) - creates dense, blob-like growth
    Extended,
}

impl NeighborhoodType {
    pub fn short_name(&self) -> &str {
        match self {
            NeighborhoodType::VonNeumann => "VonNeumann",
            NeighborhoodType::Moore => "Moore",
            NeighborhoodType::Extended => "Extended",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            NeighborhoodType::VonNeumann => NeighborhoodType::Moore,
            NeighborhoodType::Moore => NeighborhoodType::Extended,
            NeighborhoodType::Extended => NeighborhoodType::VonNeumann,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            NeighborhoodType::VonNeumann => NeighborhoodType::Extended,
            NeighborhoodType::Moore => NeighborhoodType::VonNeumann,
            NeighborhoodType::Extended => NeighborhoodType::Moore,
        }
    }

    /// Get the neighbor offsets for this neighborhood type
    pub fn offsets(&self) -> &'static [(i32, i32)] {
        match self {
            NeighborhoodType::VonNeumann => &[(-1, 0), (1, 0), (0, -1), (0, 1)],
            NeighborhoodType::Moore => &[
                (-1, -1), (0, -1), (1, -1),
                (-1, 0),          (1, 0),
                (-1, 1),  (0, 1),  (1, 1),
            ],
            NeighborhoodType::Extended => &[
                (-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2),
                (-2, -1), (-1, -1), (0, -1), (1, -1), (2, -1),
                (-2, 0),  (-1, 0),           (1, 0),  (2, 0),
                (-2, 1),  (-1, 1),  (0, 1),  (1, 1),  (2, 1),
                (-2, 2),  (-1, 2),  (0, 2),  (1, 2),  (2, 2),
            ],
        }
    }
}

/// Spawn mode - where particles spawn from
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum SpawnMode {
    /// Spawn on a circle around the structure (classic DLA)
    #[default]
    Circle,
    /// Spawn from all grid edges
    Edges,
    /// Spawn from grid corners only
    Corners,
    /// Spawn randomly anywhere outside structure
    Random,
    /// Spawn from top edge only
    Top,
    /// Spawn from bottom edge only
    Bottom,
    /// Spawn from left edge only
    Left,
    /// Spawn from right edge only
    Right,
}

impl SpawnMode {
    pub fn name(&self) -> &str {
        match self {
            SpawnMode::Circle => "Circle",
            SpawnMode::Edges => "Edges",
            SpawnMode::Corners => "Corners",
            SpawnMode::Random => "Random",
            SpawnMode::Top => "Top",
            SpawnMode::Bottom => "Bottom",
            SpawnMode::Left => "Left",
            SpawnMode::Right => "Right",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SpawnMode::Circle => SpawnMode::Edges,
            SpawnMode::Edges => SpawnMode::Corners,
            SpawnMode::Corners => SpawnMode::Random,
            SpawnMode::Random => SpawnMode::Top,
            SpawnMode::Top => SpawnMode::Bottom,
            SpawnMode::Bottom => SpawnMode::Left,
            SpawnMode::Left => SpawnMode::Right,
            SpawnMode::Right => SpawnMode::Circle,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SpawnMode::Circle => SpawnMode::Right,
            SpawnMode::Edges => SpawnMode::Circle,
            SpawnMode::Corners => SpawnMode::Edges,
            SpawnMode::Random => SpawnMode::Corners,
            SpawnMode::Top => SpawnMode::Random,
            SpawnMode::Bottom => SpawnMode::Top,
            SpawnMode::Left => SpawnMode::Bottom,
            SpawnMode::Right => SpawnMode::Left,
        }
    }
}

/// Boundary behavior - what happens when particles hit grid edges
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum BoundaryBehavior {
    /// Stop at edge
    Clamp,
    /// Wrap to opposite side (toroidal)
    Wrap,
    /// Reflect off edges
    Bounce,
    /// Particles stick to edges
    Stick,
    /// Particles are removed/respawned at edges (canonical DLA)
    #[default]
    Absorb,
}

impl BoundaryBehavior {
    pub fn name(&self) -> &str {
        match self {
            BoundaryBehavior::Clamp => "Clamp",
            BoundaryBehavior::Wrap => "Wrap",
            BoundaryBehavior::Bounce => "Bounce",
            BoundaryBehavior::Stick => "Stick",
            BoundaryBehavior::Absorb => "Absorb",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            BoundaryBehavior::Clamp => BoundaryBehavior::Wrap,
            BoundaryBehavior::Wrap => BoundaryBehavior::Bounce,
            BoundaryBehavior::Bounce => BoundaryBehavior::Stick,
            BoundaryBehavior::Stick => BoundaryBehavior::Absorb,
            BoundaryBehavior::Absorb => BoundaryBehavior::Clamp,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            BoundaryBehavior::Clamp => BoundaryBehavior::Absorb,
            BoundaryBehavior::Wrap => BoundaryBehavior::Clamp,
            BoundaryBehavior::Bounce => BoundaryBehavior::Wrap,
            BoundaryBehavior::Stick => BoundaryBehavior::Bounce,
            BoundaryBehavior::Absorb => BoundaryBehavior::Stick,
        }
    }
}

/// Color mode - what property determines particle color
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum ColorMode {
    /// Color by attachment order (current behavior)
    #[default]
    Age,
    /// Color by distance from center
    Distance,
    /// Color by local neighbor density
    Density,
    /// Color by approach direction (angle)
    Direction,
}

impl ColorMode {
    pub fn name(&self) -> &str {
        match self {
            ColorMode::Age => "Age",
            ColorMode::Distance => "Distance",
            ColorMode::Density => "Density",
            ColorMode::Direction => "Direction",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ColorMode::Age => ColorMode::Distance,
            ColorMode::Distance => ColorMode::Density,
            ColorMode::Density => ColorMode::Direction,
            ColorMode::Direction => ColorMode::Age,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            ColorMode::Age => ColorMode::Direction,
            ColorMode::Distance => ColorMode::Age,
            ColorMode::Density => ColorMode::Distance,
            ColorMode::Direction => ColorMode::Density,
        }
    }
}

/// All simulation settings consolidated into one struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSettings {
    // === Movement Parameters ===
    /// Distance particles move per random walk step (0.5-5.0)
    pub walk_step_size: f32,
    /// Bias angle in degrees for directional drift (0-360)
    pub walk_bias_angle: f32,
    /// Strength of directional bias (0.0-0.5, 0 = isotropic)
    pub walk_bias_strength: f32,
    /// Radial bias (-0.3 to 0.3, negative = outward, positive = inward)
    pub radial_bias: f32,
    /// Enable adaptive step size based on distance from cluster (circle-jumping)
    pub adaptive_step: bool,
    /// Factor controlling adaptive step scaling (1.0-10.0)
    pub adaptive_step_factor: f32,
    /// Use pure lattice walk (4 cardinal directions) instead of continuous angles
    pub lattice_walk: bool,

    // === Sticking Parameters ===
    /// Neighborhood type for checking adjacent particles
    pub neighborhood: NeighborhoodType,
    /// Minimum neighbors required to stick (1-4)
    pub multi_contact_min: u8,
    /// Stickiness at branch tips (few neighbors) (0.1-1.0)
    pub tip_stickiness: f32,
    /// Stickiness on branch sides (many neighbors) (0.1-1.0)
    pub side_stickiness: f32,
    /// Stickiness variation by distance from center (-0.5 to 0.5 per 100px)
    pub stickiness_gradient: f32,

    // === Spawn/Boundary Parameters ===
    /// Where particles spawn from
    pub spawn_mode: SpawnMode,
    /// What happens at grid boundaries
    pub boundary_behavior: BoundaryBehavior,
    /// Buffer distance between structure edge and spawn circle (5-50)
    pub spawn_radius_offset: f32,
    /// Multiplier for escape distance (2.0-6.0)
    pub escape_multiplier: f32,
    /// Minimum spawn radius (20-100)
    pub min_spawn_radius: f32,
    /// Maximum walk iterations before respawn (1000-50000)
    pub max_walk_iterations: usize,

    // === Visual Parameters ===
    /// What property determines particle color
    pub color_mode: ColorMode,
    /// Number of recent particles to highlight (0-50)
    pub highlight_recent: usize,
    /// Invert color gradient
    pub invert_colors: bool,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            // Movement - canonical DLA uses unit lattice steps
            walk_step_size: 1.0,
            walk_bias_angle: 0.0,
            walk_bias_strength: 0.0,
            radial_bias: 0.0,
            adaptive_step: false, // Disabled by default for accurate DLA
            adaptive_step_factor: 3.0,
            lattice_walk: true, // Classic 4-direction lattice walk

            // Sticking
            neighborhood: NeighborhoodType::default(), // VonNeumann (4-neighbor)
            multi_contact_min: 1,
            tip_stickiness: 1.0,
            side_stickiness: 1.0,
            stickiness_gradient: 0.0,

            // Spawn/Boundary - unbounded-space behavior
            spawn_mode: SpawnMode::default(), // Circle
            boundary_behavior: BoundaryBehavior::Absorb, // Respawn at edges for unbounded feel
            spawn_radius_offset: 10.0,
            escape_multiplier: 3.0, // Higher multiplier reduces premature respawns
            min_spawn_radius: 15.0, // Lower for faster small-cluster convergence
            max_walk_iterations: 10000,

            // Visual
            color_mode: ColorMode::default(),
            highlight_recent: 0,
            invert_colors: false,
        }
    }
}

impl SimulationSettings {
    /// Adjust walk step size within bounds
    pub fn adjust_walk_step_size(&mut self, delta: f32) {
        self.walk_step_size = (self.walk_step_size + delta).clamp(0.5, 5.0);
    }

    /// Adjust walk bias angle (wraps around)
    pub fn adjust_walk_bias_angle(&mut self, delta: f32) {
        self.walk_bias_angle = (self.walk_bias_angle + delta).rem_euclid(360.0);
    }

    /// Adjust walk bias strength within bounds
    pub fn adjust_walk_bias_strength(&mut self, delta: f32) {
        self.walk_bias_strength = (self.walk_bias_strength + delta).clamp(0.0, 0.5);
    }

    /// Adjust radial bias within bounds
    pub fn adjust_radial_bias(&mut self, delta: f32) {
        self.radial_bias = (self.radial_bias + delta).clamp(-0.3, 0.3);
    }

    /// Adjust multi-contact minimum within bounds
    pub fn adjust_multi_contact_min(&mut self, delta: i32) {
        self.multi_contact_min = (self.multi_contact_min as i32 + delta).clamp(1, 4) as u8;
    }

    /// Adjust tip stickiness within bounds
    pub fn adjust_tip_stickiness(&mut self, delta: f32) {
        self.tip_stickiness = (self.tip_stickiness + delta).clamp(0.1, 1.0);
    }

    /// Adjust side stickiness within bounds
    pub fn adjust_side_stickiness(&mut self, delta: f32) {
        self.side_stickiness = (self.side_stickiness + delta).clamp(0.1, 1.0);
    }

    /// Adjust stickiness gradient within bounds
    pub fn adjust_stickiness_gradient(&mut self, delta: f32) {
        self.stickiness_gradient = (self.stickiness_gradient + delta).clamp(-0.5, 0.5);
    }

    /// Adjust spawn radius offset within bounds
    pub fn adjust_spawn_radius_offset(&mut self, delta: f32) {
        self.spawn_radius_offset = (self.spawn_radius_offset + delta).clamp(5.0, 50.0);
    }

    /// Adjust escape multiplier within bounds
    pub fn adjust_escape_multiplier(&mut self, delta: f32) {
        self.escape_multiplier = (self.escape_multiplier + delta).clamp(2.0, 6.0);
    }

    /// Adjust min spawn radius within bounds
    pub fn adjust_min_spawn_radius(&mut self, delta: f32) {
        self.min_spawn_radius = (self.min_spawn_radius + delta).clamp(20.0, 100.0);
    }

    /// Adjust max walk iterations within bounds
    pub fn adjust_max_walk_iterations(&mut self, delta: i32) {
        let new_val = (self.max_walk_iterations as i32 + delta).clamp(1000, 50000);
        self.max_walk_iterations = new_val as usize;
    }

    /// Adjust highlight recent within bounds
    pub fn adjust_highlight_recent(&mut self, delta: i32) {
        self.highlight_recent = (self.highlight_recent as i32 + delta).clamp(0, 50) as usize;
    }

    /// Toggle adaptive step on/off
    pub fn toggle_adaptive_step(&mut self) {
        self.adaptive_step = !self.adaptive_step;
    }

    /// Adjust adaptive step factor within bounds
    pub fn adjust_adaptive_step_factor(&mut self, delta: f32) {
        self.adaptive_step_factor = (self.adaptive_step_factor + delta).clamp(1.0, 10.0);
    }

    /// Toggle lattice walk on/off
    pub fn toggle_lattice_walk(&mut self) {
        self.lattice_walk = !self.lattice_walk;
    }

    /// Calculate effective stickiness based on neighbor count and distance
    pub fn effective_stickiness(&self, neighbor_count: usize, distance_from_center: f32, base_stickiness: f32) -> f32 {
        // Determine if this is a tip (few neighbors) or side (many neighbors)
        let max_neighbors = match self.neighborhood {
            NeighborhoodType::VonNeumann => 4,
            NeighborhoodType::Moore => 8,
            NeighborhoodType::Extended => 24,
        };

        let neighbor_ratio = neighbor_count as f32 / max_neighbors as f32;

        // Interpolate between tip and side stickiness based on neighbor count
        let directional_stickiness = self.tip_stickiness * (1.0 - neighbor_ratio)
            + self.side_stickiness * neighbor_ratio;

        // Apply distance gradient
        let gradient_factor = 1.0 + (distance_from_center / 100.0) * self.stickiness_gradient;

        // Combine with base stickiness
        (base_stickiness * directional_stickiness * gradient_factor).clamp(0.0, 1.0)
    }
}
