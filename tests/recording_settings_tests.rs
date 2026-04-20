use serde_json::json;
use tempfile::tempdir;

use screen_timeline_recorder::recording_settings::{
    RecordingSettings, load_recording_settings, save_recording_settings,
};

fn custom_settings() -> RecordingSettings {
    RecordingSettings {
        sampling_interval_ms: 375,
        working_scale: 0.65,
        burn_in_enabled: false,
    }
}

#[test]
fn defaults_when_file_missing() {
    let temp_dir = tempdir().expect("tempdir");

    let loaded = load_recording_settings(temp_dir.path()).expect("load defaults");

    assert_eq!(loaded, RecordingSettings::defaults());
}

#[test]
fn save_and_load_round_trip() {
    let temp_dir = tempdir().expect("tempdir");
    let expected = custom_settings();

    save_recording_settings(temp_dir.path(), &expected).expect("save settings");

    let actual = load_recording_settings(temp_dir.path()).expect("load settings");
    assert_eq!(actual, expected);
}

#[test]
fn invalid_settings_are_rejected() {
    let temp_dir = tempdir().expect("tempdir");

    let mut invalid = RecordingSettings::defaults();
    invalid.sampling_interval_ms = 0;
    assert!(save_recording_settings(temp_dir.path(), &invalid).is_err());

    let invalid_payload = json!({
        "sampling_interval_ms": 500,
        "working_scale": 1.25,
        "burn_in_enabled": true
    });

    std::fs::write(
        temp_dir.path().join("recording-settings.json"),
        invalid_payload.to_string(),
    )
    .expect("write invalid settings");

    assert!(load_recording_settings(temp_dir.path()).is_err());
}

#[test]
fn missing_burn_in_flag_defaults_to_enabled() {
    let temp_dir = tempdir().expect("tempdir");

    let legacy_payload = json!({
        "sampling_interval_ms": 300,
        "working_scale": 0.4
    });

    std::fs::write(
        temp_dir.path().join("recording-settings.json"),
        legacy_payload.to_string(),
    )
    .expect("write legacy settings");

    let loaded = load_recording_settings(temp_dir.path()).expect("load legacy settings");

    assert!(loaded.burn_in_enabled);
    assert_eq!(loaded.sampling_interval_ms, 300);
    assert_eq!(loaded.working_scale, 0.4);
}
