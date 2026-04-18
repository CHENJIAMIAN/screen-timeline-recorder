use std::path::PathBuf;

use screen_timeline_recorder::cli::{CliOptions, Command, load_config};
use screen_timeline_recorder::config::{
    RecorderConfig, SensitivityMode, Thresholds, ViewerLanguage,
};
use screen_timeline_recorder::recording_settings::RecordingSettings;

fn assert_f32_eq(left: f32, right: f32) {
    let delta = (left - right).abs();
    assert!(delta < 1e-6, "expected {left} to be within 1e-6 of {right}");
}

#[test]
fn default_configuration_values() {
    let config = RecorderConfig::default();

    assert_eq!(config.output_dir, PathBuf::from("output"));
    assert_eq!(config.sampling_interval_ms, 100);
    assert_eq!(config.block_width, 16);
    assert_eq!(config.block_height, 16);
    assert_eq!(config.keyframe_interval_ms, 30_000);
    assert_eq!(config.sensitivity_mode, SensitivityMode::Balanced);
    assert_f32_eq(config.working_scale, 1.0);
    assert_f32_eq(config.viewer_default_zoom, 1.0);
    assert!(config.viewer_overlay_enabled_by_default);
    assert!(config.burn_in_enabled);
    assert_eq!(config.viewer_language, ViewerLanguage::Auto);
    assert!(config.max_sessions.is_none());
    assert!(config.max_age_days.is_none());
    assert!(config.max_total_bytes.is_none());
}

#[test]
fn parses_configuration_from_local_file() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("recorder.toml");

    let toml = r#"
output_dir = "custom-output"
sampling_interval_ms = 1500
block_width = 16
block_height = 16
keyframe_interval_ms = 120000
sensitivity_mode = "detailed"
working_scale = 1.0
viewer_default_zoom = 1.25
viewer_overlay_enabled_by_default = false
burn_in_enabled = false
viewer_language = "zh"
max_sessions = 5
max_age_days = 14
max_total_bytes = 1048576
"#;

    std::fs::write(&config_path, toml).expect("write config");

    let config = RecorderConfig::from_path(&config_path).expect("load config");
    config.validate().expect("valid config");

    assert_eq!(config.output_dir, PathBuf::from("custom-output"));
    assert_eq!(config.sampling_interval_ms, 1500);
    assert_eq!(config.block_width, 16);
    assert_eq!(config.block_height, 16);
    assert_eq!(config.keyframe_interval_ms, 120_000);
    assert_eq!(config.sensitivity_mode, SensitivityMode::Detailed);
    assert_f32_eq(config.working_scale, 1.0);
    assert_f32_eq(config.viewer_default_zoom, 1.25);
    assert!(!config.viewer_overlay_enabled_by_default);
    assert!(!config.burn_in_enabled);
    assert_eq!(config.viewer_language, ViewerLanguage::Zh);
    assert_eq!(config.max_sessions, Some(5));
    assert_eq!(config.max_age_days, Some(14));
    assert_eq!(config.max_total_bytes, Some(1_048_576));
}

#[test]
fn honors_user_provided_output_dir_override() {
    let config = RecorderConfig::default().with_output_dir(PathBuf::from("override-output"));
    assert_eq!(config.output_dir, PathBuf::from("override-output"));
}

#[test]
fn rejects_invalid_values() {
    let mut config = RecorderConfig::default();
    config.block_width = 0;
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.block_height = 0;
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.keyframe_interval_ms = 0;
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.working_scale = 0.0;
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.working_scale = 1.5;
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.max_sessions = Some(0);
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.max_age_days = Some(0);
    assert!(config.validate().is_err());

    let mut config = RecorderConfig::default();
    config.max_total_bytes = Some(0);
    assert!(config.validate().is_err());
}

#[test]
fn maps_sensitivity_mode_to_thresholds() {
    let mut config = RecorderConfig::default();
    config.sensitivity_mode = SensitivityMode::Conservative;
    assert_eq!(
        config.thresholds(),
        Thresholds {
            precheck_threshold: 0.02,
            block_difference_threshold: 0.08,
            changed_pixel_ratio_threshold: 0.15,
            stability_window: 3,
        }
    );

    config.sensitivity_mode = SensitivityMode::Balanced;
    assert_eq!(
        config.thresholds(),
        Thresholds {
            precheck_threshold: 0.01,
            block_difference_threshold: 0.05,
            changed_pixel_ratio_threshold: 0.0,
            stability_window: 2,
        }
    );

    config.sensitivity_mode = SensitivityMode::Detailed;
    assert_eq!(
        config.thresholds(),
        Thresholds {
            precheck_threshold: 0.005,
            block_difference_threshold: 0.02,
            changed_pixel_ratio_threshold: 0.0,
            stability_window: 1,
        }
    );
}

#[test]
fn parses_viewer_defaults() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config_path = temp_dir.path().join("viewer.toml");

    let toml = r#"
viewer_default_zoom = 1.75
viewer_overlay_enabled_by_default = true
burn_in_enabled = false
viewer_language = "en"
"#;

    std::fs::write(&config_path, toml).expect("write config");

    let config = RecorderConfig::from_path(&config_path).expect("load config");
    assert_f32_eq(config.viewer_default_zoom, 1.75);
    assert!(config.viewer_overlay_enabled_by_default);
    assert!(!config.burn_in_enabled);
    assert_eq!(config.viewer_language, ViewerLanguage::En);
}

#[test]
fn load_config_applies_saved_recording_settings_when_no_config_file_is_used() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let saved = RecordingSettings {
        sampling_interval_ms: 350,
        block_width: 24,
        block_height: 28,
        keyframe_interval_ms: 45_000,
        sensitivity_mode: SensitivityMode::Detailed,
        working_scale: 0.65,
        burn_in_enabled: false,
    };
    std::fs::write(
        temp_dir.path().join("recording-settings.json"),
        serde_json::to_string_pretty(&saved).expect("serialize settings"),
    )
    .expect("write settings");

    let options = CliOptions {
        command: Command::Record { session_id: None },
        config_path: None,
        output_dir: Some(temp_dir.path().to_path_buf()),
    };

    let config = load_config(&options).expect("load config");

    assert_eq!(config.sampling_interval_ms, 350);
    assert_eq!(config.block_width, 24);
    assert_eq!(config.block_height, 28);
    assert_eq!(config.keyframe_interval_ms, 45_000);
    assert_eq!(config.sensitivity_mode, SensitivityMode::Detailed);
    assert_f32_eq(config.working_scale, 0.65);
    assert!(!config.burn_in_enabled);
}
