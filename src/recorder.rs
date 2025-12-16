//! Recording functionality for capturing simulation as video/GIF.
//!
//! Supports two output modes:
//! - MP4/WebM via FFmpeg (if installed)
//! - GIF via native Rust (fallback)

use crate::color::ColorScheme;
use crate::settings::ColorMode;
use crate::simulation::DlaSimulation;
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::Instant;

/// RGB frame buffer for video encoding
pub struct RgbFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGB triplets, row-major
}

impl RgbFrame {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 3) as usize],
        }
    }
}

/// Output format for recording
#[derive(Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Mp4,
    WebM,
    Gif,
}

impl OutputFormat {
    /// Detect format from filename extension
    pub fn from_filename(filename: &str) -> Self {
        let lower = filename.to_lowercase();
        if lower.ends_with(".gif") {
            OutputFormat::Gif
        } else if lower.ends_with(".webm") {
            OutputFormat::WebM
        } else {
            OutputFormat::Mp4
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Mp4 => ".mp4",
            OutputFormat::WebM => ".webm",
            OutputFormat::Gif => ".gif",
        }
    }
}

/// Recording configuration
pub struct RecordingConfig {
    /// Video pixels per simulation pixel (default: 4)
    pub pixel_scale: u32,
    /// Target framerate (default: 30)
    pub framerate: u32,
    /// Background color RGB
    pub background_color: (u8, u8, u8),
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            pixel_scale: 4,
            framerate: 30,
            background_color: (0, 0, 0), // Black background
        }
    }
}

/// Recording state machine
pub enum RecordingState {
    Idle,
    Recording {
        encoder: Box<dyn FrameEncoder>,
        frame_count: usize,
        start_time: Instant,
        filename: String,
        /// Track frames for 30fps capture from 60fps loop
        frame_skip_counter: u8,
    },
}

/// Trait for frame encoders (FFmpeg or GIF)
pub trait FrameEncoder: Send {
    fn add_frame(&mut self, frame: &RgbFrame) -> Result<(), String>;
    fn finish(self: Box<Self>) -> Result<(), String>;
    fn format_name(&self) -> &str;
}

/// FFmpeg-based encoder for MP4/WebM output
pub struct FfmpegEncoder {
    child: Child,
    stdin: ChildStdin,
    format: OutputFormat,
}

impl FfmpegEncoder {
    /// Check if FFmpeg is available on the system
    pub fn is_available() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Create a new FFmpeg encoder
    pub fn new(
        filename: &str,
        width: u32,
        height: u32,
        fps: u32,
        format: OutputFormat,
    ) -> Result<Self, String> {
        if !Self::is_available() {
            return Err("FFmpeg not found. Install FFmpeg or use .gif extension.".to_string());
        }

        let codec_args: Vec<&str> = match format {
            OutputFormat::Mp4 => vec!["-c:v", "libx264", "-preset", "fast", "-crf", "23", "-pix_fmt", "yuv420p"],
            OutputFormat::WebM => vec!["-c:v", "libvpx-vp9", "-crf", "30", "-b:v", "0"],
            OutputFormat::Gif => {
                return Err("Use GifEncoder for GIF output".to_string());
            }
        };

        let mut child = Command::new("ffmpeg")
            .args([
                "-y",                                    // Overwrite output
                "-f", "rawvideo",                        // Input format
                "-pix_fmt", "rgb24",                     // Pixel format
                "-s", &format!("{}x{}", width, height),  // Size
                "-r", &fps.to_string(),                  // Framerate
                "-i", "-",                               // Read from stdin
            ])
            .args(&codec_args)
            .arg(filename)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn FFmpeg: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to get FFmpeg stdin")?;

        Ok(Self {
            child,
            stdin,
            format,
        })
    }
}

impl FrameEncoder for FfmpegEncoder {
    fn add_frame(&mut self, frame: &RgbFrame) -> Result<(), String> {
        self.stdin
            .write_all(&frame.pixels)
            .map_err(|e| format!("Failed to write frame: {}", e))
    }

    fn finish(mut self: Box<Self>) -> Result<(), String> {
        drop(self.stdin); // Close stdin to signal EOF
        self.child
            .wait()
            .map_err(|e| format!("FFmpeg failed: {}", e))?;
        Ok(())
    }

    fn format_name(&self) -> &str {
        match self.format {
            OutputFormat::Mp4 => "MP4 (FFmpeg)",
            OutputFormat::WebM => "WebM (FFmpeg)",
            OutputFormat::Gif => "GIF",
        }
    }
}

/// GIF encoder using the gif crate
pub struct GifEncoder {
    encoder: gif::Encoder<std::fs::File>,
    width: u16,
    height: u16,
    frame_delay: u16, // In centiseconds
}

impl GifEncoder {
    /// Create a new GIF encoder
    pub fn new(filename: &str, width: u32, height: u32, fps: u32) -> Result<Self, String> {
        let file = std::fs::File::create(filename)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut encoder = gif::Encoder::new(file, width as u16, height as u16, &[])
            .map_err(|e| format!("Failed to create GIF encoder: {}", e))?;

        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(|e| format!("Failed to set repeat: {}", e))?;

        // Convert FPS to centiseconds delay
        let frame_delay = (100 / fps).max(1) as u16;

        Ok(Self {
            encoder,
            width: width as u16,
            height: height as u16,
            frame_delay,
        })
    }

    /// Simple color quantization using median cut-like approach
    fn quantize_frame(pixels: &[u8]) -> (Vec<u8>, Vec<u8>) {
        // Build a simple 256-color palette from the image
        use std::collections::HashMap;

        // Count color frequencies
        let mut color_counts: HashMap<(u8, u8, u8), usize> = HashMap::new();
        for chunk in pixels.chunks_exact(3) {
            let color = (chunk[0], chunk[1], chunk[2]);
            *color_counts.entry(color).or_insert(0) += 1;
        }

        // Get most frequent colors (up to 256)
        let mut colors: Vec<_> = color_counts.into_iter().collect();
        colors.sort_by(|a, b| b.1.cmp(&a.1));
        colors.truncate(256);

        // Build palette
        let mut palette = Vec::with_capacity(256 * 3);
        for (color, _) in &colors {
            palette.push(color.0);
            palette.push(color.1);
            palette.push(color.2);
        }

        // Pad palette to 256 colors
        while palette.len() < 256 * 3 {
            palette.push(0);
        }

        // Build color lookup for fast indexing
        let color_to_idx: HashMap<(u8, u8, u8), u8> = colors
            .iter()
            .enumerate()
            .map(|(i, (c, _))| (*c, i as u8))
            .collect();

        // Map pixels to indices
        let mut indices = Vec::with_capacity(pixels.len() / 3);
        for chunk in pixels.chunks_exact(3) {
            let color = (chunk[0], chunk[1], chunk[2]);
            let idx = color_to_idx.get(&color).copied().unwrap_or_else(|| {
                // Find closest color in palette
                let mut best_idx = 0u8;
                let mut best_dist = u32::MAX;
                for (i, (c, _)) in colors.iter().enumerate() {
                    let dr = (color.0 as i32 - c.0 as i32).abs() as u32;
                    let dg = (color.1 as i32 - c.1 as i32).abs() as u32;
                    let db = (color.2 as i32 - c.2 as i32).abs() as u32;
                    let dist = dr * dr + dg * dg + db * db;
                    if dist < best_dist {
                        best_dist = dist;
                        best_idx = i as u8;
                    }
                }
                best_idx
            });
            indices.push(idx);
        }

        (indices, palette)
    }
}

impl FrameEncoder for GifEncoder {
    fn add_frame(&mut self, frame: &RgbFrame) -> Result<(), String> {
        let (indices, palette) = Self::quantize_frame(&frame.pixels);

        let mut gif_frame = gif::Frame::default();
        gif_frame.width = self.width;
        gif_frame.height = self.height;
        gif_frame.delay = self.frame_delay;
        gif_frame.buffer = std::borrow::Cow::Owned(indices);

        // Set local palette for this frame
        gif_frame.palette = Some(palette);

        self.encoder
            .write_frame(&gif_frame)
            .map_err(|e| format!("Failed to write GIF frame: {}", e))
    }

    fn finish(self: Box<Self>) -> Result<(), String> {
        // Encoder finalizes on drop
        Ok(())
    }

    fn format_name(&self) -> &str {
        "GIF"
    }
}

/// Main recorder struct
pub struct Recorder {
    pub state: RecordingState,
    pub config: RecordingConfig,
    /// Reusable frame buffer to avoid allocations
    frame_buffer: Option<RgbFrame>,
    /// Video dimensions (locked when recording starts)
    video_width: u32,
    video_height: u32,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            state: RecordingState::Idle,
            config: RecordingConfig::default(),
            frame_buffer: None,
            video_width: 0,
            video_height: 0,
        }
    }
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        matches!(self.state, RecordingState::Recording { .. })
    }

    /// Get current frame count (if recording)
    pub fn frame_count(&self) -> Option<usize> {
        match &self.state {
            RecordingState::Recording { frame_count, .. } => Some(*frame_count),
            _ => None,
        }
    }

    /// Get elapsed time (if recording)
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        match &self.state {
            RecordingState::Recording { start_time, .. } => Some(start_time.elapsed()),
            _ => None,
        }
    }

    /// Start recording
    pub fn start(
        &mut self,
        filename: String,
        sim_width: usize,
        sim_height: usize,
    ) -> Result<(), String> {
        if self.is_recording() {
            return Err("Already recording".to_string());
        }

        // Calculate video dimensions
        self.video_width = sim_width as u32 * self.config.pixel_scale;
        self.video_height = sim_height as u32 * self.config.pixel_scale;

        // Ensure even dimensions for video codecs
        self.video_width = (self.video_width / 2) * 2;
        self.video_height = (self.video_height / 2) * 2;

        let format = OutputFormat::from_filename(&filename);

        // Ensure filename has correct extension
        let filename = if !filename.to_lowercase().ends_with(format.extension()) {
            format!("{}{}", filename, format.extension())
        } else {
            filename
        };

        // Create encoder
        let encoder: Box<dyn FrameEncoder> = match format {
            OutputFormat::Gif => {
                Box::new(GifEncoder::new(
                    &filename,
                    self.video_width,
                    self.video_height,
                    self.config.framerate,
                )?)
            }
            _ => {
                // Try FFmpeg first, fall back to GIF
                if FfmpegEncoder::is_available() {
                    Box::new(FfmpegEncoder::new(
                        &filename,
                        self.video_width,
                        self.video_height,
                        self.config.framerate,
                        format,
                    )?)
                } else {
                    // Fall back to GIF
                    let gif_filename = filename.replace(".mp4", ".gif").replace(".webm", ".gif");
                    Box::new(GifEncoder::new(
                        &gif_filename,
                        self.video_width,
                        self.video_height,
                        self.config.framerate,
                    )?)
                }
            }
        };

        // Allocate frame buffer
        self.frame_buffer = Some(RgbFrame::new(self.video_width, self.video_height));

        self.state = RecordingState::Recording {
            encoder,
            frame_count: 0,
            start_time: Instant::now(),
            filename,
            frame_skip_counter: 0,
        };

        Ok(())
    }

    /// Stop recording and finalize the file
    pub fn stop(&mut self) -> Result<String, String> {
        let state = std::mem::replace(&mut self.state, RecordingState::Idle);

        match state {
            RecordingState::Recording {
                encoder, filename, frame_count, ..
            } => {
                encoder.finish()?;
                self.frame_buffer = None;
                Ok(format!("Saved {} frames to {}", frame_count, filename))
            }
            RecordingState::Idle => Err("Not recording".to_string()),
        }
    }

    /// Check if we should capture a frame (for 30fps from 60fps loop)
    pub fn should_capture(&mut self) -> bool {
        if let RecordingState::Recording { frame_skip_counter, .. } = &mut self.state {
            *frame_skip_counter = (*frame_skip_counter + 1) % 2;
            *frame_skip_counter == 0
        } else {
            false
        }
    }

    /// Capture and encode a frame
    pub fn capture_frame(
        &mut self,
        simulation: &DlaSimulation,
        color_scheme: &ColorScheme,
        color_by_age: bool,
        color_mode: ColorMode,
        invert_colors: bool,
    ) -> Result<(), String> {
        if !self.is_recording() {
            return Ok(());
        }

        // Take frame buffer temporarily to avoid borrow conflicts
        let mut frame = self.frame_buffer.take().ok_or("No frame buffer")?;

        // Render simulation to frame
        Self::render_frame_static(
            &mut frame,
            simulation,
            color_scheme,
            color_by_age,
            color_mode,
            invert_colors,
            self.config.pixel_scale,
            self.config.background_color,
        );

        // Encode frame
        if let RecordingState::Recording {
            encoder,
            frame_count,
            ..
        } = &mut self.state
        {
            encoder.add_frame(&frame)?;
            *frame_count += 1;
        }

        // Put frame buffer back
        self.frame_buffer = Some(frame);

        Ok(())
    }

    /// Render simulation state to RGB frame buffer (static version to avoid borrow issues)
    fn render_frame_static(
        frame: &mut RgbFrame,
        simulation: &DlaSimulation,
        color_scheme: &ColorScheme,
        color_by_age: bool,
        color_mode: ColorMode,
        invert_colors: bool,
        scale: u32,
        bg: (u8, u8, u8),
    ) {
        let sim_width = simulation.grid_width;
        let sim_height = simulation.grid_height;

        // Pre-calculate for color mapping
        let inv_num_particles = 1.0 / simulation.num_particles.max(1) as f32;
        let max_radius = simulation.max_radius.max(1.0);

        // Fill with background
        for chunk in frame.pixels.chunks_exact_mut(3) {
            chunk[0] = bg.0;
            chunk[1] = bg.1;
            chunk[2] = bg.2;
        }

        // Render each simulation pixel
        for sim_y in 0..sim_height {
            for sim_x in 0..sim_width {
                if let Some(particle) = simulation.get_particle(sim_x, sim_y) {
                    // Calculate color value based on mode
                    let value = match color_mode {
                        ColorMode::Age => particle.age as f32 * inv_num_particles,
                        ColorMode::Distance => particle.distance / max_radius,
                        ColorMode::Density => particle.neighbor_count as f32 / 8.0,
                        ColorMode::Direction => {
                            (particle.direction + std::f32::consts::PI) / std::f32::consts::TAU
                        }
                    };

                    // Get RGB color
                    let t = if invert_colors { 1.0 - value } else { value };
                    let color = if color_by_age {
                        color_scheme.map_rgb(t)
                    } else {
                        (255, 255, 255)
                    };

                    // Write pixel block (scale x scale)
                    for py in 0..scale {
                        for px in 0..scale {
                            let vx = sim_x as u32 * scale + px;
                            let vy = sim_y as u32 * scale + py;

                            if vx < frame.width && vy < frame.height {
                                let idx = ((vy * frame.width + vx) * 3) as usize;
                                if idx + 2 < frame.pixels.len() {
                                    frame.pixels[idx] = color.0;
                                    frame.pixels[idx + 1] = color.1;
                                    frame.pixels[idx + 2] = color.2;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
