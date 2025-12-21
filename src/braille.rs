use crate::color::{map_from_lut, ColorLut};
use crate::settings::ColorMode;
use crate::simulation::DlaSimulation;
use ratatui::style::Color;

/// Braille character rendering for high-resolution terminal graphics.
/// Each Braille character represents a 2x4 grid of dots (8 dots total).
///
/// Dot positions and their bit values:
/// ```
/// (0,0)=0x01  (1,0)=0x08
/// (0,1)=0x02  (1,1)=0x10
/// (0,2)=0x04  (1,2)=0x20
/// (0,3)=0x40  (1,3)=0x80
/// ```
///
/// Unicode Braille patterns: U+2800 to U+28FF (256 patterns)
const BRAILLE_BASE: u32 = 0x2800;

/// Dot position to bit mapping for Braille characters
const BRAILLE_DOTS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // Left column (x=0): rows 0,1,2,3
    [0x08, 0x10, 0x20, 0x80], // Right column (x=1): rows 0,1,2,3
];

/// A single rendered Braille cell with position and color
#[derive(Clone, Copy)]
pub struct BrailleCell {
    pub x: u16,
    pub y: u16,
    pub char: char,
    pub color: Color,
}

/// Render the simulation grid to Braille characters (uses LUT for fast color lookup)
#[allow(clippy::too_many_arguments)]
pub fn render_to_braille(
    simulation: &DlaSimulation,
    canvas_width: u16,
    canvas_height: u16,
    color_lut: &ColorLut,
    color_by_age: bool,
    color_mode: ColorMode,
    highlight_recent: usize,
    invert_colors: bool,
) -> Vec<BrailleCell> {
    let sim_width = simulation.grid_width;
    let sim_height = simulation.grid_height;

    // Braille effective resolution (must match calculate_simulation_size min constraints)
    let braille_width = (canvas_width as usize * 2).max(64);
    let braille_height = (canvas_height as usize * 4).max(64);

    // Scale factors (pre-calculated once)
    let scale_x = sim_width as f32 / braille_width as f32;
    let scale_y = sim_height as f32 / braille_height as f32;

    // Pre-calculate for color mapping
    let inv_num_particles = 1.0 / simulation.num_particles.max(1) as f32;
    let max_radius = simulation.max_radius.max(1.0);
    let particles_stuck = simulation.particles_stuck;

    let mut cells = Vec::with_capacity((canvas_width * canvas_height) as usize);

    for cy in 0..canvas_height {
        for cx in 0..canvas_width {
            let mut pattern: u8 = 0;
            let mut total_value: f32 = 0.0;
            let mut dot_count: usize = 0;
            let mut is_recent = false;

            // Sample the 2x4 dots for this Braille character
            let base_bx = cx as usize * 2;
            let base_by = cy as usize * 4;

            for (dx, dots_col) in BRAILLE_DOTS.iter().enumerate() {
                for (dy, &dot_pattern) in dots_col.iter().enumerate() {
                    let braille_x = base_bx + dx;
                    let braille_y = base_by + dy;

                    let sim_x = (braille_x as f32 * scale_x) as usize;
                    let sim_y = (braille_y as f32 * scale_y) as usize;

                    if let Some(particle) = simulation.get_particle(sim_x, sim_y) {
                        pattern |= dot_pattern;
                        dot_count += 1;

                        // Check if this is a recent particle
                        if highlight_recent > 0 && particle.age + highlight_recent >= particles_stuck {
                            is_recent = true;
                        }

                        // Calculate value based on color mode
                        let value = match color_mode {
                            ColorMode::Age => particle.age as f32 * inv_num_particles,
                            ColorMode::Distance => particle.distance / max_radius,
                            ColorMode::Density => particle.neighbor_count as f32 / 8.0,
                            ColorMode::Direction => {
                                // Map angle (-PI to PI) to 0-1
                                (particle.direction + std::f32::consts::PI) / std::f32::consts::TAU
                            }
                        };
                        total_value += value;
                    }
                }
            }

            // Only emit cells that have at least one dot
            if pattern != 0 {
                let braille_char = char::from_u32(BRAILLE_BASE + pattern as u32).unwrap_or(' ');

                let color = if is_recent {
                    // Highlight recent particles in a contrasting color
                    Color::Rgb(255, 255, 255)
                } else if color_by_age && dot_count > 0 {
                    let avg_value = total_value / dot_count as f32;
                    let t = if invert_colors { 1.0 - avg_value } else { avg_value };
                    map_from_lut(color_lut, t)
                } else {
                    Color::White
                };

                cells.push(BrailleCell {
                    x: cx,
                    y: cy,
                    char: braille_char,
                    color,
                });
            }
        }
    }

    cells
}

/// Calculate optimal simulation grid size for a given canvas size
/// Returns (width, height) for the simulation grid
pub fn calculate_simulation_size(canvas_width: u16, canvas_height: u16) -> (usize, usize) {
    // Braille gives 2x4 resolution per character
    // We want the simulation grid to match this resolution
    let width = (canvas_width as usize * 2).max(64);
    let height = (canvas_height as usize * 4).max(64);
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_pattern() {
        // Test that single dot patterns work correctly
        assert_eq!(BRAILLE_DOTS[0][0], 0x01); // Top-left
        assert_eq!(BRAILLE_DOTS[1][0], 0x08); // Top-right
        assert_eq!(BRAILLE_DOTS[0][3], 0x40); // Bottom-left
        assert_eq!(BRAILLE_DOTS[1][3], 0x80); // Bottom-right

        // All dots should give 0xFF
        let all_dots: u8 = BRAILLE_DOTS[0].iter().sum::<u8>() + BRAILLE_DOTS[1].iter().sum::<u8>();
        assert_eq!(all_dots, 0xFF);
    }

    #[test]
    fn test_braille_char_generation() {
        // Empty pattern
        let empty = char::from_u32(BRAILLE_BASE).unwrap();
        assert_eq!(empty, '\u{2800}');

        // Full pattern (all 8 dots)
        let full = char::from_u32(BRAILLE_BASE + 0xFF).unwrap();
        assert_eq!(full, '\u{28FF}');
    }
}
