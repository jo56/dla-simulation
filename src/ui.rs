use crate::app::{App, Focus};
use crate::braille;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 22;

/// Max scroll for help content (generous to account for text wrapping on small screens)
pub const HELP_CONTENT_LINES: u16 = 60;

/// Number of lines in controls content
pub const CONTROLS_CONTENT_LINES: u16 = 8;

// UI color scheme
const BORDER_COLOR: Color = Color::Cyan;
const HIGHLIGHT_COLOR: Color = Color::Yellow;
const TEXT_COLOR: Color = Color::White;
const DIM_TEXT_COLOR: Color = Color::Gray;

/// Creates a standard styled block with rounded borders
fn styled_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_COLOR))
        .title(title)
}

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
        render_help_overlay(frame, area, app);
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
    // Calculate remaining height after status box
    let remaining = area.height.saturating_sub(5);
    // Split remaining space: ~60% to params, ~40% to controls (params has more content typically visible)
    let params_height = (remaining * 3 / 5).max(4);
    let controls_height = remaining.saturating_sub(params_height).max(3);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),              // Status - fixed
            Constraint::Length(params_height),  // Parameters - proportional
            Constraint::Length(controls_height), // Controls - proportional
        ])
        .split(area);

    render_status_box(frame, sections[0], app);
    render_params_box(frame, sections[1], app);
    render_controls_box(frame, sections[2], app);
}

fn render_status_box(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block(" DLA Simulator ");

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
        HIGHLIGHT_COLOR
    } else if app.simulation.is_complete() {
        Color::Green
    } else {
        BORDER_COLOR
    };

    let content = vec![
        Line::from(vec![
            Span::styled(
                format!("{} / {}", app.simulation.particles_stuck, app.simulation.num_particles),
                Style::default().fg(TEXT_COLOR),
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
    let is_focused = app.focus.is_param();
    let border_color = if is_focused { HIGHLIGHT_COLOR } else { BORDER_COLOR };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(" Parameters ");

    let make_line = |label: &str, value: String, focused: bool| {
        let prefix = if focused { "> " } else { "  " };
        let style = if focused {
            Style::default().fg(HIGHLIGHT_COLOR)
        } else {
            Style::default().fg(TEXT_COLOR)
        };
        Line::from(Span::styled(format!("{}{}: {}", prefix, label, value), style))
    };

    let settings = &app.simulation.settings;

    let content = vec![
        make_line(
            "sticky",
            format!("{:.2}", app.simulation.stickiness),
            app.focus == Focus::Stickiness,
        ),
        make_line(
            "particles",
            format!("{}", app.simulation.num_particles),
            app.focus == Focus::Particles,
        ),
        make_line(
            "seed",
            app.simulation.seed_pattern.name().to_lowercase(),
            app.focus == Focus::Seed,
        ),
        make_line(
            "color",
            app.color_scheme.name().to_lowercase(),
            app.focus == Focus::ColorScheme,
        ),
        make_line(
            "speed",
            format!("{}", app.steps_per_frame),
            app.focus == Focus::Speed,
        ),
        make_line(
            "mode",
            settings.color_mode.name().to_lowercase(),
            app.focus == Focus::Mode,
        ),
        make_line(
            "neigh",
            settings.neighborhood.short_name().to_lowercase(),
            app.focus == Focus::Neighborhood,
        ),
        make_line(
            "bound",
            settings.boundary_behavior.name().to_lowercase(),
            app.focus == Focus::Boundary,
        ),
        make_line(
            "spawn",
            settings.spawn_mode.name().to_lowercase(),
            app.focus == Focus::Spawn,
        ),
        make_line(
            "step",
            format!("{:.1}", settings.walk_step_size),
            app.focus == Focus::WalkStep,
        ),
        make_line(
            "hi-lt",
            format!("{}", settings.highlight_recent),
            app.focus == Focus::Highlight,
        ),
        make_line(
            "invert",
            if settings.invert_colors { "on" } else { "off" }.to_string(),
            app.focus == Focus::Invert,
        ),
    ];

    // Calculate scroll to keep focused item visible based on actual area
    let focus_line = app.focus.line_index();
    let visible_height = area.height.saturating_sub(2); // minus borders
    let content_height = content.len() as u16;

    let scroll = if visible_height == 0 || visible_height >= content_height {
        0 // No scrolling needed
    } else if focus_line >= visible_height {
        // Scroll to show focused line at bottom of visible area
        focus_line.saturating_sub(visible_height - 1)
    } else {
        0 // Focus is within first visible lines
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_controls_box(frame: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default().fg(HIGHLIGHT_COLOR);
    let desc_style = Style::default().fg(DIM_TEXT_COLOR);

    // Compact format with 1-space indent, each line ≤ 19 chars content
    let content = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Spc", key_style),
            Span::styled(" pause ", desc_style),
            Span::styled("R", key_style),
            Span::styled(" reset", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Q", key_style),
            Span::styled(" quit ", desc_style),
            Span::styled("H/?", key_style),
            Span::styled(" help", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("V", key_style),
            Span::styled(" full ", desc_style),
            Span::styled("1-0", key_style),
            Span::styled(" seeds", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("C", key_style),
            Span::styled(" color ", desc_style),
            Span::styled("A", key_style),
            Span::styled(" age", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("M", key_style),
            Span::styled(" mode ", desc_style),
            Span::styled("I", key_style),
            Span::styled(" invert", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("N", key_style),
            Span::styled(" neigh ", desc_style),
            Span::styled("B", key_style),
            Span::styled(" bound", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("S", key_style),
            Span::styled(" spawn ", desc_style),
            Span::styled("W/E", key_style),
            Span::styled(" step", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("+/-", key_style),
            Span::styled(" spd ", desc_style),
            Span::styled("[/]", key_style),
            Span::styled(" hi-lt", desc_style),
        ]),
    ];

    let content_height = content.len() as u16;
    let visible_height = area.height.saturating_sub(2); // minus borders
    let max_scroll = content_height.saturating_sub(visible_height);
    let is_scrollable = max_scroll > 0;
    let is_focused = app.focus == Focus::Controls;

    let title = if is_focused && is_scrollable {
        " Controls (↑↓) "
    } else if is_focused {
        " Controls "
    } else if is_scrollable {
        " Controls (↑↓) "
    } else {
        " Controls "
    };

    let border_color = if is_focused { HIGHLIGHT_COLOR } else { BORDER_COLOR };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((app.controls_scroll, 0));
    frame.render_widget(paragraph, area);
}

fn render_canvas(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block("");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get settings for rendering
    let settings = &app.simulation.settings;

    // Render Braille pattern (uses LUT for fast color lookup)
    let cells = braille::render_to_braille(
        &app.simulation,
        inner.width,
        inner.height,
        &app.color_lut,
        app.color_by_age,
        settings.color_mode,
        settings.highlight_recent,
        settings.invert_colors,
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

fn render_help_overlay(frame: &mut Frame, area: Rect, app: &App) {
    // Calculate the canvas area (exclude sidebar unless fullscreen)
    let canvas_x = if app.fullscreen_mode { 0 } else { SIDEBAR_WIDTH };
    let canvas_width = if app.fullscreen_mode {
        area.width
    } else {
        area.width.saturating_sub(SIDEBAR_WIDTH)
    };

    // Center the help dialog within the canvas
    let help_width = 56.min(canvas_width.saturating_sub(4));
    let help_height = area.height.saturating_sub(4).min(40);
    let x = canvas_x + (canvas_width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;

    let help_area = Rect {
        x: area.x + x,
        y: area.y + y,
        width: help_width,
        height: help_height,
    };

    // Clear the background
    frame.render_widget(Clear, help_area);

    // Build expanded help content (formatted for wrapping)
    let content = vec![
        Line::from(""),
        Line::from(Span::styled("DIFFUSION-LIMITED AGGREGATION", Style::default().fg(BORDER_COLOR))),
        Line::from(""),
        Line::from("Particles randomly walk until they touch and stick to the growing structure, creating fractal patterns."),
        Line::from(""),
        Line::from(Span::styled("SEED PATTERNS (1-0):", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from("1=Point, 2=Line, 3=Cross, 4=Circle, 5=Ring, 6=Block, 7=Multi, 8=Starburst, 9=Noise, 0=Scatter"),
        Line::from(""),
        Line::from(Span::styled("ADVANCED SETTINGS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("M - Color Mode", Style::default().fg(TEXT_COLOR))),
        Line::from("Age (when stuck), Distance (from center), Density (neighbor count), Direction (approach angle)"),
        Line::from(""),
        Line::from(Span::styled("N - Neighborhood", Style::default().fg(TEXT_COLOR))),
        Line::from("VonNeumann (4): angular patterns"),
        Line::from("Moore (8): natural fractals"),
        Line::from("Extended (24): dense blobs"),
        Line::from(""),
        Line::from(Span::styled("B - Boundary Behavior", Style::default().fg(TEXT_COLOR))),
        Line::from("Clamp (stop), Wrap (toroidal), Bounce (reflect), Stick (edges), Absorb (respawn)"),
        Line::from(""),
        Line::from(Span::styled("S - Spawn Mode", Style::default().fg(TEXT_COLOR))),
        Line::from("Circle, Edges, Corners, Random, Top, Bottom, Left, Right"),
        Line::from(""),
        Line::from(Span::styled("W/E - Walk Step Size", Style::default().fg(TEXT_COLOR))),
        Line::from("Larger = faster but coarser, Smaller = slower but finer detail"),
        Line::from(""),
        Line::from(Span::styled("I - Invert Colors", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("[/] - Highlight Recent", Style::default().fg(TEXT_COLOR))),
        Line::from("Show newest particles in white"),
        Line::from(""),
        Line::from(Span::styled("BASIC CONTROLS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from("Space=Pause, R=Reset, C=Colors, A=Color-by-age, V=Fullscreen, Tab/Arrows=Adjust, +/-=Speed, Q=Quit"),
        Line::from(""),
    ];

    let content_height = content.len() as u16;
    let visible_height = help_height.saturating_sub(2); // minus borders
    let max_scroll = content_height.saturating_sub(visible_height);
    let is_scrollable = max_scroll > 0;

    // Update title to show scroll hint if scrollable
    let title = if is_scrollable {
        " Help (J/K scroll, H to close) "
    } else {
        " Help (H to close) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(HIGHLIGHT_COLOR))
        .title(title);

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, help_area);
}
