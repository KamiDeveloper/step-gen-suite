use ai_step_gen_suite_lib::ssc::parser::{SscDocument, SscTag};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
struct SongFolder {
    pack: String,
    folder_name: String,
    _dir_path: PathBuf,
    ssc_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BpmEntry {
    pub beat: f64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingSummary {
    pub offset: f64,
    pub bpms: Vec<BpmEntry>,
    pub display_bpm: String,
    pub has_stops: bool,
    pub has_delays: bool,
    pub has_warps: bool,
    pub has_speeds: bool,
    pub has_scrolls: bool,
    pub has_fakes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartCatalogInfo {
    pub chart_id: String,
    pub stepstype: String,
    pub mode: String,
    pub meter: u32,
    pub description: String,
    pub credit: String,
    pub stepmaker_candidate: String,
    pub measure_count: usize,
    pub is_excluded: bool,
    pub exclusion_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongCatalogRecord {
    pub schema_version: String,
    pub song_id: String,
    pub pack: String,
    pub relative_song_path: String,
    pub title: String,
    pub artist: String,
    pub genre: String,
    pub song_type: String,
    pub music_file: String,
    pub banner_file: String,
    pub background_file: String,
    pub preview_video_file: String,
    pub timing: TimingSummary,
    pub charts_total: usize,
    pub charts_included: usize,
    pub charts_excluded: usize,
    pub charts: Vec<ChartCatalogInfo>,
    pub parse_status: String,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub source_privacy: String,
    pub publicability_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityMetrics {
    pub notes_per_measure: f64,
    pub active_rows_per_measure: f64,
    pub jumps_per_measure: f64,
    pub holds_per_measure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub max_consecutive_active_rows: usize,
    pub estimated_stream_windows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestMetrics {
    pub empty_measure_count: usize,
    pub max_consecutive_empty_measures: usize,
    pub rest_measure_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechEstimates {
    pub center_usage_ratio: f64,
    pub jump_ratio: f64,
    pub triple_ratio: f64,
    pub bracket_candidate_count: usize,
    pub twist_candidate_score: f64,
    pub stamina_score: f64,
    pub local_difficulty_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagMetrics {
    pub has_mines: bool,
    pub has_unsupported_rows: bool,
    pub has_timing_gimmicks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingSummaryShort {
    pub initial_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub display_bpm: String,
    pub offset: f64,
    pub has_timing_gimmicks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub r#type: String,
    pub start_measure: usize,
    pub end_measure: usize,
    pub start_beat: f64,
    pub end_beat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowDensity {
    pub notes_per_measure: f64,
    pub active_rows_per_measure: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowTechEstimates {
    pub stream_score: f64,
    pub jump_density: f64,
    pub center_usage_ratio: f64,
    pub bracket_candidate_count: usize,
    pub twist_candidate_score: f64,
    pub local_difficulty_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSummary {
    pub normalized_signature: String,
    pub mirror_invariant_signature: String,
    pub repeated_row_motif_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub schema_version: String,
    pub severity: String,
    pub scope: String,
    pub song_id: Option<String>,
    pub chart_id: Option<String>,
    pub message: String,
    pub context: serde_json::Value,
    pub publicability_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersions {
    pub catalog: String,
    pub chart_features: String,
    pub window_features: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: String,
    pub run_started_at: String,
    pub run_finished_at: String,
    pub duration_seconds: f64,
    pub corpus_root_kind: String,
    pub output_root_kind: String,
    pub songs_scanned: usize,
    pub songs_parsed: usize,
    pub songs_failed: usize,
    pub charts_total: usize,
    pub charts_included: usize,
    pub charts_excluded: usize,
    pub single_charts_included: usize,
    pub errors_count: usize,
    pub warnings_count: usize,
    pub output_files_written: Vec<String>,
    pub schema_versions: SchemaVersions,
}

pub struct AppArgs {
    pub corpus_root: PathBuf,
    pub output_root: PathBuf,
    pub _mode: String,
    pub pretty: bool,
    pub limit_songs: Option<usize>,
    pub pack_filter: Option<String>,
    pub fail_fast: bool,
}

fn parse_args() -> Result<AppArgs, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut corpus_root = None;
    let mut output_root = None;
    let mut mode = None;
    let mut pretty = false;
    let mut limit_songs = None;
    let mut pack_filter = None;
    let mut fail_fast = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            "--corpus-root" => {
                if i + 1 < args.len() {
                    corpus_root = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing value for --corpus-root".to_string());
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
            "--mode" => {
                if i + 1 < args.len() {
                    mode = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("Missing value for --mode".to_string());
                }
            }
            "--pretty" => {
                pretty = true;
                i += 1;
            }
            "--limit-songs" => {
                if i + 1 < args.len() {
                    let limit = args[i + 1]
                        .parse::<usize>()
                        .map_err(|_| "Invalid number for --limit-songs")?;
                    limit_songs = Some(limit);
                    i += 2;
                } else {
                    return Err("Missing value for --limit-songs".to_string());
                }
            }
            "--pack-filter" => {
                if i + 1 < args.len() {
                    pack_filter = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("Missing value for --pack-filter".to_string());
                }
            }
            "--fail-fast" => {
                fail_fast = true;
                i += 1;
            }
            _ => {
                return Err(format!("Unknown argument: {}", args[i]));
            }
        }
    }

    let corpus_root = corpus_root.ok_or_else(|| {
        "Missing required flag: --corpus-root. Run with --help for details.".to_string()
    })?;
    let output_root = output_root.ok_or_else(|| {
        "Missing required flag: --output-root. Run with --help for details.".to_string()
    })?;
    let mode = mode
        .ok_or_else(|| "Missing required flag: --mode. Run with --help for details.".to_string())?;

    if mode != "single-v0" {
        return Err(format!(
            "Unsupported mode: {}. Only 'single-v0' is supported.",
            mode
        ));
    }

    Ok(AppArgs {
        corpus_root,
        output_root,
        _mode: mode,
        pretty,
        limit_songs,
        pack_filter,
        fail_fast,
    })
}

fn print_usage() {
    println!("Official Corpus Dataset Factory CLI Tool");
    println!("Usage: cargo run --bin official_corpus_factory -- [options]");
    println!();
    println!("Required options:");
    println!("  --corpus-root <path>  Path to official corpus directory");
    println!("  --output-root <path>  Path to write generated datasets");
    println!("  --mode <mode>         Processing mode (must be 'single-v0')");
    println!();
    println!("Optional options:");
    println!("  --pretty              Pretty-print manifest JSON only; JSONL remains one record per line");
    println!("  --limit-songs <n>     Process at most <n> songs (useful for smoke runs)");
    println!("  --pack-filter <name>  Process only songs in specified pack folder");
    println!("  --fail-fast           Exit immediately with error code on any fatal error");
}

fn compute_sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

fn compute_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // 16 hex chars
}

fn has_gimmick_content(val: &str) -> bool {
    let t = val.trim();
    !t.is_empty() && t != "," && t != ";"
}

fn get_tag_value(tags: &[SscTag], key: &str) -> Option<String> {
    tags.iter()
        .find(|t| t.key.as_deref() == Some(key))
        .map(|t| t.value.clone())
}

fn parse_bpms(val: &str) -> Vec<BpmEntry> {
    let mut entries = Vec::new();
    for part in val.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let kv: Vec<&str> = trimmed.split('=').collect();
        if kv.len() == 2 {
            let beat = kv[0].trim().parse::<f64>().unwrap_or(0.0);
            let bpm = kv[1].trim().parse::<f64>().unwrap_or(120.0);
            entries.push(BpmEntry { beat, bpm });
        }
    }
    entries
}

fn parse_notes_block(notes_raw: &str) -> Result<Vec<Vec<String>>, String> {
    let mut measures = Vec::new();
    let raw_measures = notes_raw.split(',');
    for raw_measure in raw_measures {
        let mut measure_rows = Vec::new();
        for line in raw_measure.lines() {
            let mut trimmed = line.trim();
            if let Some(idx) = trimmed.find("//") {
                trimmed = trimmed[..idx].trim();
            }
            if trimmed.ends_with(';') {
                trimmed = trimmed[..trimmed.len() - 1].trim();
            }
            if trimmed.is_empty() {
                continue;
            }
            measure_rows.push(trimmed.to_string());
        }
        measures.push(measure_rows);
    }
    if let Some(last) = measures.last() {
        if last.is_empty() {
            measures.pop();
        }
    }
    if measures.is_empty() {
        return Err("No measures found in notes block".to_string());
    }
    Ok(measures)
}

fn is_active_row(row: &str) -> bool {
    row.chars().any(|c| c == '1' || c == '2')
}

fn scan_corpus(corpus_root: &Path, pack_filter: Option<&str>) -> io::Result<Vec<SongFolder>> {
    let mut song_folders = Vec::new();
    if !corpus_root.exists() || !corpus_root.is_dir() {
        return Ok(song_folders);
    }
    let entries = fs::read_dir(corpus_root)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let pack_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if pack_name.starts_with('.') {
                continue;
            }
            if let Some(filter) = pack_filter {
                if pack_name != filter {
                    continue;
                }
            }
            let song_entries = fs::read_dir(&path)?;
            for song_entry in song_entries {
                let song_entry = song_entry?;
                let song_path = song_entry.path();
                if song_path.is_dir() {
                    let song_name = song_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if song_name.starts_with('.') {
                        continue;
                    }
                    let mut ssc_files = Vec::new();
                    let file_entries = fs::read_dir(&song_path)?;
                    for file_entry in file_entries {
                        let file_entry = file_entry?;
                        let file_path = file_entry.path();
                        if file_path.is_file() {
                            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                                if ext.eq_ignore_ascii_case("ssc") {
                                    ssc_files.push(file_path);
                                }
                            }
                        }
                    }
                    if !ssc_files.is_empty() {
                        song_folders.push(SongFolder {
                            pack: pack_name.to_string(),
                            folder_name: song_name.to_string(),
                            _dir_path: song_path,
                            ssc_paths: ssc_files,
                        });
                    }
                }
            }
        }
    }
    song_folders.sort_by(|a, b| (&a.pack, &a.folder_name).cmp(&(&b.pack, &b.folder_name)));
    Ok(song_folders)
}

fn write_jsonl_record<T: Serialize, W: Write>(writer: &mut W, record: &T) -> io::Result<()> {
    let serialized =
        serde_json::to_string(record).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writer.write_all(serialized.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn classify_chart_exclusion(
    stepstype: &str,
    meter: Option<u32>,
    chart_tags: &[SscTag],
) -> Option<String> {
    if stepstype != "pump-single" {
        return Some("Only pump-single is supported in feature extraction v0".to_string());
    }

    for tag in chart_tags {
        if tag.is_comment {
            continue;
        }
        if let Some(key) = &tag.key {
            let key_upper = key.to_uppercase();
            if key_upper == "NOTES"
                || key_upper == "BPMS"
                || key_upper == "STOPS"
                || key_upper == "DELAYS"
                || key_upper == "WARPS"
                || key_upper == "SPEEDS"
                || key_upper == "SCROLLS"
                || key_upper == "FAKES"
            {
                continue;
            }

            let val_upper = tag.value.to_uppercase();
            if val_upper.contains("UCS") {
                return Some("Excluded UCS chart".to_string());
            }
            if val_upper.contains("COOP")
                || val_upper.contains("CO-OP")
                || val_upper.contains("CO OP")
            {
                return Some("Excluded COOP chart".to_string());
            }
            if val_upper.contains("QUEST") {
                return Some("Excluded QUEST chart".to_string());
            }
            if val_upper.contains("TRAIN") {
                return Some("Excluded TRAIN chart".to_string());
            }
        }
    }

    if meter.is_none() || meter.unwrap() == 0 {
        return Some("Meter missing or unparseable".to_string());
    }

    None
}

pub fn run_factory(args: &AppArgs) -> Result<Manifest, String> {
    let start_time = Instant::now();
    let run_started_at = Utc::now().to_rfc3339();

    if !args.corpus_root.exists() || !args.corpus_root.is_dir() {
        return Err(format!(
            "Corpus root path does not exist or is not a directory: {:?}",
            args.corpus_root
        ));
    }

    fs::create_dir_all(&args.output_root).map_err(|e| {
        format!(
            "Failed to create output directory {:?}: {}",
            args.output_root, e
        )
    })?;

    let song_folders = scan_corpus(&args.corpus_root, args.pack_filter.as_deref())
        .map_err(|e| format!("Error scanning corpus directory: {}", e))?;

    let total_songs_to_process = if let Some(limit) = args.limit_songs {
        song_folders.len().min(limit)
    } else {
        song_folders.len()
    };

    let catalog_path = args.output_root.join("catalog-index.v0.jsonl");
    let chart_features_path = args.output_root.join("single-chart-features.v0.jsonl");
    let window_features_path = args.output_root.join("single-window-features.v0.jsonl");
    let errors_path = args.output_root.join("errors.v0.jsonl");
    let manifest_path = args.output_root.join("manifest.v0.json");

    let mut catalog_file = File::create(&catalog_path)
        .map_err(|e| format!("Cannot create catalog-index file: {}", e))?;
    let mut chart_features_file = File::create(&chart_features_path)
        .map_err(|e| format!("Cannot create single-chart-features file: {}", e))?;
    let mut window_features_file = File::create(&window_features_path)
        .map_err(|e| format!("Cannot create single-window-features file: {}", e))?;
    let mut errors_file =
        File::create(&errors_path).map_err(|e| format!("Cannot create errors file: {}", e))?;

    let mut songs_scanned = 0;
    let mut songs_parsed = 0;
    let mut songs_failed = 0;
    let mut charts_total = 0;
    let mut charts_included = 0;
    let mut charts_excluded = 0;
    let mut single_charts_included = 0;
    let mut errors_count = 0;
    let mut warnings_count = 0;

    for i in 0..total_songs_to_process {
        songs_scanned += 1;
        let song_folder = &song_folders[i];
        let ssc_path = &song_folder.ssc_paths[0];

        // Derived relative song path
        let relative_song_path = format!("{}/{}", song_folder.pack, song_folder.folder_name);
        let song_id = compute_sha256_hex(&format!("{}:{}", song_folder.pack, relative_song_path));

        let ssc_doc = match SscDocument::parse(ssc_path) {
            Ok(doc) => doc,
            Err(e) => {
                songs_failed += 1;
                let err_rec = ErrorRecord {
                    schema_version: "official-corpus-error.v0".to_string(),
                    severity: "error".to_string(),
                    scope: "song".to_string(),
                    song_id: Some(song_id.clone()),
                    chart_id: None,
                    message: format!("Failed to parse SSC document: {}", e),
                    context: serde_json::json!({
                        "relative_song_path": relative_song_path,
                        "pack": song_folder.pack,
                        "folder_name": song_folder.folder_name,
                        "ssc_file_name": ssc_path.file_name().and_then(|f| f.to_str()).unwrap_or(""),
                        "error": e.to_string()
                    }),
                    publicability_status: "private_diagnostic".to_string(),
                };
                let _ = write_jsonl_record(&mut errors_file, &err_rec);
                errors_count += 1;

                if args.fail_fast {
                    return Err(format!(
                        "Fatal error (fail-fast active): Failed to parse SSC at {:?}",
                        ssc_path
                    ));
                }
                continue;
            }
        };

        songs_parsed += 1;

        // Extract global metadata
        let title = get_tag_value(&ssc_doc.global_tags, "TITLE").unwrap_or_default();
        let artist = get_tag_value(&ssc_doc.global_tags, "ARTIST").unwrap_or_default();
        let genre = get_tag_value(&ssc_doc.global_tags, "GENRE").unwrap_or_default();
        let song_type =
            get_tag_value(&ssc_doc.global_tags, "SONGTYPE").unwrap_or_else(|| "ARCADE".to_string());
        let music_file = get_tag_value(&ssc_doc.global_tags, "MUSIC").unwrap_or_default();
        let banner_file = get_tag_value(&ssc_doc.global_tags, "BANNER").unwrap_or_default();
        let background_file = get_tag_value(&ssc_doc.global_tags, "BACKGROUND").unwrap_or_default();
        let preview_video_file =
            get_tag_value(&ssc_doc.global_tags, "PREVIEWVID").unwrap_or_default();

        let offset_str =
            get_tag_value(&ssc_doc.global_tags, "OFFSET").unwrap_or_else(|| "0.0".to_string());
        let offset = offset_str.trim().parse::<f64>().unwrap_or(0.0);

        let bpms_str = get_tag_value(&ssc_doc.global_tags, "BPMS").unwrap_or_default();
        let bpms = parse_bpms(&bpms_str);
        let display_bpm = get_tag_value(&ssc_doc.global_tags, "DISPLAYBPM").unwrap_or_else(|| {
            if let Some(first) = bpms.first() {
                format!("{:.3}", first.bpm)
            } else {
                "120.000".to_string()
            }
        });

        let has_stops = get_tag_value(&ssc_doc.global_tags, "STOPS")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);
        let has_delays = get_tag_value(&ssc_doc.global_tags, "DELAYS")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);
        let has_warps = get_tag_value(&ssc_doc.global_tags, "WARPS")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);
        let has_speeds = get_tag_value(&ssc_doc.global_tags, "SPEEDS")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);
        let has_scrolls = get_tag_value(&ssc_doc.global_tags, "SCROLLS")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);
        let has_fakes = get_tag_value(&ssc_doc.global_tags, "FAKES")
            .map(|v| has_gimmick_content(&v))
            .unwrap_or(false);

        let timing = TimingSummary {
            offset,
            bpms,
            display_bpm,
            has_stops,
            has_delays,
            has_warps,
            has_speeds,
            has_scrolls,
            has_fakes,
        };

        let mut catalog_charts = Vec::new();
        let mut warnings = Vec::new();
        let errors = Vec::<String>::new();

        charts_total += ssc_doc.charts.len();

        for chart in &ssc_doc.charts {
            let stepstype = get_tag_value(&chart.tags, "STEPSTYPE").unwrap_or_default();
            let credit = get_tag_value(&chart.tags, "CREDIT").unwrap_or_default();
            let description = get_tag_value(&chart.tags, "DESCRIPTION").unwrap_or_default();
            let meter_str = get_tag_value(&chart.tags, "METER").unwrap_or_default();
            let meter = meter_str.trim().parse::<u32>().ok();

            let mode = if stepstype == "pump-single" {
                "Single"
            } else if stepstype == "pump-double" {
                "Double"
            } else {
                "Other"
            };

            let chart_id = compute_sha256_hex(&format!(
                "{}:{}:{}:{}",
                song_id,
                stepstype,
                description,
                meter.unwrap_or(0)
            ));

            // Apply filters
            let mut exclusion_reason = classify_chart_exclusion(&stepstype, meter, &chart.tags);
            let mut is_excluded = exclusion_reason.is_some();

            // If not excluded by metadata/stepstype, parse notes
            let mut measure_count = 0;
            if !is_excluded {
                match parse_notes_block(&chart.notes_raw) {
                    Ok(measures) => {
                        measure_count = measures.len();

                        // Validate row lengths
                        let mut length_ok = true;
                        for (m_idx, measure) in measures.iter().enumerate() {
                            for (r_idx, row) in measure.iter().enumerate() {
                                if row.len() != 5 {
                                    is_excluded = true;
                                    length_ok = false;
                                    exclusion_reason = Some(format!(
                                        "Invalid row length at measure {}, row {}: expected 5, got {}",
                                        m_idx, r_idx, row.len()
                                    ));

                                    // Log warning
                                    let err_rec = ErrorRecord {
                                        schema_version: "official-corpus-error.v0".to_string(),
                                        severity: "warning".to_string(),
                                        scope: "chart".to_string(),
                                        song_id: Some(song_id.clone()),
                                        chart_id: Some(chart_id.clone()),
                                        message: "Invalid row length".to_string(),
                                        context: serde_json::json!({
                                            "relative_song_path": relative_song_path,
                                            "measure_index": m_idx,
                                            "row_index": r_idx,
                                            "expected_len": 5,
                                            "actual_len": row.len(),
                                        }),
                                        publicability_status: "private_diagnostic".to_string(),
                                    };
                                    let _ = write_jsonl_record(&mut errors_file, &err_rec);
                                    warnings_count += 1;

                                    break;
                                }
                            }
                            if !length_ok {
                                break;
                            }
                        }

                        if length_ok {
                            charts_included += 1;
                            single_charts_included += 1;

                            // Extract Chart-Level Features!
                            let mut tap_count = 0;
                            let mut hold_start_count = 0;
                            let mut hold_end_count = 0;
                            let mut jump_count = 0;
                            let mut triple_count = 0;
                            let mut quad_or_more_count = 0;
                            let mut center_note_count = 0;
                            let mut panel_counts = [0; 5];
                            let mut all_rows = Vec::new();
                            let mut has_mines = false;
                            let mut has_unsupported_rows = false;

                            for measure in &measures {
                                for row in measure {
                                    all_rows.push(row.clone());
                                    let mut active_in_row = 0;
                                    for (i, c) in row.chars().enumerate() {
                                        if c == '1' {
                                            tap_count += 1;
                                            active_in_row += 1;
                                            panel_counts[i] += 1;
                                            if i == 2 {
                                                center_note_count += 1;
                                            }
                                        } else if c == '2' {
                                            hold_start_count += 1;
                                            active_in_row += 1;
                                            panel_counts[i] += 1;
                                            if i == 2 {
                                                center_note_count += 1;
                                            }
                                        } else if c == '3' {
                                            hold_end_count += 1;
                                        } else if c == 'M' {
                                            has_mines = true;
                                        } else if c != '0' && c != '4' {
                                            has_unsupported_rows = true;
                                        }
                                    }
                                    if active_in_row == 2 {
                                        jump_count += 1;
                                    } else if active_in_row == 3 {
                                        triple_count += 1;
                                    } else if active_in_row >= 4 {
                                        quad_or_more_count += 1;
                                    }
                                }
                            }

                            let row_count = all_rows.len();
                            let active_row_count =
                                all_rows.iter().filter(|r| is_active_row(r)).count();

                            // Max consecutive active rows
                            let mut max_consecutive_active_rows = 0;
                            let mut current_consecutive = 0;
                            for row in &all_rows {
                                if is_active_row(row) {
                                    current_consecutive += 1;
                                    if current_consecutive > max_consecutive_active_rows {
                                        max_consecutive_active_rows = current_consecutive;
                                    }
                                } else {
                                    current_consecutive = 0;
                                }
                            }

                            // Empty measures & rests
                            let mut empty_measure_count = 0;
                            let mut consecutive_empty_measures = 0;
                            let mut max_consecutive_empty_measures = 0;

                            for measure in &measures {
                                let has_active = measure.iter().any(|row| is_active_row(row));
                                if !has_active {
                                    empty_measure_count += 1;
                                    consecutive_empty_measures += 1;
                                    if consecutive_empty_measures > max_consecutive_empty_measures {
                                        max_consecutive_empty_measures = consecutive_empty_measures;
                                    }
                                } else {
                                    consecutive_empty_measures = 0;
                                }
                            }
                            let rest_measure_ratio = if measure_count > 0 {
                                empty_measure_count as f64 / measure_count as f64
                            } else {
                                0.0
                            };

                            // Brackets
                            let mut bracket_candidate_count = 0;
                            for row in &all_rows {
                                let mut active_indices = Vec::new();
                                for (i, c) in row.chars().enumerate() {
                                    if c == '1' || c == '2' {
                                        active_indices.push(i);
                                    }
                                }
                                let center_active = active_indices.contains(&2);
                                let diagonal_active = active_indices
                                    .iter()
                                    .any(|&i| i == 0 || i == 1 || i == 3 || i == 4);
                                let is_bracket =
                                    (center_active && diagonal_active && active_indices.len() == 2)
                                        || active_indices.len() >= 3;
                                if is_bracket {
                                    bracket_candidate_count += 1;
                                }
                            }

                            // Twists
                            let mut twist_candidates = 0;
                            let mut total_triplets = 0;
                            let mut single_notes = Vec::new();
                            for row in &all_rows {
                                let active_indices: Vec<usize> = row
                                    .chars()
                                    .enumerate()
                                    .filter(|(_, c)| *c == '1' || *c == '2')
                                    .map(|(i, _)| i)
                                    .collect();
                                if active_indices.len() == 1 {
                                    single_notes.push(active_indices[0]);
                                }
                            }
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
                            let twist_candidate_score = if total_triplets > 0 {
                                twist_candidates as f64 / total_triplets as f64
                            } else {
                                0.0
                            };

                            let notes_per_measure = if measure_count > 0 {
                                (tap_count + hold_start_count) as f64 / measure_count as f64
                            } else {
                                0.0
                            };
                            let active_rows_per_measure = if measure_count > 0 {
                                all_rows.iter().filter(|r| is_active_row(r)).count() as f64
                                    / measure_count as f64
                            } else {
                                0.0
                            };
                            let jumps_per_measure = if measure_count > 0 {
                                jump_count as f64 / measure_count as f64
                            } else {
                                0.0
                            };
                            let holds_per_measure = if measure_count > 0 {
                                hold_start_count as f64 / measure_count as f64
                            } else {
                                0.0
                            };

                            // Stamina
                            let s_density = (active_rows_per_measure / 16.0).min(1.0);
                            let s_stream = (max_consecutive_active_rows as f64 / 64.0).min(1.0);
                            let s_rest = 1.0 - rest_measure_ratio;
                            let stamina_score =
                                (s_density * 0.4 + s_stream * 0.4 + s_rest * 0.2).clamp(0.0, 1.0);

                            // Initial BPM from timing
                            let initial_bpm = timing.bpms.first().map(|e| e.bpm).unwrap_or(120.0);

                            // Local difficulty estimate
                            let base_diff = notes_per_measure * 1.0 + (initial_bpm - 120.0) * 0.05;
                            let tech_modifier = twist_candidate_score * 5.0
                                + (if measure_count > 0 {
                                    jump_count as f64 / measure_count as f64
                                } else {
                                    0.0
                                }) * 2.0
                                + (if measure_count > 0 {
                                    bracket_candidate_count as f64 / measure_count as f64
                                } else {
                                    0.0
                                }) * 1.5;
                            let stamina_modifier = stamina_score * 3.0;
                            let local_difficulty_estimate =
                                (base_diff + tech_modifier + stamina_modifier).clamp(1.0, 28.0);

                            let has_timing_gimmicks_bool = has_stops
                                || has_delays
                                || has_warps
                                || has_speeds
                                || has_scrolls
                                || has_fakes
                                || timing.bpms.len() > 1;

                            // 4-Measure Windows Segmenter
                            let num_windows = measure_count / 4;
                            let mut estimated_stream_windows = 0;

                            for k in 0..num_windows {
                                let start_measure = k * 4;
                                let end_measure = k * 4 + 3;
                                let start_beat = start_measure as f64 * 4.0;
                                let end_beat = (end_measure + 1) as f64 * 4.0;

                                let mut window_rows = Vec::new();
                                for m in start_measure..=end_measure {
                                    for row in &measures[m] {
                                        window_rows.push(row.clone());
                                    }
                                }

                                let w_row_count = window_rows.len();
                                let w_active_row_count =
                                    window_rows.iter().filter(|r| is_active_row(r)).count();

                                if w_active_row_count >= 24 {
                                    estimated_stream_windows += 1;
                                }

                                let mut w_tap_count = 0;
                                let mut w_hold_start_count = 0;
                                let mut w_jump_count = 0;
                                let mut w_triple_count = 0;
                                let mut w_center_note_count = 0;
                                let mut w_bracket_count = 0;

                                for row in &window_rows {
                                    let mut active_in_row = 0;
                                    let mut active_indices = Vec::new();
                                    for (i, c) in row.chars().enumerate() {
                                        if c == '1' {
                                            w_tap_count += 1;
                                            active_in_row += 1;
                                            active_indices.push(i);
                                            if i == 2 {
                                                w_center_note_count += 1;
                                            }
                                        } else if c == '2' {
                                            w_hold_start_count += 1;
                                            active_in_row += 1;
                                            active_indices.push(i);
                                            if i == 2 {
                                                w_center_note_count += 1;
                                            }
                                        }
                                    }
                                    if active_in_row == 2 {
                                        w_jump_count += 1;
                                    } else if active_in_row == 3 {
                                        w_triple_count += 1;
                                    }
                                    let center_active = active_indices.contains(&2);
                                    let diagonal_active = active_indices
                                        .iter()
                                        .any(|&i| i == 0 || i == 1 || i == 3 || i == 4);
                                    let is_bracket = (center_active
                                        && diagonal_active
                                        && active_indices.len() == 2)
                                        || active_indices.len() >= 3;
                                    if is_bracket {
                                        w_bracket_count += 1;
                                    }
                                }

                                let w_empty_row_ratio = if w_row_count > 0 {
                                    (w_row_count - w_active_row_count) as f64 / w_row_count as f64
                                } else {
                                    0.0
                                };

                                let w_stream_score = if w_row_count > 0 {
                                    w_active_row_count as f64 / w_row_count as f64
                                } else {
                                    0.0
                                };

                                // Window level twists
                                let mut w_twist_candidates = 0;
                                let mut w_total_triplets = 0;
                                let mut w_single_notes = Vec::new();
                                for row in &window_rows {
                                    let active_indices: Vec<usize> = row
                                        .chars()
                                        .enumerate()
                                        .filter(|(_, c)| *c == '1' || *c == '2')
                                        .map(|(i, _)| i)
                                        .collect();
                                    if active_indices.len() == 1 {
                                        w_single_notes.push(active_indices[0]);
                                    }
                                }
                                for idx in 2..w_single_notes.len() {
                                    let a = w_single_notes[idx - 2];
                                    let b = w_single_notes[idx - 1];
                                    let c = w_single_notes[idx];
                                    w_total_triplets += 1;
                                    let is_twist =
                                        ((a == 0 || a == 1) && b == 2 && (c == 3 || c == 4))
                                            || ((a == 3 || a == 4) && b == 2 && (c == 0 || c == 1));
                                    if is_twist {
                                        w_twist_candidates += 1;
                                    }
                                }
                                let w_twist_candidate_score = if w_total_triplets > 0 {
                                    w_twist_candidates as f64 / w_total_triplets as f64
                                } else {
                                    0.0
                                };

                                let w_notes_per_measure =
                                    (w_tap_count + w_hold_start_count) as f64 / 4.0;
                                let w_active_rows_per_measure = w_active_row_count as f64 / 4.0;

                                // Window level difficulty
                                let w_base_diff =
                                    w_notes_per_measure * 1.0 + (initial_bpm - 120.0) * 0.05;
                                let w_tech_modifier = w_twist_candidate_score * 5.0
                                    + (w_jump_count as f64 / 4.0) * 2.0
                                    + (w_bracket_count as f64 / 4.0) * 1.5;
                                let w_stamina_modifier = w_stream_score * 3.0;
                                let w_local_difficulty_estimate =
                                    (w_base_diff + w_tech_modifier + w_stamina_modifier)
                                        .clamp(1.0, 28.0);

                                // Normalization & Signature
                                let mut normalized_rows = Vec::new();
                                let mut mirrored_rows = Vec::new();
                                for row in &window_rows {
                                    let mut norm = String::new();
                                    let mut mirr = String::new();
                                    let chars: Vec<char> = row.chars().collect();
                                    for c in &chars {
                                        if *c == '1' || *c == '2' || *c == '4' {
                                            norm.push('1');
                                        } else {
                                            norm.push('0');
                                        }
                                    }
                                    let rev_chars: Vec<char> =
                                        chars.iter().rev().cloned().collect();
                                    for c in &rev_chars {
                                        if *c == '1' || *c == '2' || *c == '4' {
                                            mirr.push('1');
                                        } else {
                                            mirr.push('0');
                                        }
                                    }
                                    normalized_rows.push(norm);
                                    mirrored_rows.push(mirr);
                                }

                                let normalized_str = normalized_rows.join("|");
                                let mirrored_str = mirrored_rows.join("|");

                                let h1 = compute_hash(&normalized_str);
                                let h2 = compute_hash(&mirrored_str);
                                let mirror_invariant_signature =
                                    if h1 < h2 { h1.clone() } else { h2 };

                                // Repeated row motif score
                                let w_active_rows: Vec<&String> =
                                    window_rows.iter().filter(|r| is_active_row(r)).collect();
                                let w_active_count = w_active_rows.len();
                                let w_unique_count = if w_active_count > 0 {
                                    let mut unique = w_active_rows.clone();
                                    unique.sort();
                                    unique.dedup();
                                    unique.len()
                                } else {
                                    0
                                };
                                let w_repeated_row_motif_score = if w_active_count > 0 {
                                    1.0 - (w_unique_count as f64 / w_active_count as f64)
                                } else {
                                    0.0
                                };

                                let window_id = compute_sha256_hex(&format!(
                                    "{}:{}:{}",
                                    chart_id, start_measure, end_measure
                                ));

                                let window_rec = WindowFeatureRecord {
                                    schema_version: "single-window-features.v0".to_string(),
                                    window_id,
                                    song_id: song_id.clone(),
                                    chart_id: chart_id.clone(),
                                    mode: "Single".to_string(),
                                    meter: meter.unwrap_or(0),
                                    window: WindowInfo {
                                        r#type: "measure_4".to_string(),
                                        start_measure,
                                        end_measure,
                                        start_beat,
                                        end_beat,
                                    },
                                    row_count: w_row_count,
                                    active_row_count: w_active_row_count,
                                    tap_count: w_tap_count,
                                    hold_start_count: w_hold_start_count,
                                    jump_count: w_jump_count,
                                    triple_count: w_triple_count,
                                    empty_row_ratio: (w_empty_row_ratio * 100.0).round() / 100.0,
                                    density: WindowDensity {
                                        notes_per_measure: (w_notes_per_measure * 100.0).round()
                                            / 100.0,
                                        active_rows_per_measure: (w_active_rows_per_measure
                                            * 100.0)
                                            .round()
                                            / 100.0,
                                    },
                                    tech_estimates: WindowTechEstimates {
                                        stream_score: (w_stream_score * 100.0).round() / 100.0,
                                        jump_density: (w_jump_count as f64 / 4.0 * 100.0).round()
                                            / 100.0,
                                        center_usage_ratio: if (w_tap_count + w_hold_start_count)
                                            > 0
                                        {
                                            (w_center_note_count as f64
                                                / (w_tap_count + w_hold_start_count) as f64
                                                * 100.0)
                                                .round()
                                                / 100.0
                                        } else {
                                            0.0
                                        },
                                        bracket_candidate_count: w_bracket_count,
                                        twist_candidate_score: (w_twist_candidate_score * 100.0)
                                            .round()
                                            / 100.0,
                                        local_difficulty_estimate: (w_local_difficulty_estimate
                                            * 100.0)
                                            .round()
                                            / 100.0,
                                    },
                                    pattern_summary: PatternSummary {
                                        normalized_signature: h1,
                                        mirror_invariant_signature,
                                        repeated_row_motif_score: (w_repeated_row_motif_score
                                            * 100.0)
                                            .round()
                                            / 100.0,
                                    },
                                    anti_pattern_flags: vec![],
                                    publicability_status: "private_derived".to_string(),
                                };

                                let _ = write_jsonl_record(&mut window_features_file, &window_rec);
                            }

                            // Write Chart Feature Record!
                            let bpms_short = timing.bpms.iter().map(|e| e.bpm).collect::<Vec<_>>();
                            let min_bpm = bpms_short.iter().copied().fold(f64::INFINITY, f64::min);
                            let max_bpm =
                                bpms_short.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                            let min_bpm = if min_bpm.is_infinite() {
                                120.0
                            } else {
                                min_bpm
                            };
                            let max_bpm = if max_bpm.is_infinite() {
                                120.0
                            } else {
                                max_bpm
                            };

                            let chart_rec = ChartFeatureRecord {
                                schema_version: "single-chart-features.v0".to_string(),
                                song_id: song_id.clone(),
                                chart_id: chart_id.clone(),
                                pack: song_folder.pack.clone(),
                                title: title.clone(),
                                artist: artist.clone(),
                                song_type: song_type.clone(),
                                stepstype: stepstype.clone(),
                                mode: mode.to_string(),
                                meter: meter.unwrap_or(0),
                                description: description.clone(),
                                credit: credit.clone(),
                                stepmaker_candidate: credit.clone(),
                                timing_summary: TimingSummaryShort {
                                    initial_bpm,
                                    min_bpm,
                                    max_bpm,
                                    display_bpm: timing.display_bpm.clone(),
                                    offset,
                                    has_timing_gimmicks: has_timing_gimmicks_bool,
                                },
                                measure_count,
                                row_count,
                                active_row_count,
                                empty_row_count: row_count - active_row_count,
                                tap_count,
                                hold_start_count,
                                hold_end_count,
                                jump_count,
                                triple_count,
                                quad_or_more_count,
                                center_note_count,
                                panel_counts,
                                density: DensityMetrics {
                                    notes_per_measure: (notes_per_measure * 100.0).round() / 100.0,
                                    active_rows_per_measure: (active_rows_per_measure * 100.0)
                                        .round()
                                        / 100.0,
                                    jumps_per_measure: (jumps_per_measure * 100.0).round() / 100.0,
                                    holds_per_measure: (holds_per_measure * 100.0).round() / 100.0,
                                },
                                streams: StreamMetrics {
                                    max_consecutive_active_rows,
                                    estimated_stream_windows,
                                },
                                rests: RestMetrics {
                                    empty_measure_count,
                                    max_consecutive_empty_measures,
                                    rest_measure_ratio: (rest_measure_ratio * 100.0).round()
                                        / 100.0,
                                },
                                tech_estimates: TechEstimates {
                                    center_usage_ratio: if (tap_count + hold_start_count) > 0 {
                                        (center_note_count as f64
                                            / (tap_count + hold_start_count) as f64
                                            * 100.0)
                                            .round()
                                            / 100.0
                                    } else {
                                        0.0
                                    },
                                    jump_ratio: if active_row_count > 0 {
                                        (jump_count as f64 / active_row_count as f64 * 100.0)
                                            .round()
                                            / 100.0
                                    } else {
                                        0.0
                                    },
                                    triple_ratio: if active_row_count > 0 {
                                        (triple_count as f64 / active_row_count as f64 * 100.0)
                                            .round()
                                            / 100.0
                                    } else {
                                        0.0
                                    },
                                    bracket_candidate_count,
                                    twist_candidate_score: (twist_candidate_score * 100.0).round()
                                        / 100.0,
                                    stamina_score: (stamina_score * 100.0).round() / 100.0,
                                    local_difficulty_estimate: (local_difficulty_estimate * 100.0)
                                        .round()
                                        / 100.0,
                                },
                                flags: FlagMetrics {
                                    has_mines,
                                    has_unsupported_rows,
                                    has_timing_gimmicks: has_timing_gimmicks_bool,
                                },
                                publicability_status: "private_derived".to_string(),
                            };

                            let _ = write_jsonl_record(&mut chart_features_file, &chart_rec);
                        }
                    }
                    Err(e) => {
                        is_excluded = true;
                        exclusion_reason = Some(format!("Parsing notes block failed: {}", e));

                        let err_rec = ErrorRecord {
                            schema_version: "official-corpus-error.v0".to_string(),
                            severity: "warning".to_string(),
                            scope: "chart".to_string(),
                            song_id: Some(song_id.clone()),
                            chart_id: Some(chart_id.clone()),
                            message: format!("Notes block is corrupt: {}", e),
                            context: serde_json::json!({
                                "relative_song_path": relative_song_path,
                                "error": e.to_string(),
                                "notes_raw_sha256": compute_sha256_hex(&chart.notes_raw),
                                "notes_raw_len": chart.notes_raw.len()
                            }),
                            publicability_status: "private_diagnostic".to_string(),
                        };
                        let _ = write_jsonl_record(&mut errors_file, &err_rec);
                        warnings_count += 1;
                        warnings.push(format!("Chart corrupt: {}", e));
                    }
                }
            }

            if is_excluded {
                charts_excluded += 1;
            }

            catalog_charts.push(ChartCatalogInfo {
                chart_id,
                stepstype,
                mode: mode.to_string(),
                meter: meter.unwrap_or(0),
                description,
                credit,
                stepmaker_candidate: "".to_string(),
                measure_count,
                is_excluded,
                exclusion_reason,
            });
        }

        // Write Catalog Song Record!
        let catalog_rec = SongCatalogRecord {
            schema_version: "official-corpus-catalog.v0".to_string(),
            song_id,
            pack: song_folder.pack.clone(),
            relative_song_path,
            title,
            artist,
            genre,
            song_type,
            music_file,
            banner_file,
            background_file,
            preview_video_file,
            timing,
            charts_total: ssc_doc.charts.len(),
            charts_included: ssc_doc.charts.len()
                - catalog_charts.iter().filter(|c| c.is_excluded).count(),
            charts_excluded: catalog_charts.iter().filter(|c| c.is_excluded).count(),
            charts: catalog_charts,
            parse_status: "ok".to_string(),
            warnings,
            errors,
            source_privacy: "private_official_corpus".to_string(),
            publicability_status: "private_derived".to_string(),
        };

        let _ = write_jsonl_record(&mut catalog_file, &catalog_rec);
    }

    // Flush and finish writing outputs
    drop(catalog_file);
    drop(chart_features_file);
    drop(window_features_file);
    drop(errors_file);

    let run_finished_at = Utc::now().to_rfc3339();
    let duration_seconds = start_time.elapsed().as_secs_f64();
    let manifest = Manifest {
        schema_version: "official-corpus-manifest.v0".to_string(),
        run_started_at,
        run_finished_at,
        duration_seconds: (duration_seconds * 100.0).round() / 100.0,
        corpus_root_kind: "private_local_path_redacted".to_string(),
        output_root_kind: "private_local_path_redacted".to_string(),
        songs_scanned,
        songs_parsed,
        songs_failed,
        charts_total,
        charts_included,
        charts_excluded,
        single_charts_included,
        errors_count,
        warnings_count,
        output_files_written: vec![
            "catalog-index.v0.jsonl".to_string(),
            "single-chart-features.v0.jsonl".to_string(),
            "single-window-features.v0.jsonl".to_string(),
            "errors.v0.jsonl".to_string(),
        ],
        schema_versions: SchemaVersions {
            catalog: "official-corpus-catalog.v0".to_string(),
            chart_features: "single-chart-features.v0".to_string(),
            window_features: "single-window-features.v0".to_string(),
            error: "official-corpus-error.v0".to_string(),
        },
    };

    let manifest_file =
        File::create(&manifest_path).map_err(|e| format!("Cannot create manifest file: {}", e))?;
    if args.pretty {
        serde_json::to_writer_pretty(manifest_file, &manifest)
            .map_err(|e| format!("Failed writing manifest JSON: {}", e))?;
    } else {
        serde_json::to_writer(manifest_file, &manifest)
            .map_err(|e| format!("Failed writing manifest JSON: {}", e))?;
    }

    Ok(manifest)
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Argument parsing error: {}", e);
            print_usage();
            std::process::exit(1);
        }
    };

    match run_factory(&args) {
        Ok(manifest) => {
            println!();
            println!(
                "Execution completed successfully in {:.2}s!",
                manifest.duration_seconds
            );
            println!("  Songs Scanned:          {}", manifest.songs_scanned);
            println!("  Songs Parsed:           {}", manifest.songs_parsed);
            println!("  Songs Failed:           {}", manifest.songs_failed);
            println!("  Charts Total:           {}", manifest.charts_total);
            println!("  Charts Included:        {}", manifest.charts_included);
            println!("  Charts Excluded:        {}", manifest.charts_excluded);
            println!(
                "  Single Charts Included: {}",
                manifest.single_charts_included
            );
            println!("  Errors Count:           {}", manifest.errors_count);
            println!("  Warnings Count:         {}", manifest.warnings_count);
            println!();
            println!("Datasets written to output-root: {:?}", args.output_root);
        }
        Err(e) => {
            eprintln!("Error during factory execution: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bpms() {
        let bpms_str = "0.000=128.000, 4.000=130.000";
        let bpms = parse_bpms(bpms_str);
        assert_eq!(bpms.len(), 2);
        assert_eq!(bpms[0].beat, 0.0);
        assert_eq!(bpms[0].bpm, 128.0);
        assert_eq!(bpms[1].beat, 4.0);
        assert_eq!(bpms[1].bpm, 130.0);
    }

    #[test]
    fn test_parse_notes_block_valid() {
        let notes_raw = "00000\n10001\n02000\n00000\n,\n10000\n00000\n00100\n00000\n;";
        let parsed = parse_notes_block(notes_raw).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].len(), 4);
        assert_eq!(parsed[0][1], "10001");
        assert_eq!(parsed[1].len(), 4);
        assert_eq!(parsed[1][2], "00100");
    }

    #[test]
    fn test_is_active_row() {
        assert!(is_active_row("10001"));
        assert!(is_active_row("00200"));
        assert!(!is_active_row("00000"));
        assert!(!is_active_row("00300")); // hold tail is not active start
    }

    #[test]
    fn test_mock_ssc_filtering_and_feature_extraction() {
        let mock_ssc_content = r#"
#TITLE:Test Song;
#ARTIST:Test Artist;
#GENRE:Test Genre;
#SONGTYPE:ARCADE;
#MUSIC:audio.mp3;
#BANNER:banner.png;
#BACKGROUND:bg.png;
#OFFSET:0.000000;
#BPMS:0.000=120.000;
#STOPS:;
#DELAYS:;
#WARPS:;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#DESCRIPTION:S14;
#METER:14;
#CREDIT:SUNNY;
#NOTES:
10001
02000
00300
00000
,
10101
00000
00000
00000
,
00000
00000
00000
00000
,
00000
00000
00000
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-double;
#DESCRIPTION:D18;
#METER:18;
#CREDIT:EXC;
#NOTES:
0000000000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#DESCRIPTION:S18 UCS;
#METER:18;
#CREDIT:UCS Maker;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#DESCRIPTION:S18 COOP;
#METER:18;
#CREDIT:Coop Maker;
#NOTES:
00000
;
"#;
        let ssc_doc = SscDocument::parse_str(mock_ssc_content);
        assert_eq!(ssc_doc.charts.len(), 4);

        // Verify S14 (pump-single)
        let s14 = &ssc_doc.charts[0];
        let stepstype = get_tag_value(&s14.tags, "STEPSTYPE").unwrap_or_default();
        let description = get_tag_value(&s14.tags, "DESCRIPTION").unwrap_or_default();
        let meter = get_tag_value(&s14.tags, "METER")
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

        assert_eq!(stepstype, "pump-single");
        assert_eq!(description, "S14");
        assert_eq!(meter, 14);

        // Verify exclusions
        for (i, chart) in ssc_doc.charts.iter().enumerate() {
            let st = get_tag_value(&chart.tags, "STEPSTYPE").unwrap_or_default();
            let desc = get_tag_value(&chart.tags, "DESCRIPTION").unwrap_or_default();
            let credit = get_tag_value(&chart.tags, "CREDIT").unwrap_or_default();

            let is_excluded = if st != "pump-single" {
                true
            } else {
                let credit_upper = credit.to_uppercase();
                let desc_upper = desc.to_uppercase();
                credit_upper.contains("UCS")
                    || desc_upper.contains("UCS")
                    || credit_upper.contains("COOP")
                    || desc_upper.contains("COOP")
                    || credit_upper.contains("CO-OP")
                    || desc_upper.contains("CO-OP")
                    || credit_upper.contains("QUEST")
                    || desc_upper.contains("QUEST")
                    || credit_upper.contains("TRAIN")
                    || desc_upper.contains("TRAIN")
            };

            if i == 0 {
                assert!(!is_excluded, "S14 should not be excluded");
            } else {
                assert!(
                    is_excluded,
                    "Chart {} ({} - {}) should be excluded",
                    i, st, desc
                );
            }
        }

        // Feature extraction check on S14
        let measures = parse_notes_block(&s14.notes_raw).unwrap();
        assert_eq!(measures.len(), 4);

        let mut tap_count = 0;
        let mut hold_start_count = 0;
        let mut hold_end_count = 0;
        let mut jump_count = 0;
        let mut triple_count = 0;
        let mut center_note_count = 0;
        let mut panel_counts = [0; 5];
        let mut all_rows = Vec::new();

        for measure in &measures {
            for row in measure {
                all_rows.push(row.clone());
                let mut active_in_row = 0;
                for (i, c) in row.chars().enumerate() {
                    if c == '1' {
                        tap_count += 1;
                        active_in_row += 1;
                        panel_counts[i] += 1;
                        if i == 2 {
                            center_note_count += 1;
                        }
                    } else if c == '2' {
                        hold_start_count += 1;
                        active_in_row += 1;
                        panel_counts[i] += 1;
                        if i == 2 {
                            center_note_count += 1;
                        }
                    } else if c == '3' {
                        hold_end_count += 1;
                    }
                }
                if active_in_row == 2 {
                    jump_count += 1;
                } else if active_in_row == 3 {
                    triple_count += 1;
                }
            }
        }

        assert_eq!(tap_count, 5); // Row 0 has 2 taps, Row 4 has 3 taps
        assert_eq!(hold_start_count, 1); // Row 1 has 1 hold start
        assert_eq!(hold_end_count, 1); // Row 2 has 1 hold end
        assert_eq!(jump_count, 1); // Row 0 is a jump
        assert_eq!(triple_count, 1); // Row 4 is a triple
        assert_eq!(center_note_count, 1); // Only Row 4 has center active
        assert_eq!(panel_counts, [2, 1, 1, 0, 2]);

        // Bracket candidate check
        let mut bracket_candidate_count = 0;
        for row in &all_rows {
            let mut active_indices = Vec::new();
            for (i, c) in row.chars().enumerate() {
                if c == '1' || c == '2' {
                    active_indices.push(i);
                }
            }
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
        assert_eq!(bracket_candidate_count, 1); // Only row 4 (triple) is a bracket candidate here

        // Window extraction check (4 measures window)
        let num_windows = measures.len() / 4;
        assert_eq!(num_windows, 1);

        let k = 0;
        let start_measure = k * 4;
        let end_measure = k * 4 + 3;
        let mut window_rows = Vec::new();
        for m in start_measure..=end_measure {
            for row in &measures[m] {
                window_rows.push(row.clone());
            }
        }
        assert_eq!(window_rows.len(), 16);

        // Normalize rows and verify mirror invariant signature
        let mut normalized_rows = Vec::new();
        let mut mirrored_rows = Vec::new();
        for row in &window_rows {
            let mut norm = String::new();
            let mut mirr = String::new();
            let chars: Vec<char> = row.chars().collect();
            for c in &chars {
                if *c == '1' || *c == '2' || *c == '4' {
                    norm.push('1');
                } else {
                    norm.push('0');
                }
            }
            let rev_chars: Vec<char> = chars.iter().rev().cloned().collect();
            for c in &rev_chars {
                if *c == '1' || *c == '2' || *c == '4' {
                    mirr.push('1');
                } else {
                    mirr.push('0');
                }
            }
            normalized_rows.push(norm);
            mirrored_rows.push(mirr);
        }

        let normalized_str = normalized_rows.join("|");
        let mirrored_str = mirrored_rows.join("|");

        let h1 = compute_hash(&normalized_str);
        let h2 = compute_hash(&mirrored_str);
        let mirror_invariant_signature = if h1 < h2 { h1.clone() } else { h2.clone() };
        assert_eq!(mirror_invariant_signature, h1.min(h2));
    }

    #[test]
    fn test_scan_corpus_synthetic() {
        let temp_dir = std::env::current_dir()
            .unwrap()
            .join(".ai-step-gen-private-datasets")
            .join("test-fixtures");
        fs::create_dir_all(&temp_dir).unwrap();

        let pack_dir = temp_dir.join("01-TESTPACK");
        fs::create_dir_all(&pack_dir).unwrap();

        let song_dir = pack_dir.join("TestSong");
        fs::create_dir_all(&song_dir).unwrap();

        let ssc_file = song_dir.join("TestSong.ssc");
        fs::write(&ssc_file, "#TITLE:Test;").unwrap();

        // Scan the mock corpus
        let scanned = scan_corpus(&temp_dir, None).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].pack, "01-TESTPACK");
        assert_eq!(scanned[0].folder_name, "TestSong");
        assert_eq!(scanned[0].ssc_paths.len(), 1);
        assert_eq!(scanned[0].ssc_paths[0], ssc_file);

        // Clean up
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    fn get_temp_dir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "ai-step-gen-test-{}-{}",
            name,
            rand::random::<u32>()
        ));
        let _ = fs::create_dir_all(&p);
        p
    }

    fn setup_synthetic_song(
        temp_root: &Path,
        pack_name: &str,
        song_name: &str,
        ssc_content: &str,
    ) -> PathBuf {
        let pack_dir = temp_root.join(pack_name);
        let song_dir = pack_dir.join(song_name);
        fs::create_dir_all(&song_dir).unwrap();
        let ssc_path = song_dir.join(format!("{}.ssc", song_name));
        fs::write(&ssc_path, ssc_content).unwrap();
        ssc_path
    }

    #[test]
    fn test_filter_chartname_ucs_excluded() {
        let temp_dir = get_temp_dir("ucs_excluded");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:UCS Song;
#ARTIST:Artist;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#CHARTNAME:Some UCS Chart;
#CREDIT:Maker;
#NOTES:
00000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let manifest = run_factory(&args).unwrap();
        assert_eq!(manifest.charts_excluded, 1);
        assert_eq!(manifest.single_charts_included, 0);

        // Check catalog
        let catalog_content =
            fs::read_to_string(output_dir.join("catalog-index.v0.jsonl")).unwrap();
        let val: serde_json::Value = serde_json::from_str(&catalog_content).unwrap();
        let chart = &val["charts"][0];
        assert_eq!(chart["is_excluded"].as_bool(), Some(true));
        assert_eq!(
            chart["exclusion_reason"].as_str(),
            Some("Excluded UCS chart")
        );

        // Check features
        let features_content =
            fs::read_to_string(output_dir.join("single-chart-features.v0.jsonl")).unwrap();
        assert!(features_content.trim().is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_filter_chartname_quest_train_coop_case_insensitive() {
        let temp_dir = get_temp_dir("quest_train");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:Multi Song;
#ARTIST:Artist;
#BPMS:0.000=120.000;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#CHARTNAME:my Quest chart;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#CREDIT:TRAIN_MAKER;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#DESCRIPTION:co-op play;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#DESCRIPTION:CoOp;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#DESCRIPTION:co op;
#NOTES:
00000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let manifest = run_factory(&args).unwrap();
        assert_eq!(manifest.charts_excluded, 5);

        let catalog_content =
            fs::read_to_string(output_dir.join("catalog-index.v0.jsonl")).unwrap();
        let val: serde_json::Value = serde_json::from_str(&catalog_content).unwrap();
        let charts = val["charts"].as_array().unwrap();
        assert_eq!(
            charts[0]["exclusion_reason"].as_str(),
            Some("Excluded QUEST chart")
        );
        assert_eq!(
            charts[1]["exclusion_reason"].as_str(),
            Some("Excluded TRAIN chart")
        );
        assert_eq!(
            charts[2]["exclusion_reason"].as_str(),
            Some("Excluded COOP chart")
        );
        assert_eq!(
            charts[3]["exclusion_reason"].as_str(),
            Some("Excluded COOP chart")
        );
        assert_eq!(
            charts[4]["exclusion_reason"].as_str(),
            Some("Excluded COOP chart")
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pretty_keeps_jsonl_one_record_per_line() {
        let temp_dir = get_temp_dir("pretty_jsonl");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:Clean Song;
#ARTIST:Artist;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
00000
00000
00000
00000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: true,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let _ = run_factory(&args).unwrap();

        // JSONL files must contain exactly 1 JSON object per line.
        for filename in &[
            "catalog-index.v0.jsonl",
            "single-chart-features.v0.jsonl",
            "errors.v0.jsonl",
        ] {
            let path = output_dir.join(filename);
            let content = fs::read_to_string(&path).unwrap();
            for line in content.lines() {
                if !line.trim().is_empty() {
                    let _: serde_json::Value = serde_json::from_str(line).expect(&format!(
                        "Line in {} is not valid single-line JSON: {}",
                        filename, line
                    ));
                }
            }
        }

        // manifest.v0.json should be multi-line (pretty-printed)
        let manifest_content = fs::read_to_string(output_dir.join("manifest.v0.json")).unwrap();
        assert!(manifest_content.contains('\n'));
        assert!(manifest_content.contains("  "));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_manifest_counts_errors_and_warnings() {
        let temp_dir = get_temp_dir("manifest_counts");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        // 1. Clean song
        let clean_ssc = r#"
#TITLE:Clean Song;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
00000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "CleanSong", clean_ssc);

        // 2. Song with parse error (invalid UTF-8 bytes)
        let corrupt_song_dir = corpus_dir.join("01-PACK").join("CorruptSong");
        fs::create_dir_all(&corrupt_song_dir).unwrap();
        fs::write(
            corrupt_song_dir.join("CorruptSong.ssc"),
            &[0xFF, 0xFE, 0xFD, 0xFC, 0xFB],
        )
        .unwrap();

        // 3. Chart with invalid row length (warning)
        let row_len_ssc = r#"
#TITLE:Row Len Song;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:12;
#NOTES:
000000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "RowLenSong", row_len_ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let manifest = run_factory(&args).unwrap();
        assert_eq!(manifest.songs_parsed, 2);
        assert_eq!(manifest.songs_failed, 1);
        assert_eq!(manifest.errors_count, 1);
        assert_eq!(manifest.warnings_count, 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_invalid_row_length_logs_warning_and_continues() {
        let temp_dir = get_temp_dir("row_len_warning");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:Row Len Song;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
00000
000000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let manifest = run_factory(&args).unwrap();
        assert_eq!(manifest.warnings_count, 1);
        assert_eq!(manifest.charts_excluded, 1);

        let errors_content = fs::read_to_string(output_dir.join("errors.v0.jsonl")).unwrap();
        let val: serde_json::Value = serde_json::from_str(&errors_content).unwrap();
        assert_eq!(val["severity"].as_str(), Some("warning"));
        assert_eq!(val["message"].as_str(), Some("Invalid row length"));
        assert_eq!(val["context"]["measure_index"].as_i64(), Some(0));
        assert_eq!(val["context"]["row_index"].as_i64(), Some(1));
        assert_eq!(val["context"]["actual_len"].as_i64(), Some(6));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_error_records_do_not_expose_absolute_paths_or_raw_notes() {
        let temp_dir = get_temp_dir("privacy_check");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        // Set up one song with fatal parse error, one chart with corrupt notes
        let corrupt_song_dir = corpus_dir.join("01-PACK").join("ParseCorrupt");
        fs::create_dir_all(&corrupt_song_dir).unwrap();
        fs::write(
            corrupt_song_dir.join("ParseCorrupt.ssc"),
            &[0xFF, 0xFE, 0xFD, 0xFC, 0xFB],
        )
        .unwrap();

        let notes_corrupt_ssc = r#"
#TITLE:Notes Corrupt;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
invalid_chars_here
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "NotesCorrupt", notes_corrupt_ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let _ = run_factory(&args).unwrap();

        let errors_content = fs::read_to_string(output_dir.join("errors.v0.jsonl")).unwrap();

        // Assertions for privacy
        assert!(!errors_content.contains("C:\\"));
        assert!(!errors_content.contains("/Users/"));
        assert!(!errors_content.contains("#NOTEDATA"));
        assert!(!errors_content.contains("#TITLE:"));
        assert!(!errors_content.contains("invalid_chars_here"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_single_features_exclude_double_and_special_modes() {
        let temp_dir = get_temp_dir("single_only");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:Song;
#ARTIST:Artist;
#BPMS:0.000=120.000;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-double;
#METER:12;
#NOTES:
0000000000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let manifest = run_factory(&args).unwrap();
        assert_eq!(manifest.charts_total, 2);
        assert_eq!(manifest.charts_included, 1);
        assert_eq!(manifest.charts_excluded, 1);
        assert_eq!(manifest.single_charts_included, 1);

        // Check feature output
        let features_content =
            fs::read_to_string(output_dir.join("single-chart-features.v0.jsonl")).unwrap();
        let lines: Vec<&str> = features_content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(lines.len(), 1);

        let val: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(val["stepstype"].as_str(), Some("pump-single"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_publicability_status_present_on_feature_records_and_error_records() {
        let temp_dir = get_temp_dir("pub_status");
        let corpus_dir = temp_dir.join("corpus");
        let output_dir = temp_dir.join("output");

        let ssc = r#"
#TITLE:Song;
#BPMS:0.000=120.000;
#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:10;
#NOTES:
00000
;

#NOTEDATA:;
#STEPSTYPE:pump-single;
#METER:12;
#NOTES:
000000
;
"#;
        setup_synthetic_song(&corpus_dir, "01-PACK", "Song", ssc);

        let args = AppArgs {
            corpus_root: corpus_dir,
            output_root: output_dir.clone(),
            _mode: "single-v0".to_string(),
            pretty: false,
            limit_songs: None,
            pack_filter: None,
            fail_fast: false,
        };

        let _ = run_factory(&args).unwrap();

        // 1. Catalog publicability
        let catalog_content =
            fs::read_to_string(output_dir.join("catalog-index.v0.jsonl")).unwrap();
        let val_cat: serde_json::Value = serde_json::from_str(&catalog_content).unwrap();
        assert_eq!(
            val_cat["publicability_status"].as_str(),
            Some("private_derived")
        );

        // 2. Features publicability
        let features_content =
            fs::read_to_string(output_dir.join("single-chart-features.v0.jsonl")).unwrap();
        let val_feat: serde_json::Value = serde_json::from_str(&features_content).unwrap();
        assert_eq!(
            val_feat["publicability_status"].as_str(),
            Some("private_derived")
        );

        // 3. Errors publicability
        let errors_content = fs::read_to_string(output_dir.join("errors.v0.jsonl")).unwrap();
        let val_err: serde_json::Value = serde_json::from_str(&errors_content).unwrap();
        assert_eq!(
            val_err["publicability_status"].as_str(),
            Some("private_diagnostic")
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
