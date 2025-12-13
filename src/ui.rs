use crate::app::{App, Focus};
use crate::braille;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 22;

/// Main render function
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if app.fullscreen_mode {
        render_canvas(frame, area, app);
    } else {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
            .split(area);

        render_sidebar(frame, layout[0], app);
        render_canvas(frame, layout[1], app);
    }

    if app.show_help {
        render_help_overlay(frame, area);
    }
}

/// Calculate the canvas size (excluding borders)
pub fn get_canvas_size(frame_area: Rect, fullscreen: bool) -> (u16, u16) {
    if fullscreen {
        (frame_area.width.saturating_sub(2), frame_area.height.saturating_sub(2))
    } else {
        let canvas_width = frame_area.width.saturating_sub(SIDEBAR_WIDTH + 2);
        let canvas_height = frame_area.height.saturating_sub(2);
        (canvas_width, canvas_height)
    }
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Status
            Constraint::Length(9),  // Parameters
            Constraint::Min(10),    // Controls
        ])
        .split(area);

    render_status_box(frame, sections[0], app);
    render_params_box(frame, sections[1], app);
    render_controls_box(frame, sections[2], app);
}

fn render_status_box(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" DLA Simulator ");

    let progress = app.simulation.progress();
    let progress_width = (area.width.saturating_sub(4)) as usize;
    let filled = (progress * progress_width as f32) as usize;
    let empty = progress_width.saturating_sub(filled);

    let status_text = if app.simulation.paused {
        "PAUSED"
    } else if app.simulation.is_complete() {
        "COMPLETE"
    } else {
        "RUNNING"
    };

    let status_color = if app.simulation.paused {
        Color::Yellow
    } else if app.simulation.is_complete() {
        Color::Green
    } else {
        Color::Cyan
    };

    let content = vec![
        Line::from(vec![
            Span::styled(
                format!("{} / {}", app.simulation.particles_stuck, app.simulation.num_particles),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("█".repeat(filled), Style::default().fg(Color::Green)),
            Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(status_text, Style::default().fg(status_color))),
    ];

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_params_box(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Parameters ");

    let make_line = |label: &str, value: String, focused: bool| {
        let prefix = if focused { "> " } else { "  " };
        let style = if focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };
        Line::from(Span::styled(format!("{}{}: {}", prefix, label, value), style))
    };

    let content = vec![
        make_line(
            "Sticky",
            format!("{:.2}", app.simulation.stickiness),
            app.focus == Focus::Stickiness,
        ),
        make_line(
            "Particles",
            format!("{}", app.simulation.num_particles),
            app.focus == Focus::Particles,
        ),
        make_line(
            "Seed",
            app.simulation.seed_pattern.name().to_string(),
            app.focus == Focus::Seed,
        ),
        make_line(
            "Color",
            app.color_scheme.name().to_string(),
            app.focus == Focus::ColorScheme,
        ),
        make_line(
            "Speed",
            format!("{}", app.steps_per_frame),
            app.focus == Focus::Speed,
        ),
        Line::from(Span::styled(
            format!("  Age Color: {}", if app.color_by_age { "ON" } else { "OFF" }),
            Style::default().fg(Color::Gray),
        )),
    ];

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_controls_box(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Controls ");

    let key_style = Style::default().fg(Color::Yellow);
    let desc_style = Style::default().fg(Color::Gray);

    let make_control = |key: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!("{:>6}", key), key_style),
            Span::styled(format!(" {}", desc), desc_style),
        ])
    };

    let mut content = vec![
        make_control("Space", "pause/resume"),
        make_control("R", "reset"),
        make_control("1-0", "seed patterns"),
        make_control("C", "cycle color"),
        make_control("A", "toggle age color"),
        make_control("Tab", "next param"),
        make_control("↑/↓", "adjust param"),
        make_control("+/-", "speed"),
        make_control("V", "fullscreen"),
        make_control("H/?", "help"),
        make_control("Q", "quit"),
    ];

    // Show current focus hint
    if app.focus != Focus::None {
        content.push(Line::from(""));
        content.push(Line::from(Span::styled(
            format!("Editing: {:?}", app.focus),
            Style::default().fg(Color::Yellow),
        )));
    }

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_canvas(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render Braille pattern
    let cells = braille::render_to_braille(
        &app.simulation,
        inner.width,
        inner.height,
        &app.color_scheme,
        app.color_by_age,
    );

    for cell in cells {
        let x = inner.x + cell.x;
        let y = inner.y + cell.y;

        if x < inner.x + inner.width && y < inner.y + inner.height {
            let cell_rect = Rect {
                x,
                y,
                width: 1,
                height: 1,
            };
            let span = Span::styled(cell.char.to_string(), Style::default().fg(cell.color));
            let paragraph = Paragraph::new(Line::from(span));
            frame.render_widget(paragraph, cell_rect);
        }
    }
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Center the help dialog
    let help_width = 54.min(area.width.saturating_sub(4));
    let help_height = 26.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;

    let help_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width: help_width,
        height: help_height,
    };

    // Clear the background
    frame.render_widget(Clear, help_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Help - Press H or ? to close ");

    let content = vec![
        Line::from(""),
        Line::from(Span::styled("DIFFUSION-LIMITED AGGREGATION", Style::default().fg(Color::Cyan))),
        Line::from(""),
        Line::from("DLA simulates particles randomly walking"),
        Line::from("until they stick to a growing structure,"),
        Line::from("creating fractal snowflake-like patterns."),
        Line::from(""),
        Line::from(Span::styled("PARAMETERS:", Style::default().fg(Color::Yellow))),
        Line::from("  Stickiness: Chance to stick (0.1-1.0)"),
        Line::from("  Lower = more dendritic branches"),
        Line::from(""),
        Line::from(Span::styled("SEED PATTERNS:", Style::default().fg(Color::Yellow))),
        Line::from("  1=Point    2=Line     3=Cross    4=Circle"),
        Line::from("  5=Diamond  6=Square   7=Multi    8=XShape"),
        Line::from("  9=Spiral   0=Scatter"),
        Line::from(""),
        Line::from(Span::styled("Use Tab to select parameter,", Style::default().fg(Color::Gray))),
        Line::from(Span::styled("then ↑/↓ to adjust value.", Style::default().fg(Color::Gray))),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, help_area);
}
