use crate::settings::{BoundaryBehavior, SimulationSettings, SpawnMode};
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

const BOUNDARY_MARGIN: f32 = 1.0;

/// Seed pattern types for initial structure
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum SeedPattern {
    #[default]
    Point,
    Line,
    Cross,
    Circle,
    Ring,
    Block,
    NoisePatch,
    Scatter,
    MultiPoint,
    Starburst,
}

impl SeedPattern {
    pub fn name(&self) -> &str {
        match self {
            SeedPattern::Point => "Point",
            SeedPattern::Line => "Line",
            SeedPattern::Cross => "Cross",
            SeedPattern::Circle => "Circle",
            SeedPattern::Ring => "Ring",
            SeedPattern::Block => "Block",
            SeedPattern::NoisePatch => "Noise Patch",
            SeedPattern::Scatter => "Scatter",
            SeedPattern::MultiPoint => "Multi-Point",
            SeedPattern::Starburst => "Starburst",
        }
    }

    pub fn next(&self) -> SeedPattern {
        match self {
            SeedPattern::Point => SeedPattern::Line,
            SeedPattern::Line => SeedPattern::Cross,
            SeedPattern::Cross => SeedPattern::Circle,
            SeedPattern::Circle => SeedPattern::Ring,
            SeedPattern::Ring => SeedPattern::Block,
            SeedPattern::Block => SeedPattern::NoisePatch,
            SeedPattern::NoisePatch => SeedPattern::Scatter,
            SeedPattern::Scatter => SeedPattern::MultiPoint,
            SeedPattern::MultiPoint => SeedPattern::Starburst,
            SeedPattern::Starburst => SeedPattern::Point,
        }
    }

    pub fn prev(&self) -> SeedPattern {
        match self {
            SeedPattern::Point => SeedPattern::Starburst,
            SeedPattern::Line => SeedPattern::Point,
            SeedPattern::Cross => SeedPattern::Line,
            SeedPattern::Circle => SeedPattern::Cross,
            SeedPattern::Ring => SeedPattern::Circle,
            SeedPattern::Block => SeedPattern::Ring,
            SeedPattern::NoisePatch => SeedPattern::Block,
            SeedPattern::Scatter => SeedPattern::NoisePatch,
            SeedPattern::MultiPoint => SeedPattern::Scatter,
            SeedPattern::Starburst => SeedPattern::MultiPoint,
        }
    }
}

/// Additional data stored per particle for advanced color modes
#[derive(Clone, Copy, Default)]
pub struct ParticleData {
    /// Order in which particle was stuck (age)
    pub age: usize,
    /// Distance from center when stuck
    pub distance: f32,
    /// Approach direction angle when stuck (radians)
    pub direction: f32,
    /// Number of neighbors when stuck
    pub neighbor_count: u8,
}

/// DLA simulation state
pub struct DlaSimulation {
    pub grid_width: usize,
    pub grid_height: usize,
    /// Grid stores particle data (None = empty, Some = particle)
    grid: Vec<Option<ParticleData>>,
    pub num_particles: usize,
    pub stickiness: f32,
    pub particles_stuck: usize,
    pub max_radius: f32,
    pub paused: bool,
    pub seed_pattern: SeedPattern,
    /// Advanced simulation settings
    pub settings: SimulationSettings,
    rng: ThreadRng,
}

impl DlaSimulation {
    pub fn new(width: usize, height: usize) -> Self {
        let mut sim = Self {
            grid_width: width,
            grid_height: height,
            grid: vec![None; width * height],
            num_particles: 5000,
            stickiness: 1.0,
            particles_stuck: 0,
            max_radius: 1.0,
            paused: false,
            seed_pattern: SeedPattern::Point,
            settings: SimulationSettings::default(),
            rng: rand::thread_rng(),
        };
        sim.reset();
        sim
    }

    /// Get the center coordinates of the grid
    fn center(&self) -> (f32, f32) {
        (self.grid_width as f32 / 2.0, self.grid_height as f32 / 2.0)
    }

    /// Execute one particle simulation step
    /// Returns true if simulation should continue, false if complete
    pub fn step(&mut self) -> bool {
        if self.paused || self.particles_stuck >= self.num_particles {
            return false;
        }

        let (center_x, center_y) = self.center();

        // Get settings values
        let spawn_radius_offset = self.settings.spawn_radius_offset;
        let min_spawn_radius = self.settings.min_spawn_radius;
        let escape_mult = self.settings.escape_multiplier;
        let max_iterations = self.settings.max_walk_iterations;
        let base_walk_step = self.settings.walk_step_size;
        let adaptive_step = self.settings.adaptive_step;
        let adaptive_factor = self.settings.adaptive_step_factor;
        let lattice_walk = self.settings.lattice_walk;

        // Spawn radius - outside the structure with proportional buffer
        // Uses 20% extra beyond max_radius plus fixed offset for better scaling
        let spawn_radius = (self.max_radius * 1.2 + spawn_radius_offset).max(min_spawn_radius);

        // Pre-calculate squared escape distance (avoids sqrt in hot loop)
        let escape_dist_sq = spawn_radius * spawn_radius * escape_mult * escape_mult;

        // Pre-calculate boundary limits
        let x_max = self.grid_width as f32 - BOUNDARY_MARGIN - 1.0;
        let y_max = self.grid_height as f32 - BOUNDARY_MARGIN - 1.0;

        // Spawn particle based on spawn mode
        let (mut x, mut y) = self.spawn_particle(center_x, center_y, spawn_radius);

        // Track the approach direction for color mode
        let mut last_dx = x - center_x;
        let mut last_dy = y - center_y;

        // Random walk until it sticks or escapes
        for _ in 0..max_iterations {
            // Check if we've gone too far (using squared distance to avoid sqrt)
            let dx = x - center_x;
            let dy = y - center_y;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq > escape_dist_sq {
                // Escaped, restart
                return true;
            }

            // Check if next to a stuck particle
            let ix = x as usize;
            let iy = y as usize;

            if ix > 0 && ix < self.grid_width - 1 && iy > 0 && iy < self.grid_height - 1 {
                // Count neighbors using the configured neighborhood type
                let (neighbor_count, has_neighbor) = self.count_neighbors(ix, iy);

                if has_neighbor && neighbor_count >= self.settings.multi_contact_min as usize {
                    // Calculate distance from center for stickiness gradient
                    let distance = dist_sq.sqrt();

                    // Calculate effective stickiness
                    let effective_stickiness = self.settings.effective_stickiness(
                        neighbor_count,
                        distance,
                        self.stickiness,
                    );

                    // Check if we should stick
                    if self.rng.gen::<f32>() < effective_stickiness {
                        let idx = iy * self.grid_width + ix;

                        // Only stick if cell is empty - if occupied, continue walking
                        if self.grid[idx].is_none() {
                            // Calculate approach direction
                            let direction = last_dy.atan2(last_dx);

                            // Stick here with particle data
                            self.grid[idx] = Some(ParticleData {
                                age: self.particles_stuck,
                                distance,
                                direction,
                                neighbor_count: neighbor_count as u8,
                            });
                            self.particles_stuck += 1;

                            // Update max radius
                            self.max_radius = self.max_radius.max(distance);

                            return true;
                        }
                        // Cell occupied - particle continues walking (accurate DLA behavior)
                    }
                }
            }

            // Store previous position for direction tracking
            last_dx = x - center_x;
            last_dy = y - center_y;

            // Calculate adaptive step size based on distance from cluster
            let walk_step = if adaptive_step {
                let dist = dist_sq.sqrt();
                let safe_dist = (dist - self.max_radius - 2.0).max(0.0);
                // Use large steps when far, small steps when close
                // Minimum step is base step, maximum is base * factor
                let adaptive = base_walk_step
                    + (safe_dist / 10.0).min(adaptive_factor - 1.0) * base_walk_step;
                adaptive.min(base_walk_step * adaptive_factor)
            } else {
                base_walk_step
            };

            // Calculate walk angle - either lattice (4 directions) or continuous
            let walk_angle = if lattice_walk {
                // Pure lattice: 4 cardinal directions only (classic Witten-Sander DLA)
                match self.rng.gen_range(0..4) {
                    0 => 0.0,                                    // Right
                    1 => std::f32::consts::FRAC_PI_2,            // Up
                    2 => std::f32::consts::PI,                   // Left
                    _ => 3.0 * std::f32::consts::FRAC_PI_2,      // Down
                }
            } else {
                // Continuous random angle with optional bias
                let base_angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
                self.apply_walk_bias(base_angle, x, y, center_x, center_y)
            };

            // Take walk step
            x += walk_step * walk_angle.cos();
            y += walk_step * walk_angle.sin();

            // Apply boundary behavior
            (x, y) = self.apply_boundary(x, y, x_max, y_max);

            // Handle absorb boundary - if we hit edge, respawn
            if self.settings.boundary_behavior == BoundaryBehavior::Absorb {
                if x <= BOUNDARY_MARGIN || x >= x_max || y <= BOUNDARY_MARGIN || y >= y_max {
                    return true; // Respawn
                }
            }
        }

        true
    }

    /// Spawn a particle based on the configured spawn mode
    fn spawn_particle(&mut self, center_x: f32, center_y: f32, spawn_radius: f32) -> (f32, f32) {
        let w = self.grid_width as f32;
        let h = self.grid_height as f32;

        match self.settings.spawn_mode {
            SpawnMode::Circle => {
                let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
                (
                    (center_x + spawn_radius * angle.cos()).clamp(1.0, w - 2.0),
                    (center_y + spawn_radius * angle.sin()).clamp(1.0, h - 2.0),
                )
            }
            SpawnMode::Edges => {
                // Random edge
                match self.rng.gen_range(0..4) {
                    0 => (self.rng.gen_range(1.0..w - 1.0), 1.0), // Top
                    1 => (self.rng.gen_range(1.0..w - 1.0), h - 2.0), // Bottom
                    2 => (1.0, self.rng.gen_range(1.0..h - 1.0)), // Left
                    _ => (w - 2.0, self.rng.gen_range(1.0..h - 1.0)), // Right
                }
            }
            SpawnMode::Corners => {
                match self.rng.gen_range(0..4) {
                    0 => (1.0, 1.0),
                    1 => (w - 2.0, 1.0),
                    2 => (1.0, h - 2.0),
                    _ => (w - 2.0, h - 2.0),
                }
            }
            SpawnMode::Random => {
                // Random position outside spawn radius
                loop {
                    let x = self.rng.gen_range(1.0..w - 1.0);
                    let y = self.rng.gen_range(1.0..h - 1.0);
                    let dx = x - center_x;
                    let dy = y - center_y;
                    if dx * dx + dy * dy > spawn_radius * spawn_radius * 0.5 {
                        return (x, y);
                    }
                }
            }
            SpawnMode::Top => (self.rng.gen_range(1.0..w - 1.0), 1.0),
            SpawnMode::Bottom => (self.rng.gen_range(1.0..w - 1.0), h - 2.0),
            SpawnMode::Left => (1.0, self.rng.gen_range(1.0..h - 1.0)),
            SpawnMode::Right => (w - 2.0, self.rng.gen_range(1.0..h - 1.0)),
        }
    }

    /// Count neighbors at position using configured neighborhood type
    fn count_neighbors(&self, ix: usize, iy: usize) -> (usize, bool) {
        let offsets = self.settings.neighborhood.offsets();
        let mut count = 0;
        let mut has_any = false;

        for &(ndx, ndy) in offsets {
            let nx = ix as i32 + ndx;
            let ny = iy as i32 + ndy;

            if nx >= 0 && nx < self.grid_width as i32 && ny >= 0 && ny < self.grid_height as i32 {
                let nidx = ny as usize * self.grid_width + nx as usize;
                if self.grid[nidx].is_some() {
                    count += 1;
                    has_any = true;
                }
            }
        }

        (count, has_any)
    }

    /// Apply walk bias (directional and radial)
    fn apply_walk_bias(&self, base_angle: f32, x: f32, y: f32, center_x: f32, center_y: f32) -> f32 {
        let mut angle = base_angle;

        // Apply directional bias
        if self.settings.walk_bias_strength > 0.0 {
            let bias_angle_rad = self.settings.walk_bias_angle.to_radians();
            let diff = (bias_angle_rad - base_angle).sin();
            angle += self.settings.walk_bias_strength * diff;
        }

        // Apply radial bias
        if self.settings.radial_bias.abs() > 0.001 {
            let dx = x - center_x;
            let dy = y - center_y;
            let radial_angle = dy.atan2(dx);

            // Positive radial_bias = toward center, negative = away
            let target_angle = if self.settings.radial_bias > 0.0 {
                radial_angle + std::f32::consts::PI // Toward center
            } else {
                radial_angle // Away from center
            };

            let diff = (target_angle - angle).sin();
            angle += self.settings.radial_bias.abs() * diff;
        }

        angle
    }

    /// Apply boundary behavior
    fn apply_boundary(&self, mut x: f32, mut y: f32, x_max: f32, y_max: f32) -> (f32, f32) {
        match self.settings.boundary_behavior {
            BoundaryBehavior::Clamp => {
                x = x.clamp(BOUNDARY_MARGIN, x_max);
                y = y.clamp(BOUNDARY_MARGIN, y_max);
            }
            BoundaryBehavior::Wrap => {
                let width = x_max - BOUNDARY_MARGIN;
                let height = y_max - BOUNDARY_MARGIN;
                if x < BOUNDARY_MARGIN {
                    x += width;
                } else if x > x_max {
                    x -= width;
                }
                if y < BOUNDARY_MARGIN {
                    y += height;
                } else if y > y_max {
                    y -= height;
                }
            }
            BoundaryBehavior::Bounce => {
                if x < BOUNDARY_MARGIN {
                    x = BOUNDARY_MARGIN + (BOUNDARY_MARGIN - x);
                } else if x > x_max {
                    x = x_max - (x - x_max);
                }
                if y < BOUNDARY_MARGIN {
                    y = BOUNDARY_MARGIN + (BOUNDARY_MARGIN - y);
                } else if y > y_max {
                    y = y_max - (y - y_max);
                }
            }
            BoundaryBehavior::Stick | BoundaryBehavior::Absorb => {
                // These are handled elsewhere; just clamp for safety
                x = x.clamp(BOUNDARY_MARGIN, x_max);
                y = y.clamp(BOUNDARY_MARGIN, y_max);
            }
        }
        (x, y)
    }

    /// Reset the simulation with the current seed pattern
    pub fn reset(&mut self) {
        self.reset_with_seed(self.seed_pattern);
    }

    /// Reset with a specific seed pattern
    pub fn reset_with_seed(&mut self, pattern: SeedPattern) {
        // Resize grid if dimensions changed
        let required_size = self.grid_width * self.grid_height;
        if self.grid.len() != required_size {
            self.grid = vec![None; required_size];
        } else {
            self.grid.fill(None);
        }

        self.seed_pattern = pattern;

        match pattern {
            SeedPattern::Point => self.seed_point(),
            SeedPattern::Line => self.seed_line(),
            SeedPattern::Cross => self.seed_cross(),
            SeedPattern::Circle => self.seed_circle(),
            SeedPattern::Ring => self.seed_ring(),
            SeedPattern::Block => self.seed_block(),
            SeedPattern::NoisePatch => self.seed_noise_patch(),
            SeedPattern::Scatter => self.seed_scatter(),
            SeedPattern::MultiPoint => self.seed_multi_point(),
            SeedPattern::Starburst => self.seed_starburst(),
        }

        self.paused = false;
    }

    /// Helper to create seed particle data
    fn seed_particle(&self) -> ParticleData {
        ParticleData {
            age: 0,
            distance: 0.0,
            direction: 0.0,
            neighbor_count: 0,
        }
    }

    /// Single center point seed
    fn seed_point(&mut self) {
        let center_idx = self.grid_height / 2 * self.grid_width + self.grid_width / 2;
        self.grid[center_idx] = Some(self.seed_particle());
        self.particles_stuck = 1;
        self.max_radius = 1.0;
    }

    /// Horizontal line seed
    fn seed_line(&mut self) {
        let cy = self.grid_height / 2;
        let half_len = 20.min(self.grid_width / 4);
        let start_x = self.grid_width / 2 - half_len;
        let end_x = self.grid_width / 2 + half_len;
        let seed_data = self.seed_particle();
        for x in start_x..end_x {
            self.grid[cy * self.grid_width + x] = Some(seed_data);
        }
        self.particles_stuck = end_x - start_x;
        self.max_radius = half_len as f32;
    }

    /// Cross-shaped seed
    fn seed_cross(&mut self) {
        let cx = self.grid_width / 2;
        let cy = self.grid_height / 2;
        let arm_len = 10.min(self.grid_width / 8).min(self.grid_height / 8);
        let seed_data = self.seed_particle();
        let mut count = 0;
        for i in 0..arm_len {
            if cx >= i && cy >= i {
                self.grid[cy * self.grid_width + (cx - i)] = Some(seed_data);
                self.grid[cy * self.grid_width + (cx + i)] = Some(seed_data);
                self.grid[(cy - i) * self.grid_width + cx] = Some(seed_data);
                self.grid[(cy + i) * self.grid_width + cx] = Some(seed_data);
                count += 4;
            }
        }
        self.particles_stuck = count;
        self.max_radius = arm_len as f32;
    }

    /// Circle outline seed
    fn seed_circle(&mut self) {
        let (cx, cy) = self.center();
        let radius = 15.0_f32.min((self.grid_width / 8) as f32).min((self.grid_height / 8) as f32);
        let seed_data = self.seed_particle();
        let mut count = 0;
        for angle_deg in 0..360 {
            let angle = (angle_deg as f32).to_radians();
            let x = (cx + radius * angle.cos()) as usize;
            let y = (cy + radius * angle.sin()) as usize;
            if x < self.grid_width && y < self.grid_height {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(seed_data);
                    count += 1;
                }
            }
        }
        self.particles_stuck = count;
        self.max_radius = radius;
    }

    /// Thick ring seed (hollow core)
    fn seed_ring(&mut self) {
        let (cx, cy) = self.center();
        let min_dim = self.grid_width.min(self.grid_height) as f32;
        let radius = (min_dim * 0.30).clamp(6.0, min_dim * 0.45);
        let thickness = 2.5_f32;
        let seed_data = self.seed_particle();
        let mut count = 0;

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if (dist >= radius - thickness) && (dist <= radius + thickness) {
                    let idx = y * self.grid_width + x;
                    if self.grid[idx].is_none() {
                        self.grid[idx] = Some(seed_data);
                        count += 1;
                    }
                }
            }
        }

        self.particles_stuck = count;
        self.max_radius = radius + thickness;
    }

    /// Solid block seed (forces surface roughening)
    fn seed_block(&mut self) {
        let cx = self.grid_width / 2;
        let cy = self.grid_height / 2;
        let min_dim = self.grid_width.min(self.grid_height);
        let half_size = (min_dim / 8).max(4);
        let start_x = cx.saturating_sub(half_size);
        let end_x = (cx + half_size).min(self.grid_width.saturating_sub(1));
        let start_y = cy.saturating_sub(half_size);
        let end_y = (cy + half_size).min(self.grid_height.saturating_sub(1));
        let seed_data = self.seed_particle();
        let mut count = 0;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(seed_data);
                    count += 1;
                }
            }
        }

        self.particles_stuck = count;
        self.max_radius = (half_size as f32) * 1.414;
    }

    /// Dense noisy blob offset from center for asymmetric growth
    fn seed_noise_patch(&mut self) {
        let (grid_cx, grid_cy) = self.center();
        let min_dim = self.grid_width.min(self.grid_height) as f32;
        let radius = (min_dim * 0.22).clamp(6.0, 30.0);
        let radius_i = radius as i32;
        let jitter = (radius_i / 3).max(1);
        let mut patch_cx = (self.grid_width as i32 / 3) + self.rng.gen_range(-jitter..=jitter);
        let mut patch_cy = (self.grid_height as i32 / 3) + self.rng.gen_range(-jitter..=jitter);
        patch_cx = patch_cx.clamp(1, self.grid_width as i32 - 2);
        patch_cy = patch_cy.clamp(1, self.grid_height as i32 - 2);

        let seed_data = self.seed_particle();
        let mut count = 0;
        let mut max_dist: f32 = 1.0;

        for y in (patch_cy - radius_i).max(1)..=(patch_cy + radius_i).min(self.grid_height as i32 - 2) {
            for x in (patch_cx - radius_i).max(1)..=(patch_cx + radius_i).min(self.grid_width as i32 - 2) {
                let dx = x - patch_cx;
                let dy = y - patch_cy;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist <= radius {
                    let falloff = 1.0 - dist / radius;
                    let stick_prob = 0.35 + falloff * 0.65; // Dense core, noisy edges
                    if self.rng.gen::<f32>() < stick_prob {
                        let idx = (y as usize) * self.grid_width + (x as usize);
                        if self.grid[idx].is_none() {
                            self.grid[idx] = Some(seed_data);
                            count += 1;

                            let gdx = x as f32 - grid_cx;
                            let gdy = y as f32 - grid_cy;
                            let gdist = (gdx * gdx + gdy * gdy).sqrt();
                            max_dist = max_dist.max(gdist);
                        }
                    }
                }
            }
        }

        if count == 0 {
            // Guarantee at least one seed
            let idx = (patch_cy as usize) * self.grid_width + (patch_cx as usize);
            self.grid[idx] = Some(seed_data);
            count = 1;
            let gdx = patch_cx as f32 - grid_cx;
            let gdy = patch_cy as f32 - grid_cy;
            max_dist = (gdx * gdx + gdy * gdy).sqrt();
        }

        self.particles_stuck = count;
        self.max_radius = max_dist;
    }

    /// Random scattered points in center region
    fn seed_scatter(&mut self) {
        let cx = self.grid_width / 2;
        let cy = self.grid_height / 2;
        let scatter_radius = 20.min(self.grid_width / 6).min(self.grid_height / 6);
        let num_seeds = 15;
        let seed_data = self.seed_particle();
        let mut count = 0;

        for _ in 0..num_seeds {
            let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let r = self.rng.gen_range(0.0..scatter_radius as f32);
            let x = (cx as f32 + r * angle.cos()) as usize;
            let y = (cy as f32 + r * angle.sin()) as usize;
            if x < self.grid_width && y < self.grid_height {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(seed_data);
                    count += 1;
                }
            }
        }
        self.particles_stuck = count;
        self.max_radius = scatter_radius as f32;
    }

    /// Multiple seed points spread across the grid (creates competing growth)
    fn seed_multi_point(&mut self) {
        let cx = self.grid_width / 2;
        let cy = self.grid_height / 2;
        let spread = 25.min(self.grid_width / 5).min(self.grid_height / 5);
        let seed_data = self.seed_particle();
        let mut count = 0;

        // Place 5 seed points: center and 4 around it
        let points = [
            (cx, cy),
            (cx - spread, cy),
            (cx + spread, cy),
            (cx, cy - spread),
            (cx, cy + spread),
        ];

        for (px, py) in points {
            if px < self.grid_width && py < self.grid_height {
                let idx = py * self.grid_width + px;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(seed_data);
                    count += 1;
                }
            }
        }
        self.particles_stuck = count;
        self.max_radius = spread as f32;
    }

    /// Radial spokes with a thin rim for strong anisotropy
    fn seed_starburst(&mut self) {
        let (cx, cy) = self.center();
        let min_dim = self.grid_width.min(self.grid_height) as f32;
        let spoke_len = (min_dim * 0.35).clamp(8.0, 40.0);
        let spokes = 8;
        let seed_data = self.seed_particle();
        let mut count = 0;

        // Central hub
        let hub_x = cx as usize;
        let hub_y = cy as usize;
        let hub_idx = hub_y * self.grid_width + hub_x;
        if self.grid[hub_idx].is_none() {
            self.grid[hub_idx] = Some(seed_data);
            count += 1;
        }

        for s in 0..spokes {
            let angle = (s as f32) * (std::f32::consts::TAU / spokes as f32);
            for step in 1..=(spoke_len as usize) {
                let fx = cx + (step as f32) * angle.cos();
                let fy = cy + (step as f32) * angle.sin();
                let x = fx.round() as isize;
                let y = fy.round() as isize;
                if x > 0 && x < self.grid_width as isize - 1 && y > 0 && y < self.grid_height as isize - 1 {
                    let idx = (y as usize) * self.grid_width + (x as usize);
                    if self.grid[idx].is_none() {
                        self.grid[idx] = Some(seed_data);
                        count += 1;
                    }
                }
            }
        }

        // Thin rim to connect spokes
        let rim_radius = spoke_len;
        for angle_deg in (0..360).step_by(4) {
            let angle = (angle_deg as f32).to_radians();
            let x = (cx + rim_radius * angle.cos()) as isize;
            let y = (cy + rim_radius * angle.sin()) as isize;
            if x > 0 && x < self.grid_width as isize - 1 && y > 0 && y < self.grid_height as isize - 1 {
                let idx = (y as usize) * self.grid_width + (x as usize);
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(seed_data);
                    count += 1;
                }
            }
        }

        self.particles_stuck = count;
        self.max_radius = rim_radius;
    }

    /// Get full particle data at (x, y)
    pub fn get_particle(&self, x: usize, y: usize) -> Option<ParticleData> {
        if x < self.grid_width && y < self.grid_height {
            self.grid[y * self.grid_width + x]
        } else {
            None
        }
    }

    /// Get simulation progress as a ratio (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        self.particles_stuck as f32 / self.num_particles as f32
    }

    /// Check if simulation is complete
    pub fn is_complete(&self) -> bool {
        self.particles_stuck >= self.num_particles
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Resize the simulation grid
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if new_width != self.grid_width || new_height != self.grid_height {
            self.grid_width = new_width;
            self.grid_height = new_height;
            // Cap particles to new grid's max
            let max = self.max_particles();
            if self.num_particles > max {
                self.num_particles = max;
            }
            self.reset();
        }
    }

    /// Get the maximum sensible particle count for this grid size
    /// Allows up to 75% of grid area for dense patterns
    pub fn max_particles(&self) -> usize {
        let grid_area = self.grid_width * self.grid_height;
        (grid_area * 3 / 4).max(100) // 75% of grid, minimum 100
    }

    /// Adjust num_particles (clamped to 100 and grid-based max)
    pub fn adjust_particles(&mut self, delta: i32) {
        let max = self.max_particles() as i32;
        let new_val = (self.num_particles as i32 + delta).clamp(100, max) as usize;
        self.num_particles = new_val;
    }

    /// Adjust stickiness (clamped to 0.1-1.0)
    pub fn adjust_stickiness(&mut self, delta: f32) {
        self.stickiness = (self.stickiness + delta).clamp(0.1, 1.0);
    }

    /// Calculate fractal dimension using box-counting method
    /// Returns (dimension, r_squared) where r_squared indicates fit quality
    pub fn calculate_fractal_dimension(&self) -> (f32, f32) {
        if self.particles_stuck < 50 {
            return (0.0, 0.0); // Not enough data
        }

        let (cx, cy) = self.center();
        let cx = cx as usize;
        let cy = cy as usize;

        // Box sizes to sample (powers of 2)
        let box_sizes: Vec<usize> = vec![2, 4, 8, 16, 32, 64]
            .into_iter()
            .filter(|&s| s < self.grid_width.min(self.grid_height) / 2)
            .collect();

        if box_sizes.len() < 3 {
            return (0.0, 0.0);
        }

        let mut log_n: Vec<f32> = Vec::new();
        let mut log_r: Vec<f32> = Vec::new();

        for box_size in &box_sizes {
            let mut count = 0;
            let half = self.max_radius as usize + *box_size;

            let x_start = cx.saturating_sub(half);
            let x_end = (cx + half).min(self.grid_width);
            let y_start = cy.saturating_sub(half);
            let y_end = (cy + half).min(self.grid_height);

            for by in (y_start..y_end).step_by(*box_size) {
                for bx in (x_start..x_end).step_by(*box_size) {
                    // Check if any particle in this box
                    'box_check: for dy in 0..*box_size {
                        for dx in 0..*box_size {
                            let x = bx + dx;
                            let y = by + dy;
                            if x < self.grid_width && y < self.grid_height {
                                if self.grid[y * self.grid_width + x].is_some() {
                                    count += 1;
                                    break 'box_check;
                                }
                            }
                        }
                    }
                }
            }

            if count > 0 {
                log_n.push((count as f32).ln());
                log_r.push((1.0 / *box_size as f32).ln());
            }
        }

        // Linear regression to find slope (fractal dimension)
        if log_n.len() < 3 {
            return (0.0, 0.0);
        }

        let n = log_n.len() as f32;
        let sum_x: f32 = log_r.iter().sum();
        let sum_y: f32 = log_n.iter().sum();
        let sum_xy: f32 = log_r.iter().zip(log_n.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f32 = log_r.iter().map(|x| x * x).sum();

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            return (0.0, 0.0);
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;

        // Calculate R-squared
        let mean_y = sum_y / n;
        let ss_tot: f32 = log_n.iter().map(|y| (y - mean_y).powi(2)).sum();
        if ss_tot.abs() < 1e-10 {
            return (slope.abs(), 1.0);
        }

        let intercept = (sum_y - slope * sum_x) / n;
        let ss_res: f32 = log_r
            .iter()
            .zip(log_n.iter())
            .map(|(x, y)| {
                let predicted = slope * x + intercept;
                (y - predicted).powi(2)
            })
            .sum();
        let r_squared = (1.0 - ss_res / ss_tot).max(0.0);

        (slope.abs(), r_squared)
    }

}
