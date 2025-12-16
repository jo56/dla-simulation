mod app;
mod braille;
mod color;
mod presets;
mod settings;
mod simulation;
mod ui;

use app::{App, Focus};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use simulation::SeedPattern;
use std::io;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "dla-simulator")]
#[command(about = "Diffusion-Limited Aggregation simulation in the terminal")]
struct Args {
    /// Number of particles to simulate (auto-capped to ~20% of grid area)
    #[arg(short = 'p', long, default_value = "5000")]
    particles: usize,

    /// Stickiness factor (0.1-1.0)
    #[arg(short = 's', long, default_value = "1.0")]
    stickiness: f32,

    /// Initial seed pattern (point, line, cross, circle, ring, block, noise, scatter, multipoint, starburst)
    #[arg(long, default_value = "point")]
    seed: String,

    /// Simulation speed (steps per frame, 1-50)
    #[arg(long, default_value = "5")]
    speed: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Parse seed pattern
    let seed_pattern = match args.seed.to_lowercase().as_str() {
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

    // Apply CLI args (particle count capped to grid-based max)
    let max_particles = app.simulation.max_particles();
    app.simulation.num_particles = args.particles.clamp(100, max_particles);
    app.simulation.stickiness = args.stickiness.clamp(0.1, 1.0);
    app.steps_per_frame = args.speed.clamp(1, 50);
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
                            _ => {}
                        }
                        continue;
                    }

                    // === Handle Shift+letter to open popup ===
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        if let KeyCode::Char(c) = key.code {
                            // Shift+? opens all params popup
                            if c == '?' {
                                app.open_all_params_popup();
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
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.cycle_spawn_mode();
                            app.focus = Focus::Spawn;
                        }
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            app.adjust_walk_step(0.5);
                            app.focus = Focus::WalkStep;
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') => {
                            app.adjust_walk_step(-0.5);
                            app.focus = Focus::WalkStep;
                        }

                        // Navigation
                        KeyCode::Tab => app.next_focus(),
                        KeyCode::BackTab => app.prev_focus(),
                        KeyCode::Up => {
                            if !app.show_help {
                                if app.focus.is_param() {
                                    app.adjust_focused_up();
                                } else {
                                    app.scroll_controls_up();
                                }
                            }
                        }
                        KeyCode::Down => {
                            if !app.show_help {
                                if app.focus.is_param() {
                                    app.adjust_focused_down();
                                } else {
                                    let term_size = terminal.size().unwrap_or_default();
                                    let visible = ui::get_controls_visible_lines(term_size.height);
                                    app.scroll_controls_down(ui::CONTROLS_CONTENT_LINES.saturating_sub(visible));
                                }
                            }
                        }
                        KeyCode::Esc => {
                            if app.show_help {
                                app.toggle_help();
                            } else if app.focus.is_param() {
                                app.focus = Focus::Controls;
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Char('J') => {
                            if app.show_help {
                                app.scroll_help_down(ui::HELP_CONTENT_LINES);
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Char('K') => {
                            if app.show_help {
                                app.scroll_help_up();
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
