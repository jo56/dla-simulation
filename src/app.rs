use crate::braille;
use crate::color::{ColorLut, ColorScheme};
use crate::config::AppConfig;
use crate::recorder::Recorder;
use crate::simulation::{DlaSimulation, SeedPattern};
use std::path::Path;

/// Popup menu state for Shift+letter parameter selection
#[derive(Debug, Clone)]
pub struct ParamPopup {
    pub options: Vec<(Focus, &'static str)>, // (Focus variant, display name)
    pub selected_idx: usize,
}

/// Text input popup state for filename entry
#[derive(Debug, Clone)]
pub struct TextInputPopup {
    pub title: &'static str,
    pub input: String,
    pub cursor_pos: usize,
}

impl TextInputPopup {
    pub fn new(title: &'static str, default_value: &str) -> Self {
        let input = default_value.to_string();
        let cursor_pos = input.len();
        Self {
            title,
            input,
            cursor_pos,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        self.cursor_pos = (self.cursor_pos + 1).min(self.input.len());
    }
}

/// Focus state for parameter editing in the sidebar
/// Navigation follows grouped order: Movement → Sticking → Spawn → Visual
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Focus {
    #[default]
    None,
    // Parameters (enum variants kept alphabetical, navigation order defined in next/prev)
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
    /// Navigate to next parameter (grouped, alphabetical within groups)
    pub fn next(&self) -> Focus {
        match self {
            Focus::None | Focus::Controls => Focus::Direction,
            // Movement (direction, force, radial, walk)
            Focus::Direction => Focus::Force,
            Focus::Force => Focus::RadialBias,
            Focus::RadialBias => Focus::WalkStep,
            // Sticking (contacts, gradient, neighbors, sidestick, sticky, tipstick)
            Focus::WalkStep => Focus::MultiContact,
            Focus::MultiContact => Focus::StickyGradient,
            Focus::StickyGradient => Focus::Neighborhood,
            Focus::Neighborhood => Focus::SideSticky,
            Focus::SideSticky => Focus::Stickiness,
            Focus::Stickiness => Focus::TipSticky,
            // Spawn (bound, escape, maxsteps, minradius, spawn, spawnoff)
            Focus::TipSticky => Focus::Boundary,
            Focus::Boundary => Focus::EscapeMult,
            Focus::EscapeMult => Focus::MaxIterations,
            Focus::MaxIterations => Focus::MinRadius,
            Focus::MinRadius => Focus::Spawn,
            Focus::Spawn => Focus::SpawnOffset,
            // Visual (age, color, highlight, invert, mode, particles, seed, speed)
            Focus::SpawnOffset => Focus::Age,
            Focus::Age => Focus::ColorScheme,
            Focus::ColorScheme => Focus::Highlight,
            Focus::Highlight => Focus::Invert,
            Focus::Invert => Focus::Mode,
            Focus::Mode => Focus::Particles,
            Focus::Particles => Focus::Seed,
            Focus::Seed => Focus::Speed,
            Focus::Speed => Focus::Speed, // Stop at boundary
        }
    }

    /// Navigate to previous parameter (grouped, alphabetical within groups)
    pub fn prev(&self) -> Focus {
        match self {
            Focus::None | Focus::Controls => Focus::Speed,
            // Movement (direction, force, radial, walk)
            Focus::Direction => Focus::Direction, // Stop at boundary
            Focus::Force => Focus::Direction,
            Focus::RadialBias => Focus::Force,
            Focus::WalkStep => Focus::RadialBias,
            // Sticking (contacts, gradient, neighbors, sidestick, sticky, tipstick)
            Focus::MultiContact => Focus::WalkStep,
            Focus::StickyGradient => Focus::MultiContact,
            Focus::Neighborhood => Focus::StickyGradient,
            Focus::SideSticky => Focus::Neighborhood,
            Focus::Stickiness => Focus::SideSticky,
            Focus::TipSticky => Focus::Stickiness,
            // Spawn (bound, escape, maxsteps, minradius, spawn, spawnoff)
            Focus::Boundary => Focus::TipSticky,
            Focus::EscapeMult => Focus::Boundary,
            Focus::MaxIterations => Focus::EscapeMult,
            Focus::MinRadius => Focus::MaxIterations,
            Focus::Spawn => Focus::MinRadius,
            Focus::SpawnOffset => Focus::Spawn,
            // Visual (age, color, highlight, invert, mode, particles, seed, speed)
            Focus::Age => Focus::SpawnOffset,
            Focus::ColorScheme => Focus::Age,
            Focus::Highlight => Focus::ColorScheme,
            Focus::Invert => Focus::Highlight,
            Focus::Mode => Focus::Invert,
            Focus::Particles => Focus::Mode,
            Focus::Seed => Focus::Particles,
            Focus::Speed => Focus::Seed,
        }
    }

    /// Get the line index in the parameters box for this focus (grouped, alphabetical within)
    pub fn line_index(&self) -> u16 {
        // Line indices account for section headers:
        // 0: -- movement --
        // 1-4: direction, force, radial, walk
        // 5: -- sticking --
        // 6-11: contacts, gradient, neighbors, sidestick, sticky, tipstick
        // 12: -- spawn --
        // 13-18: bound, escape, maxsteps, minradius, spawn, spawnoff
        // 19: -- visual --
        // 20-27: age, color, highlight, invert, mode, particles, seed, speed
        match self {
            Focus::None | Focus::Controls => 0,
            // Movement (after header at line 0)
            Focus::Direction => 1,
            Focus::Force => 2,
            Focus::RadialBias => 3,
            Focus::WalkStep => 4,
            // Sticking (after header at line 5)
            Focus::MultiContact => 6,
            Focus::StickyGradient => 7,
            Focus::Neighborhood => 8,
            Focus::SideSticky => 9,
            Focus::Stickiness => 10,
            Focus::TipSticky => 11,
            // Spawn (after header at line 12)
            Focus::Boundary => 13,
            Focus::EscapeMult => 14,
            Focus::MaxIterations => 15,
            Focus::MinRadius => 16,
            Focus::Spawn => 17,
            Focus::SpawnOffset => 18,
            // Visual (after header at line 19)
            Focus::Age => 20,
            Focus::ColorScheme => 21,
            Focus::Highlight => 22,
            Focus::Invert => 23,
            Focus::Mode => 24,
            Focus::Particles => 25,
            Focus::Seed => 26,
            Focus::Speed => 27,
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
    pub export_popup: Option<TextInputPopup>,
    pub export_result: Option<Result<String, String>>,
    // Recording state
    pub recorder: Recorder,
    pub recording_popup: Option<TextInputPopup>,
    pub recording_result: Option<Result<String, String>>,
    /// Tracks if simulation was paused before opening recording popup
    pub recording_was_paused: bool,
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
            focus: Focus::Controls,
            fullscreen_mode: false,
            steps_per_frame: 5,
            show_help: false,
            help_scroll: 0,
            controls_scroll: 0,
            param_popup: None,
            export_popup: None,
            export_result: None,
            recorder: Recorder::new(),
            recording_popup: None,
            recording_result: None,
            recording_was_paused: false,
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

    /// Navigate to previous parameter (Shift+Tab)
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
            ('m', Focus::MaxIterations, "Max Steps"),
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
            (Focus::MaxIterations, "Max Steps"),
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

    // === Export popup methods ===

    /// Open export popup with default filename
    pub fn open_export_popup(&mut self) {
        self.export_popup = Some(TextInputPopup::new(" Export Config ", "dla-config.json"));
    }

    /// Close export popup without saving
    pub fn close_export_popup(&mut self) {
        self.export_popup = None;
    }

    /// Confirm export and save file
    pub fn confirm_export(&mut self) {
        if let Some(popup) = &self.export_popup {
            let config = self.to_config();
            let path = Path::new(&popup.input);
            self.export_result = Some(config.save_to_file(path).map(|_| popup.input.clone()));
        }
        self.export_popup = None;
    }

    /// Clear export result (call after displaying it)
    pub fn clear_export_result(&mut self) {
        self.export_result = None;
    }

    /// Create AppConfig from current state
    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            version: 1,
            settings: self.simulation.settings.clone(),
            seed_pattern: self.simulation.seed_pattern,
            stickiness: self.simulation.stickiness,
            num_particles: self.simulation.num_particles,
            color_scheme: self.color_scheme,
            steps_per_frame: self.steps_per_frame,
            color_by_age: self.color_by_age,
        }
    }

    /// Apply AppConfig to current state
    pub fn apply_config(&mut self, config: &AppConfig) {
        self.simulation.settings = config.settings.clone();
        self.simulation.seed_pattern = config.seed_pattern;
        self.simulation.stickiness = config.stickiness;
        self.simulation.num_particles = config.num_particles;
        self.color_scheme = config.color_scheme;
        self.color_lut = self.color_scheme.build_lut();
        self.steps_per_frame = config.steps_per_frame;
        self.color_by_age = config.color_by_age;
    }

    // === Recording methods ===

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    /// Open recording popup with default filename
    /// Pauses the simulation while popup is open
    pub fn open_recording_popup(&mut self) {
        // Save current pause state and pause the simulation
        self.recording_was_paused = self.simulation.paused;
        self.simulation.paused = true;

        self.recording_popup = Some(TextInputPopup::new(
            " Start Recording ",
            "dla_recording.mp4",
        ));
    }

    /// Close recording popup without starting
    /// Restores previous pause state
    pub fn close_recording_popup(&mut self) {
        self.recording_popup = None;
        // Restore previous pause state
        self.simulation.paused = self.recording_was_paused;
    }

    /// Start recording with the given filename
    pub fn start_recording(&mut self, filename: String) -> Result<(), String> {
        self.recorder.start(
            filename,
            self.simulation.grid_width,
            self.simulation.grid_height,
        )
    }

    /// Stop recording and save the file
    pub fn stop_recording(&mut self) -> Result<String, String> {
        self.recorder.stop()
    }

    /// Capture a recording frame if recording and ready
    pub fn capture_recording_frame(&mut self) {
        if self.recorder.is_recording() && self.recorder.should_capture() {
            let color_mode = self.simulation.settings.color_mode;
            let invert = self.simulation.settings.invert_colors;
            if let Err(e) = self.recorder.capture_frame(
                &self.simulation,
                &self.color_scheme,
                self.color_by_age,
                color_mode,
                invert,
            ) {
                // Store error and stop recording
                self.recording_result = Some(Err(e));
                let _ = self.recorder.stop();
            }
        }
    }

    /// Clear recording result (call after displaying it)
    pub fn clear_recording_result(&mut self) {
        self.recording_result = None;
    }
}
