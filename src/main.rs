mod app;
mod braille;
mod color;
mod config;
mod presets;
mod recorder;
mod settings;
mod simulation;
mod ui;

use app::{App, Focus};
use clap::{CommandFactory, FromArgMatches, Parser};
use config::AppConfig;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use settings::{BoundaryBehavior, ColorMode, NeighborhoodType, SpawnMode};
use simulation::SeedPattern;
use std::io;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "dla-simulation")]
#[command(about = "Diffusion-Limited Aggregation simulation in the terminal")]
struct Args {
    /// Load settings from a config file (CLI args override config values)
    #[arg(long, value_name = "FILE")]
    config: Option<String>,

    // === Basic Parameters ===
    /// Number of particles to simulate (auto-capped to ~20% of grid area)
    #[arg(short = 'p', long, default_value = "5000")]
    particles: usize,

    /// Base stickiness factor (0.1-1.0)
    #[arg(short = 's', long, default_value = "1.0")]
    stickiness: f32,

    /// Initial seed pattern (point, line, cross, circle, ring, block, noise, scatter, multipoint, starburst)
    #[arg(long, default_value = "point")]
    seed: String,

    /// Simulation speed (steps per frame, 1-50)
    #[arg(long, default_value = "5")]
    speed: usize,

    // === Movement Parameters ===
    /// Walk step size per random walk iteration (0.5-5.0)
    #[arg(long = "walk-step", default_value = "2.0")]
    walk_step: f32,

    /// Walk bias angle in degrees (0-360)
    #[arg(long = "walk-angle", default_value = "0.0")]
    walk_angle: f32,

    /// Walk bias strength (0.0-0.5, 0 = isotropic)
    #[arg(long = "walk-force", default_value = "0.0")]
    walk_force: f32,

    /// Radial bias (-0.3 to 0.3, negative = outward, positive = inward)
    #[arg(long = "radial-bias", default_value = "0.0")]
    radial_bias: f32,

    // === Sticking Parameters ===
    /// Neighborhood type for sticking checks (vonneumann, moore, extended)
    #[arg(long, default_value = "moore")]
    neighborhood: String,

    /// Minimum neighbors required to stick (1-4)
    #[arg(long = "multi-contact", default_value = "1")]
    multi_contact: u8,

    /// Stickiness at branch tips (0.1-1.0)
    #[arg(long = "tip-stickiness", default_value = "1.0")]
    tip_stickiness: f32,

    /// Stickiness on branch sides (0.1-1.0)
    #[arg(long = "side-stickiness", default_value = "1.0")]
    side_stickiness: f32,

    /// Stickiness variation by distance from center (-0.5 to 0.5)
    #[arg(long = "stickiness-gradient", default_value = "0.0")]
    stickiness_gradient: f32,

    // === Spawn/Boundary Parameters ===
    /// Spawn mode (circle, edges, corners, random, top, bottom, left, right)
    #[arg(long = "spawn-mode", default_value = "circle")]
    spawn_mode: String,

    /// Boundary behavior (clamp, wrap, bounce, stick, absorb)
    #[arg(long, default_value = "clamp")]
    boundary: String,

    /// Buffer distance between structure and spawn circle (5-50)
    #[arg(long = "spawn-offset", default_value = "10.0")]
    spawn_offset: f32,

    /// Multiplier for escape/respawn distance (2.0-6.0)
    #[arg(long = "escape-mult", default_value = "2.0")]
    escape_mult: f32,

    /// Minimum spawn radius (20-100)
    #[arg(long = "min-radius", default_value = "50.0")]
    min_radius: f32,

    /// Maximum walk iterations before respawn (1000-50000)
    #[arg(long = "max-iterations", default_value = "10000")]
    max_iterations: usize,

    // === Visual Parameters ===
    /// Color mode (age, distance, density, direction)
    #[arg(long = "color-mode", default_value = "age")]
    color_mode: String,

    /// Number of recent particles to highlight (0-50)
    #[arg(long, default_value = "0")]
    highlight: usize,

    /// Invert color gradient
    #[arg(long, default_value = "false")]
    invert: bool,
}

fn parse_neighborhood(s: &str) -> NeighborhoodType {
    match s.to_lowercase().as_str() {
        "vonneumann" | "von-neumann" | "vn" | "4" => NeighborhoodType::VonNeumann,
        "extended" | "ext" | "24" => NeighborhoodType::Extended,
        _ => NeighborhoodType::Moore,
    }
}

fn parse_spawn_mode(s: &str) -> SpawnMode {
    match s.to_lowercase().as_str() {
        "edges" | "edge" => SpawnMode::Edges,
        "corners" | "corner" => SpawnMode::Corners,
        "random" | "rand" => SpawnMode::Random,
        "top" => SpawnMode::Top,
        "bottom" => SpawnMode::Bottom,
        "left" => SpawnMode::Left,
        "right" => SpawnMode::Right,
        _ => SpawnMode::Circle,
    }
}

fn parse_boundary(s: &str) -> BoundaryBehavior {
    match s.to_lowercase().as_str() {
        "wrap" | "toroidal" => BoundaryBehavior::Wrap,
        "bounce" | "reflect" => BoundaryBehavior::Bounce,
        "stick" => BoundaryBehavior::Stick,
        "absorb" | "respawn" => BoundaryBehavior::Absorb,
        _ => BoundaryBehavior::Clamp,
    }
}

fn parse_color_mode(s: &str) -> ColorMode {
    match s.to_lowercase().as_str() {
        "distance" | "dist" => ColorMode::Distance,
        "density" | "dens" => ColorMode::Density,
        "direction" | "dir" => ColorMode::Direction,
        _ => ColorMode::Age,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use clap::parser::ValueSource;

    // Parse args and get matches for value_source checking
    let matches = Args::command().get_matches();
    let args = Args::from_arg_matches(&matches)?;

    // Load config file if specified
    let base_config = if let Some(config_path) = &args.config {
        match AppConfig::load_from_file(std::path::Path::new(config_path)) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                eprintln!("Warning: Failed to load config file: {}", e);
                eprintln!("Using default settings.");
                None
            }
        }
    } else {
        None
    };

    // Helper to check if a CLI arg was explicitly provided
    let is_explicit = |name: &str| -> bool {
        matches.value_source(name) == Some(ValueSource::CommandLine)
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get initial terminal size and create app
    let size = terminal.size()?;
    let frame_rect = ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: size.width,
        height: size.height,
    };
    let (canvas_width, canvas_height) = ui::get_canvas_size(frame_rect, false);
    let mut app = App::new(canvas_width, canvas_height);

    // Apply config file settings first (if loaded)
    if let Some(cfg) = &base_config {
        app.apply_config(cfg);
    }

    // Apply CLI args - only if explicitly provided (override config) or no config loaded
    let max_particles = app.simulation.max_particles();

    // Basic params
    if is_explicit("particles") || base_config.is_none() {
        app.simulation.num_particles = args.particles.clamp(100, max_particles);
    } else {
        // Cap config value to max_particles
        app.simulation.num_particles = app.simulation.num_particles.min(max_particles);
    }
    if is_explicit("stickiness") || base_config.is_none() {
        app.simulation.stickiness = args.stickiness.clamp(0.1, 1.0);
    }
    if is_explicit("speed") || base_config.is_none() {
        app.steps_per_frame = args.speed.clamp(1, 50);
    }

    // Movement settings
    if is_explicit("walk_step") || base_config.is_none() {
        app.simulation.settings.walk_step_size = args.walk_step.clamp(0.5, 5.0);
    }
    if is_explicit("walk_angle") || base_config.is_none() {
        app.simulation.settings.walk_bias_angle = args.walk_angle.clamp(0.0, 360.0);
    }
    if is_explicit("walk_force") || base_config.is_none() {
        app.simulation.settings.walk_bias_strength = args.walk_force.clamp(0.0, 0.5);
    }
    if is_explicit("radial_bias") || base_config.is_none() {
        app.simulation.settings.radial_bias = args.radial_bias.clamp(-0.3, 0.3);
    }

    // Sticking settings
    if is_explicit("neighborhood") || base_config.is_none() {
        app.simulation.settings.neighborhood = parse_neighborhood(&args.neighborhood);
    }
    if is_explicit("multi_contact") || base_config.is_none() {
        app.simulation.settings.multi_contact_min = args.multi_contact.clamp(1, 4);
    }
    if is_explicit("tip_stickiness") || base_config.is_none() {
        app.simulation.settings.tip_stickiness = args.tip_stickiness.clamp(0.1, 1.0);
    }
    if is_explicit("side_stickiness") || base_config.is_none() {
        app.simulation.settings.side_stickiness = args.side_stickiness.clamp(0.1, 1.0);
    }
    if is_explicit("stickiness_gradient") || base_config.is_none() {
        app.simulation.settings.stickiness_gradient = args.stickiness_gradient.clamp(-0.5, 0.5);
    }

    // Spawn/boundary settings
    if is_explicit("spawn_mode") || base_config.is_none() {
        app.simulation.settings.spawn_mode = parse_spawn_mode(&args.spawn_mode);
    }
    if is_explicit("boundary") || base_config.is_none() {
        app.simulation.settings.boundary_behavior = parse_boundary(&args.boundary);
    }
    if is_explicit("spawn_offset") || base_config.is_none() {
        app.simulation.settings.spawn_radius_offset = args.spawn_offset.clamp(5.0, 50.0);
    }
    if is_explicit("escape_mult") || base_config.is_none() {
        app.simulation.settings.escape_multiplier = args.escape_mult.clamp(2.0, 6.0);
    }
    if is_explicit("min_radius") || base_config.is_none() {
        app.simulation.settings.min_spawn_radius = args.min_radius.clamp(20.0, 100.0);
    }
    if is_explicit("max_iterations") || base_config.is_none() {
        app.simulation.settings.max_walk_iterations = args.max_iterations.clamp(1000, 50000);
    }

    // Visual settings
    if is_explicit("color_mode") || base_config.is_none() {
        app.simulation.settings.color_mode = parse_color_mode(&args.color_mode);
    }
    if is_explicit("highlight") || base_config.is_none() {
        app.simulation.settings.highlight_recent = args.highlight.clamp(0, 50);
    }
    if is_explicit("invert") || base_config.is_none() {
        app.simulation.settings.invert_colors = args.invert;
    }

    // Determine seed pattern - CLI overrides config
    let seed_pattern = if is_explicit("seed") || base_config.is_none() {
        match args.seed.to_lowercase().as_str() {
            "line" => SeedPattern::Line,
            "cross" => SeedPattern::Cross,
            "circle" => SeedPattern::Circle,
            "ring" => SeedPattern::Ring,
            "block" | "filled" => SeedPattern::Block,
            "noise" | "noise-patch" => SeedPattern::NoisePatch,
            "scatter" => SeedPattern::Scatter,
            "multipoint" | "multi-point" => SeedPattern::MultiPoint,
            "starburst" | "spokes" | "star" => SeedPattern::Starburst,
            _ => SeedPattern::Point,
        }
    } else {
        app.simulation.seed_pattern
    };

    // Reset with seed pattern (must come after settings are applied)
    app.simulation.reset_with_seed(seed_pattern);

    // Run the app
    let res = run_app(&mut terminal, &mut app);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    // Target ~60fps for smooth animation
    const FRAME_DURATION: Duration = Duration::from_millis(16);

    loop {
        // Render current state
        terminal.draw(|frame| ui::render(frame, app))?;

        // Capture recording frame if recording
        app.capture_recording_frame();

        // Poll for events with timeout
        if event::poll(FRAME_DURATION)? {
            match event::read()? {
                Event::Key(key) => {
                    // Only process Press events
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Handle Ctrl+C
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        return Ok(());
                    }

                    // === Handle popup keys first (if popup is open) ===
                    if app.param_popup.is_some() {
                        match key.code {
                            KeyCode::Up => app.popup_nav_up(),
                            KeyCode::Down => app.popup_nav_down(),
                            KeyCode::Enter => app.confirm_param_popup(),
                            KeyCode::Esc => app.close_param_popup(),
                            KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                                app.popup_jump_to_letter(c);
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // === Handle export popup keys (if export popup is open) ===
                    if app.export_popup.is_some() {
                        match key.code {
                            KeyCode::Enter => app.confirm_export(),
                            KeyCode::Esc => app.close_export_popup(),
                            KeyCode::Backspace => {
                                if let Some(popup) = &mut app.export_popup {
                                    popup.delete_char();
                                }
                            }
                            KeyCode::Left => {
                                if let Some(popup) = &mut app.export_popup {
                                    popup.move_cursor_left();
                                }
                            }
                            KeyCode::Right => {
                                if let Some(popup) = &mut app.export_popup {
                                    popup.move_cursor_right();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(popup) = &mut app.export_popup {
                                    popup.insert_char(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // === Handle recording popup keys (if recording popup is open) ===
                    if app.recording_popup.is_some() {
                        match key.code {
                            KeyCode::Enter => {
                                if let Some(popup) = app.recording_popup.take() {
                                    // Restore previous pause state before starting recording
                                    let was_paused = app.recording_was_paused;
                                    match app.start_recording(popup.input) {
                                        Ok(()) => {
                                            // Resume simulation if it wasn't paused before
                                            app.simulation.paused = was_paused;
                                        }
                                        Err(e) => {
                                            app.recording_result = Some(Err(e));
                                            // Still restore pause state on error
                                            app.simulation.paused = was_paused;
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => app.close_recording_popup(),
                            KeyCode::Backspace => {
                                if let Some(popup) = &mut app.recording_popup {
                                    popup.delete_char();
                                }
                            }
                            KeyCode::Left => {
                                if let Some(popup) = &mut app.recording_popup {
                                    popup.move_cursor_left();
                                }
                            }
                            KeyCode::Right => {
                                if let Some(popup) = &mut app.recording_popup {
                                    popup.move_cursor_right();
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(popup) = &mut app.recording_popup {
                                    popup.insert_char(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Clear export result on any key press
                    if app.export_result.is_some() {
                        app.clear_export_result();
                    }

                    // Clear recording result on any key press
                    if app.recording_result.is_some() {
                        app.clear_recording_result();
                    }

                    // === Handle Shift+letter to open popup ===
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if let KeyCode::Char(c) = key.code {
                            // Shift+? opens all params popup
                            if c == '?' {
                                app.open_all_params_popup();
                                continue;
                            }
                            // Shift+X opens export popup
                            if c == 'X' || c == 'x' {
                                app.open_export_popup();
                                continue;
                            }
                            // Shift+W directly adjusts walk step (since w is now navigation)
                            if c == 'W' || c == 'w' {
                                app.adjust_walk_step(0.5);
                                app.focus = Focus::WalkStep;
                                continue;
                            }
                            // Shift+E directly adjusts walk step down
                            if c == 'E' || c == 'e' {
                                app.adjust_walk_step(-0.5);
                                app.focus = Focus::WalkStep;
                                continue;
                            }
                            // Shift+letter opens popup for that letter
                            if c.is_ascii_alphabetic() {
                                app.open_param_popup(c);
                                continue;
                            }
                        }
                    }

                    // === Process normal key events ===
                    match key.code {
                        // System controls
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('r') | KeyCode::Char('R') => app.reset(),
                        KeyCode::Char('v') | KeyCode::Char('V') => app.toggle_fullscreen(),
                        KeyCode::Char('h') | KeyCode::Char('H') => app.toggle_help(),
                        // Recording toggle (backtick)
                        KeyCode::Char('`') => {
                            if app.is_recording() {
                                // Stop recording
                                match app.stop_recording() {
                                    Ok(msg) => {
                                        app.recording_result = Some(Ok(msg));
                                    }
                                    Err(e) => {
                                        app.recording_result = Some(Err(e));
                                    }
                                }
                            } else if app.recording_popup.is_none() {
                                // Open filename popup
                                app.open_recording_popup();
                            }
                        }
                        KeyCode::Char('1') => app.set_seed_pattern(SeedPattern::Point),
                        KeyCode::Char('2') => app.set_seed_pattern(SeedPattern::Line),
                        KeyCode::Char('3') => app.set_seed_pattern(SeedPattern::Cross),
                        KeyCode::Char('4') => app.set_seed_pattern(SeedPattern::Circle),
                        KeyCode::Char('5') => app.set_seed_pattern(SeedPattern::Ring),
                        KeyCode::Char('6') => app.set_seed_pattern(SeedPattern::Block),
                        KeyCode::Char('7') => app.set_seed_pattern(SeedPattern::MultiPoint),
                        KeyCode::Char('8') => app.set_seed_pattern(SeedPattern::Starburst),
                        KeyCode::Char('9') => app.set_seed_pattern(SeedPattern::NoisePatch),
                        KeyCode::Char('0') => app.set_seed_pattern(SeedPattern::Scatter),
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.increase_speed();
                            app.focus = Focus::Speed;
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            app.decrease_speed();
                            app.focus = Focus::Speed;
                        }
                        KeyCode::Char('[') => {
                            app.adjust_highlight(-5);
                            app.focus = Focus::Highlight;
                        }
                        KeyCode::Char(']') => {
                            app.adjust_highlight(5);
                            app.focus = Focus::Highlight;
                        }

                        // Original cycling keys (non-shift)
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            app.cycle_color_scheme();
                            app.focus = Focus::ColorScheme;
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => app.toggle_color_by_age(),
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            app.cycle_color_mode();
                            app.focus = Focus::Mode;
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            app.toggle_invert_colors();
                            app.focus = Focus::Invert;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            app.cycle_neighborhood();
                            app.focus = Focus::Neighborhood;
                        }
                        KeyCode::Char('b') | KeyCode::Char('B') => {
                            app.cycle_boundary();
                            app.focus = Focus::Boundary;
                        }
                        // w/s for navigation (like Up/Down arrows)
                        KeyCode::Char('w') => {
                            if app.show_help {
                                app.scroll_help_up();
                            } else if app.focus.is_param() {
                                app.prev_focus();
                            } else {
                                app.scroll_controls_up();
                            }
                        }
                        KeyCode::Char('s') => {
                            if app.show_help {
                                let term_size = terminal.size().unwrap_or_default();
                                let visible = ui::get_help_visible_lines(term_size.height);
                                app.scroll_help_down(ui::HELP_CONTENT_LINES.saturating_sub(visible));
                            } else if app.focus.is_param() {
                                app.next_focus();
                            } else {
                                let term_size = terminal.size().unwrap_or_default();
                                let visible = ui::get_controls_visible_lines(term_size.height);
                                app.scroll_controls_down(ui::CONTROLS_CONTENT_LINES.saturating_sub(visible));
                            }
                        }

                        // j/k for adjusting focused parameter values
                        KeyCode::Char('j') if app.focus.is_param() => {
                            app.adjust_focused_down();
                        }
                        KeyCode::Char('k') if app.focus.is_param() => {
                            app.adjust_focused_up();
                        }

                        // Navigation
                        KeyCode::Tab => app.next_focus(),
                        KeyCode::BackTab => app.prev_focus(),
                        KeyCode::Up => {
                            if app.show_help {
                                app.scroll_help_up();
                            } else if app.focus.is_param() {
                                app.prev_focus();
                            } else {
                                app.scroll_controls_up();
                            }
                        }
                        KeyCode::Down => {
                            if app.show_help {
                                let term_size = terminal.size().unwrap_or_default();
                                let visible = ui::get_help_visible_lines(term_size.height);
                                app.scroll_help_down(ui::HELP_CONTENT_LINES.saturating_sub(visible));
                            } else if app.focus.is_param() {
                                app.next_focus();
                            } else {
                                let term_size = terminal.size().unwrap_or_default();
                                let visible = ui::get_controls_visible_lines(term_size.height);
                                app.scroll_controls_down(ui::CONTROLS_CONTENT_LINES.saturating_sub(visible));
                            }
                        }
                        KeyCode::Esc => {
                            if app.show_help {
                                app.toggle_help();
                            } else if app.focus.is_param() {
                                app.focus = Focus::Controls;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Resize(width, height) => {
                    let (canvas_width, canvas_height) = ui::get_canvas_size(
                        ratatui::layout::Rect {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        app.fullscreen_mode,
                    );
                    app.resize(canvas_width, canvas_height);
                }
                _ => {}
            }
        }

        // Run simulation tick
        app.tick();
    }
}
