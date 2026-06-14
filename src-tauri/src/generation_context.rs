use crate::biomechanics::PlayMode;
use crate::guardrail_calibration::SingleGuardrailCalibration;
use crate::music_analysis::SongAnalysisReport;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PatternFamily {
    Stream,
    JumpAccent,
    TwistTechnical,
    BracketTechnical,
    HoldControl,
    CenterControl,
    Stamina,
    Balanced,
    Unknown,
}

impl PatternFamily {
    pub fn to_string_key(&self) -> String {
        match self {
            PatternFamily::Stream => "stream".to_string(),
            PatternFamily::JumpAccent => "jump_accent".to_string(),
            PatternFamily::TwistTechnical => "twist_technical".to_string(),
            PatternFamily::BracketTechnical => "bracket_technical".to_string(),
            PatternFamily::HoldControl => "hold_control".to_string(),
            PatternFamily::CenterControl => "center_control".to_string(),
            PatternFamily::Stamina => "stamina".to_string(),
            PatternFamily::Balanced => "balanced".to_string(),
            PatternFamily::Unknown => "unknown".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace(' ', "_").as_str() {
            "stream" => PatternFamily::Stream,
            "jump_accent" | "jump_accents" | "jump" | "jumps" => PatternFamily::JumpAccent,
            "twist_technical" | "twist" | "twists" | "twist_tech" => PatternFamily::TwistTechnical,
            "bracket_technical" | "bracket" | "brackets" | "bracket_tech" => {
                PatternFamily::BracketTechnical
            }
            "hold_control" | "hold" | "holds" | "hold_ctrl" => PatternFamily::HoldControl,
            "center_control" | "center" | "center_ctrl" => PatternFamily::CenterControl,
            "stamina" => PatternFamily::Stamina,
            "balanced" => PatternFamily::Balanced,
            _ => PatternFamily::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternFamilyTargetMode {
    Auto,
    Specific(PatternFamily),
}

impl PatternFamilyTargetMode {
    pub fn from_str(s: &str) -> Self {
        if s.to_lowercase() == "auto" {
            PatternFamilyTargetMode::Auto
        } else {
            PatternFamilyTargetMode::Specific(PatternFamily::from_str(s))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternFamilyCandidate {
    pub family: PatternFamily,
    pub score: f64,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternFamilyTargetingReport {
    pub requested_mode: String,
    pub primary_family: String,
    pub secondary_families: Vec<String>,
    pub avoid_families: Vec<String>,
    pub candidates: Vec<PatternFamilyCandidate>,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionContextSummary {
    pub section_id: String,
    pub start_measure: u32,
    pub end_measure: u32,
    pub num_measures: u32,
    pub song_type: String,
    pub music_role: Option<String>,
    pub piu_role: Option<String>,
    pub energy_profile: Option<String>,
    pub density_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingContextSummary {
    pub has_reconciliation_agreement: Option<bool>,
    pub timing_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityGuidance {
    pub min_recommended: f64,
    pub max_recommended: f64,
    pub warning_threshold_p90: f64,
    pub hard_limit_p95: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailThresholdSummary {
    pub density_p90: f64,
    pub density_p95: f64,
    pub jump_rate_p90: f64,
    pub jump_rate_p95: f64,
    pub twist_rate_p90: f64,
    pub twist_rate_p95: f64,
    pub bracket_rate_p90: f64,
    pub bracket_rate_p95: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationContextSummary {
    pub available: bool,
    pub target_level: String,
    pub level_confidence: String,
    pub warning_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibratedPromptContext {
    pub schema_version: String,
    pub enabled: bool,
    pub calibration_available: bool,
    pub target_level: u8,
    pub play_mode: String,
    pub section: SectionContextSummary,
    pub timing: TimingContextSummary,
    pub density_guidance: DensityGuidance,
    pub pattern_family_targeting: PatternFamilyTargetingReport,
    pub guardrail_threshold_summary: GuardrailThresholdSummary,
    pub prompt_instructions: Vec<String>,
    pub warnings: Vec<String>,
    pub continuity_context: Option<crate::section_continuity::ContinuityContextSummary>,
}

pub fn classify_browser_bpm_reconciliation_status(recon: &str) -> Option<bool> {
    let trimmed = recon.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();

    // Check if it follows the Status: <val> format
    let status_token = if let Some(idx) = lower.find("status:") {
        let after = &lower[idx + 7..];
        let token = if let Some(comma_idx) = after.find(',') {
            &after[..comma_idx]
        } else {
            after
        };
        token.trim()
    } else {
        lower.as_str()
    };

    match status_token {
        "agrees" | "agree" | "agreed with sheet timing" => Some(true),
        "disagrees"
        | "disagree"
        | "mismatch"
        | "mismatched"
        | "mismatch or manual review suggested" => Some(false),
        "unsupported" | "no_browser_evidence" | "none" => None,
        _ => {
            // Fallback substring checks for free text
            if status_token.contains("disagree") || status_token.contains("mismatch") {
                Some(false)
            } else if status_token.contains("agree") {
                Some(true)
            } else {
                None
            }
        }
    }
}

pub fn resolve_pattern_family_targeting(
    target_level: u32,
    play_mode: PlayMode,
    requested_mode: PatternFamilyTargetMode,
    music_analysis: Option<&SongAnalysisReport>,
    section_id: &str,
    calibration: Option<&SingleGuardrailCalibration>,
    _browser_bpm_reconciliation: Option<&str>,
) -> PatternFamilyTargetingReport {
    #[allow(unused_assignments)]
    let mut primary_family = PatternFamily::Balanced;
    let mut confidence = "high".to_string();
    let mut evidence = Vec::new();
    let mut warnings = Vec::new();
    let requested_mode_str = match &requested_mode {
        PatternFamilyTargetMode::Auto => "auto".to_string(),
        PatternFamilyTargetMode::Specific(fam) => fam.to_string_key(),
    };

    // Initialize scores for all valid candidates (excluding Unknown)
    let candidate_families = &[
        PatternFamily::Stream,
        PatternFamily::JumpAccent,
        PatternFamily::TwistTechnical,
        PatternFamily::BracketTechnical,
        PatternFamily::HoldControl,
        PatternFamily::CenterControl,
        PatternFamily::Stamina,
        PatternFamily::Balanced,
    ];

    let mut scores = BTreeMap::new();
    let mut candidate_evidence = BTreeMap::new();
    let mut candidate_warnings = BTreeMap::new();

    for &fam in candidate_families {
        scores.insert(
            fam,
            if fam == PatternFamily::Balanced {
                1.0
            } else {
                0.0
            },
        );
        candidate_evidence.insert(fam, Vec::new());
        candidate_warnings.insert(fam, Vec::new());
    }

    // Process Level-Based Hard Limits (Conservadurismo)
    let is_avoided = |fam: PatternFamily| -> bool {
        match play_mode {
            PlayMode::Single => {
                if target_level <= 6 {
                    fam == PatternFamily::BracketTechnical
                        || fam == PatternFamily::TwistTechnical
                        || fam == PatternFamily::Stamina
                } else if target_level <= 10 {
                    fam == PatternFamily::BracketTechnical
                } else {
                    false
                }
            }
            PlayMode::Double => false, // Double is blocked in this phase anyway
        }
    };

    // Apply signals from Music Analysis
    let mut has_ma_signals = false;
    let mut ma_confidence = 1.0;
    if let Some(report) = music_analysis {
        if let Some(intent) = report
            .choreographic_intent
            .iter()
            .find(|i| i.section_id == section_id)
        {
            has_ma_signals = true;
            ma_confidence = intent.confidence;

            evidence.push(format!(
                "Análisis musical disponible con confianza {:.2} para la sección '{}'.",
                ma_confidence, section_id
            ));

            for rec_str in &intent.recommended_pattern_families {
                let rec_fam = PatternFamily::from_str(rec_str);
                if rec_fam != PatternFamily::Unknown {
                    if let Some(score) = scores.get_mut(&rec_fam) {
                        *score += 2.5;
                        candidate_evidence.get_mut(&rec_fam).unwrap().push(format!(
                            "Recomendado por Music Analysis (Intent Density: {})",
                            intent.density_target
                        ));
                    }
                }
            }

            for avoid_str in &intent.avoid_pattern_families {
                let avoid_fam = PatternFamily::from_str(avoid_str);
                if avoid_fam != PatternFamily::Unknown {
                    if let Some(score) = scores.get_mut(&avoid_fam) {
                        *score -= 10.0;
                        candidate_evidence
                            .get_mut(&avoid_fam)
                            .unwrap()
                            .push("Evitado explícitamente por Music Analysis".to_string());
                    }
                }
            }
        }

        // Map piu_role and music_role to family boosts
        if let Some(sec) = report.sections.iter().find(|s| s.section_id == section_id) {
            match sec.piu_role.to_lowercase().as_str() {
                "final_burst" => {
                    if target_level >= 15 {
                        *scores.get_mut(&PatternFamily::Stamina).unwrap() += 2.0;
                        candidate_evidence
                            .get_mut(&PatternFamily::Stamina)
                            .unwrap()
                            .push(
                                "Sección final burst a nivel alto (S15+), se sugiere resistencia"
                                    .to_string(),
                            );
                    } else {
                        *scores.get_mut(&PatternFamily::Stream).unwrap() += 1.5;
                    }
                }
                "climax_run" => {
                    *scores.get_mut(&PatternFamily::Stream).unwrap() += 1.5;
                    if target_level >= 15 {
                        *scores.get_mut(&PatternFamily::Stamina).unwrap() += 1.5;
                    }
                    if target_level >= 12 {
                        *scores.get_mut(&PatternFamily::TwistTechnical).unwrap() += 1.0;
                    }
                }
                "crossover_drill" | "twist_train" => {
                    if target_level >= 7 {
                        *scores.get_mut(&PatternFamily::TwistTechnical).unwrap() += 2.0;
                        candidate_evidence
                            .get_mut(&PatternFamily::TwistTechnical)
                            .unwrap()
                            .push(format!("PIU Role '{}' detectado", sec.piu_role));
                    }
                }
                "accent_drop" => {
                    *scores.get_mut(&PatternFamily::JumpAccent).unwrap() += 2.0;
                    candidate_evidence
                        .get_mut(&PatternFamily::JumpAccent)
                        .unwrap()
                        .push("Sección con acentos musicales marcados".to_string());
                }
                "hold_check" => {
                    *scores.get_mut(&PatternFamily::HoldControl).unwrap() += 2.0;
                }
                "rest_step" => {
                    if target_level >= 10 {
                        *scores.get_mut(&PatternFamily::HoldControl).unwrap() += 1.5;
                    }
                    *scores.get_mut(&PatternFamily::Balanced).unwrap() += 1.0;
                }
                _ => {}
            }
        }
    }

    // Apply signals from Calibration Profiles if available
    let mut has_calibration = false;
    if let Some(calib) = calibration {
        has_calibration = true;
        for &fam in candidate_families {
            let key = fam.to_string_key();
            if let Some(signal) = calib.pattern_family_thresholds.get(&key) {
                let min = signal.typical_level_range.min;
                let max = signal.typical_level_range.max;

                if target_level >= min && target_level <= max {
                    if let Some(score) = scores.get_mut(&fam) {
                        *score += 1.0;
                        candidate_evidence.get_mut(&fam).unwrap().push(format!(
                            "Nivel solicitado S{} está dentro del rango típico de calibración para {} (S{}-S{})",
                            target_level, key, min, max
                        ));
                    }
                } else if target_level < min {
                    let diff = min - target_level;
                    if let Some(score) = scores.get_mut(&fam) {
                        *score -= diff as f64 * 1.5;
                        let msg = format!(
                            "Nivel solicitado S{} es menor que el mínimo calibrado (S{}) para {}",
                            target_level, min, key
                        );
                        candidate_evidence.get_mut(&fam).unwrap().push(msg.clone());
                        candidate_warnings.get_mut(&fam).unwrap().push(msg);
                    }
                } else {
                    if let Some(score) = scores.get_mut(&fam) {
                        *score += 0.5;
                        candidate_evidence.get_mut(&fam).unwrap().push(format!(
                            "Nivel solicitado S{} es mayor que el máximo calibrado (S{}) para {}",
                            target_level, max, key
                        ));
                    }
                }
            }
        }
    }

    // Specific Overrides Logic
    match requested_mode {
        PatternFamilyTargetMode::Specific(requested_fam) => {
            let mut is_valid_specific = true;

            // Incompatibility check by level limits
            if is_avoided(requested_fam) {
                is_valid_specific = false;
                warnings.push(format!(
                    "La familia de patrones requested '{}' es incompatible con el nivel bajo S{} y ha sido evitada.",
                    requested_fam.to_string_key(), target_level
                ));
            }

            // Severe typical range mismatch checks (target_level < typical_min - 3)
            if is_valid_specific && has_calibration {
                let key = requested_fam.to_string_key();
                if let Some(signal) =
                    calibration.and_then(|c| c.pattern_family_thresholds.get(&key))
                {
                    if target_level + 3 < signal.typical_level_range.min {
                        is_valid_specific = false;
                        warnings.push(format!(
                            "La familia requested '{}' tiene un rango mínimo calibrado de S{}, lo cual es demasiado alto para S{}.",
                            key, signal.typical_level_range.min, target_level
                        ));
                    }
                }
            }

            if is_valid_specific && requested_fam != PatternFamily::Unknown {
                primary_family = requested_fam;
                evidence.push(format!(
                    "Selección manual del usuario para focus: '{}'.",
                    primary_family.to_string_key()
                ));
            } else {
                primary_family = PatternFamily::Balanced;
                evidence.push(format!(
                    "Fallback a 'balanced' debido a incompatibilidad con '{}'.",
                    requested_fam.to_string_key()
                ));
            }
        }
        PatternFamilyTargetMode::Auto => {
            // Apply level limits to scores
            for &fam in candidate_families {
                if is_avoided(fam) {
                    if let Some(score) = scores.get_mut(&fam) {
                        *score = -999.0;
                    }
                    candidate_warnings
                        .get_mut(&fam)
                        .unwrap()
                        .push(format!("Evitado para niveles bajos (S1-S6/S10)"));
                }
            }

            // Adjust scores for low confidence music analysis
            if has_ma_signals && ma_confidence < 0.5 {
                confidence = "low".to_string();
                warnings.push(format!(
                    "La confianza del análisis musical es baja ({:.2}). Se prioriza 'balanced'.",
                    ma_confidence
                ));
                *scores.get_mut(&PatternFamily::Balanced).unwrap() += 2.0;
                for &fam in candidate_families {
                    if fam != PatternFamily::Balanced {
                        if let Some(score) = scores.get_mut(&fam) {
                            *score -= 1.5;
                        }
                    }
                }
            }

            // Find primary family (highest score)
            let mut best_fam = PatternFamily::Balanced;
            let mut best_score = -99.0;
            for &fam in candidate_families {
                let score = *scores.get(&fam).unwrap();
                if score > best_score {
                    best_score = score;
                    best_fam = fam;
                }
            }

            if best_score > 0.0 {
                primary_family = best_fam;
                evidence.push(format!(
                    "Auto seleccionó '{}' con score {:.2}.",
                    primary_family.to_string_key(),
                    best_score
                ));
            } else {
                primary_family = PatternFamily::Balanced;
                evidence.push("Auto usa 'balanced' por falta de señales fuertes.".to_string());
            }
        }
    }

    // Group candidates, secondary, and avoided families
    let mut secondary_families = Vec::new();
    let mut avoid_families = Vec::new();
    let mut candidates = Vec::new();

    for &fam in candidate_families {
        let score = *scores.get(&fam).unwrap();
        let cand_ev = candidate_evidence.get(&fam).cloned().unwrap_or_default();
        let cand_wrn = candidate_warnings.get(&fam).cloned().unwrap_or_default();

        let cand_conf = if cand_wrn.is_empty() {
            "high".to_string()
        } else {
            "medium".to_string()
        };

        candidates.push(PatternFamilyCandidate {
            family: fam,
            score,
            confidence: cand_conf,
            evidence: cand_ev,
            warnings: cand_wrn,
        });

        if fam != primary_family && fam != PatternFamily::Unknown && fam != PatternFamily::Balanced
        {
            if score > 0.0 {
                secondary_families.push(fam.to_string_key());
            } else if score < 0.0 || is_avoided(fam) {
                avoid_families.push(fam.to_string_key());
            }
        }
    }

    // Set overall report confidence
    if !has_calibration && !has_ma_signals {
        confidence = "low".to_string();
    } else if confidence != "low" {
        if !has_calibration || target_level >= 25 {
            confidence = "medium".to_string();
        } else {
            confidence = "high".to_string();
        }
    }

    PatternFamilyTargetingReport {
        requested_mode: requested_mode_str,
        primary_family: primary_family.to_string_key(),
        secondary_families,
        avoid_families,
        candidates,
        confidence,
        evidence,
        warnings,
    }
}

pub fn self_audit_prompt_context(prompt: &str) -> Result<(), String> {
    let lower_prompt = prompt.to_lowercase();

    // 1. Forbidden StepMania tags (case-insensitive)
    let forbidden_tags = &[
        "#notedata",
        "#title",
        "#artist",
        "#bpms",
        "#offset",
        "#stops",
        "#delays",
        "#warps",
    ];
    for tag in forbidden_tags {
        if lower_prompt.contains(tag) {
            return Err(format!(
                "Privacy Violation: Forbidden StepMania tag '{}' detected in prompt context.",
                tag
            ));
        }
    }

    // 2. Forbidden metadata / keywords (case-insensitive)
    let forbidden_keywords = &[
        "base64",
        "data:audio",
        ".ai-step-gen-private-datasets",
        "official_songs",
        "canonical bpm",
        "browser bpm",
        "candidates:",
        "duración:",
        "segundos",
    ];
    for kw in forbidden_keywords {
        if lower_prompt.contains(kw) {
            return Err(format!(
                "Privacy Violation: Forbidden keyword '{}' detected in prompt context.",
                kw
            ));
        }
    }

    // 3. Forbidden file extensions (case-insensitive)
    let forbidden_extensions = &[
        ".ssc", ".mp3", ".ogg", ".flac", ".wav", ".mp4", ".mpg", ".png", ".jpg", ".jpeg",
    ];
    for ext in forbidden_extensions {
        if lower_prompt.contains(ext) {
            return Err(format!(
                "Privacy Violation: Forbidden extension '{}' detected in prompt context.",
                ext
            ));
        }
    }

    // 4. Drive letters / Windows paths (case-insensitive check for X:\ or X:/)
    for c in b'a'..=b'z' {
        let drive_win = format!("{}:\\", c as char);
        if lower_prompt.contains(&drive_win) {
            return Err(format!(
                "Privacy Violation: Forbidden Windows path prefix '{}' detected in prompt context.",
                drive_win
            ));
        }
        let drive_unix = format!("{}:/", c as char);
        if lower_prompt.contains(&drive_unix) {
            return Err(format!(
                "Privacy Violation: Forbidden Windows path prefix '{}' detected in prompt context.",
                drive_unix
            ));
        }
    }

    // 5. Unix absolute paths / sensitive paths (case-insensitive)
    let forbidden_path_prefixes = &["/users/", "/home/", "/var/", "/tmp/", "/etc/", "/opt/"];
    for prefix in forbidden_path_prefixes {
        if lower_prompt.contains(prefix) {
            return Err(format!(
                "Privacy Violation: Forbidden Unix path prefix '{}' detected in prompt context.",
                prefix
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::PlayMode;

    #[test]
    fn test_auto_target_defaults_to_balanced_without_signals() {
        let report = resolve_pattern_family_targeting(
            10,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            None,
            "sec1",
            None,
            None,
        );
        assert_eq!(report.primary_family, "balanced");
        assert_eq!(report.confidence, "low");
    }

    #[test]
    fn test_manual_pattern_family_override_sets_primary() {
        let report = resolve_pattern_family_targeting(
            12,
            PlayMode::Single,
            PatternFamilyTargetMode::Specific(PatternFamily::Stream),
            None,
            "sec1",
            None,
            None,
        );
        assert_eq!(report.primary_family, "stream");
    }

    #[test]
    fn test_low_level_avoids_bracket_and_heavy_twist() {
        let report = resolve_pattern_family_targeting(
            5,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            None,
            "sec1",
            None,
            None,
        );
        assert!(report
            .avoid_families
            .contains(&"bracket_technical".to_string()));
        assert!(report
            .avoid_families
            .contains(&"twist_technical".to_string()));
        assert!(report.avoid_families.contains(&"stamina".to_string()));
    }

    #[test]
    fn test_music_analysis_stream_signal_prefers_stream() {
        let report_json = r#"{
          "schema_version": "v1",
          "song_id": "test_song",
          "title": "Test",
          "artist": "Test",
          "duration_seconds": 120.0,
          "audio_summary": {
            "sample_rate": 44100,
            "detected_bpm": 120.0,
            "rms_energy_mean": 0.5,
            "rms_energy_max": 0.8,
            "spectral_centroid_mean": 1000.0,
            "spectral_flatness_mean": 0.1,
            "zero_crossing_rate_mean": 0.05,
            "chroma_mean": null,
            "spectral_contrast_mean": null,
            "analysis_mode": "offline"
          },
          "timing_grid": {
            "initial_offset": 0.0,
            "bpms": [],
            "display_bpm": "120",
            "song_type": "Arcade"
          },
          "event_features": {
            "beats": []
          },
          "sections": [
            {
              "section_id": "sec1",
              "start_beat": 0.0,
              "end_beat": 16.0,
              "start_measure": 0,
              "end_measure": 4,
              "music_role": "verse",
              "piu_role": "stream_opportunity",
              "boundary_confidence": 0.9,
              "energy_profile": "mid"
            }
          ],
          "choreographic_intent": [
            {
              "schema_version": "v1",
              "section_id": "sec1",
              "mode": "Single",
              "target_level": 12,
              "measure_start": 0,
              "measure_end": 4,
              "density_target": "moderate",
              "difficulty_budget": 10.0,
              "recommended_pattern_families": ["stream"],
              "avoid_pattern_families": ["jump_accent"],
              "accent_plan": [],
              "rest_plan": [],
              "motif_strategy": "motif",
              "evidence": [],
              "confidence": 0.8
            }
          ],
          "diagnostics": {
            "audio_bpm_detected": 120.0,
            "ssc_initial_bpm": 120.0,
            "audio_vs_ssc_tempo_agreement": true,
            "beat_grid_error_ms_mean": 0.0,
            "timing_confidence": 1.0,
            "requires_manual_timing_review": false,
            "warnings": [],
            "analysis_mode": "offline"
          },
          "publicability": {
            "contains_original_audio": false,
            "contains_full_chart": false,
            "exportable": true
          }
        }"#;

        let report_data: SongAnalysisReport = serde_json::from_str(report_json).unwrap();

        let report = resolve_pattern_family_targeting(
            12,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            Some(&report_data),
            "sec1",
            None,
            None,
        );
        assert_eq!(report.primary_family, "stream");
        assert!(report.avoid_families.contains(&"jump_accent".to_string()));
    }

    #[test]
    fn test_jump_accent_signal_prefers_jump_accent() {
        let report_json = r#"{
          "schema_version": "v1",
          "song_id": "test_song",
          "title": "Test",
          "artist": "Test",
          "duration_seconds": 120.0,
          "audio_summary": {
            "sample_rate": 44100,
            "detected_bpm": 120.0,
            "rms_energy_mean": 0.5,
            "rms_energy_max": 0.8,
            "spectral_centroid_mean": 1000.0,
            "spectral_flatness_mean": 0.1,
            "zero_crossing_rate_mean": 0.05,
            "chroma_mean": null,
            "spectral_contrast_mean": null,
            "analysis_mode": "offline"
          },
          "timing_grid": {
            "initial_offset": 0.0,
            "bpms": [],
            "display_bpm": "120",
            "song_type": "Arcade"
          },
          "event_features": {
            "beats": []
          },
          "sections": [
            {
              "section_id": "sec1",
              "start_beat": 0.0,
              "end_beat": 16.0,
              "start_measure": 0,
              "end_measure": 4,
              "music_role": "verse",
              "piu_role": "accent_drop",
              "boundary_confidence": 0.9,
              "energy_profile": "mid"
            }
          ],
          "choreographic_intent": [],
          "diagnostics": {
            "audio_bpm_detected": 120.0,
            "ssc_initial_bpm": 120.0,
            "audio_vs_ssc_tempo_agreement": true,
            "beat_grid_error_ms_mean": 0.0,
            "timing_confidence": 1.0,
            "requires_manual_timing_review": false,
            "warnings": [],
            "analysis_mode": "offline"
          },
          "publicability": {
            "contains_original_audio": false,
            "contains_full_chart": false,
            "exportable": true
          }
        }"#;

        let report_data: SongAnalysisReport = serde_json::from_str(report_json).unwrap();

        let report = resolve_pattern_family_targeting(
            12,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            Some(&report_data),
            "sec1",
            None,
            None,
        );
        assert_eq!(report.primary_family, "jump_accent");
    }

    #[test]
    fn test_low_confidence_music_analysis_lowers_targeting_confidence() {
        let report_json = r#"{
          "schema_version": "v1",
          "song_id": "test_song",
          "title": "Test",
          "artist": "Test",
          "duration_seconds": 120.0,
          "audio_summary": {
            "sample_rate": 44100,
            "detected_bpm": 120.0,
            "rms_energy_mean": 0.5,
            "rms_energy_max": 0.8,
            "spectral_centroid_mean": 1000.0,
            "spectral_flatness_mean": 0.1,
            "zero_crossing_rate_mean": 0.05,
            "chroma_mean": null,
            "spectral_contrast_mean": null,
            "analysis_mode": "offline"
          },
          "timing_grid": {
            "initial_offset": 0.0,
            "bpms": [],
            "display_bpm": "120",
            "song_type": "Arcade"
          },
          "event_features": {
            "beats": []
          },
          "sections": [
            {
              "section_id": "sec1",
              "start_beat": 0.0,
              "end_beat": 16.0,
              "start_measure": 0,
              "end_measure": 4,
              "music_role": "verse",
              "piu_role": "stream_opportunity",
              "boundary_confidence": 0.9,
              "energy_profile": "mid"
            }
          ],
          "choreographic_intent": [
            {
              "schema_version": "v1",
              "section_id": "sec1",
              "mode": "Single",
              "target_level": 12,
              "measure_start": 0,
              "measure_end": 4,
              "density_target": "moderate",
              "difficulty_budget": 10.0,
              "recommended_pattern_families": ["stream"],
              "avoid_pattern_families": [],
              "accent_plan": [],
              "rest_plan": [],
              "motif_strategy": "motif",
              "evidence": [],
              "confidence": 0.3
            }
          ],
          "diagnostics": {
            "audio_bpm_detected": 120.0,
            "ssc_initial_bpm": 120.0,
            "audio_vs_ssc_tempo_agreement": true,
            "beat_grid_error_ms_mean": 0.0,
            "timing_confidence": 1.0,
            "requires_manual_timing_review": false,
            "warnings": [],
            "analysis_mode": "offline"
          },
          "publicability": {
            "contains_original_audio": false,
            "contains_full_chart": false,
            "exportable": true
          }
        }"#;

        let report_data: SongAnalysisReport = serde_json::from_str(report_json).unwrap();

        let report = resolve_pattern_family_targeting(
            12,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            Some(&report_data),
            "sec1",
            None,
            None,
        );
        assert_eq!(report.confidence, "low");
    }

    #[test]
    fn test_missing_calibration_still_builds_prompt_context() {
        let report = resolve_pattern_family_targeting(
            12,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            None,
            "sec1",
            None,
            None,
        );
        assert_eq!(report.confidence, "low");
    }

    #[test]
    fn test_calibration_thresholds_are_included_when_available() {
        let calib_json = r#"{
            "schema_version": "single-guardrail-calibration.v0",
            "publicability_status": "private_derived",
            "play_mode": "Single",
            "source_dataset_summary": {},
            "level_thresholds": {
                "S14": {
                    "density": {
                        "warning_p90": 10.0,
                        "hard_limit_p95": 15.0,
                        "typical_p50": 8.0
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
            "pattern_family_thresholds": {},
            "confidence_policy": {},
            "recommended_runtime_usage": []
        }"#;
        let calib: SingleGuardrailCalibration = serde_json::from_str(calib_json).unwrap();
        let report = resolve_pattern_family_targeting(
            14,
            PlayMode::Single,
            PatternFamilyTargetMode::Auto,
            None,
            "sec1",
            Some(&calib),
            None,
        );
        assert_eq!(report.confidence, "high");
    }

    #[test]
    fn test_prompt_context_contains_no_private_paths_or_raw_tags() {
        assert!(self_audit_prompt_context("Normal prompt here with no private details").is_ok());
        assert!(self_audit_prompt_context("Forbidden #NOTEDATA raw tag").is_err());
        assert!(self_audit_prompt_context("Forbidden #notedata lowercase tag").is_err());
        assert!(self_audit_prompt_context("C:\\Users\\Desktop").is_err());
        assert!(self_audit_prompt_context("d:\\SomeFolder").is_err());
        assert!(self_audit_prompt_context("e:/SomeFolder").is_err());
        assert!(self_audit_prompt_context("/Users/admin/file.ssc").is_err());
        assert!(self_audit_prompt_context("/home/user/workspace").is_err());
        assert!(
            self_audit_prompt_context("dataset in .ai-step-gen-private-datasets folder").is_err()
        );
        assert!(self_audit_prompt_context("contains official_songs reference").is_err());
        assert!(self_audit_prompt_context("contains base64 data").is_err());
        assert!(self_audit_prompt_context("contains data:audio stream").is_err());
        assert!(self_audit_prompt_context("contains file.mp3").is_err());
        assert!(self_audit_prompt_context("contains file.ogg").is_err());
        assert!(self_audit_prompt_context("contains file.wav").is_err());
        assert!(self_audit_prompt_context("contains file.png").is_err());
        assert!(self_audit_prompt_context("contains #TITLE tag").is_err());
        assert!(self_audit_prompt_context("contains #ARTIST tag").is_err());
        assert!(self_audit_prompt_context("contains #BPMS tag").is_err());
        assert!(self_audit_prompt_context("contains #OFFSET tag").is_err());
        assert!(self_audit_prompt_context("contains #STOPS tag").is_err());
        assert!(self_audit_prompt_context("contains #DELAYS tag").is_err());
        assert!(self_audit_prompt_context("contains #WARPS tag").is_err());
        assert!(self_audit_prompt_context("contains Canonical BPM").is_err());
        assert!(self_audit_prompt_context("contains Browser BPM").is_err());
        assert!(self_audit_prompt_context("contains Candidates:").is_err());
        assert!(self_audit_prompt_context("contains Duración:").is_err());
        assert!(self_audit_prompt_context("contains segundos").is_err());
    }

    #[test]
    fn test_classify_browser_bpm_reconciliation_status_all_cases() {
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Status: agrees, Canonical BPM: 120"),
            Some(true)
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Status: disagrees, Canonical BPM: 120"),
            Some(false)
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Status: no_browser_evidence"),
            None
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Status: unsupported"),
            None
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("status: DISAGREES"),
            Some(false)
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Mismatch or manual review suggested"),
            Some(false)
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Agreed with sheet timing"),
            Some(true)
        );
        assert_eq!(
            classify_browser_bpm_reconciliation_status("Some text with disagrees in it"),
            Some(false)
        );
        assert_eq!(classify_browser_bpm_reconciliation_status(""), None);
        assert_eq!(classify_browser_bpm_reconciliation_status("   "), None);
    }
}
