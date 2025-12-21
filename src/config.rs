use crate::color::ColorScheme;
use crate::settings::SimulationSettings;
use crate::simulation::SeedPattern;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Complete application configuration for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Version field for future compatibility
    pub version: u32,
    /// All simulation settings
    pub settings: SimulationSettings,
    /// Seed pattern
    pub seed_pattern: SeedPattern,
    /// Base stickiness (simulation-level)
    pub stickiness: f32,
    /// Number of particles
    pub num_particles: usize,
    /// Color scheme (app-level)
    pub color_scheme: ColorScheme,
    /// Steps per frame (app-level)
    pub steps_per_frame: usize,
    /// Color by age toggle (app-level)
    pub color_by_age: bool,
}

impl AppConfig {
    /// Export config to a JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, json).map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }

    /// Import config from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            settings: SimulationSettings::default(),
            seed_pattern: SeedPattern::default(),
            stickiness: 1.0,
            num_particles: 5000,
            color_scheme: ColorScheme::default(),
            steps_per_frame: 5,
            color_by_age: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{BoundaryBehavior, ColorMode, NeighborhoodType, SpawnMode};
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig {
            version: 1,
            settings: SimulationSettings {
                walk_step_size: 3.5,
                walk_bias_angle: 45.0,
                walk_bias_strength: 0.25,
                radial_bias: -0.1,
                adaptive_step: true,
                adaptive_step_factor: 5.0,
                lattice_walk: false,
                neighborhood: NeighborhoodType::VonNeumann,
                multi_contact_min: 2,
                tip_stickiness: 0.8,
                side_stickiness: 0.6,
                stickiness_gradient: 0.2,
                spawn_mode: SpawnMode::Edges,
                boundary_behavior: BoundaryBehavior::Wrap,
                spawn_radius_offset: 15.0,
                escape_multiplier: 3.0,
                min_spawn_radius: 30.0,
                max_walk_iterations: 5000,
                color_mode: ColorMode::Distance,
                highlight_recent: 10,
                invert_colors: true,
            },
            seed_pattern: SeedPattern::Cross,
            stickiness: 0.7,
            num_particles: 3000,
            color_scheme: ColorScheme::Fire,
            steps_per_frame: 10,
            color_by_age: false,
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&config).unwrap();

        // Deserialize back
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        // Verify all fields
        assert_eq!(parsed.version, config.version);
        assert_eq!(
            parsed.settings.walk_step_size,
            config.settings.walk_step_size
        );
        assert_eq!(parsed.settings.neighborhood, config.settings.neighborhood);
        assert_eq!(parsed.seed_pattern, config.seed_pattern);
        assert_eq!(parsed.stickiness, config.stickiness);
        assert_eq!(parsed.num_particles, config.num_particles);
        assert_eq!(parsed.color_scheme, config.color_scheme);
        assert_eq!(parsed.steps_per_frame, config.steps_per_frame);
        assert_eq!(parsed.color_by_age, config.color_by_age);
    }

    #[test]
    fn test_config_file_save_and_load() {
        let config = AppConfig::default();

        // Create temp file
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Save
        config.save_to_file(&path).unwrap();

        // Load
        let loaded = AppConfig::load_from_file(&path).unwrap();

        assert_eq!(loaded.version, config.version);
        assert_eq!(loaded.num_particles, config.num_particles);
    }

    #[test]
    fn test_all_fields_preserved() {
        // Create config with non-default values for every field
        let original = AppConfig {
            version: 1,
            settings: SimulationSettings {
                walk_step_size: 4.0,
                walk_bias_angle: 180.0,
                walk_bias_strength: 0.4,
                radial_bias: 0.2,
                adaptive_step: true,
                adaptive_step_factor: 8.0,
                lattice_walk: false,
                neighborhood: NeighborhoodType::Extended,
                multi_contact_min: 3,
                tip_stickiness: 0.5,
                side_stickiness: 0.9,
                stickiness_gradient: -0.3,
                spawn_mode: SpawnMode::Corners,
                boundary_behavior: BoundaryBehavior::Bounce,
                spawn_radius_offset: 25.0,
                escape_multiplier: 4.5,
                min_spawn_radius: 60.0,
                max_walk_iterations: 20000,
                color_mode: ColorMode::Density,
                highlight_recent: 25,
                invert_colors: true,
            },
            seed_pattern: SeedPattern::Starburst,
            stickiness: 0.5,
            num_particles: 8000,
            color_scheme: ColorScheme::Neon,
            steps_per_frame: 25,
            color_by_age: false,
        };

        let json = serde_json::to_string(&original).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        // Comprehensive field check
        assert_eq!(restored.settings.walk_step_size, 4.0);
        assert_eq!(restored.settings.walk_bias_angle, 180.0);
        assert_eq!(restored.settings.walk_bias_strength, 0.4);
        assert_eq!(restored.settings.radial_bias, 0.2);
        assert_eq!(restored.settings.neighborhood, NeighborhoodType::Extended);
        assert_eq!(restored.settings.multi_contact_min, 3);
        assert_eq!(restored.settings.tip_stickiness, 0.5);
        assert_eq!(restored.settings.side_stickiness, 0.9);
        assert_eq!(restored.settings.stickiness_gradient, -0.3);
        assert_eq!(restored.settings.spawn_mode, SpawnMode::Corners);
        assert_eq!(
            restored.settings.boundary_behavior,
            BoundaryBehavior::Bounce
        );
        assert_eq!(restored.settings.spawn_radius_offset, 25.0);
        assert_eq!(restored.settings.escape_multiplier, 4.5);
        assert_eq!(restored.settings.min_spawn_radius, 60.0);
        assert_eq!(restored.settings.max_walk_iterations, 20000);
        assert_eq!(restored.settings.color_mode, ColorMode::Density);
        assert_eq!(restored.settings.highlight_recent, 25);
        assert!(restored.settings.invert_colors);
        assert_eq!(restored.seed_pattern, SeedPattern::Starburst);
        assert_eq!(restored.stickiness, 0.5);
        assert_eq!(restored.num_particles, 8000);
        assert_eq!(restored.color_scheme, ColorScheme::Neon);
        assert_eq!(restored.steps_per_frame, 25);
        assert!(!restored.color_by_age);
    }

    #[test]
    fn test_invalid_config_file() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "not valid json").unwrap();

        let result = AppConfig::load_from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_config_file() {
        let result = AppConfig::load_from_file(Path::new("/nonexistent/path/config.json"));
        assert!(result.is_err());
    }
}
