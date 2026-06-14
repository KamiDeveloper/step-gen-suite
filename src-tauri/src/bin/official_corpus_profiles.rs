use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ==========================================
// Input Structs (Deserialization)
// ==========================================

#[derive(Debug, Clone, Deserialize)]
pub struct TimingSummaryShort {
    pub initial_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub display_bpm: String,
    pub offset: f64,
    pub has_timing_gimmicks: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DensityMetrics {
    pub notes_per_measure: f64,
    pub active_rows_per_measure: f64,
    pub jumps_per_measure: f64,
    pub holds_per_measure: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamMetrics {
    pub max_consecutive_active_rows: usize,
    pub estimated_stream_windows: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RestMetrics {
    pub empty_measure_count: usize,
    pub max_consecutive_empty_measures: usize,
    pub rest_measure_ratio: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TechEstimates {
    pub center_usage_ratio: f64,
    pub jump_ratio: f64,
    pub triple_ratio: f64,
    pub bracket_candidate_count: usize,
    pub twist_candidate_score: f64,
    pub stamina_score: f64,
    pub local_difficulty_estimate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagMetrics {
    pub has_mines: bool,
    pub has_unsupported_rows: bool,
    pub has_timing_gimmicks: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChartFeatureRecord {
    pub schema_version: String,
    pub song_id: String,
    pub chart_id: String,
    pub pack: String,
    pub title: String,
    pub artist: String,
    pub song_type: String,
    pub stepstype: String,
    pub mode: String,
    pub meter: u32,
    pub description: String,
    pub credit: String,
    pub stepmaker_candidate: String,
    pub timing_summary: TimingSummaryShort,
    pub measure_count: usize,
    pub row_count: usize,
    pub active_row_count: usize,
    pub empty_row_count: usize,
    pub tap_count: usize,
    pub hold_start_count: usize,
    pub hold_end_count: usize,
    pub jump_count: usize,
    pub triple_count: usize,
    pub quad_or_more_count: usize,
    pub center_note_count: usize,
    pub panel_counts: [usize; 5],
    pub density: DensityMetrics,
    pub streams: StreamMetrics,
    pub rests: RestMetrics,
    pub tech_estimates: TechEstimates,
    pub flags: FlagMetrics,
    pub publicability_status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowInfo {
    pub r#type: String,
    pub start_measure: usize,
    pub end_measure: usize,
    pub start_beat: f64,
    pub end_beat: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowDensity {
    pub notes_per_measure: f64,
    pub active_rows_per_measure: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowTechEstimates {
    pub stream_score: f64,
    pub jump_density: f64,
    pub center_usage_ratio: f64,
    pub bracket_candidate_count: usize,
    pub twist_candidate_score: f64,
    pub local_difficulty_estimate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternSummary {
    pub normalized_signature: String,
    pub mirror_invariant_signature: String,
    pub repeated_row_motif_score: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WindowFeatureRecord {
    pub schema_version: String,
    pub window_id: String,
    pub song_id: String,
    pub chart_id: String,
    pub mode: String,
    pub meter: u32,
    pub window: WindowInfo,
    pub row_count: usize,
    pub active_row_count: usize,
    pub tap_count: usize,
    pub hold_start_count: usize,
    pub jump_count: usize,
    pub triple_count: usize,
    pub empty_row_ratio: f64,
    pub density: WindowDensity,
    pub tech_estimates: WindowTechEstimates,
    pub pattern_summary: PatternSummary,
    pub anti_pattern_flags: Vec<String>,
    pub publicability_status: String,
}

// ==========================================
// Output Structs (Serialization)
// ==========================================

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

#[derive(Debug, Clone, Serialize)]
pub struct LevelChartFeatureProfile {
    pub measure_count: StatsSummary,
    pub row_count: StatsSummary,
    pub active_row_count: StatsSummary,
    pub empty_row_count: StatsSummary,
    pub tap_count: StatsSummary,
    pub hold_start_count: StatsSummary,
    pub hold_end_count: StatsSummary,
    pub jump_count: StatsSummary,
    pub triple_count: StatsSummary,
    pub quad_or_more_count: StatsSummary,
    pub center_note_count: StatsSummary,
    pub density_notes_per_measure: StatsSummary,
    pub density_active_rows_per_measure: StatsSummary,
    pub density_jumps_per_measure: StatsSummary,
    pub density_holds_per_measure: StatsSummary,
    pub streams_max_consecutive_active_rows: StatsSummary,
    pub streams_estimated_stream_windows: StatsSummary,
    pub rests_empty_measure_count: StatsSummary,
    pub rests_max_consecutive_empty_measures: StatsSummary,
    pub rests_rest_measure_ratio: StatsSummary,
    pub tech_center_usage_ratio: StatsSummary,
    pub tech_jump_ratio: StatsSummary,
    pub tech_triple_ratio: StatsSummary,
    pub tech_bracket_candidate_count: StatsSummary,
    pub tech_twist_candidate_score: StatsSummary,
    pub tech_stamina_score: StatsSummary,
    pub tech_local_difficulty_estimate: StatsSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct LevelWindowFeatureProfile {
    pub row_count: StatsSummary,
    pub active_row_count: StatsSummary,
    pub tap_count: StatsSummary,
    pub hold_start_count: StatsSummary,
    pub jump_count: StatsSummary,
    pub triple_count: StatsSummary,
    pub empty_row_ratio: StatsSummary,
    pub density_notes_per_measure: StatsSummary,
    pub density_active_rows_per_measure: StatsSummary,
    pub tech_stream_score: StatsSummary,
    pub tech_jump_density: StatsSummary,
    pub tech_center_usage_ratio: StatsSummary,
    pub tech_bracket_candidate_count: StatsSummary,
    pub tech_twist_candidate_score: StatsSummary,
    pub tech_local_difficulty_estimate: StatsSummary,
    pub pattern_repeated_row_motif_score: StatsSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationTargetRange {
    pub low: f64,
    pub typical: f64,
    pub high: f64,
    pub burst: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateRange {
    pub typical_min: f64,
    pub typical_max: f64,
    pub warning_high: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendedGenerationRanges {
    pub density_target: GenerationTargetRange,
    pub jump_rate: RateRange,
    pub twist_rate: RateRange,
    pub bracket_candidate_rate: RateRange,
}

#[derive(Debug, Clone, Serialize)]
pub struct LevelProfileRecord {
    pub schema_version: String,
    pub publicability_status: String,
    pub play_mode: String,
    pub level: u32,
    pub level_label: String,
    pub chart_count: usize,
    pub song_count_estimate: usize,
    pub window_count: usize,
    pub sample_confidence: String,
    pub song_type_distribution: BTreeMap<String, usize>,
    pub pack_distribution: BTreeMap<String, usize>,
    pub bpm_profile: StatsSummary,
    pub chart_feature_profile: LevelChartFeatureProfile,
    pub window_feature_profile: LevelWindowFeatureProfile,
    pub recommended_generation_ranges: RecommendedGenerationRanges,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternFamilySignature {
    pub primary_metrics: BTreeMap<String, f64>,
    pub secondary_metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypicalLevelRange {
    pub min: u32,
    pub median: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationGuidance {
    pub recommended_when: Vec<String>,
    pub avoid_when: Vec<String>,
    pub guardrail_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatternFamilyProfileRecord {
    pub schema_version: String,
    pub publicability_status: String,
    pub play_mode: String,
    pub pattern_family: String,
    pub level: u32,
    pub chart_count: usize,
    pub window_count: usize,
    pub sample_confidence: String,
    pub feature_signature: PatternFamilySignature,
    pub typical_level_range: TypicalLevelRange,
    pub generation_guidance: GenerationGuidance,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyleArchetypeBias {
    pub density_bias: String,
    pub accent_bias: String,
    pub twist_bias: String,
    pub bracket_bias: String,
    pub rest_bias: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyleArchetypeProfileRecord {
    pub schema_version: String,
    pub publicability_status: String,
    pub play_mode: String,
    pub style_archetype: String,
    pub level_band: String,
    pub chart_count: usize,
    pub window_count: usize,
    pub sample_confidence: String,
    pub feature_signature: BTreeMap<String, f64>,
    pub pattern_families: Vec<String>,
    pub generation_bias: StyleArchetypeBias,
    pub guardrail_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationLevelThreshold {
    pub density: BTreeMap<String, f64>,
    pub jump_rate: BTreeMap<String, f64>,
    pub twist_rate: BTreeMap<String, f64>,
    pub bracket_candidate_rate: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct GuardrailCalibration {
    pub schema_version: String,
    pub publicability_status: String,
    pub play_mode: String,
    pub source_dataset_summary: serde_json::Value,
    pub level_thresholds: BTreeMap<String, CalibrationLevelThreshold>,
    pub pattern_family_thresholds: BTreeMap<String, FamilyCalibrationSignal>,
    pub confidence_policy: BTreeMap<String, String>,
    pub recommended_runtime_usage: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticRecord {
    pub schema_version: String,
    pub publicability_status: String,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaVersions {
    pub manifest: String,
    pub level_profile: String,
    pub pattern_family_profile: String,
    pub style_archetype_profile: String,
    pub guardrail_calibration: String,
    pub diagnostic: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Manifest {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub dataset_root_kind: String,
    pub output_root_kind: String,
    pub input_schema_versions: BTreeMap<String, String>,
    pub output_schema_versions: SchemaVersions,
    pub input_catalog_records: usize,
    pub input_chart_feature_records: usize,
    pub input_window_feature_records: usize,
    pub input_diagnostic_records: usize,
    pub level_profile_records: usize,
    pub pattern_family_profile_records: usize,
    pub style_archetype_profile_records: usize,
    pub diagnostics_count: usize,
    pub validation_summary: serde_json::Value,
    pub privacy_summary: serde_json::Value,
    pub duration_seconds: f64,
}

// ==========================================
// Argument Parsing
// ==========================================

pub struct AppArgs {
    pub dataset_root: PathBuf,
    pub output_root: PathBuf,
    pub pretty: bool,
    pub fail_fast: bool,
    pub min_sample_size: usize,
    pub level_range: std::ops::RangeInclusive<u32>,
}

fn parse_level_range(s: &str) -> Result<std::ops::RangeInclusive<u32>, String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err("Level range must be in format 'MIN-MAX', e.g. '1-26'".to_string());
    }
    let min = parts[0].parse::<u32>().map_err(|_| "Invalid min level")?;
    let max = parts[1].parse::<u32>().map_err(|_| "Invalid max level")?;
    if min > max {
        return Err("Min level cannot be greater than max level".to_string());
    }
    Ok(min..=max)
}

fn parse_args() -> Result<AppArgs, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut dataset_root = None;
    let mut output_root = None;
    let mut pretty = false;
    let mut fail_fast = false;
    let mut min_sample_size = 10;
    let mut level_range = 1..=26;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--dataset-root" => {
                if i + 1 < args.len() {
                    dataset_root = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing value for --dataset-root".to_string());
                }
            }
            "--output-root" => {
                if i + 1 < args.len() {
                    output_root = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing value for --output-root".to_string());
                }
            }
            "--pretty" => {
                pretty = true;
                i += 1;
            }
            "--fail-fast" => {
                fail_fast = true;
                i += 1;
            }
            "--min-sample-size" => {
                if i + 1 < args.len() {
                    min_sample_size = args[i + 1]
                        .parse::<usize>()
                        .map_err(|_| "Invalid number for --min-sample-size")?;
                    i += 2;
                } else {
                    return Err("Missing value for --min-sample-size".to_string());
                }
            }
            "--level-range" => {
                if i + 1 < args.len() {
                    level_range = parse_level_range(&args[i + 1])?;
                    i += 2;
                } else {
                    return Err("Missing value for --level-range".to_string());
                }
            }
            _ => {
                return Err(format!("Unknown argument: {}", args[i]));
            }
        }
    }

    let dataset_root = dataset_root.ok_or_else(|| {
        "Missing required flag: --dataset-root. Run with --help for details.".to_string()
    })?;
    let output_root = output_root.ok_or_else(|| {
        "Missing required flag: --output-root. Run with --help for details.".to_string()
    })?;

    Ok(AppArgs {
        dataset_root,
        output_root,
        pretty,
        fail_fast,
        min_sample_size,
        level_range,
    })
}

fn print_usage() {
    println!("Official Corpus Statistical Aggregates & Profiles CLI Tool");
    println!("Usage: cargo run --bin official_corpus_profiles -- [options]");
    println!();
    println!("Required options:");
    println!("  --dataset-root <path>   Path containing the factory outputs");
    println!("  --output-root <path>    Path to write the profiles and calibration outputs");
    println!();
    println!("Optional options:");
    println!("  --pretty                Pretty-print manifest & calibration JSONs");
    println!("  --fail-fast             Abort immediately on first contract validation failure");
    println!(
        "  --min-sample-size <n>   Minimum sample size threshold for confidence (default: 10)"
    );
    println!("  --level-range <min-max> Range of levels to process (default: 1-26)");
}

// ==========================================
// Statistical Calculation Helpers
// ==========================================

pub fn compute_percentile(sorted_data: &[f64], p: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }
    if sorted_data.len() == 1 {
        return sorted_data[0];
    }
    let idx = (p * (sorted_data.len() - 1) as f64) / 100.0;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let fraction = idx - lower as f64;
    sorted_data[lower] + fraction * (sorted_data[upper] - sorted_data[lower])
}

pub fn compute_std_dev(data: &[f64], mean: f64) -> f64 {
    if data.len() <= 1 {
        return 0.0;
    }
    let variance: f64 =
        data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
    variance.sqrt()
}

pub fn compute_stats(data: &[f64]) -> StatsSummary {
    if data.is_empty() {
        return StatsSummary {
            min: 0.0,
            p10: 0.0,
            p25: 0.0,
            median: 0.0,
            p75: 0.0,
            p90: 0.0,
            p95: 0.0,
            max: 0.0,
            mean: 0.0,
            std_dev: 0.0,
        };
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];

    let sum: f64 = sorted.iter().sum();
    let mean = sum / sorted.len() as f64;

    let std_dev = compute_std_dev(&sorted, mean);

    StatsSummary {
        min: (min * 100.0).round() / 100.0,
        p10: (compute_percentile(&sorted, 10.0) * 100.0).round() / 100.0,
        p25: (compute_percentile(&sorted, 25.0) * 100.0).round() / 100.0,
        median: (compute_percentile(&sorted, 50.0) * 100.0).round() / 100.0,
        p75: (compute_percentile(&sorted, 75.0) * 100.0).round() / 100.0,
        p90: (compute_percentile(&sorted, 90.0) * 100.0).round() / 100.0,
        p95: (compute_percentile(&sorted, 95.0) * 100.0).round() / 100.0,
        max: (max * 100.0).round() / 100.0,
        mean: (mean * 100.0).round() / 100.0,
        std_dev: (std_dev * 100.0).round() / 100.0,
    }
}

// ==========================================
// Input Validation Helper
// ==========================================

#[derive(Debug, Deserialize)]
pub struct BaseRecordHeader {
    pub schema_version: Option<String>,
    pub publicability_status: Option<String>,
}

fn validate_input_record(
    line_num: usize,
    filename: &str,
    header: &BaseRecordHeader,
    expected_schema: &str,
    expected_status: &str,
    fail_fast: bool,
    diagnostics: &mut Vec<DiagnosticRecord>,
    missing_required_fields_count: &mut usize,
    invalid_publicability_status_count: &mut usize,
) -> Result<bool, String> {
    let schema = match &header.schema_version {
        Some(s) => s.as_str(),
        None => {
            let msg = format!(
                "Missing schema_version in {} at line {}",
                filename, line_num
            );
            *missing_required_fields_count += 1;
            diagnostics.push(DiagnosticRecord {
                schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                publicability_status: "private_diagnostic".to_string(),
                severity: "error".to_string(),
                code: "MISSING_SCHEMA_VERSION".to_string(),
                message: msg.clone(),
                context: serde_json::json!({ "file": filename, "line": line_num }),
            });
            if fail_fast {
                return Err(msg);
            }
            return Ok(false);
        }
    };

    if schema != expected_schema {
        let msg = format!(
            "Incorrect schema_version in {} at line {}: expected {}, got {}",
            filename, line_num, expected_schema, schema
        );
        diagnostics.push(DiagnosticRecord {
            schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
            publicability_status: "private_diagnostic".to_string(),
            severity: "warning".to_string(),
            code: "UNSUPPORTED_SCHEMA_VERSION".to_string(),
            message: msg.clone(),
            context: serde_json::json!({ "file": filename, "line": line_num, "schema": schema }),
        });
        if fail_fast {
            return Err(msg);
        }
        return Ok(false);
    }

    let status = match &header.publicability_status {
        Some(s) => s.as_str(),
        None => {
            let msg = format!(
                "Missing publicability_status in {} at line {}",
                filename, line_num
            );
            *missing_required_fields_count += 1;
            diagnostics.push(DiagnosticRecord {
                schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                publicability_status: "private_diagnostic".to_string(),
                severity: "error".to_string(),
                code: "MISSING_PUBLICABILITY_STATUS".to_string(),
                message: msg.clone(),
                context: serde_json::json!({ "file": filename, "line": line_num }),
            });
            if fail_fast {
                return Err(msg);
            }
            return Ok(false);
        }
    };

    if status != expected_status {
        let msg = format!(
            "Invalid publicability_status in {} at line {}: expected {}, got {}",
            filename, line_num, expected_status, status
        );
        *invalid_publicability_status_count += 1;
        diagnostics.push(DiagnosticRecord {
            schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
            publicability_status: "private_diagnostic".to_string(),
            severity: "warning".to_string(),
            code: "INVALID_PUBLICABILITY_STATUS".to_string(),
            message: msg.clone(),
            context: serde_json::json!({ "file": filename, "line": line_num, "status": status }),
        });
        if fail_fast {
            return Err(msg);
        }
        return Ok(false);
    }

    Ok(true)
}

// ==========================================
// Classifiers
// ==========================================

pub fn classify_window_families(w: &WindowFeatureRecord) -> Vec<&'static str> {
    let mut families = Vec::new();
    let active = w.active_row_count as f64;

    if w.tech_estimates.stream_score >= 0.50 {
        families.push("stream");
    }
    if active > 0.0 && (w.jump_count as f64 / active) >= 0.20 {
        families.push("jump_accent");
    }
    if w.tech_estimates.twist_candidate_score >= 0.20 {
        families.push("twist_technical");
    }
    if w.tech_estimates.bracket_candidate_count >= 3 {
        families.push("bracket_technical");
    }
    if active > 0.0 && (w.hold_start_count as f64 / active) >= 0.30 {
        families.push("hold_control");
    }
    if active > 0.0 && w.tech_estimates.center_usage_ratio >= 0.40 {
        families.push("center_control");
    }
    if w.tech_estimates.local_difficulty_estimate >= 16.0 && w.tech_estimates.stream_score >= 0.45 {
        families.push("stamina");
    }

    if families.is_empty() && active > 0.0 {
        families.push("balanced");
    }
    if active == 0.0 {
        families.push("unknown");
    }

    families
}

fn classify_chart_archetype(
    chart: &ChartFeatureRecord,
    windows: &[WindowFeatureRecord],
) -> &'static str {
    if chart.meter <= 6 {
        return "low_level_foundation";
    }
    if chart.meter >= 23 {
        return "high_level_pressure";
    }

    let mut family_counts = HashMap::new();
    let mut tot_active_windows = 0;

    for w in windows {
        let families = classify_window_families(w);
        let active = w.active_row_count as f64;
        if active > 0.0 {
            tot_active_windows += 1;
            for f in families {
                *family_counts.entry(f).or_insert(0) += 1;
            }
        }
    }

    if tot_active_windows > 0 {
        let p_stream =
            *family_counts.get("stream").unwrap_or(&0) as f64 / tot_active_windows as f64;
        let p_twist =
            *family_counts.get("twist_technical").unwrap_or(&0) as f64 / tot_active_windows as f64;
        let p_bracket = *family_counts.get("bracket_technical").unwrap_or(&0) as f64
            / tot_active_windows as f64;
        let p_jump =
            *family_counts.get("jump_accent").unwrap_or(&0) as f64 / tot_active_windows as f64;
        let p_hold =
            *family_counts.get("hold_control").unwrap_or(&0) as f64 / tot_active_windows as f64;

        if p_stream >= 0.30 {
            return "stream_endurance";
        }
        if p_twist >= 0.20 {
            return "twist_technical";
        }
        if p_bracket >= 0.15 {
            return "bracket_technical";
        }
        if p_jump >= 0.20 {
            return "jump_accent";
        }
        if p_hold >= 0.25 {
            return "hold_control";
        }
    }

    // Check for speed burst: if maximum window density is at least 1.5x median density
    let densities: Vec<f64> = windows
        .iter()
        .map(|w| w.density.notes_per_measure)
        .collect();
    if !densities.is_empty() {
        let mut sorted = densities.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = compute_percentile(&sorted, 50.0);
        let max_val = sorted[sorted.len() - 1];
        if median > 0.0 && (max_val / median) >= 1.5 {
            return "speed_burst";
        }
    }

    "balanced_official"
}

fn get_level_band(level: u32) -> &'static str {
    match level {
        1..=6 => "S1-S6",
        7..=10 => "S7-S10",
        11..=14 => "S11-S14",
        15..=18 => "S15-S18",
        19..=22 => "S19-S22",
        23..=26 => "S23-S26",
        _ => "S15-S18",
    }
}

// ==========================================
// Output Audit Checker
// ==========================================

fn perform_self_audit(output_root: &Path) -> Result<(), String> {
    let files_to_audit = [
        "manifest.v0.json",
        "single-level-profiles.v0.jsonl",
        "single-pattern-family-profiles.v0.jsonl",
        "single-style-archetype-profiles.v0.jsonl",
        "single-guardrail-calibration.v0.json",
        "diagnostics.v0.jsonl",
    ];

    let forbidden_patterns = [
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

    // OPTIMIZACIÓN EXTRA: Pre-calculamos los patrones en mayúsculas una sola vez
    let forbidden_patterns_upper: Vec<String> = forbidden_patterns
        .iter()
        .map(|p| p.to_uppercase())
        .collect();

    for filename in &files_to_audit {
        let file_path = output_root.join(filename);
        if !file_path.exists() {
            continue;
        }

        // 1. Abrimos el archivo de forma eficiente sin cargar su contenido aún
        let file = File::open(&file_path).map_err(|e| {
            format!(
                "Failed to open output file {:?} for audit: {}",
                file_path, e
            )
        })?;
        let reader = BufReader::new(file);

        // 2. Iteramos línea por línea. El uso de memoria ahora es O(longitud de la línea) y no O(tamaño del archivo)
        for (line_idx, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| {
                format!(
                    "Failed to read line {} in {:?}: {}",
                    line_idx + 1,
                    file_path,
                    e
                )
            })?;

            // 3. Pasamos a mayúsculas únicamente la línea actual
            let line_upper = line.to_uppercase();

            for (idx, pattern_upper) in forbidden_patterns_upper.iter().enumerate() {
                if line_upper.contains(pattern_upper) {
                    return Err(format!(
                        "PRIVACY VIOLATION: Output file {} contains forbidden pattern '{}' at line {}!",
                        filename, forbidden_patterns[idx], line_idx + 1
                    ));
                }
            }
        }
    }
    Ok(())
}

fn write_jsonl_record<T: Serialize, W: Write>(writer: &mut W, record: &T) -> io::Result<()> {
    let serialized =
        serde_json::to_string(record).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer.write_all(serialized.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

// ==========================================
// Primary Execution Runner
// ==========================================

pub fn run_aggregator(args: &AppArgs) -> Result<Manifest, String> {
    let start_time = Instant::now();
    let generated_at_utc = Utc::now().to_rfc3339();

    fs::create_dir_all(&args.output_root).map_err(|e| {
        format!(
            "Failed to create output directory {:?}: {}",
            args.output_root, e
        )
    })?;

    let mut diagnostics = Vec::new();

    // 1. Read input manifest
    let manifest_path = args.dataset_root.join("manifest.v0.json");
    let input_manifest: serde_json::Value = if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Cannot read input manifest: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in input manifest: {}", e))?
    } else {
        serde_json::json!({})
    };

    let mut input_schema_versions = BTreeMap::new();
    if let Some(svs) = input_manifest
        .get("schema_versions")
        .and_then(|v| v.as_object())
    {
        for (k, v) in svs {
            if let Some(s) = v.as_str() {
                input_schema_versions.insert(k.clone(), s.to_string());
            }
        }
    }

    let mut input_catalog_records = 0;
    let mut input_chart_feature_records = 0;
    let mut input_window_feature_records = 0;
    let mut input_diagnostic_records = 0;
    let mut skipped_records_count = 0;
    let mut missing_required_fields_count = 0;
    let mut invalid_publicability_status_count = 0;

    // We will parse all inputs line-by-line
    // A. Catalog index (for counts)
    let catalog_path = args.dataset_root.join("catalog-index.v0.jsonl");
    if catalog_path.exists() {
        let file =
            File::open(&catalog_path).map_err(|e| format!("Cannot open catalog file: {}", e))?;
        let reader = BufReader::new(file);
        for (line_idx, line) in reader.lines().enumerate() {
            let line_num = line_idx + 1;
            let content = line.map_err(|e| format!("Error reading catalog line: {}", e))?;
            if content.trim().is_empty() {
                continue;
            }
            input_catalog_records += 1;
            match serde_json::from_str::<BaseRecordHeader>(&content) {
                Ok(header) => {
                    let _valid = validate_input_record(
                        line_num,
                        "catalog-index.v0.jsonl",
                        &header,
                        "official-corpus-catalog.v0",
                        "private_derived",
                        args.fail_fast,
                        &mut diagnostics,
                        &mut missing_required_fields_count,
                        &mut invalid_publicability_status_count,
                    )?;
                }
                Err(e) => {
                    diagnostics.push(DiagnosticRecord {
                        schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                        publicability_status: "private_diagnostic".to_string(),
                        severity: "error".to_string(),
                        code: "INVALID_JSON".to_string(),
                        message: format!("Catalog line {} is invalid JSON: {}", line_num, e),
                        context: serde_json::json!({ "file": "catalog-index.v0.jsonl", "line": line_num }),
                    });
                    if args.fail_fast {
                        return Err(format!(
                            "Invalid JSON in catalog at line {}: {}",
                            line_num, e
                        ));
                    }
                }
            }
        }
    }

    // B. Diagnostics/errors (for counts)
    let errors_path = args.dataset_root.join("errors.v0.jsonl");
    if errors_path.exists() {
        let file =
            File::open(&errors_path).map_err(|e| format!("Cannot open errors file: {}", e))?;
        let reader = BufReader::new(file);
        for (line_idx, line) in reader.lines().enumerate() {
            let line_num = line_idx + 1;
            let content = line.map_err(|e| format!("Error reading errors line: {}", e))?;
            if content.trim().is_empty() {
                continue;
            }
            input_diagnostic_records += 1;
            match serde_json::from_str::<BaseRecordHeader>(&content) {
                Ok(header) => {
                    let _valid = validate_input_record(
                        line_num,
                        "errors.v0.jsonl",
                        &header,
                        "official-corpus-error.v0",
                        "private_diagnostic",
                        args.fail_fast,
                        &mut diagnostics,
                        &mut missing_required_fields_count,
                        &mut invalid_publicability_status_count,
                    )?;
                }
                Err(e) => {
                    diagnostics.push(DiagnosticRecord {
                        schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                        publicability_status: "private_diagnostic".to_string(),
                        severity: "error".to_string(),
                        code: "INVALID_JSON".to_string(),
                        message: format!("Errors line {} is invalid JSON: {}", line_num, e),
                        context: serde_json::json!({ "file": "errors.v0.jsonl", "line": line_num }),
                    });
                    if args.fail_fast {
                        return Err(format!(
                            "Invalid JSON in errors at line {}: {}",
                            line_num, e
                        ));
                    }
                }
            }
        }
    }

    // C. Read Chart Feature Records
    let mut charts = Vec::new();
    let chart_features_path = args.dataset_root.join("single-chart-features.v0.jsonl");
    if chart_features_path.exists() {
        let file = File::open(&chart_features_path)
            .map_err(|e| format!("Cannot open chart features file: {}", e))?;
        let reader = BufReader::new(file);
        for (line_idx, line) in reader.lines().enumerate() {
            let line_num = line_idx + 1;
            let content = line.map_err(|e| format!("Error reading chart features line: {}", e))?;
            if content.trim().is_empty() {
                continue;
            }
            input_chart_feature_records += 1;

            // Try direct deserialization first
            match serde_json::from_str::<ChartFeatureRecord>(&content) {
                Ok(record) => {
                    let header = BaseRecordHeader {
                        schema_version: Some(record.schema_version.clone()),
                        publicability_status: Some(record.publicability_status.clone()),
                    };

                    let valid = validate_input_record(
                        line_num,
                        "single-chart-features.v0.jsonl",
                        &header,
                        "single-chart-features.v0",
                        "private_derived",
                        args.fail_fast,
                        &mut diagnostics,
                        &mut missing_required_fields_count,
                        &mut invalid_publicability_status_count,
                    )?;

                    if !valid {
                        skipped_records_count += 1;
                        continue;
                    }

                    if record.mode != "Single" && record.stepstype != "pump-single" {
                        diagnostics.push(DiagnosticRecord {
                            schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                            publicability_status: "private_diagnostic".to_string(),
                            severity: "warning".to_string(),
                            code: "UNSUPPORTED_PLAY_MODE".to_string(),
                            message: format!("Unsupported play mode '{}' at line {}", record.mode, line_num),
                            context: serde_json::json!({ "file": "single-chart-features.v0.jsonl", "line": line_num, "mode": record.mode }),
                        });
                        skipped_records_count += 1;
                        continue;
                    }

                    if !args.level_range.contains(&record.meter) {
                        diagnostics.push(DiagnosticRecord {
                            schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                            publicability_status: "private_diagnostic".to_string(),
                            severity: "warning".to_string(),
                            code: "INVALID_LEVEL".to_string(),
                            message: format!("Level S{} at line {} out of range", record.meter, line_num),
                            context: serde_json::json!({ "file": "single-chart-features.v0.jsonl", "line": line_num, "level": record.meter }),
                        });
                        skipped_records_count += 1;
                        continue;
                    }

                    charts.push(record);
                }
                Err(e) => {
                    // Fall back to header-only parsing to diagnose the exact error
                    match serde_json::from_str::<BaseRecordHeader>(&content) {
                        Ok(header) => {
                            let valid = validate_input_record(
                                line_num,
                                "single-chart-features.v0.jsonl",
                                &header,
                                "single-chart-features.v0",
                                "private_derived",
                                args.fail_fast,
                                &mut diagnostics,
                                &mut missing_required_fields_count,
                                &mut invalid_publicability_status_count,
                            )?;

                            // If validation is Ok(true) but full deserialization failed, it's a field parsing error
                            if valid {
                                diagnostics.push(DiagnosticRecord {
                                    schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                                    publicability_status: "private_diagnostic".to_string(),
                                    severity: "error".to_string(),
                                    code: "DESERIALIZATION_ERROR".to_string(),
                                    message: format!("Failed to deserialize chart record at line {}: {}", line_num, e),
                                    context: serde_json::json!({ "file": "single-chart-features.v0.jsonl", "line": line_num }),
                                });
                                if args.fail_fast {
                                    return Err(format!(
                                        "Deserialization error in chart features at line {}",
                                        line_num
                                    ));
                                }
                            }
                        }
                        Err(header_err) => {
                            diagnostics.push(DiagnosticRecord {
                                schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                                publicability_status: "private_diagnostic".to_string(),
                                severity: "error".to_string(),
                                code: "INVALID_JSON".to_string(),
                                message: format!("Chart features line {} is invalid JSON: {}", line_num, header_err),
                                context: serde_json::json!({ "file": "single-chart-features.v0.jsonl", "line": line_num }),
                            });
                            if args.fail_fast {
                                return Err(format!(
                                    "Invalid JSON in chart features at line {}",
                                    line_num
                                ));
                            }
                        }
                    }
                    skipped_records_count += 1;
                }
            }
        }
    }

    // D. Read Window Feature Records
    let mut windows = Vec::new();
    let window_features_path = args.dataset_root.join("single-window-features.v0.jsonl");
    if window_features_path.exists() {
        let file = File::open(&window_features_path)
            .map_err(|e| format!("Cannot open window features file: {}", e))?;
        let reader = BufReader::new(file);
        for (line_idx, line) in reader.lines().enumerate() {
            let line_num = line_idx + 1;
            let content = line.map_err(|e| format!("Error reading window features line: {}", e))?;
            if content.trim().is_empty() {
                continue;
            }
            input_window_feature_records += 1;

            // Try direct deserialization first
            match serde_json::from_str::<WindowFeatureRecord>(&content) {
                Ok(record) => {
                    let header = BaseRecordHeader {
                        schema_version: Some(record.schema_version.clone()),
                        publicability_status: Some(record.publicability_status.clone()),
                    };

                    let valid = validate_input_record(
                        line_num,
                        "single-window-features.v0.jsonl",
                        &header,
                        "single-window-features.v0",
                        "private_derived",
                        args.fail_fast,
                        &mut diagnostics,
                        &mut missing_required_fields_count,
                        &mut invalid_publicability_status_count,
                    )?;

                    if !valid {
                        skipped_records_count += 1;
                        continue;
                    }

                    if record.mode != "Single" {
                        skipped_records_count += 1;
                        continue;
                    }

                    if !args.level_range.contains(&record.meter) {
                        skipped_records_count += 1;
                        continue;
                    }

                    windows.push(record);
                }
                Err(e) => {
                    // Fall back to header-only parsing to diagnose the exact error
                    match serde_json::from_str::<BaseRecordHeader>(&content) {
                        Ok(header) => {
                            let valid = validate_input_record(
                                line_num,
                                "single-window-features.v0.jsonl",
                                &header,
                                "single-window-features.v0",
                                "private_derived",
                                args.fail_fast,
                                &mut diagnostics,
                                &mut missing_required_fields_count,
                                &mut invalid_publicability_status_count,
                            )?;

                            // If validation is Ok(true) but full deserialization failed, it's a field parsing error
                            if valid {
                                diagnostics.push(DiagnosticRecord {
                                    schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                                    publicability_status: "private_diagnostic".to_string(),
                                    severity: "error".to_string(),
                                    code: "DESERIALIZATION_ERROR".to_string(),
                                    message: format!("Failed to deserialize window record at line {}: {}", line_num, e),
                                    context: serde_json::json!({ "file": "single-window-features.v0.jsonl", "line": line_num }),
                                });
                                if args.fail_fast {
                                    return Err(format!(
                                        "Deserialization error in window features at line {}",
                                        line_num
                                    ));
                                }
                            }
                        }
                        Err(header_err) => {
                            diagnostics.push(DiagnosticRecord {
                                schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                                publicability_status: "private_diagnostic".to_string(),
                                severity: "error".to_string(),
                                code: "INVALID_JSON".to_string(),
                                message: format!("Window features line {} is invalid JSON: {}", line_num, header_err),
                                context: serde_json::json!({ "file": "single-window-features.v0.jsonl", "line": line_num }),
                            });
                            if args.fail_fast {
                                return Err(format!(
                                    "Invalid JSON in window features at line {}",
                                    line_num
                                ));
                            }
                        }
                    }
                    skipped_records_count += 1;
                }
            }
        }
    }

    // Grouping by level
    let mut charts_by_level: HashMap<u32, Vec<ChartFeatureRecord>> = HashMap::new();
    for c in &charts {
        charts_by_level.entry(c.meter).or_default().push(c.clone());
    }

    let mut windows_by_level: HashMap<u32, Vec<WindowFeatureRecord>> = HashMap::new();
    for w in &windows {
        windows_by_level.entry(w.meter).or_default().push(w.clone());
    }

    // Output files
    let l_profile_path = args.output_root.join("single-level-profiles.v0.jsonl");
    let f_profile_path = args
        .output_root
        .join("single-pattern-family-profiles.v0.jsonl");
    let a_profile_path = args
        .output_root
        .join("single-style-archetype-profiles.v0.jsonl");
    let calib_path = args
        .output_root
        .join("single-guardrail-calibration.v0.json");
    let diag_path = args.output_root.join("diagnostics.v0.jsonl");

    let mut l_file = File::create(&l_profile_path)
        .map_err(|e| format!("Cannot create level profiles file: {}", e))?;
    let mut f_file = File::create(&f_profile_path)
        .map_err(|e| format!("Cannot create pattern family profiles file: {}", e))?;
    let mut a_file = File::create(&a_profile_path)
        .map_err(|e| format!("Cannot create style archetype profiles file: {}", e))?;

    let mut level_profile_records = 0;
    let mut pattern_family_profile_records = 0;
    let mut style_archetype_profile_records = 0;

    let mut level_profile_records_list = Vec::new();
    let mut pattern_family_profiles_list = Vec::new();
    let mut style_archetype_profiles_list = Vec::new();

    let mut calibration_thresholds = BTreeMap::new();

    // Loop S1 to S26
    for level in args.level_range.clone() {
        let level_label = format!("S{}", level);
        let level_charts = charts_by_level.get(&level);
        if level_charts.is_none() || level_charts.unwrap().is_empty() {
            // Omit if no charts found at this level
            continue;
        }
        let level_charts = level_charts.unwrap();
        let chart_count = level_charts.len();

        let song_count_estimate = level_charts
            .iter()
            .map(|c| &c.song_id)
            .collect::<HashSet<_>>()
            .len();

        let level_windows = windows_by_level.get(&level);
        let window_count = level_windows.map(|w| w.len()).unwrap_or(0);
        let level_windows_empty = Vec::new();
        let level_windows = level_windows.unwrap_or(&level_windows_empty);

        // Confidence
        let sample_confidence = if chart_count >= 50 {
            "high".to_string()
        } else if chart_count >= args.min_sample_size {
            "medium".to_string()
        } else {
            "low".to_string()
        };

        if chart_count < args.min_sample_size {
            diagnostics.push(DiagnosticRecord {
                schema_version: "official-corpus-profile-diagnostic.v0".to_string(),
                publicability_status: "private_diagnostic".to_string(),
                severity: "warning".to_string(),
                code: "LOW_SAMPLE_SIZE".to_string(),
                message: format!(
                    "Level S{} has only {} charts; profile confidence is low.",
                    level, chart_count
                ),
                context: serde_json::json!({ "level": level_label, "chart_count": chart_count }),
            });
        }

        // Distributions
        let mut song_type_distribution = BTreeMap::new();
        for c in level_charts {
            *song_type_distribution
                .entry(c.song_type.clone())
                .or_insert(0) += 1;
        }

        let mut pack_distribution = BTreeMap::new();
        for c in level_charts {
            *pack_distribution.entry(c.pack.clone()).or_insert(0) += 1;
        }

        // BPM profile
        let bpms: Vec<f64> = level_charts
            .iter()
            .map(|c| c.timing_summary.initial_bpm)
            .collect();
        let bpm_profile = compute_stats(&bpms);

        // Chart stats summaries
        let c_measure_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.measure_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_row_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.row_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_active_row_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.active_row_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_empty_row_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.empty_row_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_tap_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tap_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_hold_start_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.hold_start_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_hold_end_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.hold_end_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_jump_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.jump_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_triple_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.triple_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_quad_or_more_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.quad_or_more_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_center_note_count = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.center_note_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_density_notes = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.density.notes_per_measure)
                .collect::<Vec<_>>(),
        );
        let c_density_active_rows = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.density.active_rows_per_measure)
                .collect::<Vec<_>>(),
        );
        let c_density_jumps = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.density.jumps_per_measure)
                .collect::<Vec<_>>(),
        );
        let c_density_holds = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.density.holds_per_measure)
                .collect::<Vec<_>>(),
        );
        let c_streams_max = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.streams.max_consecutive_active_rows as f64)
                .collect::<Vec<_>>(),
        );
        let c_streams_est = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.streams.estimated_stream_windows as f64)
                .collect::<Vec<_>>(),
        );
        let c_rests_empty = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.rests.empty_measure_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_rests_max = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.rests.max_consecutive_empty_measures as f64)
                .collect::<Vec<_>>(),
        );
        let c_rests_ratio = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.rests.rest_measure_ratio)
                .collect::<Vec<_>>(),
        );
        let c_tech_center = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.center_usage_ratio)
                .collect::<Vec<_>>(),
        );
        let c_tech_jump = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.jump_ratio)
                .collect::<Vec<_>>(),
        );
        let c_tech_triple = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.triple_ratio)
                .collect::<Vec<_>>(),
        );
        let c_tech_bracket = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.bracket_candidate_count as f64)
                .collect::<Vec<_>>(),
        );
        let c_tech_twist = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.twist_candidate_score)
                .collect::<Vec<_>>(),
        );
        let c_tech_stamina = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.stamina_score)
                .collect::<Vec<_>>(),
        );
        let c_tech_diff = compute_stats(
            &level_charts
                .iter()
                .map(|c| c.tech_estimates.local_difficulty_estimate)
                .collect::<Vec<_>>(),
        );

        let chart_feature_profile = LevelChartFeatureProfile {
            measure_count: c_measure_count,
            row_count: c_row_count,
            active_row_count: c_active_row_count,
            empty_row_count: c_empty_row_count,
            tap_count: c_tap_count,
            hold_start_count: c_hold_start_count,
            hold_end_count: c_hold_end_count,
            jump_count: c_jump_count,
            triple_count: c_triple_count,
            quad_or_more_count: c_quad_or_more_count,
            center_note_count: c_center_note_count,
            density_notes_per_measure: c_density_notes,
            density_active_rows_per_measure: c_density_active_rows,
            density_jumps_per_measure: c_density_jumps,
            density_holds_per_measure: c_density_holds,
            streams_max_consecutive_active_rows: c_streams_max,
            streams_estimated_stream_windows: c_streams_est,
            rests_empty_measure_count: c_rests_empty,
            rests_max_consecutive_empty_measures: c_rests_max,
            rests_rest_measure_ratio: c_rests_ratio,
            tech_center_usage_ratio: c_tech_center,
            tech_jump_ratio: c_tech_jump,
            tech_triple_ratio: c_tech_triple,
            tech_bracket_candidate_count: c_tech_bracket,
            tech_twist_candidate_score: c_tech_twist,
            tech_stamina_score: c_tech_stamina,
            tech_local_difficulty_estimate: c_tech_diff,
        };

        // Window stats summaries
        let w_row_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.row_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_active_row_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.active_row_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_tap_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tap_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_hold_start_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.hold_start_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_jump_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.jump_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_triple_count = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.triple_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_empty_row_ratio = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.empty_row_ratio)
                .collect::<Vec<_>>(),
        );
        let w_density_notes = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.density.notes_per_measure)
                .collect::<Vec<_>>(),
        );
        let w_density_active_rows = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.density.active_rows_per_measure)
                .collect::<Vec<_>>(),
        );
        let w_tech_stream = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.stream_score)
                .collect::<Vec<_>>(),
        );
        let w_tech_jump = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.jump_density)
                .collect::<Vec<_>>(),
        );
        let w_tech_center = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.center_usage_ratio)
                .collect::<Vec<_>>(),
        );
        let w_tech_bracket = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.bracket_candidate_count as f64)
                .collect::<Vec<_>>(),
        );
        let w_tech_twist = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.twist_candidate_score)
                .collect::<Vec<_>>(),
        );
        let w_tech_diff = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.tech_estimates.local_difficulty_estimate)
                .collect::<Vec<_>>(),
        );
        let w_pattern_repeated = compute_stats(
            &level_windows
                .iter()
                .map(|w| w.pattern_summary.repeated_row_motif_score)
                .collect::<Vec<_>>(),
        );

        let window_feature_profile = LevelWindowFeatureProfile {
            row_count: w_row_count,
            active_row_count: w_active_row_count,
            tap_count: w_tap_count,
            hold_start_count: w_hold_start_count,
            jump_count: w_jump_count,
            triple_count: w_triple_count,
            empty_row_ratio: w_empty_row_ratio,
            density_notes_per_measure: w_density_notes,
            density_active_rows_per_measure: w_density_active_rows,
            tech_stream_score: w_tech_stream,
            tech_jump_density: w_tech_jump,
            tech_center_usage_ratio: w_tech_center,
            tech_bracket_candidate_count: w_tech_bracket,
            tech_twist_candidate_score: w_tech_twist,
            tech_local_difficulty_estimate: w_tech_diff,
            pattern_repeated_row_motif_score: w_pattern_repeated,
        };

        // Generation ranges
        let recommended_generation_ranges = RecommendedGenerationRanges {
            density_target: GenerationTargetRange {
                low: window_feature_profile.density_notes_per_measure.p25,
                typical: window_feature_profile.density_notes_per_measure.median,
                high: window_feature_profile.density_notes_per_measure.p75,
                burst: window_feature_profile.density_notes_per_measure.p90,
            },
            jump_rate: RateRange {
                typical_min: window_feature_profile.tech_jump_density.p25,
                typical_max: window_feature_profile.tech_jump_density.p75,
                warning_high: window_feature_profile.tech_jump_density.p90,
            },
            twist_rate: RateRange {
                typical_min: window_feature_profile.tech_twist_candidate_score.p25,
                typical_max: window_feature_profile.tech_twist_candidate_score.p75,
                warning_high: window_feature_profile.tech_twist_candidate_score.p90,
            },
            bracket_candidate_rate: RateRange {
                typical_min: window_feature_profile.tech_bracket_candidate_count.p25,
                typical_max: window_feature_profile.tech_bracket_candidate_count.p75,
                warning_high: window_feature_profile.tech_bracket_candidate_count.p90,
            },
        };

        let level_rec = LevelProfileRecord {
            schema_version: "single-level-profile.v0".to_string(),
            publicability_status: "private_derived".to_string(),
            play_mode: "Single".to_string(),
            level,
            level_label: level_label.clone(),
            chart_count,
            song_count_estimate,
            window_count,
            sample_confidence,
            song_type_distribution,
            pack_distribution,
            bpm_profile,
            chart_feature_profile,
            window_feature_profile,
            recommended_generation_ranges,
            notes: Vec::new(),
        };

        level_profile_records_list.push(level_rec.clone());

        // Populate Calibration Threshold map
        let mut density_thresh = BTreeMap::new();
        density_thresh.insert(
            "typical_p50".to_string(),
            level_rec
                .chart_feature_profile
                .density_notes_per_measure
                .median,
        );
        density_thresh.insert(
            "warning_p90".to_string(),
            level_rec
                .chart_feature_profile
                .density_notes_per_measure
                .p90,
        );
        density_thresh.insert(
            "hard_limit_p95".to_string(),
            level_rec
                .chart_feature_profile
                .density_notes_per_measure
                .p95,
        );

        let mut jump_thresh = BTreeMap::new();
        jump_thresh.insert(
            "warning_p90".to_string(),
            level_rec.chart_feature_profile.tech_jump_ratio.p90,
        );
        jump_thresh.insert(
            "hard_limit_p95".to_string(),
            level_rec.chart_feature_profile.tech_jump_ratio.p95,
        );

        let mut twist_thresh = BTreeMap::new();
        twist_thresh.insert(
            "warning_p90".to_string(),
            level_rec
                .chart_feature_profile
                .tech_twist_candidate_score
                .p90,
        );
        twist_thresh.insert(
            "hard_limit_p95".to_string(),
            level_rec
                .chart_feature_profile
                .tech_twist_candidate_score
                .p95,
        );

        let mut bracket_thresh = BTreeMap::new();
        bracket_thresh.insert(
            "warning_p90".to_string(),
            level_rec
                .chart_feature_profile
                .tech_bracket_candidate_count
                .p90,
        );
        bracket_thresh.insert(
            "hard_limit_p95".to_string(),
            level_rec
                .chart_feature_profile
                .tech_bracket_candidate_count
                .p95,
        );

        calibration_thresholds.insert(
            level_label,
            CalibrationLevelThreshold {
                density: density_thresh,
                jump_rate: jump_thresh,
                twist_rate: twist_thresh,
                bracket_candidate_rate: bracket_thresh,
            },
        );
    }

    for level_rec in &level_profile_records_list {
        let _ = write_jsonl_record(&mut l_file, level_rec);
        level_profile_records += 1;
    }

    // Pattern families calculations
    let families = [
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

    // Compute chart classifications to locate family level ranges
    let mut chart_classifications: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_family_levels: HashMap<String, Vec<u32>> = HashMap::new();

    for c in &charts {
        let c_windows: Vec<WindowFeatureRecord> = windows
            .iter()
            .filter(|w| w.chart_id == c.chart_id)
            .cloned()
            .collect();
        let tot_w = c_windows.len();
        if tot_w > 0 {
            let mut family_counts = HashMap::new();
            for w in &c_windows {
                for fam in classify_window_families(w) {
                    *family_counts.entry(fam.to_string()).or_insert(0) += 1;
                }
            }

            for (fam, count) in family_counts {
                let ratio = count as f64 / tot_w as f64;
                let thresh = if fam == "balanced" { 0.30 } else { 0.15 };
                if ratio >= thresh {
                    chart_classifications
                        .entry(c.chart_id.clone())
                        .or_default()
                        .insert(fam.clone());
                    all_family_levels.entry(fam).or_default().push(c.meter);
                }
            }
        }
    }

    // Map of level ranges per family
    let mut family_level_ranges = HashMap::new();
    for f in &families {
        let fam_str = f.to_string();
        let mut lvls = all_family_levels.get(&fam_str).cloned().unwrap_or_default();
        if !lvls.is_empty() {
            lvls.sort();
            let min = lvls[0];
            let max = lvls[lvls.len() - 1];
            let median =
                compute_percentile(&lvls.iter().map(|&x| x as f64).collect::<Vec<_>>(), 50.0)
                    as u32;
            family_level_ranges.insert(fam_str, TypicalLevelRange { min, median, max });
        } else {
            family_level_ranges.insert(
                fam_str,
                TypicalLevelRange {
                    min: 0,
                    median: 0,
                    max: 0,
                },
            );
        }
    }

    // Write Pattern Family Profiles
    for lvl in args.level_range.clone() {
        let lvl_charts = match charts_by_level.get(&lvl) {
            Some(v) => v,
            None => continue,
        };
        let lvl_windows = windows_by_level.get(&lvl);
        let level_windows_empty = Vec::new();
        let lvl_windows = lvl_windows.unwrap_or(&level_windows_empty);

        for fam in &families {
            let fam_str = fam.to_string();

            // Count charts at this level matching family
            let f_chart_count = lvl_charts
                .iter()
                .filter(|c| {
                    chart_classifications
                        .get(&c.chart_id)
                        .map(|set| set.contains(&fam_str))
                        .unwrap_or(false)
                })
                .count();

            // Count windows at this level matching family
            let f_windows: Vec<&WindowFeatureRecord> = lvl_windows
                .iter()
                .filter(|w| classify_window_families(w).contains(fam))
                .collect();
            let f_window_count = f_windows.len();

            if f_chart_count == 0 && f_window_count == 0 {
                continue;
            }

            let sample_confidence = if f_chart_count >= 15 {
                "high".to_string()
            } else if f_chart_count >= 5 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            // Calculate feature signature primary/secondary metrics
            let mut primary_metrics = BTreeMap::new();
            let mut secondary_metrics = BTreeMap::new();

            if f_window_count > 0 {
                let densities: Vec<f64> = f_windows
                    .iter()
                    .map(|w| w.density.notes_per_measure)
                    .collect();
                let densities_avg = compute_stats(&densities).mean;
                secondary_metrics.insert("density_notes_per_measure".to_string(), densities_avg);

                match fam_str.as_str() {
                    "stream" => {
                        let scores: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.stream_score)
                            .collect();
                        primary_metrics
                            .insert("stream_score".to_string(), compute_stats(&scores).mean);
                    }
                    "jump_accent" => {
                        let densities: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.jump_density)
                            .collect();
                        primary_metrics
                            .insert("jump_density".to_string(), compute_stats(&densities).mean);
                    }
                    "twist_technical" => {
                        let scores: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.twist_candidate_score)
                            .collect();
                        primary_metrics.insert(
                            "twist_candidate_score".to_string(),
                            compute_stats(&scores).mean,
                        );
                    }
                    "bracket_technical" => {
                        let counts: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.bracket_candidate_count as f64)
                            .collect();
                        primary_metrics.insert(
                            "bracket_candidate_count".to_string(),
                            compute_stats(&counts).mean,
                        );
                    }
                    "hold_control" => {
                        let ratios: Vec<f64> = f_windows
                            .iter()
                            .map(|w| {
                                if w.active_row_count > 0 {
                                    w.hold_start_count as f64 / w.active_row_count as f64
                                } else {
                                    0.0
                                }
                            })
                            .collect();
                        primary_metrics
                            .insert("hold_ratio".to_string(), compute_stats(&ratios).mean);
                    }
                    "center_control" => {
                        let ratios: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.center_usage_ratio)
                            .collect();
                        primary_metrics.insert(
                            "center_usage_ratio".to_string(),
                            compute_stats(&ratios).mean,
                        );
                    }
                    "stamina" => {
                        let diffs: Vec<f64> = f_windows
                            .iter()
                            .map(|w| w.tech_estimates.local_difficulty_estimate)
                            .collect();
                        primary_metrics.insert(
                            "local_difficulty_estimate".to_string(),
                            compute_stats(&diffs).mean,
                        );
                    }
                    _ => {}
                }
            }

            let guidance = match fam_str.as_str() {
                "stream" => GenerationGuidance {
                    recommended_when: vec![
                        "Building running streams".to_string(),
                        "Increasing stamina requirements".to_string(),
                    ],
                    avoid_when: vec![
                        "Low level charts under S7 where players lack physical speed".to_string(),
                    ],
                    guardrail_notes: vec![
                        "Keep streams uninterrupted under 16 active rows at lower levels"
                            .to_string(),
                    ],
                },
                "jump_accent" => GenerationGuidance {
                    recommended_when: vec![
                        "Adding impact on heavy musical beats or downbeats".to_string()
                    ],
                    avoid_when: vec![
                        "High-speed streams where consecutive jumps cause exhaustion".to_string(),
                    ],
                    guardrail_notes: vec!["Avoid consecutive jumps under level 16".to_string()],
                },
                "twist_technical" => GenerationGuidance {
                    recommended_when: vec![
                        "Creating directional transitions and flow changes".to_string()
                    ],
                    avoid_when: vec![
                        "Very high speeds where player cannot safely rotate hips".to_string()
                    ],
                    guardrail_notes: vec![
                        "Twists require alternating feet to avoid double-stepping".to_string(),
                    ],
                },
                "bracket_technical" => GenerationGuidance {
                    recommended_when: vec![
                        "Level 16+ where double steps or multi-panel taps are introduced"
                            .to_string(),
                    ],
                    avoid_when: vec![
                        "Low levels under S12 where players expect single-note panels".to_string(),
                    ],
                    guardrail_notes: vec!["Use brackets primarily on corner panels".to_string()],
                },
                "hold_control" => GenerationGuidance {
                    recommended_when: vec!["Resting sections or slow pacing segments".to_string()],
                    avoid_when: vec![
                        "Intense technical streams where holding a foot locks player mobility"
                            .to_string(),
                    ],
                    guardrail_notes: vec!["Ensure free foot has accessible patterns".to_string()],
                },
                "center_control" => GenerationGuidance {
                    recommended_when: vec![
                        "Pacing transitions or low-intensity bridge sections".to_string()
                    ],
                    avoid_when: vec![
                        "Technical streams requiring rapid side-to-side transitions".to_string()
                    ],
                    guardrail_notes: vec!["Center panel is a neutral pivot point".to_string()],
                },
                "stamina" => GenerationGuidance {
                    recommended_when: vec![
                        "Endgame boss sections or high-difficulty stamina tests".to_string(),
                    ],
                    avoid_when: vec!["Low difficulty levels under S15".to_string()],
                    guardrail_notes: vec![
                        "Ensure rests are provided before and after stamina burst segments"
                            .to_string(),
                    ],
                },
                _ => GenerationGuidance {
                    recommended_when: vec!["Standard chart progression".to_string()],
                    avoid_when: vec!["Highly specialized technical styles".to_string()],
                    guardrail_notes: vec!["Maintains average baseline metrics".to_string()],
                },
            };

            let fam_rec = PatternFamilyProfileRecord {
                schema_version: "single-pattern-family-profile.v0".to_string(),
                publicability_status: "private_derived".to_string(),
                play_mode: "Single".to_string(),
                pattern_family: fam_str.clone(),
                level: lvl,
                chart_count: f_chart_count,
                window_count: f_window_count,
                sample_confidence,
                feature_signature: PatternFamilySignature {
                    primary_metrics,
                    secondary_metrics,
                },
                typical_level_range: family_level_ranges.get(&fam_str).cloned().unwrap_or(
                    TypicalLevelRange {
                        min: 0,
                        median: 0,
                        max: 0,
                    },
                ),
                generation_guidance: guidance,
            };

            pattern_family_profiles_list.push(fam_rec);
        }
    }

    pattern_family_profiles_list.sort_by(|a, b| {
        a.pattern_family
            .cmp(&b.pattern_family)
            .then(a.level.cmp(&b.level))
    });
    for fam_rec in &pattern_family_profiles_list {
        let _ = write_jsonl_record(&mut f_file, fam_rec);
        pattern_family_profile_records += 1;
    }

    // Style archetype profiles
    let archetypes = [
        "balanced_official",
        "stream_endurance",
        "twist_technical",
        "jump_accent",
        "bracket_technical",
        "hold_control",
        "speed_burst",
        "low_level_foundation",
        "high_level_pressure",
    ];

    let level_bands = [
        "S1-S6", "S7-S10", "S11-S14", "S15-S18", "S19-S22", "S23-S26",
    ];

    // Classify all charts in workspace
    let mut chart_archetypes = HashMap::new();
    for c in &charts {
        let c_windows: Vec<WindowFeatureRecord> = windows
            .iter()
            .filter(|w| w.chart_id == c.chart_id)
            .cloned()
            .collect();
        let arch = classify_chart_archetype(c, &c_windows);
        chart_archetypes.insert(c.chart_id.clone(), arch);
    }

    // Compute baseline averages per level band
    let mut band_baselines = HashMap::new();
    for band in &level_bands {
        let band_charts: Vec<&ChartFeatureRecord> = charts
            .iter()
            .filter(|c| get_level_band(c.meter) == *band)
            .collect();
        if !band_charts.is_empty() {
            let dens: f64 = band_charts
                .iter()
                .map(|c| c.density.notes_per_measure)
                .sum::<f64>()
                / band_charts.len() as f64;
            let jump: f64 = band_charts
                .iter()
                .map(|c| c.tech_estimates.jump_ratio)
                .sum::<f64>()
                / band_charts.len() as f64;
            let twist: f64 = band_charts
                .iter()
                .map(|c| c.tech_estimates.twist_candidate_score)
                .sum::<f64>()
                / band_charts.len() as f64;
            let bracket: f64 = band_charts
                .iter()
                .map(|c| c.tech_estimates.bracket_candidate_count as f64)
                .sum::<f64>()
                / band_charts.len() as f64;
            let rest: f64 = band_charts
                .iter()
                .map(|c| c.rests.rest_measure_ratio)
                .sum::<f64>()
                / band_charts.len() as f64;

            band_baselines.insert(*band, (dens, jump, twist, bracket, rest));
        }
    }

    // Write Archetype Profiles
    for band in &level_bands {
        let band_charts: Vec<&ChartFeatureRecord> = charts
            .iter()
            .filter(|c| get_level_band(c.meter) == *band)
            .collect();
        if band_charts.is_empty() {
            continue;
        }

        let baseline = band_baselines
            .get(band)
            .cloned()
            .unwrap_or((1.0, 1.0, 1.0, 1.0, 1.0));

        for arch in &archetypes {
            let arch_charts: Vec<&ChartFeatureRecord> = band_charts
                .iter()
                .filter(|c| {
                    chart_archetypes
                        .get(&c.chart_id)
                        .map(|&a| a == *arch)
                        .unwrap_or(false)
                })
                .copied()
                .collect();
            let arch_chart_count = arch_charts.len();
            if arch_chart_count == 0 {
                continue;
            }

            let mut tot_w = 0;
            for c in &arch_charts {
                tot_w += windows.iter().filter(|w| w.chart_id == c.chart_id).count();
            }

            let sample_confidence = if arch_chart_count >= 30 {
                "high".to_string()
            } else if arch_chart_count >= 8 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            // Compute averages for signature
            let avg_dens = arch_charts
                .iter()
                .map(|c| c.density.notes_per_measure)
                .sum::<f64>()
                / arch_chart_count as f64;
            let avg_jump = arch_charts
                .iter()
                .map(|c| c.tech_estimates.jump_ratio)
                .sum::<f64>()
                / arch_chart_count as f64;
            let avg_twist = arch_charts
                .iter()
                .map(|c| c.tech_estimates.twist_candidate_score)
                .sum::<f64>()
                / arch_chart_count as f64;
            let avg_bracket = arch_charts
                .iter()
                .map(|c| c.tech_estimates.bracket_candidate_count as f64)
                .sum::<f64>()
                / arch_chart_count as f64;
            let avg_rest = arch_charts
                .iter()
                .map(|c| c.rests.rest_measure_ratio)
                .sum::<f64>()
                / arch_chart_count as f64;

            let mut feature_signature = BTreeMap::new();
            feature_signature.insert(
                "density_notes_per_measure".to_string(),
                (avg_dens * 100.0).round() / 100.0,
            );
            feature_signature.insert("jump_ratio".to_string(), (avg_jump * 100.0).round() / 100.0);
            feature_signature.insert(
                "twist_candidate_score".to_string(),
                (avg_twist * 100.0).round() / 100.0,
            );
            feature_signature.insert(
                "bracket_candidate_count".to_string(),
                (avg_bracket * 100.0).round() / 100.0,
            );
            feature_signature.insert(
                "rest_measure_ratio".to_string(),
                (avg_rest * 100.0).round() / 100.0,
            );

            // Compute biases relative to level band baseline
            let dens_ratio = if baseline.0 > 0.0 {
                avg_dens / baseline.0
            } else {
                1.0
            };
            let jump_ratio = if baseline.1 > 0.0 {
                avg_jump / baseline.1
            } else {
                1.0
            };
            let twist_ratio = if baseline.2 > 0.0 {
                avg_twist / baseline.2
            } else {
                1.0
            };
            let bracket_ratio = if baseline.3 > 0.0 {
                avg_bracket / baseline.3
            } else {
                1.0
            };
            let rest_ratio = if baseline.4 > 0.0 {
                avg_rest / baseline.4
            } else {
                1.0
            };

            let density_bias = if dens_ratio >= 1.25 {
                "very_high".to_string()
            } else if dens_ratio >= 1.05 {
                "high".to_string()
            } else if dens_ratio >= 0.90 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            let accent_bias = if jump_ratio >= 1.30 {
                "high".to_string()
            } else if jump_ratio >= 0.85 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            let twist_bias = if twist_ratio >= 1.30 {
                "high".to_string()
            } else if twist_ratio >= 0.85 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            let bracket_bias = if bracket_ratio >= 1.30 {
                "high".to_string()
            } else if bracket_ratio >= 0.85 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            let rest_bias = if rest_ratio >= 1.30 {
                "high".to_string()
            } else if rest_ratio >= 0.85 {
                "medium".to_string()
            } else {
                "low".to_string()
            };

            // Map corresponding active pattern families
            let mut pattern_families = Vec::new();
            if density_bias == "high" || density_bias == "very_high" {
                pattern_families.push("stream".to_string());
            }
            if accent_bias == "high" {
                pattern_families.push("jump_accent".to_string());
            }
            if twist_bias == "high" {
                pattern_families.push("twist_technical".to_string());
            }
            if bracket_bias == "high" {
                pattern_families.push("bracket_technical".to_string());
            }
            if rest_bias == "low" {
                pattern_families.push("stamina".to_string());
            }

            let arch_rec = StyleArchetypeProfileRecord {
                schema_version: "single-style-archetype-profile.v0".to_string(),
                publicability_status: "private_derived".to_string(),
                play_mode: "Single".to_string(),
                style_archetype: arch.to_string(),
                level_band: band.to_string(),
                chart_count: arch_chart_count,
                window_count: tot_w,
                sample_confidence,
                feature_signature,
                pattern_families,
                generation_bias: StyleArchetypeBias {
                    density_bias,
                    accent_bias,
                    twist_bias,
                    bracket_bias,
                    rest_bias,
                },
                guardrail_warnings: Vec::new(),
            };

            style_archetype_profiles_list.push(arch_rec);
        }
    }

    let band_order = |band: &str| -> usize {
        match band {
            "S1-S6" => 0,
            "S7-S10" => 1,
            "S11-S14" => 2,
            "S15-S18" => 3,
            "S19-S22" => 4,
            "S23-S26" => 5,
            _ => 6,
        }
    };

    style_archetype_profiles_list.sort_by(|a, b| {
        band_order(&a.level_band)
            .cmp(&band_order(&b.level_band))
            .then(a.style_archetype.cmp(&b.style_archetype))
    });

    for arch_rec in &style_archetype_profiles_list {
        let _ = write_jsonl_record(&mut a_file, arch_rec);
        style_archetype_profile_records += 1;
    }

    // Flush and close all files
    drop(l_file);
    drop(f_file);
    drop(a_file);

    // Pattern Family Thresholds Calibration
    let mut pattern_family_thresholds = BTreeMap::new();
    for fam in &families {
        let fam_str = fam.to_string();

        let fam_windows: Vec<&WindowFeatureRecord> = windows
            .iter()
            .filter(|w| classify_window_families(w).contains(fam))
            .collect();

        let sample_count = fam_windows.len();
        let sample_confidence = if sample_count >= 50 {
            "high".to_string()
        } else if sample_count >= 10 {
            "medium".to_string()
        } else {
            "low".to_string()
        };

        let metrics_vec: Vec<f64> = fam_windows
            .iter()
            .map(|w| match fam_str.as_str() {
                "stream" => w.tech_estimates.stream_score,
                "jump_accent" => {
                    if w.active_row_count > 0 {
                        w.jump_count as f64 / w.active_row_count as f64
                    } else {
                        0.0
                    }
                }
                "twist_technical" => w.tech_estimates.twist_candidate_score,
                "bracket_technical" => w.tech_estimates.bracket_candidate_count as f64,
                "hold_control" => {
                    if w.active_row_count > 0 {
                        w.hold_start_count as f64 / w.active_row_count as f64
                    } else {
                        0.0
                    }
                }
                "center_control" => w.tech_estimates.center_usage_ratio,
                "stamina" => w.tech_estimates.local_difficulty_estimate,
                "balanced" => w.density.notes_per_measure,
                _ => w.row_count as f64,
            })
            .collect();

        let metric_stats = compute_stats(&metrics_vec);

        let (rule, thresholds) = match fam_str.as_str() {
            "stream" => {
                let mut m = BTreeMap::new();
                m.insert("stream_score".to_string(), 0.50);
                ("stream_score >= 0.50".to_string(), m)
            }
            "jump_accent" => {
                let mut m = BTreeMap::new();
                m.insert("jump_ratio".to_string(), 0.20);
                ("jump_ratio >= 0.20".to_string(), m)
            }
            "twist_technical" => {
                let mut m = BTreeMap::new();
                m.insert("twist_candidate_score".to_string(), 0.20);
                ("twist_candidate_score >= 0.20".to_string(), m)
            }
            "bracket_technical" => {
                let mut m = BTreeMap::new();
                m.insert("bracket_candidate_count".to_string(), 3.0);
                ("bracket_candidate_count >= 3".to_string(), m)
            }
            "hold_control" => {
                let mut m = BTreeMap::new();
                m.insert("hold_ratio".to_string(), 0.30);
                ("hold_ratio >= 0.30".to_string(), m)
            }
            "center_control" => {
                let mut m = BTreeMap::new();
                m.insert("center_usage_ratio".to_string(), 0.40);
                ("center_usage_ratio >= 0.40".to_string(), m)
            }
            "stamina" => {
                let mut m = BTreeMap::new();
                m.insert("local_difficulty_estimate".to_string(), 16.0);
                m.insert("stream_score".to_string(), 0.45);
                (
                    "local_difficulty_estimate >= 16.0 && stream_score >= 0.45".to_string(),
                    m,
                )
            }
            "balanced" => (
                "active_rows > 0 && no other family matches".to_string(),
                BTreeMap::new(),
            ),
            _ => ("active_rows == 0".to_string(), BTreeMap::new()),
        };

        let guidance = match fam_str.as_str() {
            "stream" => (
                vec![
                    "Building running streams".to_string(),
                    "Increasing stamina requirements".to_string(),
                ],
                vec!["Low level charts under S7 where players lack physical speed".to_string()],
                vec!["Keep streams uninterrupted under 16 active rows at lower levels".to_string()],
            ),
            "jump_accent" => (
                vec!["Adding impact on heavy musical beats or downbeats".to_string()],
                vec!["High-speed streams where consecutive jumps cause exhaustion".to_string()],
                vec!["Avoid consecutive jumps under level 16".to_string()],
            ),
            "twist_technical" => (
                vec!["Creating directional transitions and flow changes".to_string()],
                vec!["Very high speeds where player cannot safely rotate hips".to_string()],
                vec!["Twists require alternating feet to avoid double-stepping".to_string()],
            ),
            "bracket_technical" => (
                vec!["Level 16+ where double steps or multi-panel taps are introduced".to_string()],
                vec!["Low levels under S12 where players expect single-note panels".to_string()],
                vec!["Use brackets primarily on corner panels".to_string()],
            ),
            "hold_control" => (
                vec!["Resting sections or slow pacing segments".to_string()],
                vec![
                    "Intense technical streams where holding a foot locks player mobility"
                        .to_string(),
                ],
                vec!["Ensure free foot has accessible patterns".to_string()],
            ),
            "center_control" => (
                vec!["Pacing transitions or low-intensity bridge sections".to_string()],
                vec!["Technical streams requiring rapid side-to-side transitions".to_string()],
                vec!["Center panel is a neutral pivot point".to_string()],
            ),
            "stamina" => (
                vec!["Endgame boss sections or high-difficulty stamina tests".to_string()],
                vec!["Low difficulty levels under S15".to_string()],
                vec![
                    "Ensure rests are provided before and after stamina burst segments".to_string(),
                ],
            ),
            _ => (
                vec!["Standard chart progression".to_string()],
                vec!["Highly specialized technical styles".to_string()],
                vec!["Maintains average baseline metrics".to_string()],
            ),
        };

        pattern_family_thresholds.insert(
            fam_str.clone(),
            FamilyCalibrationSignal {
                pattern_family: fam_str.clone(),
                classification_rule: rule,
                classifier_thresholds: thresholds,
                sample_count,
                sample_confidence,
                typical_level_range: family_level_ranges.get(&fam_str).cloned().unwrap_or(
                    TypicalLevelRange {
                        min: 0,
                        median: 0,
                        max: 0,
                    },
                ),
                metric_stats,
                recommended_when: guidance.0,
                avoid_when: guidance.1,
                guardrail_notes: guidance.2,
            },
        );
    }

    // Write Guardrail Calibration JSON
    let mut confidence_policy = BTreeMap::new();
    confidence_policy.insert("high".to_string(), "n >= 50".to_string());
    confidence_policy.insert("medium".to_string(), "10 <= n < 50".to_string());
    confidence_policy.insert("low".to_string(), "n < 10".to_string());

    let calibration = GuardrailCalibration {
        schema_version: "single-guardrail-calibration.v0".to_string(),
        publicability_status: "private_derived".to_string(),
        play_mode: "Single".to_string(),
        source_dataset_summary: serde_json::json!({
            "total_songs": input_catalog_records,
            "total_charts": charts.len(),
            "total_windows": windows.len(),
        }),
        level_thresholds: calibration_thresholds,
        pattern_family_thresholds,
        confidence_policy,
        recommended_runtime_usage: vec![
            "Use p50-p75 as default generation target.".to_string(),
            "Use p90 as warning threshold.".to_string(),
            "Use p95 as hard validation guidance unless biomechanical validator is stricter."
                .to_string(),
        ],
    };

    let calib_file =
        File::create(&calib_path).map_err(|e| format!("Cannot create calibration file: {}", e))?;
    if args.pretty {
        serde_json::to_writer_pretty(calib_file, &calibration)
            .map_err(|e| format!("Failed writing calibration JSON: {}", e))?;
    } else {
        serde_json::to_writer(calib_file, &calibration)
            .map_err(|e| format!("Failed writing calibration JSON: {}", e))?;
    }

    // Write Diagnostics JSONL
    let mut diag_file =
        File::create(&diag_path).map_err(|e| format!("Cannot create diagnostics file: {}", e))?;
    for diag in &diagnostics {
        let _ = write_jsonl_record(&mut diag_file, diag);
    }
    let diagnostics_count = diagnostics.len();
    drop(diag_file);

    // Self audit check
    perform_self_audit(&args.output_root)?;

    let duration_seconds = start_time.elapsed().as_secs_f64();

    // Write Output Manifest
    let output_manifest = Manifest {
        schema_version: "official-corpus-profiles-manifest.v0".to_string(),
        generated_at_utc,
        dataset_root_kind: "private_local_path_redacted".to_string(),
        output_root_kind: "private_local_path_redacted".to_string(),
        input_schema_versions,
        output_schema_versions: SchemaVersions {
            manifest: "official-corpus-profiles-manifest.v0".to_string(),
            level_profile: "single-level-profile.v0".to_string(),
            pattern_family_profile: "single-pattern-family-profile.v0".to_string(),
            style_archetype_profile: "single-style-archetype-profile.v0".to_string(),
            guardrail_calibration: "single-guardrail-calibration.v0".to_string(),
            diagnostic: "official-corpus-profile-diagnostic.v0".to_string(),
        },
        input_catalog_records,
        input_chart_feature_records,
        input_window_feature_records,
        input_diagnostic_records,
        level_profile_records,
        pattern_family_profile_records,
        style_archetype_profile_records,
        diagnostics_count,
        validation_summary: serde_json::json!({
            "missing_required_fields_count": missing_required_fields_count,
            "invalid_publicability_status_count": invalid_publicability_status_count,
            "skipped_records_count": skipped_records_count,
        }),
        privacy_summary: serde_json::json!({
            "contains_raw_notes": false,
            "contains_absolute_paths": false,
            "contains_media_assets": false,
        }),
        duration_seconds: (duration_seconds * 100.0).round() / 100.0,
    };

    let manifest_path = args.output_root.join("manifest.v0.json");
    let manifest_file =
        File::create(&manifest_path).map_err(|e| format!("Cannot create manifest file: {}", e))?;
    if args.pretty {
        serde_json::to_writer_pretty(manifest_file, &output_manifest)
            .map_err(|e| format!("Failed writing manifest JSON: {}", e))?;
    } else {
        serde_json::to_writer(manifest_file, &output_manifest)
            .map_err(|e| format!("Failed writing manifest JSON: {}", e))?;
    }

    Ok(output_manifest)
}

// ==========================================
// Main Entrypoint
// ==========================================

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Argument parsing error: {}", e);
            print_usage();
            std::process::exit(1);
        }
    };

    match run_aggregator(&args) {
        Ok(manifest) => {
            println!();
            println!(
                "Statistical profile aggregation completed successfully in {:.2}s!",
                manifest.duration_seconds
            );
            println!(
                "  Input Catalog Records:          {}",
                manifest.input_catalog_records
            );
            println!(
                "  Input Chart Feature Records:    {}",
                manifest.input_chart_feature_records
            );
            println!(
                "  Input Window Feature Records:   {}",
                manifest.input_window_feature_records
            );
            println!(
                "  Level Profile Records:          {}",
                manifest.level_profile_records
            );
            println!(
                "  Pattern Family Records:         {}",
                manifest.pattern_family_profile_records
            );
            println!(
                "  Style Archetype Records:        {}",
                manifest.style_archetype_profile_records
            );
            println!(
                "  Diagnostics Count:              {}",
                manifest.diagnostics_count
            );
            println!();
            println!("Profiles written to output-root: {:?}", args.output_root);
        }
        Err(e) => {
            eprintln!("Error during profiles aggregation execution: {}", e);
            std::process::exit(1);
        }
    }
}

// ==========================================
// Synthetic Test Module
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    fn get_temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "profiles_test_{}_{}",
            name,
            Utc::now().timestamp_micros()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_stats_percentiles_and_std_dev() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = compute_stats(&v);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 10.0);
        assert_eq!(stats.mean, 5.5);
        assert_eq!(stats.median, 5.5);
        // p90 should be 9.1 using linear interpolation between 9th (9) and 10th (10) indices
        assert_eq!(stats.p90, 9.1);
        assert_eq!(stats.p95, 9.55);
        assert_eq!(stats.std_dev, 3.03);
    }

    #[test]
    fn test_level_profiles_group_by_meter() {
        let temp = get_temp_dir("group_by_meter");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        // Write small chart features
        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let manifest = run_aggregator(&args).unwrap();
        assert_eq!(manifest.level_profile_records, 1);

        let profiles_content =
            fs::read_to_string(args.output_root.join("single-level-profiles.v0.jsonl")).unwrap();
        let val: serde_json::Value = serde_json::from_str(&profiles_content).unwrap();
        assert_eq!(val["level"].as_u64(), Some(14));
        assert_eq!(val["chart_count"].as_u64(), Some(1));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_sample_confidence_policy() {
        let temp = get_temp_dir("confidence");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let mut raw_chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });

        // Write 11 identical charts to cross the min sample size threshold of 10
        for i in 0..11 {
            raw_chart["chart_id"] = serde_json::json!(format!("chart_{}", i));
            write_jsonl_record(&mut chart_file, &raw_chart).unwrap();
        }
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();
        let profiles_content =
            fs::read_to_string(args.output_root.join("single-level-profiles.v0.jsonl")).unwrap();
        let val: serde_json::Value = serde_json::from_str(&profiles_content).unwrap();

        // Confidence should be "medium" since 10 <= count (11) < 50
        assert_eq!(val["sample_confidence"].as_str(), Some("medium"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_requires_publicability_status() {
        let temp = get_temp_dir("requires_pub");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        // A record missing publicability_status
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "meter": 14,
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: true, // Should abort immediately
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let result = run_aggregator(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing publicability_status"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_rejects_wrong_publicability_status() {
        let temp = get_temp_dir("wrong_pub");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        // Wrong status
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "publicability_status": "public_derived" // WRONG STATUS (expected private_derived)
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: true, // Should abort immediately
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let result = run_aggregator(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid publicability_status"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_jsonl_outputs_are_single_line_records() {
        let temp = get_temp_dir("single_line_jsonl");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: true, // Should still output JSONL as single line
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();

        let profiles_content =
            fs::read_to_string(args.output_root.join("single-level-profiles.v0.jsonl")).unwrap();
        let lines: Vec<&str> = profiles_content.lines().collect();
        assert_eq!(lines.len(), 1); // Single line record
        assert!(!lines[0].contains('\n'));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_no_private_identifiers_in_profile_outputs() {
        let temp = get_temp_dir("no_leaks");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "01-PRIME",
            "title": "Nemesis_Private_Leak_Title",
            "artist": "Artist_Leak",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14_Leak",
            "credit": "Credit_Leak",
            "stepmaker_candidate": "Credit_Leak",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();

        // Read Level Profiles and check for title/artist leaks
        let level_content =
            fs::read_to_string(args.output_root.join("single-level-profiles.v0.jsonl")).unwrap();
        assert!(!level_content.contains("Nemesis_Private_Leak_Title"));
        assert!(!level_content.contains("Artist_Leak"));
        assert!(!level_content.contains("Credit_Leak"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_pattern_family_classifier_is_deterministic() {
        let w = WindowFeatureRecord {
            schema_version: "single-window-features.v0".to_string(),
            window_id: "w1".to_string(),
            song_id: "s1".to_string(),
            chart_id: "c1".to_string(),
            mode: "Single".to_string(),
            meter: 14,
            window: WindowInfo {
                r#type: "measure_4".to_string(),
                start_measure: 0,
                end_measure: 3,
                start_beat: 0.0,
                end_beat: 16.0,
            },
            row_count: 64,
            active_row_count: 32,
            tap_count: 40,
            hold_start_count: 5,
            jump_count: 10,
            triple_count: 0,
            empty_row_ratio: 0.5,
            density: WindowDensity {
                notes_per_measure: 11.25,
                active_rows_per_measure: 8.0,
            },
            tech_estimates: WindowTechEstimates {
                stream_score: 0.55, // meets stream threshold (>= 0.5)
                jump_density: 2.5,
                center_usage_ratio: 0.1,
                bracket_candidate_count: 0,
                twist_candidate_score: 0.0,
                local_difficulty_estimate: 12.0,
            },
            pattern_summary: PatternSummary {
                normalized_signature: "".to_string(),
                mirror_invariant_signature: "".to_string(),
                repeated_row_motif_score: 0.0,
            },
            anti_pattern_flags: vec![],
            publicability_status: "private_derived".to_string(),
        };

        let families1 = classify_window_families(&w);
        let families2 = classify_window_families(&w);
        assert_eq!(families1, families2);
        assert!(families1.contains(&"stream"));
    }

    #[test]
    fn test_guardrail_calibration_contains_level_thresholds() {
        let temp = get_temp_dir("guardrail_thresholds");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();

        let calib_content = fs::read_to_string(
            args.output_root
                .join("single-guardrail-calibration.v0.json"),
        )
        .unwrap();
        let val: serde_json::Value = serde_json::from_str(&calib_content).unwrap();

        assert!(val.get("level_thresholds").is_some());
        assert!(val["level_thresholds"].get("S14").is_some());
        assert_eq!(
            val["level_thresholds"]["S14"]["density"]["typical_p50"].as_f64(),
            Some(2.5)
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_diagnostics_for_low_sample_levels() {
        let temp = get_temp_dir("low_sample_diag");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 26, // high level S26 with count 1 < 10
            "description": "s26",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 26.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();

        let diag_content =
            fs::read_to_string(args.output_root.join("diagnostics.v0.jsonl")).unwrap();
        assert!(diag_content.contains("LOW_SAMPLE_SIZE"));
        assert!(diag_content.contains("Level S26"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_perform_self_audit_catches_forbidden_patterns() {
        let temp = get_temp_dir("self_audit_check");
        let output_root = temp.join("output");
        fs::create_dir_all(&output_root).unwrap();

        // Write a valid dummy file
        fs::write(output_root.join("manifest.v0.json"), "{}").unwrap();

        // Write a file with a forbidden pattern
        fs::write(
            output_root.join("single-level-profiles.v0.jsonl"),
            "some prefix #TITLE: forbidden suffix",
        )
        .unwrap();

        // Run self audit, it should fail
        let result = perform_self_audit(&output_root);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("PRIVACY VIOLATION"));

        // Clean up
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_schema_mismatch_skips_record() {
        let val = BaseRecordHeader {
            schema_version: Some("incorrect-schema.v0".to_string()),
            publicability_status: Some("private_derived".to_string()),
        };
        let mut diagnostics = Vec::new();
        let mut missing_fields = 0;
        let mut invalid_status = 0;

        let result = validate_input_record(
            1,
            "test_file.jsonl",
            &val,
            "expected-schema.v0",
            "private_derived",
            false,
            &mut diagnostics,
            &mut missing_fields,
            &mut invalid_status,
        );

        assert_eq!(result, Ok(false));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "UNSUPPORTED_SCHEMA_VERSION");
    }

    #[test]
    fn test_schema_mismatch_aborts_immediately() {
        let val = BaseRecordHeader {
            schema_version: Some("incorrect-schema.v0".to_string()),
            publicability_status: Some("private_derived".to_string()),
        };
        let mut diagnostics = Vec::new();
        let mut missing_fields = 0;
        let mut invalid_status = 0;

        let result = validate_input_record(
            1,
            "test_file.jsonl",
            &val,
            "expected-schema.v0",
            "private_derived",
            true,
            &mut diagnostics,
            &mut missing_fields,
            &mut invalid_status,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Incorrect schema_version"));
    }

    #[test]
    fn test_outputs_are_stable_and_deterministic() {
        let temp = get_temp_dir("stability");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        // Write small chart features
        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        // Run aggregator twice
        let output1 = temp.join("output1");
        let args1 = AppArgs {
            dataset_root: ds_root.clone(),
            output_root: output1.clone(),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };
        let _ = run_aggregator(&args1).unwrap();

        let output2 = temp.join("output2");
        let args2 = AppArgs {
            dataset_root: ds_root.clone(),
            output_root: output2.clone(),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };
        let _ = run_aggregator(&args2).unwrap();

        // Compare all files
        let files = [
            "manifest.v0.json",
            "single-level-profiles.v0.jsonl",
            "single-pattern-family-profiles.v0.jsonl",
            "single-style-archetype-profiles.v0.jsonl",
            "single-guardrail-calibration.v0.json",
            "diagnostics.v0.jsonl",
        ];
        for f in &files {
            let content1 = fs::read(output1.join(f)).unwrap();
            let content2 = fs::read(output2.join(f)).unwrap();
            if *f == "manifest.v0.json" {
                let mut v1: serde_json::Value = serde_json::from_slice(&content1).unwrap();
                let mut v2: serde_json::Value = serde_json::from_slice(&content2).unwrap();
                if let Some(obj) = v1.as_object_mut() {
                    obj.remove("generated_at_utc");
                    obj.remove("duration_seconds");
                }
                if let Some(obj) = v2.as_object_mut() {
                    obj.remove("generated_at_utc");
                    obj.remove("duration_seconds");
                }
                assert_eq!(
                    v1, v2,
                    "manifest.v0.json mismatches (excluding dynamic fields)"
                );
            } else {
                assert_eq!(
                    content1, content2,
                    "File {} is not byte-for-byte identical!",
                    f
                );
            }
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_pattern_family_thresholds_calibration() {
        let temp = get_temp_dir("family_thresholds");
        let ds_root = temp.join("dataset");
        fs::create_dir_all(&ds_root).unwrap();

        // Write small chart features
        let mut chart_file = File::create(ds_root.join("single-chart-features.v0.jsonl")).unwrap();
        let chart = serde_json::json!({
            "schema_version": "single-chart-features.v0",
            "song_id": "song1",
            "chart_id": "chart1",
            "pack": "Pack1",
            "title": "Title",
            "artist": "Artist",
            "song_type": "ARCADE",
            "stepstype": "pump-single",
            "mode": "Single",
            "meter": 14,
            "description": "s14",
            "credit": "Credit",
            "stepmaker_candidate": "Credit",
            "timing_summary": {
                "initial_bpm": 150.0,
                "min_bpm": 150.0,
                "max_bpm": 150.0,
                "display_bpm": "150",
                "offset": -0.1,
                "has_timing_gimmicks": false
            },
            "measure_count": 80,
            "row_count": 320,
            "active_row_count": 160,
            "empty_row_count": 160,
            "tap_count": 200,
            "hold_start_count": 10,
            "hold_end_count": 10,
            "jump_count": 15,
            "triple_count": 0,
            "quad_or_more_count": 0,
            "center_note_count": 40,
            "panel_counts": [30, 30, 40, 30, 30],
            "density": {
                "notes_per_measure": 2.5,
                "active_rows_per_measure": 2.0,
                "jumps_per_measure": 0.2,
                "holds_per_measure": 0.1
            },
            "streams": {
                "max_consecutive_active_rows": 8,
                "estimated_stream_windows": 2
            },
            "rests": {
                "empty_measure_count": 10,
                "max_consecutive_empty_measures": 3,
                "rest_measure_ratio": 0.125
            },
            "tech_estimates": {
                "center_usage_ratio": 0.2,
                "jump_ratio": 0.1,
                "triple_ratio": 0.0,
                "bracket_candidate_count": 0,
                "twist_candidate_score": 0.05,
                "stamina_score": 0.3,
                "local_difficulty_estimate": 14.0
            },
            "flags": {
                "has_mines": false,
                "has_unsupported_rows": false,
                "has_timing_gimmicks": false
            },
            "publicability_status": "private_derived"
        });
        write_jsonl_record(&mut chart_file, &chart).unwrap();
        drop(chart_file);

        let args = AppArgs {
            dataset_root: ds_root,
            output_root: temp.join("output"),
            pretty: false,
            fail_fast: false,
            min_sample_size: 10,
            level_range: 1..=26,
        };

        let _ = run_aggregator(&args).unwrap();

        let calib_content = fs::read_to_string(
            args.output_root
                .join("single-guardrail-calibration.v0.json"),
        )
        .unwrap();
        let val: serde_json::Value = serde_json::from_str(&calib_content).unwrap();

        assert!(val.get("pattern_family_thresholds").is_some());
        let fam_thresh = &val["pattern_family_thresholds"];
        assert!(fam_thresh.get("stream").is_some());
        assert_eq!(
            fam_thresh["stream"]["classification_rule"].as_str(),
            Some("stream_score >= 0.50")
        );

        let _ = fs::remove_dir_all(&temp);
    }
}
