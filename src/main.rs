mod app;
mod braille;
mod color;
mod simulation;
mod ui;

use app::App;
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

    /// Initial seed pattern (point, line, cross, circle, diamond, square, triangle, star, spiral, scatter, multipoint, xshape)
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
        "diamond" => SeedPattern::Diamond,
        "square" => SeedPattern::Square,
        "triangle" => SeedPattern::Triangle,
        "star" => SeedPattern::Star,
        "spiral" => SeedPattern::Spiral,
        "scatter" => SeedPattern::Scatter,
        "multipoint" | "multi-point" => SeedPattern::MultiPoint,
        "xshape" | "x-shape" | "x" => SeedPattern::XShape,
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

                    // Process key events
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                        KeyCode::Char(' ') => app.toggle_pause(),
                        KeyCode::Char('r') | KeyCode::Char('R') => app.reset(),
                        KeyCode::Char('1') => app.set_seed_pattern(SeedPattern::Point),
                        KeyCode::Char('2') => app.set_seed_pattern(SeedPattern::Line),
                        KeyCode::Char('3') => app.set_seed_pattern(SeedPattern::Cross),
                        KeyCode::Char('4') => app.set_seed_pattern(SeedPattern::Circle),
                        KeyCode::Char('5') => app.set_seed_pattern(SeedPattern::Diamond),
                        KeyCode::Char('6') => app.set_seed_pattern(SeedPattern::Square),
                        KeyCode::Char('7') => app.set_seed_pattern(SeedPattern::Triangle),
                        KeyCode::Char('8') => app.set_seed_pattern(SeedPattern::Star),
                        KeyCode::Char('9') => app.set_seed_pattern(SeedPattern::Spiral),
                        KeyCode::Char('0') => app.set_seed_pattern(SeedPattern::Scatter),
                        KeyCode::Char('[') => app.set_seed_pattern(SeedPattern::MultiPoint),
                        KeyCode::Char(']') => app.set_seed_pattern(SeedPattern::XShape),
                        KeyCode::Char('c') | KeyCode::Char('C') => app.cycle_color_scheme(),
                        KeyCode::Char('a') | KeyCode::Char('A') => app.toggle_color_by_age(),
                        KeyCode::Char('v') | KeyCode::Char('V') => app.toggle_fullscreen(),
                        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
                            app.toggle_help()
                        }
                        KeyCode::Tab => app.next_focus(),
                        KeyCode::BackTab => app.prev_focus(),
                        KeyCode::Up => app.adjust_focused_up(),
                        KeyCode::Down => app.adjust_focused_down(),
                        KeyCode::Char('+') | KeyCode::Char('=') => app.increase_speed(),
                        KeyCode::Char('-') | KeyCode::Char('_') => app.decrease_speed(),
                        KeyCode::Esc => {
                            if app.show_help {
                                app.toggle_help();
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
