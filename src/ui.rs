use crate::app::{App, Focus, ParamPopup, TextInputPopup};
use crate::braille;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 22;

/// Max scroll for help content (generous to account for text wrapping on small screens)
pub const HELP_CONTENT_LINES: u16 = 73;

/// Number of lines in controls content (5 main + 18 Shift+letter hints + 1 record)
pub const CONTROLS_CONTENT_LINES: u16 = 25;

/// Number of lines in parameters content
pub const PARAMS_CONTENT_LINES: u16 = 24;

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

    // Render param popup if open
    if let Some(popup) = &app.param_popup {
        render_param_popup(frame, area, popup);
    }

    // Render export popup if open (overlays everything)
    if let Some(popup) = &app.export_popup {
        render_export_popup(frame, area, popup);
    }

    // Render export result toast if present
    if let Some(result) = &app.export_result {
        render_export_result(frame, area, result);
    }

    // Render recording popup if open (overlays everything)
    if let Some(popup) = &app.recording_popup {
        render_recording_popup(frame, area, popup);
    }

    // Render recording result toast if present
    if let Some(result) = &app.recording_result {
        render_recording_result(frame, area, result);
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

/// Calculate the number of visible lines in the help popup based on terminal height
pub fn get_help_visible_lines(terminal_height: u16) -> u16 {
    // Help popup height calculation (from render_help_overlay)
    let help_height = terminal_height.saturating_sub(4).min(40);
    // Visible lines = height - borders
    help_height.saturating_sub(2)
}

/// Calculate the number of visible lines in the controls box based on terminal height
pub fn get_controls_visible_lines(terminal_height: u16) -> u16 {
    const STATUS_HEIGHT: u16 = 5;
    const NAV_HEIGHT: u16 = 4;
    const MIN_CONTROLS_VISIBLE: u16 = 4;
    const BORDERS: u16 = 2;

    let fixed_height = STATUS_HEIGHT + NAV_HEIGHT;
    let available = terminal_height.saturating_sub(fixed_height);

    let params_ideal = PARAMS_CONTENT_LINES + BORDERS; // 14
    let controls_min = MIN_CONTROLS_VISIBLE + BORDERS; // 5
    let controls_max = CONTROLS_CONTENT_LINES + BORDERS; // 10

    let controls_height = if available < params_ideal + controls_min {
        controls_min.min(available)
    } else {
        let extra = available - params_ideal - controls_min;
        let controls_extra = extra.min(controls_max - controls_min);
        controls_min + controls_extra
    };

    // Visible lines = height - borders
    controls_height.saturating_sub(BORDERS)
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    // Fixed component heights
    const STATUS_HEIGHT: u16 = 5;
    const NAV_HEIGHT: u16 = 4;
    const MIN_CONTROLS_VISIBLE: u16 = 4;
    const BORDERS: u16 = 2;

    let fixed_height = STATUS_HEIGHT + NAV_HEIGHT;
    let available = area.height.saturating_sub(fixed_height);

    // Calculate ideal heights (content + borders)
    let params_ideal = PARAMS_CONTENT_LINES + BORDERS; // 14
    let controls_min = MIN_CONTROLS_VISIBLE + BORDERS; // 5
    let controls_max = CONTROLS_CONTENT_LINES + BORDERS; // 10

    // Allocate space with priority:
    // 1. Parameters needs its content (no whitespace) - up to 14
    // 2. Controls expands from 3 to 8 visible lines
    // 3. Remaining whitespace goes to Parameters
    let (params_height, controls_height) = if available < params_ideal + controls_min {
        // Not enough space - give controls its minimum, params gets the rest
        let controls_h = controls_min.min(available);
        let params_h = available.saturating_sub(controls_h).max(4);
        (params_h, controls_h)
    } else {
        // Enough for params ideal + controls min, see how much extra we have
        let extra = available - params_ideal - controls_min;
        // Controls gets extra up to its max (8 visible lines)
        let controls_extra = extra.min(controls_max - controls_min);
        // Any remainder goes to params as whitespace
        let params_extra = extra.saturating_sub(controls_extra);
        (params_ideal + params_extra, controls_min + controls_extra)
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(STATUS_HEIGHT),   // Status - fixed
            Constraint::Length(params_height),   // Parameters - dynamic
            Constraint::Length(controls_height), // Controls - dynamic (3-8 visible lines)
            Constraint::Length(NAV_HEIGHT),      // Nav - fixed
        ])
        .split(area);

    render_status_box(frame, sections[0], app);
    render_params_box(frame, sections[1], app);
    render_controls_box(frame, sections[2], app);
    render_nav_box(frame, sections[3], app);
}

fn render_status_box(frame: &mut Frame, area: Rect, app: &App) {
    let block = styled_block(" DLA Simulation ");

    let progress = app.simulation.progress();
    let progress_width = (area.width.saturating_sub(4)) as usize;
    let filled = (progress * progress_width as f32) as usize;
    let empty = progress_width.saturating_sub(filled);

    // Recording indicator takes priority, then simulation status
    let (status_text, status_color) = if app.is_recording() {
        let frame_count = app.recorder.frame_count().unwrap_or(0);
        (format!("REC {}", frame_count), Color::Red)
    } else if app.simulation.paused {
        ("PAUSED".to_string(), HIGHLIGHT_COLOR)
    } else if app.simulation.is_complete() {
        ("COMPLETE".to_string(), Color::Green)
    } else {
        ("RUNNING".to_string(), BORDER_COLOR)
    };

    // Calculate fractal dimension (only when enough particles)
    let (fractal_dim, r_squared) = app.simulation.calculate_fractal_dimension();
    let dim_text = if fractal_dim > 0.0 {
        format!("D_f: {:.2} (R²={:.2})", fractal_dim, r_squared)
    } else {
        "D_f: --".to_string()
    };

    let content = vec![
        Line::from(Span::styled(dim_text, Style::default().fg(TEXT_COLOR))),
        Line::from(vec![
            Span::styled(
                format!("N: {} / {}", app.simulation.particles_stuck, app.simulation.num_particles),
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
    let title = if is_focused {
        " Params (w/s/j/k) "
    } else {
        " Params "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title);

    let make_line = |label: &str, value: String, focused: bool| {
        let prefix = if focused { ">" } else { " " };
        let style = if focused {
            Style::default().fg(HIGHLIGHT_COLOR)
        } else {
            Style::default().fg(TEXT_COLOR)
        };
        Line::from(Span::styled(format!("{}{}: {}", prefix, label, value), style))
    };

    let settings = &app.simulation.settings;

    let make_header = |label: &str| {
        Line::from(Span::styled(
            format!(" - {} -", label.to_lowercase()),
            Style::default().fg(DIM_TEXT_COLOR),
        ))
    };

    // Parameters grouped by type, alphabetical within each group
    let content = vec![
        // === Movement (alphabetical: adaptfactor, adaptive, direction, force, lattice, radial, walk) ===
        make_header("Movement"),
        make_line(
            "adaptive",
            if settings.adaptive_step { "on" } else { "off" }.to_string(),
            app.focus == Focus::AdaptiveStep,
        ),
        make_line(
            "adapt factor",
            format!("{:.2}", settings.adaptive_step_factor),
            app.focus == Focus::AdaptiveFactor,
        ),
        make_line(        
            "direction",
            format!("{:.0}°", settings.walk_bias_angle),
            app.focus == Focus::Direction,
        ),
        make_line(
            "force",
            format!("{:.2}", settings.walk_bias_strength),
            app.focus == Focus::Force,
        ),
        make_line(
            "lattice",
            if settings.lattice_walk { "on" } else { "off" }.to_string(),
            app.focus == Focus::LatticeWalk,
        ),
        make_line(
            "radial",
            format!("{:.2}", settings.radial_bias),
            app.focus == Focus::RadialBias,
        ),
        make_line(
            "walk",
            format!("{:.1}", settings.walk_step_size),
            app.focus == Focus::WalkStep,
        ),
        // === Sticking (alphabetical: contacts, gradient, neighbors, sidestick, sticky, tipstick) ===
        make_header("Sticking"),
        make_line(
            "contacts",
            format!("{}", settings.multi_contact_min),
            app.focus == Focus::MultiContact,
        ),
        make_line(
            "gradient",
            format!("{:.1}", settings.stickiness_gradient),
            app.focus == Focus::StickyGradient,
        ),
        make_line(
            "neighbors",
            settings.neighborhood.short_name().to_lowercase(),
            app.focus == Focus::Neighborhood,
        ),
        make_line(
            "sticky",
            format!("{:.2}", app.simulation.stickiness),
            app.focus == Focus::Stickiness,
        ),
        make_line(
            "side stick",
            format!("{:.1}", settings.side_stickiness),
            app.focus == Focus::SideSticky,
        ),
        make_line(
            "tip stick",
            format!("{:.1}", settings.tip_stickiness),
            app.focus == Focus::TipSticky,
        ),
        // === Spawn (alphabetical: bound, escape, maxsteps, minradius, spawn, spawnoff) ===
        make_header("Spawn"),
        make_line(
            "bound",
            settings.boundary_behavior.name().to_lowercase(),
            app.focus == Focus::Boundary,
        ),
        make_line(
            "escape",
            format!("{:.1}", settings.escape_multiplier),
            app.focus == Focus::EscapeMult,
        ),
        make_line(
            "max steps",
            format!("{}", settings.max_walk_iterations),
            app.focus == Focus::MaxIterations,
        ),
        make_line(
            "min radius",
            format!("{:.0}", settings.min_spawn_radius),
            app.focus == Focus::MinRadius,
        ),
        make_line(
            "spawn",
            settings.spawn_mode.name().to_lowercase(),
            app.focus == Focus::Spawn,
        ),
        make_line(
            "spawn off",
            format!("{:.0}", settings.spawn_radius_offset),
            app.focus == Focus::SpawnOffset,
        ),
        // === Visual (alphabetical: age, color, highlight, invert, mode, particles, seed, speed) ===
        make_header("Visual"),
        make_line(
            "age",
            if app.color_by_age { "on" } else { "off" }.to_string(),
            app.focus == Focus::Age,
        ),
        make_line(
            "color",
            app.color_scheme.name().to_lowercase(),
            app.focus == Focus::ColorScheme,
        ),
        make_line(
            "highlight",
            format!("{}", settings.highlight_recent),
            app.focus == Focus::Highlight,
        ),
        make_line(
            "invert",
            if settings.invert_colors { "on" } else { "off" }.to_string(),
            app.focus == Focus::Invert,
        ),
        make_line(
            "mode",
            settings.color_mode.name().to_lowercase(),
            app.focus == Focus::Mode,
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
            "speed",
            format!("{}", app.steps_per_frame),
            app.focus == Focus::Speed,
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

    // Main controls (top 4 lines) + Shift+letter hints below
    let content = vec![
        // Main controls
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
            Span::styled(" quit  ", desc_style),
            Span::styled("H", key_style),
            Span::styled(" help", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("V", key_style),
            Span::styled(" view  ", desc_style),
            Span::styled("1-0", key_style),
            Span::styled(" seeds", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("w/s/↑↓", key_style),
            Span::styled(" navigate", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+?:", key_style),
            Span::styled(" lookup", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+Tab:", key_style),
            Span::styled(" prev", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+X:", key_style),
            Span::styled(" export", desc_style),
        ]),
        // Shift+letter hints (alphabetical)
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+A:", key_style),
            Span::styled(" age", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+B:", key_style),
            Span::styled(" bound", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+C:", key_style),
            Span::styled(" color", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+D:", key_style),
            Span::styled(" direction", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+E:", key_style),
            Span::styled(" escape", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+F:", key_style),
            Span::styled(" force", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+G:", key_style),
            Span::styled(" gradient", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+H:", key_style),
            Span::styled(" highlight", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+I:", key_style),
            Span::styled(" invert", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+M:", key_style),
            Span::styled(" 4 options", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+N:", key_style),
            Span::styled(" neighbors", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+O:", key_style),
            Span::styled(" offset", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+P:", key_style),
            Span::styled(" particles", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+R:", key_style),
            Span::styled(" radial", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+S:", key_style),
            Span::styled(" 5 options", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+T:", key_style),
            Span::styled(" tip", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Shift+W:", key_style),
            Span::styled(" walk", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("`:", key_style),
            Span::styled(" record", desc_style),
        ]),
    ];

    let is_focused = app.focus == Focus::Controls;

    let title = if is_focused {
        " Controls (w/s/↑↓) "
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

fn render_nav_box(frame: &mut Frame, area: Rect, _app: &App) {
    let key_style = Style::default().fg(HIGHLIGHT_COLOR);
    let desc_style = Style::default().fg(DIM_TEXT_COLOR);

    let content = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Tab", key_style),
            Span::styled(" Parameters", desc_style),
        ]),
        Line::from(vec![
            Span::raw(" "),
            Span::styled("Esc", key_style),
            Span::styled(" Controls", desc_style),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER_COLOR))
        .title(" Focus ");

    let paragraph = Paragraph::new(content).block(block);
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
        Line::from(Span::styled("BASIC CONTROLS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("Space - Pause/Resume", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("R - Reset simulation", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Tab - Next parameter", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Shift+Tab - Previous parameter", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("w/s/↑↓ - Navigate/Scroll", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("j/k - Adjust focused value", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Esc - Close help / exit focus", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("V - Toggle fullscreen", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("` - Start/stop recording", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Shift+X - Export config to file", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Q - Quit", Style::default().fg(TEXT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("PARAMETER POPUP:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("Shift+? - Open ALL parameters popup", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Shift+letter - Filter by first letter", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Enter - Select from popup", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Esc - Close popup", Style::default().fg(TEXT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("QUICK KEYS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("1-0 - Seed patterns (Point to Scatter)", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("+/- - Adjust speed", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("[/] - Adjust highlight count", Style::default().fg(TEXT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("DIRECT PARAM KEYS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("A - Toggle color-by-age", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("C - Cycle color scheme", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("M - Cycle color mode", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("N - Cycle neighborhood type", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("B - Cycle boundary behavior", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Shift+S - Spawn popup", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("Shift+W/E - Walk step +/-", Style::default().fg(TEXT_COLOR))),
        Line::from(Span::styled("I - Invert colors", Style::default().fg(TEXT_COLOR))),
        Line::from(""),
        Line::from(Span::styled("MOVEMENT PARAMETERS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from("Walk Step (0.5-5.0) - Distance per step"),
        Line::from("Direction (0-360) - Bias angle"),
        Line::from("Force (0-0.5) - Bias strength"),
        Line::from("Radial (-0.3 to 0.3) - Inward/outward drift"),
        Line::from(""),
        Line::from(Span::styled("STICKING PARAMETERS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from("Stickiness (0.1-1.0) - Base stick chance"),
        Line::from("Neighborhood - VonNeumann/Moore/Extended"),
        Line::from("Multi-Contact (1-4) - Min neighbors to stick"),
        Line::from("Tip/Side Sticky - Stickiness by position"),
        Line::from("Gradient - Distance-based stickiness"),
        Line::from(""),
        Line::from(Span::styled("SPAWN/BOUNDARY:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from("Spawn - Circle/Edges/Corners/Random/Dir"),
        Line::from("Boundary - Clamp/Wrap/Bounce/Stick/Absorb"),
        Line::from("Offset/Escape/MinRadius/MaxIter"),
        Line::from(""),
        Line::from(Span::styled("VISUAL PARAMETERS:", Style::default().fg(HIGHLIGHT_COLOR))),
        Line::from(""),
        Line::from("Particles (100-10000) - Total count"),
        Line::from("Speed (1-50) - Steps per frame"),
        Line::from("Color - 8 schemes, 4 modes"),
        Line::from("Highlight (0-50) - Recent particles in white"),
        Line::from(""),
    ];

    let content_height = content.len() as u16;
    let visible_height = help_height.saturating_sub(2); // minus borders
    let max_scroll = content_height.saturating_sub(visible_height);
    let is_scrollable = max_scroll > 0;

    // Update title to show scroll hint if scrollable
    let title = if is_scrollable {
        " Help (Up/Down scroll, H to close) "
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

/// Render parameter selection popup
fn render_param_popup(frame: &mut Frame, area: Rect, popup: &ParamPopup) {
    // Calculate popup size based on content
    let max_name_len = popup
        .options
        .iter()
        .map(|(_, name)| name.len())
        .max()
        .unwrap_or(10);

    let popup_width = (max_name_len as u16 + 6).min(area.width.saturating_sub(4)).max(20);
    let popup_height = (popup.options.len() as u16 + 2).min(area.height.saturating_sub(4)).max(3);

    // Center the popup
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Build content with highlighted selection
    let content: Vec<Line> = popup
        .options
        .iter()
        .enumerate()
        .map(|(idx, (_, name))| {
            let is_selected = idx == popup.selected_idx;
            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(HIGHLIGHT_COLOR)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT_COLOR)
            };
            Line::from(Span::styled(format!("{}{}", prefix, name), style))
        })
        .collect();

    // Calculate scroll to keep selection visible
    let visible_height = popup_height.saturating_sub(2); // minus borders
    let selected = popup.selected_idx as u16;
    let scroll = if visible_height == 0 || selected < visible_height {
        0
    } else {
        selected.saturating_sub(visible_height - 1)
    };

    let title = " Lookup (Enter/Esc) ";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(HIGHLIGHT_COLOR))
        .title(title);

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left)
        .scroll((scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

/// Render text input popup for export filename
fn render_export_popup(frame: &mut Frame, area: Rect, popup: &TextInputPopup) {
    let popup_width = 44.min(area.width.saturating_sub(4));
    let popup_height = 5;

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    // Build input line with cursor
    let (before_cursor, after_cursor) = popup.input.split_at(popup.cursor_pos);
    let content = vec![
        Line::from(vec![
            Span::styled(before_cursor, Style::default().fg(TEXT_COLOR)),
            Span::styled(
                "_",
                Style::default()
                    .fg(HIGHLIGHT_COLOR)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(after_cursor, Style::default().fg(TEXT_COLOR)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Enter: save | Esc: cancel",
            Style::default().fg(DIM_TEXT_COLOR),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(HIGHLIGHT_COLOR))
        .title(popup.title);

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, popup_area);
}

/// Render export result toast (success or error message)
fn render_export_result(frame: &mut Frame, area: Rect, result: &Result<String, String>) {
    let (message, color) = match result {
        Ok(filename) => (format!("Saved: {}", filename), Color::Green),
        Err(e) => (format!("Error: {}", e), Color::Red),
    };

    let msg_width = (message.len() as u16 + 4).min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(msg_width)) / 2;
    let popup_y = area.y + area.height.saturating_sub(5);

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: msg_width,
        height: 3,
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    let paragraph = Paragraph::new(Line::from(Span::styled(
        message,
        Style::default().fg(color),
    )))
    .block(block)
    .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}

/// Render text input popup for recording filename
fn render_recording_popup(frame: &mut Frame, area: Rect, popup: &TextInputPopup) {
    let popup_width = 44.min(area.width.saturating_sub(4));
    let popup_height = 6;

    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    // Build input line with cursor
    let (before_cursor, after_cursor) = popup.input.split_at(popup.cursor_pos);
    let content = vec![
        Line::from(vec![
            Span::styled(before_cursor, Style::default().fg(TEXT_COLOR)),
            Span::styled(
                "_",
                Style::default()
                    .fg(HIGHLIGHT_COLOR)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(after_cursor, Style::default().fg(TEXT_COLOR)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            ".mp4/.webm (FFmpeg) or .gif",
            Style::default().fg(DIM_TEXT_COLOR),
        )),
        Line::from(Span::styled(
            "Enter: start | Esc: cancel",
            Style::default().fg(DIM_TEXT_COLOR),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red))
        .title(popup.title);

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, popup_area);
}

/// Render recording result toast (success or error message)
fn render_recording_result(frame: &mut Frame, area: Rect, result: &Result<String, String>) {
    let (message, color) = match result {
        Ok(msg) => (msg.clone(), Color::Green),
        Err(e) => (format!("Error: {}", e), Color::Red),
    };

    let msg_width = (message.len() as u16 + 4).min(area.width.saturating_sub(4)).max(20);
    let popup_x = area.x + (area.width.saturating_sub(msg_width)) / 2;
    let popup_y = area.y + area.height.saturating_sub(5);

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: msg_width,
        height: 3,
    };

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    let paragraph = Paragraph::new(Line::from(Span::styled(
        message,
        Style::default().fg(color),
    )))
    .block(block)
    .alignment(Alignment::Center);

    frame.render_widget(paragraph, popup_area);
}
