use crate::braille;
use crate::color::{ColorLut, ColorScheme};
use crate::simulation::{DlaSimulation, SeedPattern};

/// Focus state for parameter editing in the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    None,
    Stickiness,
    Particles,
    Seed,
    ColorScheme,
    Speed,
}

impl Focus {
    pub fn next(&self) -> Focus {
        match self {
            Focus::None => Focus::Stickiness,
            Focus::Stickiness => Focus::Particles,
            Focus::Particles => Focus::Seed,
            Focus::Seed => Focus::ColorScheme,
            Focus::ColorScheme => Focus::Speed,
            Focus::Speed => Focus::None,
        }
    }

    pub fn prev(&self) -> Focus {
        match self {
            Focus::None => Focus::Speed,
            Focus::Stickiness => Focus::None,
            Focus::Particles => Focus::Stickiness,
            Focus::Seed => Focus::Particles,
            Focus::ColorScheme => Focus::Seed,
            Focus::Speed => Focus::ColorScheme,
        }
    }
}

/// Main application state
pub struct App {
    pub simulation: DlaSimulation,
    pub color_scheme: ColorScheme,
    pub color_lut: ColorLut,
    pub color_by_age: bool,
    pub focus: Focus,
    pub fullscreen_mode: bool,
    pub steps_per_frame: usize,
    pub show_help: bool,
}

impl App {
    pub fn new(canvas_width: u16, canvas_height: u16) -> Self {
        let (sim_width, sim_height) = braille::calculate_simulation_size(canvas_width, canvas_height);
        let color_scheme = ColorScheme::default();
        Self {
            simulation: DlaSimulation::new(sim_width, sim_height),
            color_lut: color_scheme.build_lut(),
            color_scheme,
            color_by_age: true,
            focus: Focus::default(),
            fullscreen_mode: false,
            steps_per_frame: 5,
            show_help: false,
        }
    }

    /// Run simulation steps for current frame
    pub fn tick(&mut self) {
        if !self.simulation.paused {
            for _ in 0..self.steps_per_frame {
                if !self.simulation.step() {
                    break;
                }
            }
        }
    }

    /// Handle adjusting the currently focused parameter
    pub fn adjust_focused_up(&mut self) {
        match self.focus {
            Focus::None => {}
            Focus::Stickiness => self.simulation.adjust_stickiness(0.05),
            Focus::Particles => self.simulation.adjust_particles(500),
            Focus::Seed => {
                let new_pattern = self.simulation.seed_pattern.next();
                self.simulation.reset_with_seed(new_pattern);
            }
            Focus::ColorScheme => {
                self.color_scheme = self.color_scheme.next();
                self.color_lut = self.color_scheme.build_lut();
            }
            Focus::Speed => self.steps_per_frame = (self.steps_per_frame + 1).min(50),
        }
    }

    /// Handle adjusting the currently focused parameter
    pub fn adjust_focused_down(&mut self) {
        match self.focus {
            Focus::None => {}
            Focus::Stickiness => self.simulation.adjust_stickiness(-0.05),
            Focus::Particles => self.simulation.adjust_particles(-500),
            Focus::Seed => {
                let new_pattern = self.simulation.seed_pattern.prev();
                self.simulation.reset_with_seed(new_pattern);
            }
            Focus::ColorScheme => {
                self.color_scheme = self.color_scheme.prev();
                self.color_lut = self.color_scheme.build_lut();
            }
            Focus::Speed => self.steps_per_frame = (self.steps_per_frame.saturating_sub(1)).max(1),
        }
    }

    /// Cycle to next focus
    pub fn next_focus(&mut self) {
        self.focus = self.focus.next();
    }

    /// Cycle to previous focus
    pub fn prev_focus(&mut self) {
        self.focus = self.focus.prev();
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.simulation.toggle_pause();
    }

    /// Reset simulation
    pub fn reset(&mut self) {
        self.simulation.reset();
    }

    /// Set seed pattern directly (1-0 keys)
    pub fn set_seed_pattern(&mut self, pattern: SeedPattern) {
        self.simulation.reset_with_seed(pattern);
    }

    /// Toggle color-by-age mode
    pub fn toggle_color_by_age(&mut self) {
        self.color_by_age = !self.color_by_age;
    }

    /// Cycle color scheme
    pub fn cycle_color_scheme(&mut self) {
        self.color_scheme = self.color_scheme.next();
        self.color_lut = self.color_scheme.build_lut();
    }

    /// Toggle fullscreen mode
    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen_mode = !self.fullscreen_mode;
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Resize simulation to match new canvas size
    pub fn resize(&mut self, canvas_width: u16, canvas_height: u16) {
        let (sim_width, sim_height) = braille::calculate_simulation_size(canvas_width, canvas_height);
        self.simulation.resize(sim_width, sim_height);
    }

    /// Increase simulation speed
    pub fn increase_speed(&mut self) {
        self.steps_per_frame = (self.steps_per_frame + 1).min(50);
    }

    /// Decrease simulation speed
    pub fn decrease_speed(&mut self) {
        self.steps_per_frame = self.steps_per_frame.saturating_sub(1).max(1);
    }
}
