use ratatui::style::Color;

/// Color schemes for visualization
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ColorScheme {
    #[default]
    Ice,
    Fire,
    Plasma,
    Viridis,
    Rainbow,
    Grayscale,
    Ocean,
    Neon,
}

impl ColorScheme {
    pub fn name(&self) -> &str {
        match self {
            ColorScheme::Ice => "Ice",
            ColorScheme::Fire => "Fire",
            ColorScheme::Plasma => "Plasma",
            ColorScheme::Viridis => "Viridis",
            ColorScheme::Rainbow => "Rainbow",
            ColorScheme::Grayscale => "Grayscale",
            ColorScheme::Ocean => "Ocean",
            ColorScheme::Neon => "Neon",
        }
    }

    pub fn next(&self) -> ColorScheme {
        match self {
            ColorScheme::Ice => ColorScheme::Fire,
            ColorScheme::Fire => ColorScheme::Plasma,
            ColorScheme::Plasma => ColorScheme::Viridis,
            ColorScheme::Viridis => ColorScheme::Rainbow,
            ColorScheme::Rainbow => ColorScheme::Grayscale,
            ColorScheme::Grayscale => ColorScheme::Ocean,
            ColorScheme::Ocean => ColorScheme::Neon,
            ColorScheme::Neon => ColorScheme::Ice,
        }
    }

    pub fn prev(&self) -> ColorScheme {
        match self {
            ColorScheme::Ice => ColorScheme::Neon,
            ColorScheme::Fire => ColorScheme::Ice,
            ColorScheme::Plasma => ColorScheme::Fire,
            ColorScheme::Viridis => ColorScheme::Plasma,
            ColorScheme::Rainbow => ColorScheme::Viridis,
            ColorScheme::Grayscale => ColorScheme::Rainbow,
            ColorScheme::Ocean => ColorScheme::Grayscale,
            ColorScheme::Neon => ColorScheme::Ocean,
        }
    }

    /// Map a value from 0.0-1.0 to a terminal color
    pub fn map(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let (r, g, b) = match self {
            ColorScheme::Ice => Self::ice_gradient(t),
            ColorScheme::Fire => Self::fire_gradient(t),
            ColorScheme::Plasma => Self::plasma_gradient(t),
            ColorScheme::Viridis => Self::viridis_gradient(t),
            ColorScheme::Rainbow => Self::rainbow_gradient(t),
            ColorScheme::Grayscale => Self::grayscale_gradient(t),
            ColorScheme::Ocean => Self::ocean_gradient(t),
            ColorScheme::Neon => Self::neon_gradient(t),
        };
        Color::Rgb(r, g, b)
    }

    fn ice_gradient(t: f32) -> (u8, u8, u8) {
        // Dark blue -> cyan -> white
        let r = (t * 200.0 + 55.0 * t * t) as u8;
        let g = (t * 220.0 + 35.0 * t) as u8;
        let b = (180.0 + 75.0 * t) as u8;
        (r, g, b)
    }

    fn fire_gradient(t: f32) -> (u8, u8, u8) {
        // Black -> red -> orange -> yellow -> white
        if t < 0.33 {
            let s = t / 0.33;
            ((s * 200.0) as u8, 0, 0)
        } else if t < 0.66 {
            let s = (t - 0.33) / 0.33;
            (200 + (s * 55.0) as u8, (s * 150.0) as u8, 0)
        } else {
            let s = (t - 0.66) / 0.34;
            (255, 150 + (s * 105.0) as u8, (s * 200.0) as u8)
        }
    }

    fn plasma_gradient(t: f32) -> (u8, u8, u8) {
        // Purple -> pink -> orange -> yellow
        let r = ((0.5 + 0.5 * (std::f32::consts::TAU * (t + 0.0)).sin()) * 255.0) as u8;
        let g = ((0.5 + 0.5 * (std::f32::consts::TAU * (t + 0.33)).sin()) * 200.0) as u8;
        let b = ((0.5 + 0.5 * (std::f32::consts::TAU * (t + 0.67)).sin()) * 255.0) as u8;
        (r.max(50), g, b)
    }

    fn viridis_gradient(t: f32) -> (u8, u8, u8) {
        // Dark purple -> teal -> yellow-green
        let r = (68.0 + t * 185.0 * t) as u8;
        let g = (1.0 + t * 220.0) as u8;
        let b = (84.0 + 90.0 * (1.0 - t) * (1.0 - t * 0.5)) as u8;
        (r, g, b)
    }

    fn rainbow_gradient(t: f32) -> (u8, u8, u8) {
        // HSV rotation through the rainbow
        let h = t * 360.0;
        let s = 1.0;
        let v = 1.0;
        Self::hsv_to_rgb(h, s, v)
    }

    fn grayscale_gradient(t: f32) -> (u8, u8, u8) {
        let v = (t * 255.0) as u8;
        (v, v, v)
    }

    fn ocean_gradient(t: f32) -> (u8, u8, u8) {
        // Deep blue -> teal -> aqua
        let r = (t * 100.0) as u8;
        let g = (50.0 + t * 150.0) as u8;
        let b = (100.0 + t * 155.0) as u8;
        (r, g, b)
    }

    fn neon_gradient(t: f32) -> (u8, u8, u8) {
        // Bright neon colors: magenta -> cyan -> green
        if t < 0.5 {
            let s = t / 0.5;
            (255 - (s * 255.0) as u8, (s * 255.0) as u8, 255)
        } else {
            let s = (t - 0.5) / 0.5;
            (0, 255, 255 - (s * 255.0) as u8)
        }
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
}
