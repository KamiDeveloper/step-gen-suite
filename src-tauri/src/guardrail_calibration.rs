use crate::biomechanics::GeminiChartSectionPayload;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub min: f64,
    pub p10: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
    pub std_dev: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypicalLevelRange {
    pub min: u32,
    pub median: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationLevelThreshold {
    pub density: BTreeMap<String, f64>,
    pub jump_rate: BTreeMap<String, f64>,
    pub twist_rate: BTreeMap<String, f64>,
    pub bracket_candidate_rate: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyCalibrationSignal {
    pub pattern_family: String,
    pub classification_rule: String,
    pub classifier_thresholds: BTreeMap<String, f64>,
    pub sample_count: usize,
    pub sample_confidence: String,
    pub typical_level_range: TypicalLevelRange,
    pub metric_stats: StatsSummary,
    pub recommended_when: Vec<String>,
    pub avoid_when: Vec<String>,
    pub guardrail_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleGuardrailCalibration {
    pub schema_version: String,
    pub publicability_status: String,
    pub play_mode: String,
    pub source_dataset_summary: serde_json::Value,
    pub level_thresholds: BTreeMap<String, CalibrationLevelThreshold>,
    #[serde(default)]
    pub pattern_family_thresholds: BTreeMap<String, FamilyCalibrationSignal>,
    pub confidence_policy: BTreeMap<String, String>,
    pub recommended_runtime_usage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationWarning {
    pub issue_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationError {
    pub issue_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationValidationReport {
    pub calibration_available: bool,
    pub schema_version: Option<String>,
    pub target_level: Option<u32>,
    pub level_confidence: Option<String>,
    pub warnings: Vec<CalibrationWarning>,
    pub errors: Vec<CalibrationError>,
    pub matched_thresholds: Option<serde_json::Value>,
    pub pattern_family_signals: Option<serde_json::Value>,
    pub summary: String,
}

pub const FORBIDDEN_STRINGS: &[&str] = &[
    "#NOTEDATA",
    "#TITLE:",
    "#BPMS:",
    "#OFFSET:",
    "base64",
    "data:audio",
    "C:\\",
    "/Users/",
    ".ssc",
    ".mp3",
    ".ogg",
    ".flac",
    ".wav",
    ".mp4",
    ".mpg",
    ".png",
    ".jpg",
    ".jpeg",
];

pub fn load_single_guardrail_calibration(
    path: &Path,
) -> Result<SingleGuardrailCalibration, String> {
    if !path.exists() {
        return Err(format!("Calibration file does not exist at: {:?}", path));
    }
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read calibration file: {}", e))?;

    // Check for forbidden strings in raw content first
    for s in FORBIDDEN_STRINGS {
        if content.contains(s) {
            return Err(format!(
                "Privacy Violation: Forbidden pattern '{}' detected in calibration JSON file.",
                s
            ));
        }
    }

    let calib: SingleGuardrailCalibration = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse calibration JSON: {}", e))?;

    validate_single_guardrail_calibration(&calib)?;

    Ok(calib)
}

pub fn validate_single_guardrail_calibration(
    calibration: &SingleGuardrailCalibration,
) -> Result<(), String> {
    if calibration.schema_version != "single-guardrail-calibration.v0" {
        return Err(format!(
            "Invalid schema_version: expected 'single-guardrail-calibration.v0', got '{}'",
            calibration.schema_version
        ));
    }

    if calibration.publicability_status != "private_derived" {
        return Err(format!(
            "Invalid publicability_status: expected 'private_derived', got '{}'",
            calibration.publicability_status
        ));
    }

    if calibration.play_mode != "Single" {
        return Err(format!(
            "Invalid play_mode: expected 'Single', got '{}'",
            calibration.play_mode
        ));
    }

    // Validate level thresholds: S1 to S26
    for (level_key, threshold) in &calibration.level_thresholds {
        if !level_key.starts_with('S') {
            return Err(format!("Invalid level key format: '{}'", level_key));
        }
        let num_part = &level_key[1..];
        let level_num: u32 = num_part
            .parse()
            .map_err(|_| format!("Invalid numeric part in level key: '{}'", level_key))?;
        if level_num < 1 || level_num > 26 {
            return Err(format!("Level key '{}' is out of range 1..=26", level_key));
        }

        // Validate finite numbers
        let check_finite = |map: &BTreeMap<String, f64>, name: &str| -> Result<(), String> {
            for (k, &val) in map {
                if !val.is_finite() {
                    return Err(format!(
                        "Non-finite value found in {} threshold for {}: {}={}",
                        name, level_key, k, val
                    ));
                }
            }
            Ok(())
        };

        check_finite(&threshold.density, "density")?;
        check_finite(&threshold.jump_rate, "jump_rate")?;
        check_finite(&threshold.twist_rate, "twist_rate")?;
        check_finite(&threshold.bracket_candidate_rate, "bracket_candidate_rate")?;
    }

    Ok(())
}

pub fn classify_section_families(
    active_row_count: usize,
    row_count: usize,
    jump_count: usize,
    hold_start_count: usize,
    center_note_count: usize,
    twist_candidate_score: f64,
    bracket_candidate_count: usize,
    initial_bpm: f64,
    measure_count: f64,
) -> Vec<String> {
    let mut families = Vec::new();
    let active = active_row_count as f64;
    let stream_score = if row_count > 0 {
        active / row_count as f64
    } else {
        0.0
    };

    if stream_score >= 0.50 {
        families.push("stream".to_string());
    }
    if active > 0.0 && (jump_count as f64 / active) >= 0.20 {
        families.push("jump_accent".to_string());
    }
    if twist_candidate_score >= 0.20 {
        families.push("twist_technical".to_string());
    }
    if bracket_candidate_count >= 3 {
        families.push("bracket_technical".to_string());
    }
    if active > 0.0 && (hold_start_count as f64 / active) >= 0.30 {
        families.push("hold_control".to_string());
    }
    if active > 0.0 && (center_note_count as f64 / active) >= 0.40 {
        families.push("center_control".to_string());
    }

    // Stamina calculation (matching factory)
    let s_density = (active / measure_count / 16.0).min(1.0);
    let s_stream = (active_row_count as f64 / 64.0).min(1.0);
    let s_rest = 1.0; // Assume no rest measures for a single preview section (typically intense)
    let stamina_score = (s_density * 0.4 + s_stream * 0.4 + s_rest * 0.2).clamp(0.0, 1.0);
    let stamina_modifier = stamina_score * 3.0;

    let density = if measure_count > 0.0 {
        (active_row_count + hold_start_count) as f64 / measure_count
    } else {
        0.0
    };
    let base_diff = density * 1.0 + (initial_bpm - 120.0) * 0.05;
    let tech_modifier = twist_candidate_score * 5.0
        + (if measure_count > 0.0 {
            jump_count as f64 / measure_count
        } else {
            0.0
        }) * 2.0
        + (if measure_count > 0.0 {
            bracket_candidate_count as f64 / measure_count
        } else {
            0.0
        }) * 1.5;
    let local_difficulty_estimate = (base_diff + tech_modifier + stamina_modifier).clamp(1.0, 28.0);

    if local_difficulty_estimate >= 16.0 && stream_score >= 0.45 {
        families.push("stamina".to_string());
    }

    if families.is_empty() && active > 0.0 {
        families.push("balanced".to_string());
    }
    if active == 0.0 {
        families.push("unknown".to_string());
    }

    families
}

pub fn evaluate_section_against_calibration(
    payload: &GeminiChartSectionPayload,
    calibration: &SingleGuardrailCalibration,
    target_level: u32,
    initial_bpm: f64,
) -> CalibrationValidationReport {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let level_key = format!("S{}", target_level);
    let level_confidence = if target_level >= 25 {
        "low".to_string()
    } else {
        "high".to_string()
    };

    let threshold_opt = calibration.level_thresholds.get(&level_key);
    if threshold_opt.is_none() {
        warnings.push(CalibrationWarning {
            issue_type: "CalibrationMissing".to_string(),
            message: format!(
                "La calibración para el nivel S{} no está disponible.",
                target_level
            ),
        });
        return CalibrationValidationReport {
            calibration_available: true,
            schema_version: Some(calibration.schema_version.clone()),
            target_level: Some(target_level),
            level_confidence: Some(level_confidence),
            warnings,
            errors,
            matched_thresholds: None,
            pattern_family_signals: None,
            summary: "Calibración no disponible para el nivel especificado.".to_string(),
        };
    }

    let threshold = threshold_opt.unwrap();

    // 1. Compute section features from payload
    let mut tap_count = 0;
    let mut hold_start_count = 0;
    let mut active_row_count = 0;
    let mut row_count = 0;
    let mut jump_count = 0;
    let mut center_note_count = 0;
    let mut bracket_candidate_count = 0;
    let mut single_notes = Vec::new();

    let measure_count = payload.measures.len() as f64;

    for measure in &payload.measures {
        for row in &measure.rows {
            row_count += 1;
            let mut active_in_row = 0;
            let mut active_indices = Vec::new();

            for (col_idx, c) in row.chars().enumerate() {
                if c == '1' {
                    tap_count += 1;
                    active_in_row += 1;
                    active_indices.push(col_idx);
                    if col_idx == 2 {
                        center_note_count += 1;
                    }
                } else if c == '2' {
                    hold_start_count += 1;
                    active_in_row += 1;
                    active_indices.push(col_idx);
                    if col_idx == 2 {
                        center_note_count += 1;
                    }
                }
            }

            if active_in_row > 0 {
                active_row_count += 1;
                if active_in_row == 2 {
                    jump_count += 1;
                }

                if active_in_row == 1 {
                    single_notes.push(active_indices[0]);
                }

                // Brackets candidate check
                let center_active = active_indices.contains(&2);
                let diagonal_active = active_indices
                    .iter()
                    .any(|&i| i == 0 || i == 1 || i == 3 || i == 4);
                let is_bracket = (center_active && diagonal_active && active_indices.len() == 2)
                    || active_indices.len() >= 3;
                if is_bracket {
                    bracket_candidate_count += 1;
                }
            }
        }
    }

    // Twist score
    let mut twist_candidates = 0;
    let mut total_triplets = 0;
    for idx in 2..single_notes.len() {
        let a = single_notes[idx - 2];
        let b = single_notes[idx - 1];
        let c = single_notes[idx];
        total_triplets += 1;
        let is_twist = ((a == 0 || a == 1) && b == 2 && (c == 3 || c == 4))
            || ((a == 3 || a == 4) && b == 2 && (c == 0 || c == 1));
        if is_twist {
            twist_candidates += 1;
        }
    }
    let twist_rate = if total_triplets > 0 {
        twist_candidates as f64 / total_triplets as f64
    } else {
        0.0
    };

    let density = if measure_count > 0.0 {
        (tap_count + hold_start_count) as f64 / measure_count
    } else {
        0.0
    };

    let jump_rate = if active_row_count > 0 {
        jump_count as f64 / active_row_count as f64
    } else {
        0.0
    };

    // Scale bracket candidates to a 100-measure song
    let scaled_preview_bracket_count = if measure_count > 0.0 {
        (bracket_candidate_count as f64 / measure_count) * 100.0
    } else {
        0.0
    };

    // 2. Evaluate against thresholds
    let evaluate_metric = |val: f64,
                           limits: &BTreeMap<String, f64>,
                           name: &str,
                           label: &str,
                           warnings: &mut Vec<CalibrationWarning>,
                           errors: &mut Vec<CalibrationError>| {
        let p90 = limits.get("warning_p90").copied().unwrap_or(0.0);
        let p95 = limits.get("hard_limit_p95").copied().unwrap_or(0.0);

        if val > p95 {
            if target_level >= 25 {
                // Low confidence level, degrade error to warning
                warnings.push(CalibrationWarning {
                    issue_type: format!("{}LimitExceeded", name),
                    message: format!(
                        "La {} ({:.2}) excede el límite estricto p95 ({:.2}) para S{}. Se reporta como advertencia por confianza baja.",
                        label, val, p95, target_level
                    ),
                });
            } else {
                errors.push(CalibrationError {
                    issue_type: format!("{}LimitExceeded", name),
                    message: format!(
                        "La {} ({:.2}) excede el límite estricto p95 ({:.2}) para S{}.",
                        label, val, p95, target_level
                    ),
                });
            }
        } else if val > p90 {
            warnings.push(CalibrationWarning {
                issue_type: format!("{}WarningExceeded", name),
                message: format!(
                    "La {} ({:.2}) excede el umbral de advertencia p90 ({:.2}) para S{}.",
                    label, val, p90, target_level
                ),
            });
        }
    };

    evaluate_metric(
        density,
        &threshold.density,
        "Density",
        "densidad de notas por compás",
        &mut warnings,
        &mut errors,
    );
    evaluate_metric(
        jump_rate,
        &threshold.jump_rate,
        "JumpRate",
        "tasa de saltos (jumps)",
        &mut warnings,
        &mut errors,
    );
    evaluate_metric(
        twist_rate,
        &threshold.twist_rate,
        "TwistRate",
        "tasa de giros (twists)",
        &mut warnings,
        &mut errors,
    );

    // Evaluate bracket candidate count (scaled to 100 measures)
    let bracket_p90 = threshold
        .bracket_candidate_rate
        .get("warning_p90")
        .copied()
        .unwrap_or(0.0);
    let bracket_p95 = threshold
        .bracket_candidate_rate
        .get("hard_limit_p95")
        .copied()
        .unwrap_or(0.0);

    if scaled_preview_bracket_count > bracket_p95 {
        if target_level >= 25 {
            warnings.push(CalibrationWarning {
                issue_type: "BracketLimitExceeded".to_string(),
                message: format!(
                    "La cantidad de brackets (equiv. {:.1} en 100 compases) excede el límite estricto p95 ({:.1}) para S{}. Se reporta como advertencia por confianza baja.",
                    scaled_preview_bracket_count, bracket_p95, target_level
                ),
            });
        } else {
            errors.push(CalibrationError {
                issue_type: "BracketLimitExceeded".to_string(),
                message: format!(
                    "La cantidad de brackets (equiv. {:.1} en 100 compases) excede el límite estricto p95 ({:.1}) para S{}.",
                    scaled_preview_bracket_count, bracket_p95, target_level
                ),
            });
        }
    } else if scaled_preview_bracket_count > bracket_p90 {
        warnings.push(CalibrationWarning {
            issue_type: "BracketWarningExceeded".to_string(),
            message: format!(
                "La cantidad de brackets (equiv. {:.1} en 100 compases) excede el umbral de advertencia p90 ({:.1}) para S{}.",
                scaled_preview_bracket_count, bracket_p90, target_level
            ),
        });
    }

    // 3. Classify pattern families and extract signals
    let section_families = classify_section_families(
        active_row_count,
        row_count,
        jump_count,
        hold_start_count,
        center_note_count,
        twist_rate,
        bracket_candidate_count,
        initial_bpm,
        measure_count,
    );

    let mut matched_family_signals = BTreeMap::new();
    for family in &section_families {
        if let Some(signal) = calibration.pattern_family_thresholds.get(family) {
            matched_family_signals.insert(family.clone(), signal.clone());

            // Check if level is outside typical range
            if target_level < signal.typical_level_range.min
                || target_level > signal.typical_level_range.max
            {
                warnings.push(CalibrationWarning {
                    issue_type: "PatternFamilyLevelMismatch".to_string(),
                    message: format!(
                        "La familia de patrones '{}' se detectó en el preview, pero típicamente se usa en niveles del {} al {} (Nivel solicitado: S{}).",
                        family, signal.typical_level_range.min, signal.typical_level_range.max, target_level
                    ),
                });
            }
        }
    }

    // Summary description
    let summary = if errors.is_empty() && warnings.is_empty() {
        format!(
            "Sección validada exitosamente contra el corpus oficial para S{}. Densidad = {:.2}, Brackets (equiv. 100 compases) = {:.1}, Giros = {:.2}.",
            target_level, density, scaled_preview_bracket_count, twist_rate
        )
    } else {
        format!(
            "Se encontraron {} advertencias y {} errores estadísticos de calibración para S{}.",
            warnings.len(),
            errors.len(),
            target_level
        )
    };

    CalibrationValidationReport {
        calibration_available: true,
        schema_version: Some(calibration.schema_version.clone()),
        target_level: Some(target_level),
        level_confidence: Some(level_confidence),
        warnings,
        errors,
        matched_thresholds: Some(
            serde_json::to_value(threshold).unwrap_or(serde_json::Value::Null),
        ),
        pattern_family_signals: Some(
            serde_json::to_value(matched_family_signals).unwrap_or(serde_json::Value::Null),
        ),
        summary,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationCandidateScore {
    pub has_all_single_levels: bool,
    pub known_family_count: usize,
    pub level_count: usize,
    pub unknown_family_count: usize,
}

impl Ord for CalibrationCandidateScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.has_all_single_levels
            .cmp(&other.has_all_single_levels)
            .then_with(|| self.known_family_count.cmp(&other.known_family_count))
            .then_with(|| self.level_count.cmp(&other.level_count))
            .then_with(|| other.unknown_family_count.cmp(&self.unknown_family_count))
    }
}

impl PartialOrd for CalibrationCandidateScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn score_calibration_candidate(
    calibration: &SingleGuardrailCalibration,
) -> CalibrationCandidateScore {
    let mut has_all = true;
    for l in 1..=26 {
        let key = format!("S{}", l);
        if !calibration.level_thresholds.contains_key(&key) {
            has_all = false;
            break;
        }
    }

    let known_families = &[
        "stream",
        "jump_accent",
        "twist_technical",
        "bracket_technical",
        "hold_control",
        "center_control",
        "stamina",
        "balanced",
        "unknown",
    ];

    let mut known_count = 0;
    let mut unknown_count = 0;
    for family in calibration.pattern_family_thresholds.keys() {
        if known_families.contains(&family.as_str()) {
            known_count += 1;
        } else {
            unknown_count += 1;
        }
    }

    let level_count = calibration.level_thresholds.len();

    CalibrationCandidateScore {
        has_all_single_levels: has_all,
        known_family_count: known_count,
        level_count,
        unknown_family_count: unknown_count,
    }
}

pub fn get_standard_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = manifest_dir.parent() {
        roots.push(parent.to_path_buf());
    }

    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.clone());
        if cwd.file_name().map_or(false, |name| name == "src-tauri") {
            if let Some(parent) = cwd.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }

    roots
}

pub fn build_calibration_candidate_paths(
    explicit_path: Option<&Path>,
    roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path) = explicit_path {
        paths.push(path.to_path_buf());
        return paths;
    }

    let candidate_relatives = &[
        ".ai-step-gen-private-datasets/official-corpus-profiles-v0-runtime-check/single-guardrail-calibration.v0.json",
        ".ai-step-gen-private-datasets/official-corpus-profiles-v0/single-guardrail-calibration.v0.json",
        ".ai-step-gen-private-datasets/official-corpus-profiles-v0-fixes-audit/single-guardrail-calibration.v0.json",
    ];

    for root in roots {
        for relative in candidate_relatives {
            paths.push(root.join(relative));
        }
    }

    for relative in candidate_relatives {
        paths.push(PathBuf::from(relative));
    }

    paths
}

pub fn resolve_calibration_file_from_roots(
    explicit_path: Option<&Path>,
    roots: &[PathBuf],
) -> Option<SingleGuardrailCalibration> {
    let candidate_paths = build_calibration_candidate_paths(explicit_path, roots);
    let mut best_candidate: Option<(CalibrationCandidateScore, SingleGuardrailCalibration)> = None;

    for path in candidate_paths {
        if path.exists() {
            if let Ok(calib) = load_single_guardrail_calibration(&path) {
                let score = score_calibration_candidate(&calib);
                match &best_candidate {
                    None => {
                        best_candidate = Some((score, calib));
                    }
                    Some((best_score, _)) => {
                        if score > *best_score {
                            best_candidate = Some((score, calib));
                        }
                        // If score == best_score, we preserve the existing best_candidate
                        // because candidate_paths are processed in order of priority.
                    }
                }
            }
        }
    }

    best_candidate.map(|(_, calib)| calib)
}

pub fn resolve_calibration_file(
    explicit_path: Option<&Path>,
) -> Option<SingleGuardrailCalibration> {
    let roots = get_standard_roots();
    resolve_calibration_file_from_roots(explicit_path, &roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::{GeminiMeasure, PlayMode};
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    fn get_temp_file(name: &str, content: &str) -> (PathBuf, PathBuf) {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "calib_test_{}_{}",
            name,
            chrono::Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&p).unwrap();
        let file_path = p.join(name);
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        (p, file_path)
    }

    fn make_valid_calibration_json() -> String {
        r#"{
            "schema_version": "single-guardrail-calibration.v0",
            "publicability_status": "private_derived",
            "play_mode": "Single",
            "source_dataset_summary": {
                "total_charts": 10,
                "total_songs": 5,
                "total_windows": 100
            },
            "level_thresholds": {
                "S14": {
                    "density": {
                        "warning_p90": 22.0,
                        "hard_limit_p95": 25.0,
                        "typical_p50": 18.0
                    },
                    "jump_rate": {
                        "warning_p90": 0.15,
                        "hard_limit_p95": 0.20
                    },
                    "twist_rate": {
                        "warning_p90": 0.10,
                        "hard_limit_p95": 0.15
                    },
                    "bracket_candidate_rate": {
                        "warning_p90": 30.0,
                        "hard_limit_p95": 40.0
                    }
                }
            },
            "pattern_family_thresholds": {
                "stream": {
                    "pattern_family": "stream",
                    "classification_rule": "stream_score >= 0.50",
                    "classifier_thresholds": { "stream_score": 0.5 },
                    "sample_count": 100,
                    "sample_confidence": "high",
                    "typical_level_range": { "min": 10, "median": 15, "max": 20 },
                    "metric_stats": {
                        "min": 0.5, "p10": 0.5, "p25": 0.55, "median": 0.65, "p75": 0.75, "p90": 0.85, "p95": 0.90, "max": 1.0, "mean": 0.65, "std_dev": 0.15
                    },
                    "recommended_when": ["Stream sections"],
                    "avoid_when": ["Rest sections"],
                    "guardrail_notes": ["Maintain alternating feet"]
                }
            },
            "confidence_policy": {
                "high": "n >= 50",
                "medium": "10 <= n < 50",
                "low": "n < 10"
            },
            "recommended_runtime_usage": [
                "Use p50 as typical, p90 as warning, p95 as error."
            ]
        }"#.to_string()
    }

    #[test]
    fn test_valid_calibration_parse() {
        let content = make_valid_calibration_json();
        let (dir, file_path) = get_temp_file("valid.json", &content);
        let calib = load_single_guardrail_calibration(&file_path);
        assert!(
            calib.is_ok(),
            "Failed parsing valid calibration: {:?}",
            calib.err()
        );
        let calib = calib.unwrap();
        assert_eq!(calib.schema_version, "single-guardrail-calibration.v0");
        assert_eq!(calib.play_mode, "Single");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rejects_incorrect_schema_version() {
        let mut content = make_valid_calibration_json();
        content = content.replace("single-guardrail-calibration.v0", "incorrect-schema");
        let (dir, file_path) = get_temp_file("invalid_schema.json", &content);
        let calib = load_single_guardrail_calibration(&file_path);
        assert!(calib.is_err());
        assert!(calib.unwrap_err().contains("schema_version"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rejects_incorrect_publicability_status() {
        let mut content = make_valid_calibration_json();
        content = content.replace("private_derived", "public_exposed");
        let (dir, file_path) = get_temp_file("invalid_status.json", &content);
        let calib = load_single_guardrail_calibration(&file_path);
        assert!(calib.is_err());
        assert!(calib.unwrap_err().contains("publicability_status"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rejects_non_single_play_mode() {
        let mut content = make_valid_calibration_json();
        content = content.replace("\"play_mode\": \"Single\"", "\"play_mode\": \"Double\"");
        let (dir, file_path) = get_temp_file("invalid_mode.json", &content);
        let calib = load_single_guardrail_calibration(&file_path);
        assert!(calib.is_err());
        assert!(calib.unwrap_err().contains("play_mode"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rejects_forbidden_strings() {
        let mut content = make_valid_calibration_json();
        content = content.replace("Stream sections", "Stream sections in C:\\Users\\user");
        let (dir, file_path) = get_temp_file("forbidden.json", &content);
        let calib = load_single_guardrail_calibration(&file_path);
        assert!(calib.is_err());
        assert!(calib.unwrap_err().contains("Privacy Violation"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_evaluation_without_calibration_graceful() {
        let payload = GeminiChartSectionPayload {
            section_id: "chorus_1".to_string(),
            difficulty_level: 14,
            play_mode: PlayMode::Single,
            biomechanical_state: crate::biomechanics::GeminiBiomechanicalState {
                current_twist_debt: 0.0,
                current_stamina_debt: 0.0,
                last_left_foot_lane: None,
                last_right_foot_lane: None,
            },
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 4,
                rows: vec![
                    "10000".to_string(),
                    "00000".to_string(),
                    "00000".to_string(),
                    "00000".to_string(),
                ],
            }],
        };

        let report = evaluate_section_against_calibration(
            &payload,
            &SingleGuardrailCalibration {
                schema_version: "single-guardrail-calibration.v0".to_string(),
                publicability_status: "private_derived".to_string(),
                play_mode: "Single".to_string(),
                source_dataset_summary: serde_json::Value::Null,
                level_thresholds: BTreeMap::new(), // Empty thresholds, representing missing S14 calibration
                pattern_family_thresholds: BTreeMap::new(),
                confidence_policy: BTreeMap::new(),
                recommended_runtime_usage: Vec::new(),
            },
            14,
            120.0,
        );

        assert!(report.calibration_available);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.issue_type == "CalibrationMissing"));
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_evaluation_with_threshold_exceeded() {
        let content = make_valid_calibration_json();
        let calib: SingleGuardrailCalibration = serde_json::from_str(&content).unwrap();

        // Target level S14 has warning_p90 = 22.0, hard_limit_p95 = 25.0
        // We generate 1 measure with 32 notes (density = 64.0 > 25.0 limit)
        let payload = GeminiChartSectionPayload {
            section_id: "chorus_1".to_string(),
            difficulty_level: 14,
            play_mode: PlayMode::Single,
            biomechanical_state: crate::biomechanics::GeminiBiomechanicalState {
                current_twist_debt: 0.0,
                current_stamina_debt: 0.0,
                last_left_foot_lane: None,
                last_right_foot_lane: None,
            },
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 32,
                rows: vec!["10001".to_string(); 32], // 32 rows, each with active taps -> 64 notes (density = 64.0 > 25.0 limit)
            }],
        };

        let report = evaluate_section_against_calibration(&payload, &calib, 14, 120.0);
        assert!(report.calibration_available);
        assert!(report
            .errors
            .iter()
            .any(|e| e.issue_type == "DensityLimitExceeded"));
    }

    #[test]
    fn test_evaluation_low_confidence_level_conservativeness() {
        let mut content = make_valid_calibration_json();
        // Insert thresholds for S25 with density p95 limit = 20.0
        content = content.replace(
            "\"level_thresholds\": {",
            "\"level_thresholds\": {\n\"S25\": {\n\"density\": {\"warning_p90\": 15.0, \"hard_limit_p95\": 20.0, \"typical_p50\": 10.0},\n\"jump_rate\": {\"warning_p90\": 0.15, \"hard_limit_p95\": 0.20},\n\"twist_rate\": {\"warning_p90\": 0.10, \"hard_limit_p95\": 0.15},\n\"bracket_candidate_rate\": {\"warning_p90\": 30.0, \"hard_limit_p95\": 40.0}\n},"
        );

        let calib: SingleGuardrailCalibration = serde_json::from_str(&content).unwrap();

        // S25 density p95 = 20.0. We request target level 25 and generate density = 32.0 (exceeds limit)
        let payload = GeminiChartSectionPayload {
            section_id: "chorus_1".to_string(),
            difficulty_level: 25,
            play_mode: PlayMode::Single,
            biomechanical_state: crate::biomechanics::GeminiBiomechanicalState {
                current_twist_debt: 0.0,
                current_stamina_debt: 0.0,
                last_left_foot_lane: None,
                last_right_foot_lane: None,
            },
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 16,
                rows: vec!["10001".to_string(); 16], // 32 notes in 1 measure -> density = 32.0
            }],
        };

        let report = evaluate_section_against_calibration(&payload, &calib, 25, 120.0);
        assert!(report.calibration_available);
        // Errors must be degraded to warnings for low confidence levels (like S25)
        assert!(
            report.errors.is_empty(),
            "Expected no hard errors for S25, got {:?}",
            report.errors
        );
        assert!(report
            .warnings
            .iter()
            .any(|w| w.issue_type == "DensityLimitExceeded"));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.message.contains("confianza baja")));
    }

    fn make_synthetic_calibration_json(
        levels: &[&str],
        schema_version: &str,
        publicability_status: &str,
        recommended_usage: &[&str],
    ) -> String {
        let mut lvl_str = String::new();
        for (idx, lvl) in levels.iter().enumerate() {
            if idx > 0 {
                lvl_str.push_str(", ");
            }
            lvl_str.push_str(&format!(
                "\"{}\": {{ \"density\": {{}}, \"jump_rate\": {{}}, \"twist_rate\": {{}}, \"bracket_candidate_rate\": {{}} }}",
                lvl
            ));
        }

        let mut rec_str = String::new();
        for (idx, rec) in recommended_usage.iter().enumerate() {
            if idx > 0 {
                rec_str.push_str(", ");
            }
            rec_str.push_str(&format!("\"{}\"", rec));
        }

        format!(
            r#"{{
                "schema_version": "{}",
                "publicability_status": "{}",
                "play_mode": "Single",
                "source_dataset_summary": {{}},
                "level_thresholds": {{ {} }},
                "pattern_family_thresholds": {{}},
                "confidence_policy": {{}},
                "recommended_runtime_usage": [ {} ]
            }}"#,
            schema_version, publicability_status, lvl_str, rec_str
        )
    }

    fn write_candidate(root: &Path, relative_idx: usize, content: &str) -> PathBuf {
        let relative = &[
            ".ai-step-gen-private-datasets/official-corpus-profiles-v0-runtime-check/single-guardrail-calibration.v0.json",
            ".ai-step-gen-private-datasets/official-corpus-profiles-v0/single-guardrail-calibration.v0.json",
            ".ai-step-gen-private-datasets/official-corpus-profiles-v0-fixes-audit/single-guardrail-calibration.v0.json",
        ][relative_idx];

        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = File::create(&path).unwrap();
        write!(file, "{}", content).unwrap();
        path
    }

    #[test]
    fn test_resolver_priority_equal_score_tiebreaker() {
        let mut temp_root = std::env::temp_dir();
        temp_root.push(format!(
            "calib_tie_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        // 1. Two calibrations valid with score identical:
        // Candidate 0 (runtime-check) and Candidate 1 (v0) both have only level S14 (identical score).
        // Candidate 0 should win because it is first in priority list.
        let json_0 = make_synthetic_calibration_json(
            &["S14"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_0"],
        );
        let json_1 = make_synthetic_calibration_json(
            &["S14"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_1"],
        );

        write_candidate(&temp_root, 0, &json_0);
        write_candidate(&temp_root, 1, &json_1);

        let resolved = resolve_calibration_file_from_roots(None, &[temp_root.clone()]);
        assert!(resolved.is_some());
        let res = resolved.unwrap();
        assert_eq!(
            res.recommended_runtime_usage,
            vec!["candidate_0".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_resolver_invalid_earlier_valid_later() {
        let mut temp_root = std::env::temp_dir();
        temp_root.push(format!(
            "calib_inv_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        // 2. Candidate 0 is invalid JSON (should be ignored). Candidate 1 is valid.
        // Candidate 1 should be selected.
        write_candidate(&temp_root, 0, "{ invalid json }");
        let json_1 = make_synthetic_calibration_json(
            &["S14"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_1"],
        );
        write_candidate(&temp_root, 1, &json_1);

        let resolved = resolve_calibration_file_from_roots(None, &[temp_root.clone()]);
        assert!(resolved.is_some());
        let res = resolved.unwrap();
        assert_eq!(
            res.recommended_runtime_usage,
            vec!["candidate_1".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_resolver_later_better_score_wins() {
        let mut temp_root = std::env::temp_dir();
        temp_root.push(format!(
            "calib_better_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        // 3. Candidate 0 has level S14 (score: level_count=1).
        // Candidate 1 has S14, S15 (score: level_count=2).
        // Candidate 1 should win even though Candidate 0 is earlier in priority.
        let json_0 = make_synthetic_calibration_json(
            &["S14"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_0"],
        );
        let json_1 = make_synthetic_calibration_json(
            &["S14", "S15"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_1"],
        );

        write_candidate(&temp_root, 0, &json_0);
        write_candidate(&temp_root, 1, &json_1);

        let resolved = resolve_calibration_file_from_roots(None, &[temp_root.clone()]);
        assert!(resolved.is_some());
        let res = resolved.unwrap();
        assert_eq!(
            res.recommended_runtime_usage,
            vec!["candidate_1".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_resolver_priority_multiple_best_score_ties() {
        let mut temp_root = std::env::temp_dir();
        temp_root.push(format!(
            "calib_mult_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&temp_root).unwrap();

        // 4. Candidate 0 has level S14 (score: level_count=1).
        // Candidate 1 has S14, S15 (score: level_count=2).
        // Candidate 2 has S14, S15 (score: level_count=2).
        // Best score is level_count=2, tied between Candidate 1 and Candidate 2.
        // Candidate 1 should win because it is earlier in priority than Candidate 2.
        let json_0 = make_synthetic_calibration_json(
            &["S14"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_0"],
        );
        let json_1 = make_synthetic_calibration_json(
            &["S14", "S15"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_1"],
        );
        let json_2 = make_synthetic_calibration_json(
            &["S14", "S15"],
            "single-guardrail-calibration.v0",
            "private_derived",
            &["candidate_2"],
        );

        write_candidate(&temp_root, 0, &json_0);
        write_candidate(&temp_root, 1, &json_1);
        write_candidate(&temp_root, 2, &json_2);

        let resolved = resolve_calibration_file_from_roots(None, &[temp_root.clone()]);
        assert!(resolved.is_some());
        let res = resolved.unwrap();
        assert_eq!(
            res.recommended_runtime_usage,
            vec!["candidate_1".to_string()]
        );

        let _ = fs::remove_dir_all(&temp_root);
    }
}
