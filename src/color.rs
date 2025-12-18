use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Pre-computed color lookup table (256 entries for fast gradient access)
pub type ColorLut = [Color; 256];

/// Fast color lookup from pre-computed LUT (t should be 0.0-1.0)
#[inline]
pub fn map_from_lut(lut: &ColorLut, t: f32) -> Color {
    let idx = (t.clamp(0.0, 1.0) * 255.0) as usize;
    lut[idx]
}

/// Color schemes for visualization
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
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
        let (r, g, b) = self.map_rgb(t);
        Color::Rgb(r, g, b)
    }

    /// Map a value from 0.0-1.0 to raw RGB values (for video recording)
    pub fn map_rgb(&self, t: f32) -> (u8, u8, u8) {
        let t = t.clamp(0.0, 1.0);
        match self {
            ColorScheme::Ice => Self::ice_gradient(t),
            ColorScheme::Fire => Self::fire_gradient(t),
            ColorScheme::Plasma => Self::plasma_gradient(t),
            ColorScheme::Viridis => Self::viridis_gradient(t),
            ColorScheme::Rainbow => Self::rainbow_gradient(t),
            ColorScheme::Grayscale => Self::grayscale_gradient(t),
            ColorScheme::Ocean => Self::ocean_gradient(t),
            ColorScheme::Neon => Self::neon_gradient(t),
        }
    }

    /// Build a 256-entry lookup table for fast color access
    /// Call this once when color scheme changes, then use map_from_lut() for rendering
    pub fn build_lut(&self) -> ColorLut {
        let mut lut = [Color::White; 256];
        for i in 0..256 {
            lut[i] = self.map(i as f32 / 255.0);
        }
        lut
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

/// UI color supporting both named terminal colors and hex RGB values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    /// Custom RGB color (for hex values)
    Rgb(u8, u8, u8),
}

impl Default for UiColor {
    fn default() -> Self {
        UiColor::White
    }
}

impl UiColor {
    /// Convert to ratatui Color for rendering
    pub fn to_color(&self) -> Color {
        match self {
            UiColor::Black => Color::Black,
            UiColor::Red => Color::Red,
            UiColor::Green => Color::Green,
            UiColor::Yellow => Color::Yellow,
            UiColor::Blue => Color::Blue,
            UiColor::Magenta => Color::Magenta,
            UiColor::Cyan => Color::Cyan,
            UiColor::Gray => Color::Gray,
            UiColor::DarkGray => Color::DarkGray,
            UiColor::LightRed => Color::LightRed,
            UiColor::LightGreen => Color::LightGreen,
            UiColor::LightYellow => Color::LightYellow,
            UiColor::LightBlue => Color::LightBlue,
            UiColor::LightMagenta => Color::LightMagenta,
            UiColor::LightCyan => Color::LightCyan,
            UiColor::White => Color::White,
            UiColor::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
        }
    }

    /// Get display name for the color (used in UI and serialization)
    pub fn name(&self) -> String {
        match self {
            UiColor::Black => "Black".to_string(),
            UiColor::Red => "Red".to_string(),
            UiColor::Green => "Green".to_string(),
            UiColor::Yellow => "Yellow".to_string(),
            UiColor::Blue => "Blue".to_string(),
            UiColor::Magenta => "Magenta".to_string(),
            UiColor::Cyan => "Cyan".to_string(),
            UiColor::Gray => "Gray".to_string(),
            UiColor::DarkGray => "DarkGray".to_string(),
            UiColor::LightRed => "LightRed".to_string(),
            UiColor::LightGreen => "LightGreen".to_string(),
            UiColor::LightYellow => "LightYellow".to_string(),
            UiColor::LightBlue => "LightBlue".to_string(),
            UiColor::LightMagenta => "LightMagenta".to_string(),
            UiColor::LightCyan => "LightCyan".to_string(),
            UiColor::White => "White".to_string(),
            UiColor::Rgb(r, g, b) => format!("#{:02X}{:02X}{:02X}", r, g, b),
        }
    }

    /// Cycle to next named color (for j/k adjustment)
    pub fn next(&self) -> Self {
        match self {
            UiColor::Black => UiColor::Red,
            UiColor::Red => UiColor::Green,
            UiColor::Green => UiColor::Yellow,
            UiColor::Yellow => UiColor::Blue,
            UiColor::Blue => UiColor::Magenta,
            UiColor::Magenta => UiColor::Cyan,
            UiColor::Cyan => UiColor::Gray,
            UiColor::Gray => UiColor::DarkGray,
            UiColor::DarkGray => UiColor::LightRed,
            UiColor::LightRed => UiColor::LightGreen,
            UiColor::LightGreen => UiColor::LightYellow,
            UiColor::LightYellow => UiColor::LightBlue,
            UiColor::LightBlue => UiColor::LightMagenta,
            UiColor::LightMagenta => UiColor::LightCyan,
            UiColor::LightCyan => UiColor::White,
            UiColor::White => UiColor::Black,
            UiColor::Rgb(_, _, _) => UiColor::Black, // Reset to named colors on cycle
        }
    }

    /// Cycle to previous named color
    pub fn prev(&self) -> Self {
        match self {
            UiColor::Black => UiColor::White,
            UiColor::Red => UiColor::Black,
            UiColor::Green => UiColor::Red,
            UiColor::Yellow => UiColor::Green,
            UiColor::Blue => UiColor::Yellow,
            UiColor::Magenta => UiColor::Blue,
            UiColor::Cyan => UiColor::Magenta,
            UiColor::Gray => UiColor::Cyan,
            UiColor::DarkGray => UiColor::Gray,
            UiColor::LightRed => UiColor::DarkGray,
            UiColor::LightGreen => UiColor::LightRed,
            UiColor::LightYellow => UiColor::LightGreen,
            UiColor::LightBlue => UiColor::LightYellow,
            UiColor::LightMagenta => UiColor::LightBlue,
            UiColor::LightCyan => UiColor::LightMagenta,
            UiColor::White => UiColor::LightCyan,
            UiColor::Rgb(_, _, _) => UiColor::White, // Reset to named colors on cycle
        }
    }

    /// Parse from hex string (e.g., "#FF5500" or "FF5500")
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(UiColor::Rgb(r, g, b))
    }

    /// Parse from named color string
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "black" => Some(UiColor::Black),
            "red" => Some(UiColor::Red),
            "green" => Some(UiColor::Green),
            "yellow" => Some(UiColor::Yellow),
            "blue" => Some(UiColor::Blue),
            "magenta" => Some(UiColor::Magenta),
            "cyan" => Some(UiColor::Cyan),
            "gray" | "grey" => Some(UiColor::Gray),
            "darkgray" | "darkgrey" => Some(UiColor::DarkGray),
            "lightred" => Some(UiColor::LightRed),
            "lightgreen" => Some(UiColor::LightGreen),
            "lightyellow" => Some(UiColor::LightYellow),
            "lightblue" => Some(UiColor::LightBlue),
            "lightmagenta" => Some(UiColor::LightMagenta),
            "lightcyan" => Some(UiColor::LightCyan),
            "white" => Some(UiColor::White),
            _ => None,
        }
    }

    /// Parse from string (tries hex first, then named colors)
    pub fn from_str(s: &str) -> Option<Self> {
        if s.starts_with('#') || s.chars().all(|c| c.is_ascii_hexdigit()) && s.len() == 6 {
            Self::from_hex(s)
        } else {
            Self::from_name(s)
        }
    }
}

// Custom serialization for human-readable color strings
impl Serialize for UiColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.name())
    }
}

impl<'de> Deserialize<'de> for UiColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Try hex first (starts with #)
        if s.starts_with('#') {
            UiColor::from_hex(&s)
                .ok_or_else(|| serde::de::Error::custom(format!("Invalid hex color: {}", s)))
        } else {
            // Parse named color
            UiColor::from_name(&s)
                .ok_or_else(|| serde::de::Error::custom(format!("Unknown color: {}", s)))
        }
    }
}

/// UI theme containing all customizable UI colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTheme {
    pub border_color: UiColor,
    pub text_color: UiColor,
    pub highlight_color: UiColor,
    pub dim_text_color: UiColor,
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            border_color: UiColor::Cyan,
            text_color: UiColor::White,
            highlight_color: UiColor::Yellow,
            dim_text_color: UiColor::Gray,
        }
    }
}

impl UiTheme {
    /// Get border color as ratatui Color
    pub fn border(&self) -> Color {
        self.border_color.to_color()
    }

    /// Get text color as ratatui Color
    pub fn text(&self) -> Color {
        self.text_color.to_color()
    }

    /// Get highlight color as ratatui Color
    pub fn highlight(&self) -> Color {
        self.highlight_color.to_color()
    }

    /// Get dim text color as ratatui Color
    pub fn dim_text(&self) -> Color {
        self.dim_text_color.to_color()
    }
}
