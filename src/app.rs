use crate::braille;
use crate::color::{ColorLut, ColorScheme};
use crate::simulation::{DlaSimulation, SeedPattern};

/// Popup menu state for Shift+letter parameter selection
#[derive(Debug, Clone)]
pub struct ParamPopup {
    pub letter: char,
    pub options: Vec<(Focus, &'static str)>, // (Focus variant, display name)
    pub selected_idx: usize,
}

/// Focus state for parameter editing in the sidebar
/// Alphabetically ordered for consistent UI display
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    None,
    // Alphabetical order
    Age,            // color by age toggle
    Boundary,
    ColorScheme,
    Direction,
    EscapeMult,
    Force,
    Highlight,
    Invert,
    MaxIterations,
    MinRadius,
    Mode,
    MultiContact,
    Neighborhood,
    Particles,
    RadialBias,
    Seed,
    SideSticky,
    Spawn,
    SpawnOffset,
    Speed,
    Stickiness,
    StickyGradient,
    TipSticky,
    WalkStep,
    // Controls box (not a param)
    Controls,
}

impl Focus {
    /// Tab cycles through parameters in alphabetical order
    pub fn next(&self) -> Focus {
        match self {
            Focus::None | Focus::Controls => Focus::Age,
            Focus::Age => Focus::Boundary,
            Focus::Boundary => Focus::ColorScheme,
            Focus::ColorScheme => Focus::Direction,
            Focus::Direction => Focus::EscapeMult,
            Focus::EscapeMult => Focus::Force,
            Focus::Force => Focus::Highlight,
            Focus::Highlight => Focus::Invert,
            Focus::Invert => Focus::MaxIterations,
            Focus::MaxIterations => Focus::MinRadius,
            Focus::MinRadius => Focus::Mode,
            Focus::Mode => Focus::MultiContact,
            Focus::MultiContact => Focus::Neighborhood,
            Focus::Neighborhood => Focus::Particles,
            Focus::Particles => Focus::RadialBias,
            Focus::RadialBias => Focus::Seed,
            Focus::Seed => Focus::SideSticky,
            Focus::SideSticky => Focus::Spawn,
            Focus::Spawn => Focus::SpawnOffset,
            Focus::SpawnOffset => Focus::Speed,
            Focus::Speed => Focus::Stickiness,
            Focus::Stickiness => Focus::StickyGradient,
            Focus::StickyGradient => Focus::TipSticky,
            Focus::TipSticky => Focus::WalkStep,
            Focus::WalkStep => Focus::Age, // Loop back
        }
    }

    /// Shift+Tab exits to Controls mode
    pub fn deselect(&self) -> Focus {
        Focus::Controls
    }

    /// Get the line index in the parameters box for this focus (alphabetical order)
    pub fn line_index(&self) -> u16 {
        match self {
            Focus::None | Focus::Controls => 0,
            Focus::Age => 0,
            Focus::Boundary => 1,
            Focus::ColorScheme => 2,
            Focus::Direction => 3,
            Focus::EscapeMult => 4,
            Focus::Force => 5,
            Focus::Highlight => 6,
            Focus::Invert => 7,
            Focus::MaxIterations => 8,
            Focus::MinRadius => 9,
            Focus::Mode => 10,
            Focus::MultiContact => 11,
            Focus::Neighborhood => 12,
            Focus::Particles => 13,
            Focus::RadialBias => 14,
            Focus::Seed => 15,
            Focus::SideSticky => 16,
            Focus::Spawn => 17,
            Focus::SpawnOffset => 18,
            Focus::Speed => 19,
            Focus::Stickiness => 20,
            Focus::StickyGradient => 21,
            Focus::TipSticky => 22,
            Focus::WalkStep => 23,
        }
    }

    /// Check if focus is on a parameter (not Controls or None)
    pub fn is_param(&self) -> bool {
        !matches!(self, Focus::None | Focus::Controls)
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
    pub help_scroll: u16,
    pub controls_scroll: u16,
    pub param_popup: Option<ParamPopup>,
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
            help_scroll: 0,
            controls_scroll: 0,
            param_popup: None,
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
            Focus::None | Focus::Controls => {}
            Focus::Age => self.toggle_color_by_age(),
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
            // Visual
            Focus::Mode => self.cycle_color_mode(),
            Focus::Highlight => self.adjust_highlight(5),
            Focus::Invert => self.toggle_invert_colors(),
            // Movement
            Focus::WalkStep => self.adjust_walk_step(0.5),
            Focus::Direction => self.simulation.settings.adjust_walk_bias_angle(15.0),
            Focus::Force => self.simulation.settings.adjust_walk_bias_strength(0.05),
            Focus::RadialBias => self.simulation.settings.adjust_radial_bias(0.05),
            // Sticking
            Focus::Neighborhood => self.cycle_neighborhood(),
            Focus::TipSticky => self.simulation.settings.adjust_tip_stickiness(0.1),
            Focus::SideSticky => self.simulation.settings.adjust_side_stickiness(0.1),
            Focus::MultiContact => self.simulation.settings.adjust_multi_contact_min(1),
            Focus::StickyGradient => self.simulation.settings.adjust_stickiness_gradient(0.1),
            // Spawn
            Focus::Spawn => self.cycle_spawn_mode(),
            Focus::Boundary => self.cycle_boundary(),
            Focus::SpawnOffset => self.simulation.settings.adjust_spawn_radius_offset(5.0),
            Focus::EscapeMult => self.simulation.settings.adjust_escape_multiplier(0.5),
            Focus::MinRadius => self.simulation.settings.adjust_min_spawn_radius(10.0),
            Focus::MaxIterations => self.simulation.settings.adjust_max_walk_iterations(1000),
        }
    }

    /// Handle adjusting the currently focused parameter
    pub fn adjust_focused_down(&mut self) {
        match self.focus {
            Focus::None | Focus::Controls => {}
            Focus::Age => self.toggle_color_by_age(),
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
            // Visual
            Focus::Mode => self.cycle_color_mode_prev(),
            Focus::Highlight => self.adjust_highlight(-5),
            Focus::Invert => self.toggle_invert_colors(),
            // Movement
            Focus::WalkStep => self.adjust_walk_step(-0.5),
            Focus::Direction => self.simulation.settings.adjust_walk_bias_angle(-15.0),
            Focus::Force => self.simulation.settings.adjust_walk_bias_strength(-0.05),
            Focus::RadialBias => self.simulation.settings.adjust_radial_bias(-0.05),
            // Sticking
            Focus::Neighborhood => self.cycle_neighborhood_prev(),
            Focus::TipSticky => self.simulation.settings.adjust_tip_stickiness(-0.1),
            Focus::SideSticky => self.simulation.settings.adjust_side_stickiness(-0.1),
            Focus::MultiContact => self.simulation.settings.adjust_multi_contact_min(-1),
            Focus::StickyGradient => self.simulation.settings.adjust_stickiness_gradient(-0.1),
            // Spawn
            Focus::Spawn => self.cycle_spawn_mode_prev(),
            Focus::Boundary => self.cycle_boundary_prev(),
            Focus::SpawnOffset => self.simulation.settings.adjust_spawn_radius_offset(-5.0),
            Focus::EscapeMult => self.simulation.settings.adjust_escape_multiplier(-0.5),
            Focus::MinRadius => self.simulation.settings.adjust_min_spawn_radius(-10.0),
            Focus::MaxIterations => self.simulation.settings.adjust_max_walk_iterations(-1000),
        }
    }

    /// Cycle to next focus
    pub fn next_focus(&mut self) {
        self.focus = self.focus.next();
    }

    /// Exit to Controls mode (Shift+Tab)
    pub fn prev_focus(&mut self) {
        self.focus = self.focus.deselect();
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
        if self.show_help {
            self.help_scroll = 0; // Reset scroll when opening
        }
    }

    /// Scroll help content up
    pub fn scroll_help_up(&mut self) {
        self.help_scroll = self.help_scroll.saturating_sub(1);
    }

    /// Scroll help content down
    pub fn scroll_help_down(&mut self, max_scroll: u16) {
        self.help_scroll = (self.help_scroll + 1).min(max_scroll);
    }

    /// Scroll controls box up
    pub fn scroll_controls_up(&mut self) {
        self.controls_scroll = self.controls_scroll.saturating_sub(1);
    }

    /// Scroll controls box down
    pub fn scroll_controls_down(&mut self, max_scroll: u16) {
        self.controls_scroll = (self.controls_scroll + 1).min(max_scroll);
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

    // === New settings methods ===

    /// Cycle through color modes
    pub fn cycle_color_mode(&mut self) {
        self.simulation.settings.color_mode = self.simulation.settings.color_mode.next();
    }

    /// Toggle invert colors
    pub fn toggle_invert_colors(&mut self) {
        self.simulation.settings.invert_colors = !self.simulation.settings.invert_colors;
    }

    /// Cycle through neighborhood types
    pub fn cycle_neighborhood(&mut self) {
        self.simulation.settings.neighborhood = self.simulation.settings.neighborhood.next();
    }

    /// Cycle through boundary behaviors
    pub fn cycle_boundary(&mut self) {
        self.simulation.settings.boundary_behavior = self.simulation.settings.boundary_behavior.next();
    }

    /// Cycle through spawn modes
    pub fn cycle_spawn_mode(&mut self) {
        self.simulation.settings.spawn_mode = self.simulation.settings.spawn_mode.next();
    }

    /// Adjust walk step size
    pub fn adjust_walk_step(&mut self, delta: f32) {
        self.simulation.settings.adjust_walk_step_size(delta);
    }

    /// Adjust highlight recent count
    pub fn adjust_highlight(&mut self, delta: i32) {
        self.simulation.settings.adjust_highlight_recent(delta);
    }

    /// Cycle color mode backward
    pub fn cycle_color_mode_prev(&mut self) {
        self.simulation.settings.color_mode = self.simulation.settings.color_mode.prev();
    }

    /// Cycle neighborhood backward
    pub fn cycle_neighborhood_prev(&mut self) {
        self.simulation.settings.neighborhood = self.simulation.settings.neighborhood.prev();
    }

    /// Cycle boundary backward
    pub fn cycle_boundary_prev(&mut self) {
        self.simulation.settings.boundary_behavior = self.simulation.settings.boundary_behavior.prev();
    }

    /// Cycle spawn mode backward
    pub fn cycle_spawn_mode_prev(&mut self) {
        self.simulation.settings.spawn_mode = self.simulation.settings.spawn_mode.prev();
    }

    // === Popup methods ===

    /// Get parameters that start with a given letter
    fn get_params_for_letter(letter: char) -> Vec<(Focus, &'static str)> {
        let letter = letter.to_ascii_lowercase();
        let all_params: &[(char, Focus, &str)] = &[
            ('a', Focus::Age, "Age (Color by)"),
            ('b', Focus::Boundary, "Boundary"),
            ('c', Focus::ColorScheme, "Color Scheme"),
            ('d', Focus::Direction, "Direction"),
            ('e', Focus::EscapeMult, "Escape Multiplier"),
            ('f', Focus::Force, "Force (Bias Strength)"),
            ('g', Focus::StickyGradient, "Gradient (Stickiness)"),
            ('h', Focus::Highlight, "Highlight"),
            ('i', Focus::Invert, "Invert"),
            ('m', Focus::Mode, "Mode (Color)"),
            ('m', Focus::MultiContact, "Multi-Contact Min"),
            ('m', Focus::MinRadius, "Min Spawn Radius"),
            ('m', Focus::MaxIterations, "Max Iterations"),
            ('n', Focus::Neighborhood, "Neighborhood"),
            ('o', Focus::SpawnOffset, "Offset (Spawn)"),
            ('p', Focus::Particles, "Particles"),
            ('r', Focus::RadialBias, "Radial Bias"),
            ('s', Focus::Stickiness, "Stickiness"),
            ('s', Focus::Seed, "Seed Pattern"),
            ('s', Focus::Speed, "Speed"),
            ('s', Focus::SideSticky, "Side Stickiness"),
            ('s', Focus::Spawn, "Spawn Mode"),
            ('t', Focus::TipSticky, "Tip Stickiness"),
            ('w', Focus::WalkStep, "Walk Step"),
        ];

        all_params
            .iter()
            .filter(|(c, _, _)| *c == letter)
            .map(|(_, focus, name)| (*focus, *name))
            .collect()
    }

    /// Open parameter popup for a given letter
    pub fn open_param_popup(&mut self, letter: char) {
        let options = Self::get_params_for_letter(letter);
        if !options.is_empty() {
            self.param_popup = Some(ParamPopup {
                letter: letter.to_ascii_uppercase(),
                options,
                selected_idx: 0,
            });
        }
    }

    /// Get all parameters in alphabetical order
    fn get_all_params() -> Vec<(Focus, &'static str)> {
        vec![
            (Focus::Age, "Age (Color by)"),
            (Focus::Boundary, "Boundary"),
            (Focus::ColorScheme, "Color Scheme"),
            (Focus::Direction, "Direction"),
            (Focus::EscapeMult, "Escape Multiplier"),
            (Focus::Force, "Force (Bias Strength)"),
            (Focus::StickyGradient, "Gradient (Stickiness)"),
            (Focus::Highlight, "Highlight"),
            (Focus::Invert, "Invert"),
            (Focus::MaxIterations, "Max Iterations"),
            (Focus::MinRadius, "Min Spawn Radius"),
            (Focus::Mode, "Mode (Color)"),
            (Focus::MultiContact, "Multi-Contact Min"),
            (Focus::Neighborhood, "Neighborhood"),
            (Focus::SpawnOffset, "Offset (Spawn)"),
            (Focus::Particles, "Particles"),
            (Focus::RadialBias, "Radial Bias"),
            (Focus::Seed, "Seed Pattern"),
            (Focus::SideSticky, "Side Stickiness"),
            (Focus::Spawn, "Spawn Mode"),
            (Focus::Speed, "Speed"),
            (Focus::Stickiness, "Stickiness"),
            (Focus::TipSticky, "Tip Stickiness"),
            (Focus::WalkStep, "Walk Step"),
        ]
    }

    /// Open popup with all parameters (Shift+?)
    pub fn open_all_params_popup(&mut self) {
        self.param_popup = Some(ParamPopup {
            letter: '?',
            options: Self::get_all_params(),
            selected_idx: 0,
        });
    }

    /// Close the parameter popup without selecting
    pub fn close_param_popup(&mut self) {
        self.param_popup = None;
    }

    /// Confirm selection and close popup
    pub fn confirm_param_popup(&mut self) {
        if let Some(popup) = &self.param_popup {
            if let Some((focus, _)) = popup.options.get(popup.selected_idx) {
                self.focus = *focus;
            }
        }
        self.param_popup = None;
    }

    /// Navigate up in popup
    pub fn popup_nav_up(&mut self) {
        if let Some(popup) = &mut self.param_popup {
            if popup.selected_idx > 0 {
                popup.selected_idx -= 1;
            } else {
                popup.selected_idx = popup.options.len().saturating_sub(1);
            }
        }
    }

    /// Navigate down in popup
    pub fn popup_nav_down(&mut self) {
        if let Some(popup) = &mut self.param_popup {
            if popup.selected_idx < popup.options.len().saturating_sub(1) {
                popup.selected_idx += 1;
            } else {
                popup.selected_idx = 0;
            }
        }
    }
}
