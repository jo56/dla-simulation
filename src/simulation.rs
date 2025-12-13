use rand::rngs::ThreadRng;
use rand::Rng;

// Simulation constants
const WALK_STEP_SIZE: f32 = 2.0;
const MAX_WALK_ITERATIONS: usize = 10000;
const SPAWN_RADIUS_OFFSET: f32 = 10.0;
const MIN_SPAWN_RADIUS: f32 = 50.0;
const ESCAPE_MULTIPLIER_SQ: f32 = 4.0; // 2.0 squared, for distance comparisons
const BOUNDARY_MARGIN: f32 = 1.0;

/// Seed pattern types for initial structure
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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

/// DLA simulation state
pub struct DlaSimulation {
    pub grid_width: usize,
    pub grid_height: usize,
    grid: Vec<Option<usize>>,
    pub num_particles: usize,
    pub stickiness: f32,
    pub particles_stuck: usize,
    max_radius: f32,
    pub paused: bool,
    pub seed_pattern: SeedPattern,
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

        // Spawn radius - outside the structure
        let spawn_radius = (self.max_radius + SPAWN_RADIUS_OFFSET).max(MIN_SPAWN_RADIUS);

        // Pre-calculate squared escape distance (avoids sqrt in hot loop)
        let escape_dist_sq = spawn_radius * spawn_radius * ESCAPE_MULTIPLIER_SQ;

        // Pre-calculate boundary limits (avoids repeated subtraction)
        let x_max = self.grid_width as f32 - BOUNDARY_MARGIN - 1.0;
        let y_max = self.grid_height as f32 - BOUNDARY_MARGIN - 1.0;

        // Spawn particle on a circle
        let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
        let mut x = center_x + spawn_radius * angle.cos();
        let mut y = center_y + spawn_radius * angle.sin();

        // Random walk until it sticks or escapes
        for _ in 0..MAX_WALK_ITERATIONS {
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
                let mut should_stick = false;

                // Check 8 neighbors
                'outer: for ndy in -1..=1i32 {
                    for ndx in -1..=1i32 {
                        if ndx == 0 && ndy == 0 {
                            continue;
                        }

                        let nx = (ix as i32 + ndx) as usize;
                        let ny = (iy as i32 + ndy) as usize;
                        let nidx = ny * self.grid_width + nx;

                        if self.grid[nidx].is_some() {
                            // Neighbor is stuck, maybe stick here
                            if self.rng.gen::<f32>() < self.stickiness {
                                should_stick = true;
                                break 'outer;
                            }
                        }
                    }
                }

                if should_stick {
                    // Stick here
                    let idx = iy * self.grid_width + ix;
                    self.grid[idx] = Some(self.particles_stuck);
                    self.particles_stuck += 1;

                    // Update max radius (need actual distance for spawn radius calculation)
                    let dx = ix as f32 - center_x;
                    let dy = iy as f32 - center_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    self.max_radius = self.max_radius.max(dist);

                    return true;
                }
            }

            // Random walk step
            let walk_angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            x += WALK_STEP_SIZE * walk_angle.cos();
            y += WALK_STEP_SIZE * walk_angle.sin();

            // Clamp to bounds
            x = x.clamp(BOUNDARY_MARGIN, x_max);
            y = y.clamp(BOUNDARY_MARGIN, y_max);
        }

        true
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

    /// Single center point seed
    fn seed_point(&mut self) {
        let center_idx = self.grid_height / 2 * self.grid_width + self.grid_width / 2;
        self.grid[center_idx] = Some(0);
        self.particles_stuck = 1;
        self.max_radius = 1.0;
    }

    /// Horizontal line seed
    fn seed_line(&mut self) {
        let cy = self.grid_height / 2;
        let half_len = 20.min(self.grid_width / 4);
        let start_x = self.grid_width / 2 - half_len;
        let end_x = self.grid_width / 2 + half_len;
        for x in start_x..end_x {
            self.grid[cy * self.grid_width + x] = Some(0);
        }
        self.particles_stuck = end_x - start_x;
        self.max_radius = half_len as f32;
    }

    /// Cross-shaped seed
    fn seed_cross(&mut self) {
        let cx = self.grid_width / 2;
        let cy = self.grid_height / 2;
        let arm_len = 10.min(self.grid_width / 8).min(self.grid_height / 8);
        let mut count = 0;
        for i in 0..arm_len {
            if cx >= i && cy >= i {
                self.grid[cy * self.grid_width + (cx - i)] = Some(0);
                self.grid[cy * self.grid_width + (cx + i)] = Some(0);
                self.grid[(cy - i) * self.grid_width + cx] = Some(0);
                self.grid[(cy + i) * self.grid_width + cx] = Some(0);
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
        let mut count = 0;
        for angle_deg in 0..360 {
            let angle = (angle_deg as f32).to_radians();
            let x = (cx + radius * angle.cos()) as usize;
            let y = (cy + radius * angle.sin()) as usize;
            if x < self.grid_width && y < self.grid_height {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(0);
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
        let mut count = 0;

        for y in 0..self.grid_height {
            for x in 0..self.grid_width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if (dist >= radius - thickness) && (dist <= radius + thickness) {
                    let idx = y * self.grid_width + x;
                    if self.grid[idx].is_none() {
                        self.grid[idx] = Some(0);
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
        let mut count = 0;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(0);
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
                            self.grid[idx] = Some(0);
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
            self.grid[idx] = Some(0);
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
        let mut count = 0;

        for _ in 0..num_seeds {
            let angle = self.rng.gen_range(0.0..std::f32::consts::TAU);
            let r = self.rng.gen_range(0.0..scatter_radius as f32);
            let x = (cx as f32 + r * angle.cos()) as usize;
            let y = (cy as f32 + r * angle.sin()) as usize;
            if x < self.grid_width && y < self.grid_height {
                let idx = y * self.grid_width + x;
                if self.grid[idx].is_none() {
                    self.grid[idx] = Some(0);
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
                    self.grid[idx] = Some(0);
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
        let mut count = 0;

        // Central hub
        let hub_x = cx as usize;
        let hub_y = cy as usize;
        let hub_idx = hub_y * self.grid_width + hub_x;
        if self.grid[hub_idx].is_none() {
            self.grid[hub_idx] = Some(0);
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
                        self.grid[idx] = Some(0);
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
                    self.grid[idx] = Some(0);
                    count += 1;
                }
            }
        }

        self.particles_stuck = count;
        self.max_radius = rim_radius;
    }

    /// Get cell state at (x, y)
    pub fn get_cell(&self, x: usize, y: usize) -> Option<usize> {
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
    /// DLA patterns are sparse fractals, so ~20% of grid area is a reasonable max
    pub fn max_particles(&self) -> usize {
        let grid_area = self.grid_width * self.grid_height;
        (grid_area / 5).max(100) // 20% of grid, minimum 100
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
}
