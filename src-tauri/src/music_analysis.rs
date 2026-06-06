use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioSummary {
    pub sample_rate: u32,
    pub detected_bpm: f64,
    pub rms_energy_mean: f64,
    pub rms_energy_max: f64,
    pub spectral_centroid_mean: f64,
    pub spectral_flatness_mean: f64,
    pub zero_crossing_rate_mean: f64,
    pub chroma_mean: Option<Vec<f64>>,
    pub spectral_contrast_mean: Option<Vec<f64>>,
    pub analysis_mode: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimingGrid {
    pub initial_offset: f64,
    pub bpms: Vec<(f64, f64)>,
    pub display_bpm: String,
    pub song_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AudioEventSummary {
    pub onset_strength: f64,
    pub energy: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeatFrame {
    pub beat: f64,
    pub time_seconds: f64,
    pub measure_index: u32,
    pub beat_in_measure: f64,
    pub bpm: f64,
    pub confidence: f64,
    pub audio_event_summary: AudioEventSummary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventFeatures {
    pub beats: Vec<BeatFrame>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SectionFrame {
    pub section_id: String,
    pub start_beat: f64,
    pub end_beat: f64,
    pub start_measure: u32,
    pub end_measure: u32,
    pub music_role: String,
    pub piu_role: String,
    pub boundary_confidence: f64,
    pub energy_profile: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccentFrame {
    pub beat: f64,
    pub strength: f64,
    pub suggestion: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RestFrame {
    pub beat: f64,
    pub strength: f64,
    pub suggestion: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChoreographicIntentMap {
    pub schema_version: String,
    pub section_id: String,
    pub mode: String,
    pub target_level: u32,
    pub measure_start: u32,
    pub measure_end: u32,
    pub density_target: String,
    pub difficulty_budget: f64,
    pub recommended_pattern_families: Vec<String>,
    pub avoid_pattern_families: Vec<String>,
    pub accent_plan: Vec<AccentFrame>,
    pub rest_plan: Vec<RestFrame>,
    pub motif_strategy: String,
    pub evidence: Vec<String>,
    pub confidence: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimingDiagnostics {
    pub audio_bpm_detected: f64,
    pub ssc_initial_bpm: f64,
    pub audio_vs_ssc_tempo_agreement: bool,
    pub beat_grid_error_ms_mean: f64,
    pub timing_confidence: f64,
    pub requires_manual_timing_review: bool,
    pub warnings: Vec<String>,
    pub analysis_mode: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Publicability {
    pub contains_original_audio: bool,
    pub contains_full_chart: bool,
    pub exportable: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SongAnalysisReport {
    pub schema_version: String,
    pub song_id: String,
    pub title: String,
    pub artist: String,
    pub duration_seconds: f64,
    pub audio_summary: AudioSummary,
    pub timing_grid: TimingGrid,
    pub event_features: EventFeatures,
    pub sections: Vec<SectionFrame>,
    pub choreographic_intent: Vec<ChoreographicIntentMap>,
    pub diagnostics: TimingDiagnostics,
    pub publicability: Publicability,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalysisCommandResult {
    pub report: SongAnalysisReport,
    pub report_path: Option<String>,
    pub analysis_mode: String,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn analyze_song_offline(
    ssc_path: String,
    audio_path: String,
    write_report: bool,
) -> Result<AnalysisCommandResult, String> {
    // 1. Validate paths
    let ssc_p = Path::new(&ssc_path);
    let audio_p = Path::new(&audio_path);

    if !ssc_p.exists() || !ssc_p.is_file() {
        return Err(format!(
            "SSC file path does not exist or is not a file: {}",
            ssc_path
        ));
    }
    if !audio_p.exists() || !audio_p.is_file() {
        return Err(format!(
            "Audio file path does not exist or is not a file: {}",
            audio_path
        ));
    }

    // 2. Resolve sidecar script path
    let dev_paths = vec![
        PathBuf::from("sidecar/music_analysis/analyze_song.py"),
        PathBuf::from("../sidecar/music_analysis/analyze_song.py"),
    ];

    let mut script_path = None;
    for path in dev_paths {
        if path.exists() {
            script_path = Some(path);
            break;
        }
    }

    let script_path = match script_path {
        Some(p) => p,
        None => {
            return Err("Could not find sidecar python script analyze_song.py".to_string());
        }
    };

    // 3. Define output path if writing report
    let song_dir = ssc_p
        .parent()
        .ok_or_else(|| "Could not resolve parent directory of SSC path".to_string())?;
    let report_dir = song_dir.join(".ai-step-gen-analysis");
    let report_file = report_dir.join("song-analysis-report.v1.json");

    // 4. Try running python, fallback to python3
    let python_execs = vec!["python", "python3"];
    let mut command_output = None;
    let mut last_error = String::new();

    for exec in python_execs {
        let mut cmd = Command::new(exec);
        cmd.arg(&script_path)
            .arg("--ssc-path")
            .arg(&ssc_path)
            .arg("--audio-path")
            .arg(&audio_path);

        if write_report {
            cmd.arg("--output").arg(&report_file);
            cmd.arg("--pretty");
        }

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    command_output = Some(output);
                    break;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    last_error = format!(
                        "Python exited with code {}.\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap_or(-1),
                        stdout,
                        stderr
                    );
                }
            }
            Err(e) => {
                last_error = format!("Failed to spawn command '{}': {}", exec, e);
            }
        }
    }

    let output = match command_output {
        Some(out) => out,
        None => {
            return Err(format!(
                "Error running offline analysis. Please ensure Python is installed and has the required dependencies (librosa, numpy, scipy, mutagen, soundfile).\nDetails:\n{}",
                last_error
            ));
        }
    };

    let result_json = String::from_utf8(output.stdout)
        .map_err(|e| format!("Failed to decode output as UTF-8: {}", e))?;

    let report: SongAnalysisReport = serde_json::from_str(&result_json).map_err(|e| {
        format!(
            "Failed to parse song analysis report JSON: {}. Ensure the Python script returns a valid contract.",
            e
        )
    })?;

    let report_path = if write_report {
        if !report_file.exists() || !report_file.is_file() {
            return Err(format!(
                "Report file was not saved successfully to disk: {}",
                report_file.to_string_lossy()
            ));
        }
        Some(report_file.to_string_lossy().into_owned())
    } else {
        None
    };

    Ok(AnalysisCommandResult {
        analysis_mode: report.diagnostics.analysis_mode.clone(),
        warnings: report.diagnostics.warnings.clone(),
        report_path,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_song_analysis_report() {
        let mock_report_json = r#"{
            "schema_version": "music-analysis-report.v1",
            "song_id": "test-id",
            "title": "Mock Song",
            "artist": "Mock Artist",
            "duration_seconds": 120.5,
            "audio_summary": {
                "sample_rate": 44100,
                "detected_bpm": 130.0,
                "rms_energy_mean": 0.15,
                "rms_energy_max": 0.35,
                "spectral_centroid_mean": 1500.0,
                "spectral_flatness_mean": 0.05,
                "zero_crossing_rate_mean": 0.08,
                "chroma_mean": [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.9, 0.8],
                "spectral_contrast_mean": [10.0, 12.0, 15.0, 14.0, 13.0, 12.0, 11.0],
                "analysis_mode": "dsp"
            },
            "timing_grid": {
                "initial_offset": -0.123,
                "bpms": [[0.0, 130.0]],
                "display_bpm": "130.000",
                "song_type": "ARCADE"
            },
            "event_features": {
                "beats": [
                    {
                        "beat": 0.0,
                        "time_seconds": 0.123,
                        "measure_index": 0,
                        "beat_in_measure": 0.0,
                        "bpm": 130.0,
                        "confidence": 1.0,
                        "audio_event_summary": {
                            "onset_strength": 0.85,
                            "energy": 0.2
                        }
                    }
                ]
            },
            "sections": [
                {
                    "section_id": "section_1",
                    "start_beat": 0.0,
                    "end_beat": 32.0,
                    "start_measure": 0,
                    "end_measure": 8,
                    "music_role": "intro",
                    "piu_role": "warmup",
                    "boundary_confidence": 0.8,
                    "energy_profile": "moderate"
                }
            ],
            "choreographic_intent": [
                {
                    "schema_version": "choreographic-intent.v1",
                    "section_id": "section_1",
                    "mode": "Single",
                    "target_level": 10,
                    "measure_start": 0,
                    "measure_end": 8,
                    "density_target": "medium",
                    "difficulty_budget": 0.5,
                    "recommended_pattern_families": ["stream"],
                    "avoid_pattern_families": ["holds"],
                    "accent_plan": [],
                    "rest_plan": [],
                    "motif_strategy": "introduce_theme",
                    "evidence": ["test"],
                    "confidence": 0.85
                }
            ],
            "diagnostics": {
                "audio_bpm_detected": 130.0,
                "ssc_initial_bpm": 130.0,
                "audio_vs_ssc_tempo_agreement": true,
                "beat_grid_error_ms_mean": 0.0,
                "timing_confidence": 1.0,
                "requires_manual_timing_review": false,
                "warnings": [],
                "analysis_mode": "dsp"
            },
            "publicability": {
                "contains_original_audio": false,
                "contains_full_chart": false,
                "exportable": true
            }
        }"#;

        let parsed: SongAnalysisReport = serde_json::from_str(mock_report_json).unwrap();
        assert_eq!(parsed.schema_version, "music-analysis-report.v1");
        assert_eq!(parsed.title, "Mock Song");
        assert_eq!(parsed.audio_summary.detected_bpm, 130.0);
        assert_eq!(parsed.audio_summary.analysis_mode, "dsp");
        assert!(parsed.audio_summary.chroma_mean.is_some());
        assert_eq!(parsed.diagnostics.analysis_mode, "dsp");
        assert_eq!(parsed.sections[0].piu_role, "warmup");
        assert_eq!(parsed.choreographic_intent[0].density_target, "medium");
    }

    #[test]
    fn test_parse_song_analysis_report_invalid_types() {
        // requires_manual_timing_review as string "" instead of boolean should fail parsing
        let mock_invalid_report_json = r#"{
            "schema_version": "music-analysis-report.v1",
            "song_id": "test-id",
            "title": "Mock Song",
            "artist": "Mock Artist",
            "duration_seconds": 120.5,
            "audio_summary": {
                "sample_rate": 44100,
                "detected_bpm": 130.0,
                "rms_energy_mean": 0.15,
                "rms_energy_max": 0.35,
                "spectral_centroid_mean": 1500.0,
                "spectral_flatness_mean": 0.05,
                "zero_crossing_rate_mean": 0.08,
                "chroma_mean": null,
                "spectral_contrast_mean": null,
                "analysis_mode": "dsp"
            },
            "timing_grid": {
                "initial_offset": -0.123,
                "bpms": [[0.0, 130.0]],
                "display_bpm": "130.000",
                "song_type": "ARCADE"
            },
            "event_features": { "beats": [] },
            "sections": [],
            "choreographic_intent": [],
            "diagnostics": {
                "audio_bpm_detected": 130.0,
                "ssc_initial_bpm": 130.0,
                "audio_vs_ssc_tempo_agreement": true,
                "beat_grid_error_ms_mean": 0.0,
                "timing_confidence": 1.0,
                "requires_manual_timing_review": "",
                "warnings": [],
                "analysis_mode": "dsp"
            },
            "publicability": {
                "contains_original_audio": false,
                "contains_full_chart": false,
                "exportable": true
            }
        }"#;

        let parsed: Result<SongAnalysisReport, serde_json::Error> =
            serde_json::from_str(mock_invalid_report_json);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_analyze_song_offline_invalid_paths() {
        let result = analyze_song_offline(
            "nonexistent_file.ssc".to_string(),
            "nonexistent_audio.mp3".to_string(),
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }
}
