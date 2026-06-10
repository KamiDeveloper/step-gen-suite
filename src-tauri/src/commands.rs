use crate::biomechanics::{
    validate_chart, GeminiChartSectionPayload, PlayMode, ValidatedChartSection, ValidationIssue,
    ValidationIssueType, ValidationSeverity,
};
use crate::gemini::{GeminiClient, GeminiWriteMode};
use crate::ssc::parser::{SscChart, SscDocument, SscTag};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartDetails {
    pub steps_type: String,
    pub difficulty: String,
    pub meter: u32,
    pub description: String,
    pub credit: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetStatus {
    pub key: String,         // "audio", "banner", "background", "video"
    pub status_type: String, // "DeclaredAndFound", "DeclaredButMissing", "FoundButNotDeclared", "NotDeclared"
    pub file_name: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SongAssetsStatus {
    pub audio: AssetStatus,
    pub banner: AssetStatus,
    pub background: AssetStatus,
    pub video: AssetStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDetails {
    pub song_id: String,
    pub song_name: String,
    pub artist: String,
    pub bpm: f64,
    pub offset: f64,
    pub ssc_path: String,
    pub audio_path: Option<String>,
    pub banner_path: Option<String>,
    pub background_path: Option<String>,
    pub video_path: Option<String>,
    pub charts: Vec<ChartDetails>,
    pub asset_statuses: SongAssetsStatus,
    pub ssc_bpms: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSongPayload {
    pub target_folder_path: String,
    pub title: String,
    pub artist: String,
    pub genre: String,
    pub credit: String,
    pub song_type: String,
    pub display_bpm: String,
    pub timing_bpm: f64,
    pub offset: f64,
    pub audio_path: String,
    pub banner_path: Option<String>,
    pub background_path: Option<String>,
    pub video_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppendChartResult {
    pub charts: Vec<ChartDetails>,
    pub validation: ValidatedChartSection,
    pub written: bool,
    pub message: String,
    pub generated_notes: Option<String>,
    pub raw_payload: Option<String>,
    pub backup_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FileFingerprint {
    pub file_size: u64,
    pub sha256: String,
    pub modified_time: u64,
}

#[tauri::command]
pub fn get_file_fingerprint(path: String) -> Result<FileFingerprint, String> {
    let file_path = Path::new(&path);
    if !file_path.exists() || !file_path.is_file() {
        return Err(format!(
            "El archivo especificado no existe o no es un archivo válido: {}",
            path
        ));
    }

    if file_path
        .extension()
        .map_or(true, |ext| !ext.eq_ignore_ascii_case("ssc"))
    {
        return Err(
            "Solo se permite calcular el fingerprint de archivos con extensión .ssc".to_string(),
        );
    }

    let metadata = fs::metadata(file_path)
        .map_err(|e| format!("Error al leer los metadatos del archivo: {}", e))?;
    let file_size = metadata.len();

    let modified_time = metadata
        .modified()
        .map_err(|e| format!("Error al obtener la fecha de modificación: {}", e))?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut file = fs::File::open(file_path)
        .map_err(|e| format!("Error al abrir el archivo para calcular el hash: {}", e))?;

    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Error al leer el archivo para calcular el hash: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let sha256 = format!("{:x}", hasher.finalize());

    Ok(FileFingerprint {
        file_size,
        sha256,
        modified_time,
    })
}

fn create_ssc_backup(ssc_path: &Path) -> Result<Option<String>, String> {
    if !ssc_path.exists() {
        return Ok(None);
    }
    let parent_dir = ssc_path
        .parent()
        .ok_or_else(|| "Failed to get parent directory of SSC path".to_string())?;

    // Create the backup directory inside the song's folder
    let backup_dir = parent_dir.join(".ai-step-gen-backups");
    fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {}", e))?;

    // Get the file stem (name without extension)
    let stem = ssc_path
        .file_stem()
        .ok_or_else(|| "Failed to get file stem of SSC path".to_string())?
        .to_string_lossy();

    // Get current Unix timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Initial backup file name: {stem}.{timestamp}.bak.ssc
    let mut backup_file_name = format!("{}.{}.bak.ssc", stem, timestamp);
    let mut backup_path = backup_dir.join(&backup_file_name);

    // Prevent overwriting existing backups
    let mut counter = 1;
    while backup_path.exists() {
        backup_file_name = format!("{}.{}_{}.bak.ssc", stem, timestamp, counter);
        backup_path = backup_dir.join(&backup_file_name);
        counter += 1;
    }

    // Copy original file content to the backup path
    fs::copy(ssc_path, &backup_path).map_err(|e| format!("Failed to copy backup file: {}", e))?;

    Ok(Some(backup_path.to_string_lossy().to_string()))
}

/// Helper atómico interno para validar y escribir un chart en un archivo .ssc.
/// Este helper evita la duplicación de código en los comandos de Tauri.
pub fn append_chart_to_ssc_atomic(
    ssc_path: &Path,
    play_mode: PlayMode,
    target_level: u32,
    notes_raw: &str,
    description: String,
    author: String,
) -> Result<AppendChartResult, String> {
    // Validate metadata fields to prevent tag injection
    validate_metadata_field(&author, "Author/Credit")?;
    validate_metadata_field(&description, "Description")?;

    // 1. Biomechanical validation
    let validation_result = validate_chart(play_mode, target_level, notes_raw);
    let has_errors = validation_result
        .issues
        .iter()
        .any(|i| i.severity == ValidationSeverity::Error);

    if has_errors {
        let current_charts = list_charts(ssc_path.to_string_lossy().to_string())?;
        return Ok(AppendChartResult {
            charts: current_charts,
            validation: validation_result,
            written: false,
            message: "Se abortó la escritura en disco debido a errores de validación biomecánica."
                .to_string(),
            generated_notes: None,
            raw_payload: None,
            backup_path: None,
        });
    }

    // 2. Parse existing ssc
    let mut doc = SscDocument::parse(ssc_path)
        .map_err(|e| format!("Error al parsear el archivo .ssc: {}", e))?;

    let steps_type = match play_mode {
        PlayMode::Single => "pump-single",
        PlayMode::Double => "pump-double",
    };

    // 3. Create chart tags
    let tags = vec![
        SscTag {
            key: Some("NOTEDATA".to_string()),
            value: "".to_string(),
            is_comment: false,
        },
        SscTag {
            key: Some("STEPSTYPE".to_string()),
            value: steps_type.to_string(),
            is_comment: false,
        },
        SscTag {
            key: Some("DESCRIPTION".to_string()),
            value: description,
            is_comment: false,
        },
        SscTag {
            key: Some("DIFFICULTY".to_string()),
            value: "Edit".to_string(),
            is_comment: false,
        },
        SscTag {
            key: Some("METER".to_string()),
            value: target_level.to_string(),
            is_comment: false,
        },
        SscTag {
            key: Some("CREDIT".to_string()),
            value: author,
            is_comment: false,
        },
        SscTag {
            key: Some("NOTES".to_string()),
            value: "".to_string(),
            is_comment: false,
        },
    ];

    let new_chart = SscChart {
        tags,
        notes_raw: notes_raw.to_string(),
    };

    doc.charts.push(new_chart);

    // Create backup before writing
    let backup_path = match create_ssc_backup(ssc_path) {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to create backup: {}", e)),
    };

    // 4. Atomic write
    let dir = ssc_path
        .parent()
        .ok_or("No se pudo obtener el directorio padre del archivo SSC.")?;
    let file_name = ssc_path
        .file_name()
        .ok_or("No se pudo obtener el nombre del archivo SSC.")?;
    let temp_file_name = format!("{}.tmp", file_name.to_string_lossy());
    let temp_path = dir.join(temp_file_name);

    let serialized_doc = doc.serialize_to_string();

    {
        let mut temp_file = fs::File::create(&temp_path)
            .map_err(|e| format!("Error al crear el archivo temporal: {}", e))?;
        temp_file
            .write_all(serialized_doc.as_bytes())
            .map_err(|e| format!("Error al escribir en el archivo temporal: {}", e))?;
        temp_file
            .sync_all()
            .map_err(|e| format!("Error al sincronizar el archivo temporal: {}", e))?;
    }

    fs::rename(&temp_path, ssc_path)
        .map_err(|e| format!("Error al reemplazar el archivo SSC original: {}", e))?;

    let updated_charts = list_charts(ssc_path.to_string_lossy().to_string())?;

    Ok(AppendChartResult {
        charts: updated_charts,
        validation: validation_result,
        written: true,
        message: "Chart añadido y guardado con éxito.".to_string(),
        generated_notes: None,
        raw_payload: None,
        backup_path,
    })
}

use tauri::{AppHandle, Runtime};

pub fn validate_metadata_field(value: &str, field_name: &str) -> Result<(), String> {
    if value.contains(';') {
        return Err(format!(
            "Field '{}' cannot contain semicolons (';').",
            field_name
        ));
    }
    if value.contains('\n') || value.contains('\r') {
        return Err(format!("Field '{}' cannot contain newlines.", field_name));
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(format!(
            "Field '{}' cannot contain control characters.",
            field_name
        ));
    }
    Ok(())
}

pub fn validate_folder_name_rules(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Folder name cannot be empty.".to_string());
    }

    // Windows prohibited characters
    let prohibited = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if trimmed.chars().any(|c| prohibited.contains(&c)) {
        return Err("Folder name contains invalid characters: < > : \" / \\ | ? *".to_string());
    }

    // Control characters (0..31)
    if trimmed.chars().any(|c| c.is_control()) {
        return Err("Folder name contains control characters.".to_string());
    }

    // Windows reserved names
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let upper = trimmed.to_uppercase();
    if reserved.contains(&upper.as_str()) {
        return Err(format!(
            "Folder name '{}' is a Windows reserved name.",
            trimmed
        ));
    }

    // Trailing dot or space
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err("Folder name cannot end with a dot or space.".to_string());
    }

    Ok(trimmed.to_string())
}

pub fn determine_asset_status(
    folder: &Path,
    declared_name: Option<&str>,
    kind: &str,
    files_in_folder: &[(String, PathBuf)],
) -> AssetStatus {
    let declared_val = declared_name.filter(|v| !v.trim().is_empty());

    if let Some(val) = declared_val {
        let resolved_path = folder.join(val);

        let is_safe = if let (Ok(canonical_folder), Ok(canonical_resolved)) =
            (fs::canonicalize(folder), fs::canonicalize(&resolved_path))
        {
            canonical_resolved.starts_with(&canonical_folder)
        } else {
            false
        };

        if is_safe && resolved_path.exists() && resolved_path.is_file() {
            AssetStatus {
                key: kind.to_string(),
                status_type: "DeclaredAndFound".to_string(),
                file_name: Some(val.to_string()),
                file_path: Some(resolved_path.to_string_lossy().to_string()),
            }
        } else {
            AssetStatus {
                key: kind.to_string(),
                status_type: "DeclaredButMissing".to_string(),
                file_name: Some(val.to_string()),
                file_path: None,
            }
        }
    } else {
        // Look for a candidate
        let candidate = match kind {
            "audio" => files_in_folder.iter().find(|(name, _)| {
                name.ends_with(".mp3")
                    || name.ends_with(".ogg")
                    || name.ends_with(".flac")
                    || name.ends_with(".wav")
            }),
            "banner" => files_in_folder.iter().find(|(name, _)| {
                (name.contains("banner")
                    || name.ends_with("bn.png")
                    || name.ends_with("bn.jpg")
                    || name.ends_with("bn.jpeg"))
                    && (name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg"))
            }),
            "background" => files_in_folder.iter().find(|(name, _)| {
                (name.contains("bg") || name.contains("background") || name.contains("back"))
                    && (name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg"))
            }),
            "video" => files_in_folder.iter().find(|(name, _)| {
                name.ends_with(".mp4")
                    || name.ends_with(".mov")
                    || name.ends_with(".avi")
                    || name.ends_with(".mpg")
                    || name.ends_with(".mpeg")
            }),
            _ => None,
        };

        if let Some((name, path)) = candidate {
            AssetStatus {
                key: kind.to_string(),
                status_type: "FoundButNotDeclared".to_string(),
                file_name: Some(name.clone()),
                file_path: Some(path.to_string_lossy().to_string()),
            }
        } else {
            AssetStatus {
                key: kind.to_string(),
                status_type: "NotDeclared".to_string(),
                file_name: None,
                file_path: None,
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub name: String,
    pub extension: String,
    pub size: u64,
}

#[tauri::command]
pub fn get_file_metadata(path: String) -> Result<FileMetadata, String> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_file() {
        return Err("File does not exist or is not a file.".to_string());
    }
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Failed to get file name.".to_string())?
        .to_string();
    let extension = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let size = fs::metadata(p)
        .map(|m| m.len())
        .map_err(|e| format!("Failed to read file size: {}", e))?;

    Ok(FileMetadata {
        name,
        extension,
        size,
    })
}

#[tauri::command]
pub fn validate_folder_name(name: String) -> Result<String, String> {
    validate_folder_name_rules(&name)
}

#[tauri::command]
pub fn check_destination_folder(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Ok("NotExist".to_string());
    }
    if !p.is_dir() {
        return Err("The specified path exists but is not a directory.".to_string());
    }
    // Check if empty
    let is_empty = match fs::read_dir(p) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) => {
            return Err(format!(
                "Failed to read directory: {}. (Check permissions)",
                e
            ))
        }
    };
    if is_empty {
        Ok("ExistEmpty".to_string())
    } else {
        // Check if has ssc
        let has_ssc = fs::read_dir(p).map_or(false, |entries| {
            entries.flatten().any(|e| {
                e.path().is_file()
                    && e.path()
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("ssc"))
            })
        });
        if has_ssc {
            Ok("ExistWithSsc".to_string())
        } else {
            Ok("ExistNotEmpty".to_string())
        }
    }
}

#[tauri::command]
pub fn create_destination_folder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("Failed to create folder: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn select_song_asset_file<R: Runtime>(
    app_handle: AppHandle<R>,
    kind: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let mut picker = app_handle.dialog().file();

    picker = match kind.as_str() {
        "audio" => picker.add_filter("Audio Files", &["mp3", "ogg", "flac", "wav"]),
        "banner" | "background" => picker.add_filter("Image Files", &["png", "jpg", "jpeg"]),
        "video" => picker.add_filter("Video Files", &["mp4", "mov", "avi", "mpg", "mpeg"]),
        _ => return Err(format!("Unsupported asset kind: {}", kind)),
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    picker.pick_file(move |file| {
        if let Some(f) = file {
            let path_result = f.into_path().map(|p| p.to_string_lossy().to_string()).ok();
            let _ = tx.send(path_result);
        } else {
            let _ = tx.send(None);
        }
    });
    rx.await.map_err(|e| format!("Failed to pick file: {}", e))
}

#[tauri::command]
pub async fn select_song_destination_folder<R: Runtime>(
    app_handle: AppHandle<R>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app_handle.dialog().file().pick_folder(move |folder| {
        if let Some(f) = folder {
            let path_result = f.into_path().map(|p| p.to_string_lossy().to_string()).ok();
            let _ = tx.send(path_result);
        } else {
            let _ = tx.send(None);
        }
    });
    rx.await
        .map_err(|e| format!("Failed to pick folder: {}", e))
}

#[tauri::command]
pub async fn create_song_project(payload: CreateSongPayload) -> Result<SongDetails, String> {
    let folder_path = Path::new(&payload.target_folder_path);

    // Validate folder name sanitization
    let folder_name = folder_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid target folder path".to_string())?;

    let _sanitized_name = validate_folder_name_rules(folder_name)?;

    // Validate metadata fields to reject unsafe characters (semicolons, newlines, control characters)
    validate_metadata_field(&payload.title, "Title")?;
    validate_metadata_field(&payload.artist, "Artist")?;
    validate_metadata_field(&payload.genre, "Genre")?;
    validate_metadata_field(&payload.credit, "Credit")?;
    validate_metadata_field(&payload.display_bpm, "Display BPM")?;

    // Validate audio file
    let audio_src = Path::new(&payload.audio_path);
    if !audio_src.exists() || !audio_src.is_file() {
        return Err("Audio file does not exist or is not a file.".to_string());
    }
    let audio_ext = audio_src
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| "Audio file has no extension.".to_string())?
        .to_lowercase();
    if !["mp3", "ogg", "flac", "wav"].contains(&audio_ext.as_str()) {
        return Err("Unsupported audio format. Allowed: .mp3, .ogg, .flac, .wav".to_string());
    }

    // Validate optional assets extensions and physical existence
    if let Some(ref bp) = payload.banner_path {
        let bp_path = Path::new(bp);
        if !bp_path.exists() || !bp_path.is_file() {
            return Err("Banner file does not exist or is not a file.".to_string());
        }
        let ext = bp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["png", "jpg", "jpeg"].contains(&ext.as_str()) {
            return Err("Unsupported banner format. Allowed: .png, .jpg, .jpeg".to_string());
        }
    }
    if let Some(ref bgp) = payload.background_path {
        let bgp_path = Path::new(bgp);
        if !bgp_path.exists() || !bgp_path.is_file() {
            return Err("Background file does not exist or is not a file.".to_string());
        }
        let ext = bgp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["png", "jpg", "jpeg"].contains(&ext.as_str()) {
            return Err("Unsupported background format. Allowed: .png, .jpg, .jpeg".to_string());
        }
    }
    if let Some(ref vp) = payload.video_path {
        let vp_path = Path::new(vp);
        if !vp_path.exists() || !vp_path.is_file() {
            return Err("Video file does not exist or is not a file.".to_string());
        }
        let ext = vp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !["mp4", "mov", "avi", "mpg", "mpeg"].contains(&ext.as_str()) {
            return Err(
                "Unsupported video format. Allowed: .mp4, .mov, .avi, .mpg, .mpeg".to_string(),
            );
        }
    }

    // Validate BPMs
    if payload.timing_bpm < 10.0 || payload.timing_bpm > 1000.0 {
        return Err("Timing BPM must be a reasonable number between 10.0 and 1000.0.".to_string());
    }

    // Validate Song Type
    if !["ARCADE", "SHORTCUT", "REMIX", "FULLSONG"].contains(&payload.song_type.as_str()) {
        return Err(
            "Unsupported song type. Allowed: ARCADE, SHORTCUT, REMIX, FULLSONG".to_string(),
        );
    }

    // Pre-calculate target destinations for collision check
    let audio_dest = folder_path.join(format!("audio.{}", audio_ext));

    let banner_dest = if let Some(ref bp) = payload.banner_path {
        let bp_path = Path::new(bp);
        let ext = bp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();
        Some(folder_path.join(format!("banner.{}", ext)))
    } else {
        None
    };

    let background_dest = if let Some(ref bgp) = payload.background_path {
        let bgp_path = Path::new(bgp);
        let ext = bgp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();
        Some(folder_path.join(format!("background.{}", ext)))
    } else {
        None
    };

    let video_dest = if let Some(ref vp) = payload.video_path {
        let vp_path = Path::new(vp);
        let ext = vp_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_lowercase();
        Some(folder_path.join(format!("video.{}", ext)))
    } else {
        None
    };

    let ssc_name = format!("{}.ssc", folder_name);
    let ssc_path = folder_path.join(&ssc_name);

    // Check directory existence and preflight collision check
    if folder_path.exists() {
        let mut collisions = Vec::new();
        if audio_dest.exists() && audio_src != audio_dest {
            collisions.push(format!("audio.{}", audio_ext));
        }
        if let Some(ref bd) = banner_dest {
            let bp_src = Path::new(payload.banner_path.as_ref().unwrap());
            if bd.exists() && bp_src != bd {
                collisions.push(bd.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        if let Some(ref bgd) = background_dest {
            let bgp_src = Path::new(payload.background_path.as_ref().unwrap());
            if bgd.exists() && bgp_src != bgd {
                collisions.push(bgd.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        if let Some(ref vd) = video_dest {
            let vp_src = Path::new(payload.video_path.as_ref().unwrap());
            if vd.exists() && vp_src != vd {
                collisions.push(vd.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        if ssc_path.exists() {
            collisions.push(ssc_name.clone());
        }

        if !collisions.is_empty() {
            return Err(format!(
                "File collision detected. The following destination files already exist: {}. Overwrite blocked.",
                collisions.join(", ")
            ));
        }
    } else {
        fs::create_dir_all(folder_path)
            .map_err(|e| format!("Failed to create destination folder: {}", e))?;
    }

    // Copy files safely
    let copy_safe = |src_path: &Path, dest_path: &Path| -> Result<(), String> {
        if src_path == dest_path {
            return Ok(());
        }
        fs::copy(src_path, dest_path).map(|_| ()).map_err(|e| {
            format!(
                "Failed to copy file from {:?} to {:?}: {}",
                src_path, dest_path, e
            )
        })
    };

    copy_safe(audio_src, &audio_dest)?;

    if let Some(ref bd) = banner_dest {
        copy_safe(Path::new(payload.banner_path.as_ref().unwrap()), bd)?;
    }
    if let Some(ref bgd) = background_dest {
        copy_safe(Path::new(payload.background_path.as_ref().unwrap()), bgd)?;
    }
    if let Some(ref vd) = video_dest {
        copy_safe(Path::new(payload.video_path.as_ref().unwrap()), vd)?;
    }

    // Create base SSC document
    let audio_filename = audio_dest
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let banner_filename = banner_dest.map(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let background_filename =
        background_dest.map(|p| p.file_name().unwrap().to_string_lossy().to_string());
    let video_filename = video_dest.map(|p| p.file_name().unwrap().to_string_lossy().to_string());

    let ssc_name = format!("{}.ssc", folder_name);
    let ssc_path = folder_path.join(&ssc_name);

    let doc = SscDocument {
        global_tags: vec![
            SscTag {
                key: Some("VERSION".to_string()),
                value: "0.81".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("TITLE".to_string()),
                value: payload.title.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("SUBTITLE".to_string()),
                value: "".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("ARTIST".to_string()),
                value: payload.artist.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("TITLETRANSLIT".to_string()),
                value: "".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SUBTITLETRANSLIT".to_string()),
                value: "".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("ARTISTTRANSLIT".to_string()),
                value: "".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("GENRE".to_string()),
                value: payload.genre.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("ORIGIN".to_string()),
                value: "AI_SUITE".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("CREDIT".to_string()),
                value: payload.credit.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("BANNER".to_string()),
                value: banner_filename.unwrap_or_default(),
                is_comment: false,
            },
            SscTag {
                key: Some("BACKGROUND".to_string()),
                value: background_filename.unwrap_or_default(),
                is_comment: false,
            },
            SscTag {
                key: Some("PREVIEWVID".to_string()),
                value: video_filename.unwrap_or_default(),
                is_comment: false,
            },
            SscTag {
                key: Some("CDTITLE".to_string()),
                value: "".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("MUSIC".to_string()),
                value: audio_filename,
                is_comment: false,
            },
            SscTag {
                key: Some("OFFSET".to_string()),
                value: format!("{:.6}", payload.offset),
                is_comment: false,
            },
            SscTag {
                key: Some("SAMPLESTART".to_string()),
                value: "30.000000".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SAMPLELENGTH".to_string()),
                value: "10.000000".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SELECTABLE".to_string()),
                value: "YES".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SONGTYPE".to_string()),
                value: payload.song_type.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("SONGCATEGORY".to_string()),
                value: "WORLD MUSIC".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("VOLUME".to_string()),
                value: "100".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("DISPLAYBPM".to_string()),
                value: payload.display_bpm.clone(),
                is_comment: false,
            },
            SscTag {
                key: Some("BPMS".to_string()),
                value: format!("0.000={:.3}", payload.timing_bpm),
                is_comment: false,
            },
            SscTag {
                key: Some("TIMESIGNATURES".to_string()),
                value: "0.000=4=4".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("TICKCOUNTS".to_string()),
                value: "0.000=4".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("COMBOS".to_string()),
                value: "0.000=1".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SPEEDS".to_string()),
                value: "0.000=1.000=0.000=0".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("SCROLLS".to_string()),
                value: "0.000=1.000".to_string(),
                is_comment: false,
            },
            SscTag {
                key: Some("LABELS".to_string()),
                value: "0.000=Song Start".to_string(),
                is_comment: false,
            },
        ],
        charts: vec![],
        trailing_comments: vec![],
    };

    let temp_ssc_path = ssc_path.with_extension("ssc.tmp");
    let serialized_doc = doc.serialize_to_string();
    {
        let mut temp_file = fs::File::create(&temp_ssc_path)
            .map_err(|e| format!("Error creating temporary SSC file: {}", e))?;
        temp_file
            .write_all(serialized_doc.as_bytes())
            .map_err(|e| format!("Error writing temporary SSC file: {}", e))?;
        temp_file
            .sync_all()
            .map_err(|e| format!("Error syncing temporary SSC file: {}", e))?;
    }
    fs::rename(&temp_ssc_path, &ssc_path)
        .map_err(|e| format!("Error replacing SSC file: {}", e))?;

    import_song_folder(payload.target_folder_path)
}

#[tauri::command]
pub fn import_song_folder(folder_path: String) -> Result<SongDetails, String> {
    let folder = Path::new(&folder_path);
    if !folder.exists() || !folder.is_dir() {
        return Err(format!(
            "Folder path does not exist or is not a directory: {}",
            folder_path
        ));
    }

    // Find the first .ssc file
    let mut ssc_file_path: Option<PathBuf> = None;
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("ssc"))
            {
                ssc_file_path = Some(path);
                break;
            }
        }
    }

    let ssc_path = match ssc_file_path {
        Some(path) => path,
        None => return Err(format!("No .ssc file found in folder: {}", folder_path)),
    };

    // Parse the .ssc file
    let doc =
        SscDocument::parse(&ssc_path).map_err(|e| format!("Failed to parse .ssc file: {}", e))?;

    // Extract global metadata
    let mut song_name = String::new();
    let mut artist = String::new();
    let mut bpm = 120.0;
    let mut ssc_bpms = Vec::new();
    let mut offset = 0.0;
    let mut music_file = None;
    let mut banner_file = None;
    let mut bg_file = None;
    let mut video_file = None;

    for tag in &doc.global_tags {
        if let Some(key) = &tag.key {
            match key.as_str() {
                "TITLE" => song_name = tag.value.clone(),
                "ARTIST" => artist = tag.value.clone(),
                "MUSIC" => music_file = Some(tag.value.clone()),
                "BANNER" => banner_file = Some(tag.value.clone()),
                "BACKGROUND" => bg_file = Some(tag.value.clone()),
                "PREVIEWVID" => video_file = Some(tag.value.clone()),
                "OFFSET" => {
                    if let Ok(val) = tag.value.parse::<f64>() {
                        offset = val;
                    }
                }
                "BPMS" => {
                    let mut parsed_bpms = Vec::new();
                    for part in tag.value.split(',') {
                        if let Some(val_str) = part.split('=').nth(1) {
                            if let Ok(val) = val_str.trim().parse::<f64>() {
                                parsed_bpms.push(val);
                            }
                        }
                    }
                    if !parsed_bpms.is_empty() {
                        bpm = parsed_bpms[0];
                        ssc_bpms = parsed_bpms;
                    }
                }
                _ => {}
            }
        }
    }

    if ssc_bpms.is_empty() {
        ssc_bpms.push(bpm);
    }

    // Scan physical files in folder for candidate matching
    let mut files_in_folder: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files_in_folder.push((name.to_lowercase(), path));
                }
            }
        }
    }

    let audio_status =
        determine_asset_status(folder, music_file.as_deref(), "audio", &files_in_folder);
    let banner_status =
        determine_asset_status(folder, banner_file.as_deref(), "banner", &files_in_folder);
    let background_status =
        determine_asset_status(folder, bg_file.as_deref(), "background", &files_in_folder);
    let video_status =
        determine_asset_status(folder, video_file.as_deref(), "video", &files_in_folder);

    let asset_statuses = SongAssetsStatus {
        audio: audio_status,
        banner: banner_status,
        background: background_status,
        video: video_status,
    };

    // Resolve file paths
    let audio_path = asset_statuses.audio.file_path.clone();
    let banner_path = asset_statuses.banner.file_path.clone();
    let background_path = asset_statuses.background.file_path.clone();
    let video_path = asset_statuses.video.file_path.clone();

    // Extract charts
    let charts = doc
        .charts
        .iter()
        .map(|chart| {
            let mut steps_type = String::new();
            let mut difficulty = String::new();
            let mut meter = 0;
            let mut description = String::new();
            let mut credit = String::new();

            for tag in &chart.tags {
                if let Some(key) = &tag.key {
                    match key.as_str() {
                        "STEPSTYPE" => steps_type = tag.value.clone(),
                        "DIFFICULTY" => difficulty = tag.value.clone(),
                        "DESCRIPTION" => description = tag.value.clone(),
                        "CREDIT" => credit = tag.value.clone(),
                        "METER" => {
                            if let Ok(val) = tag.value.parse::<u32>() {
                                meter = val;
                            }
                        }
                        _ => {}
                    }
                }
            }

            ChartDetails {
                steps_type,
                difficulty,
                meter,
                description,
                credit,
            }
        })
        .collect();

    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    song_name.hash(&mut hasher);
    let song_id = format!("{:016x}", hasher.finish());

    Ok(SongDetails {
        song_id,
        song_name,
        artist,
        bpm,
        offset,
        ssc_path: ssc_path.to_string_lossy().to_string(),
        audio_path,
        banner_path,
        background_path,
        video_path,
        charts,
        asset_statuses,
        ssc_bpms,
    })
}

#[tauri::command]
pub fn list_charts(ssc_path: String) -> Result<Vec<ChartDetails>, String> {
    let path = Path::new(&ssc_path);
    if !path.exists() || !path.is_file() {
        return Err(format!(
            "SSC file path does not exist or is not a file: {}",
            ssc_path
        ));
    }

    let doc = SscDocument::parse(path).map_err(|e| format!("Failed to parse .ssc file: {}", e))?;

    let charts = doc
        .charts
        .iter()
        .map(|chart| {
            let mut steps_type = String::new();
            let mut difficulty = String::new();
            let mut meter = 0;
            let mut description = String::new();
            let mut credit = String::new();

            for tag in &chart.tags {
                if let Some(key) = &tag.key {
                    match key.as_str() {
                        "STEPSTYPE" => steps_type = tag.value.clone(),
                        "DIFFICULTY" => difficulty = tag.value.clone(),
                        "DESCRIPTION" => description = tag.value.clone(),
                        "CREDIT" => credit = tag.value.clone(),
                        "METER" => {
                            if let Ok(val) = tag.value.parse::<u32>() {
                                meter = val;
                            }
                        }
                        _ => {}
                    }
                }
            }

            ChartDetails {
                steps_type,
                difficulty,
                meter,
                description,
                credit,
            }
        })
        .collect();

    Ok(charts)
}

#[tauri::command]
pub fn validate_chart_notes(
    play_mode: PlayMode,
    difficulty_level: u32,
    notes_raw: String,
) -> ValidatedChartSection {
    validate_chart(play_mode, difficulty_level, &notes_raw)
}

#[tauri::command]
pub fn append_ai_chart_stub(
    ssc_path: String,
    play_mode: String,
    target_level: u32,
    author: String,
) -> Result<AppendChartResult, String> {
    let env_mode = crate::settings::get_app_env();
    if env_mode != "dev" {
        return Err(
            "This command is only available in development mode (AI_STEP_GEN_ENV=dev).".to_string(),
        );
    }

    let path = Path::new(&ssc_path);
    if !path.exists() || !path.is_file() {
        return Err(format!(
            "SSC file path does not exist or is not a file: {}",
            ssc_path
        ));
    }

    let mode = match play_mode.as_str() {
        "Single" => PlayMode::Single,
        "Double" => PlayMode::Double,
        _ => return Err(format!("Unsupported play mode: {}", play_mode)),
    };

    let notes_raw = match mode {
        PlayMode::Single => concat!(
            "00000\n00000\n00000\n00000\n",
            ",\n",
            "10000\n00100\n00001\n00100\n",
            ",\n",
            "01000\n00100\n00010\n00100\n",
            ",\n",
            "00000\n00000\n00000\n00000\n",
            ";"
        )
        .to_string(),
        PlayMode::Double => concat!(
            "0000000000\n0000000000\n0000000000\n0000000000\n",
            ",\n",
            "1000000000\n0010000000\n0000100000\n0000000100\n",
            ",\n",
            "0000000001\n0000000100\n0000010000\n0000000100\n",
            ",\n",
            "0000000000\n0000000000\n0000000000\n0000000000\n",
            ";"
        )
        .to_string(),
    };

    let description = format!(
        "Local Test {}",
        match mode {
            PlayMode::Single => format!("S{}", target_level),
            PlayMode::Double => format!("D{}", target_level),
        }
    );

    append_chart_to_ssc_atomic(path, mode, target_level, &notes_raw, description, author)
}

#[tauri::command]
pub fn append_mock_gemini_payload(
    ssc_path: String,
    payload_json: String,
    author: String,
) -> Result<AppendChartResult, String> {
    let env_mode = crate::settings::get_app_env();
    if env_mode != "dev" {
        return Err(
            "This command is only available in development mode (AI_STEP_GEN_ENV=dev).".to_string(),
        );
    }

    let path = Path::new(&ssc_path);
    if !path.exists() || !path.is_file() {
        return Err(format!(
            "SSC file path does not exist or is not a file: {}",
            ssc_path
        ));
    }

    // 1. Parse payload JSON
    let payload: GeminiChartSectionPayload = serde_json::from_str(&payload_json)
        .map_err(|e| format!("Error parsing payload JSON: {}", e))?;

    // 2. Structural validation
    match payload.validate_structure() {
        Ok(()) => {
            // Convert to SSC notes
            let notes_raw = payload.to_ssc_notes();

            let description = format!(
                "AI Mock {} {}",
                payload.section_id,
                match payload.play_mode {
                    PlayMode::Single => format!("S{}", payload.difficulty_level),
                    PlayMode::Double => format!("D{}", payload.difficulty_level),
                }
            );

            append_chart_to_ssc_atomic(
                path,
                payload.play_mode,
                payload.difficulty_level,
                &notes_raw,
                description,
                author,
            )
        }
        Err(err_msg) => {
            // Build the ValidatedChartSection for the structural error
            let validation_result = ValidatedChartSection {
                play_mode: payload.play_mode,
                difficulty_level: payload.difficulty_level,
                issues: vec![ValidationIssue {
                    measure_index: 0,
                    row_index: 0,
                    severity: ValidationSeverity::Error,
                    issue_type: ValidationIssueType::InvalidGeminiStructure,
                    message: err_msg,
                }],
            };

            let current_charts = list_charts(ssc_path)?;
            Ok(AppendChartResult {
                charts: current_charts,
                validation: validation_result,
                written: false,
                message: "Aborted: Gemini payload failed structural validation.".to_string(),
                generated_notes: None,
                raw_payload: None,
                backup_path: None,
            })
        }
    }
}

pub fn sanitize_gemini_json_payload(raw: &str) -> String {
    let raw_trimmed = raw.trim();

    // Check if the response contains markdown code block fences
    if let Some(json_start) = raw_trimmed.find("```json") {
        let after_fence = &raw_trimmed[json_start + 7..];
        if let Some(fence_end) = after_fence.find("```") {
            return after_fence[..fence_end].trim().to_string();
        }
    } else if let Some(code_start) = raw_trimmed.find("```") {
        let after_fence = &raw_trimmed[code_start + 3..];
        if let Some(fence_end) = after_fence.find("```") {
            return after_fence[..fence_end].trim().to_string();
        }
    }

    raw_trimmed.to_string()
}

#[tauri::command]
pub fn append_approved_gemini_payload(
    ssc_path: String,
    payload_json: String,
    author: String,
    expected_sha256: String,
) -> Result<AppendChartResult, String> {
    let path = Path::new(&ssc_path);
    if !path.exists() || !path.is_file() {
        return Err(format!(
            "SSC file path does not exist or is not a file: {}",
            ssc_path
        ));
    }

    // 1. Parse payload JSON
    let clean_json = sanitize_gemini_json_payload(&payload_json);
    let payload: GeminiChartSectionPayload = serde_json::from_str(&clean_json)
        .map_err(|e| format!("Error parsing approved payload JSON: {}", e))?;

    // 2. Structural validation
    payload
        .validate_structure()
        .map_err(|e| format!("Approved payload structural validation failed: {}", e))?;

    // 3. Biomechanical validation (but write only if no severe errors)
    let notes_raw = payload.to_ssc_notes();
    let validation_result = validate_chart(payload.play_mode, payload.difficulty_level, &notes_raw);
    let has_errors = validation_result
        .issues
        .iter()
        .any(|i| i.severity == ValidationSeverity::Error);

    if has_errors {
        return Err(
            "Cannot commit chart: Biomechanical validation detected severe errors.".to_string(),
        );
    }

    // 4. Formulate the production description without "Mock"
    let description = format!(
        "AI {} {}",
        payload.section_id,
        match payload.play_mode {
            PlayMode::Single => format!("S{}", payload.difficulty_level),
            PlayMode::Double => format!("D{}", payload.difficulty_level),
        }
    );

    // 5. Fingerprint verification right before writing
    let current_fp = get_file_fingerprint(ssc_path.clone())
        .map_err(|e| format!("Error al calcular el fingerprint del archivo: {}", e))?;
    if current_fp.sha256 != expected_sha256 {
        return Err(format!(
            "El fingerprint del archivo .ssc ha cambiado desde el preview. Esperado: {}, Actual: {}",
            expected_sha256, current_fp.sha256
        ));
    }

    // 6. Append to SSC (which will automatically handle backups)
    append_chart_to_ssc_atomic(
        path,
        payload.play_mode,
        payload.difficulty_level,
        &notes_raw,
        description,
        author,
    )
}

pub async fn generate_gemini_chart_preview_core(
    api_key: &str,
    ssc_path: &str,
    audio_path: &str,
    play_mode: PlayMode,
    target_level: u32,
    section_id: &str,
    _author: &str,
    client: &GeminiClient,
    start_measure: Option<u32>,
    end_measure: Option<u32>,
    song_type: Option<String>,
) -> Result<AppendChartResult, String> {
    // 1. Check environment variable gate
    if !crate::settings::is_gemini_enabled() {
        return Err("La integración real con Gemini está deshabilitada en esta sesión. Configure la variable de entorno AI_STEP_GEN_ENABLE_REAL_GEMINI=1 para habilitarla.".to_string());
    }

    let mut start_m = start_measure;
    let mut end_m = end_measure;
    let mut s_type = song_type.clone();

    let mut intent_info = String::new();
    let mut report_opt = None;
    let ssc_p = Path::new(ssc_path);
    if let Some(dir) = ssc_p.parent() {
        let report_file = dir
            .join(".ai-step-gen-analysis")
            .join("song-analysis-report.v1.json");
        if report_file.exists() && report_file.is_file() {
            if let Ok(report_content) = fs::read_to_string(&report_file) {
                if let Ok(report) = serde_json::from_str::<crate::music_analysis::SongAnalysisReport>(
                    &report_content,
                ) {
                    report_opt = Some(report);
                }
            }
        }
    }

    if let Some(ref report) = report_opt {
        // If start/end measures are not specified, try to find them in the section boundaries
        if start_m.is_none() || end_m.is_none() {
            if let Some(sec) = report.sections.iter().find(|s| s.section_id == section_id) {
                if start_m.is_none() {
                    start_m = Some(sec.start_measure);
                }
                if end_m.is_none() {
                    end_m = Some(sec.end_measure);
                }
            }
        }
        // If song_type is not specified, get it from timing_grid
        if s_type.is_none() {
            s_type = Some(report.timing_grid.song_type.clone());
        }

        // Find matching intent map by section_id
        let matched_intent = report
            .choreographic_intent
            .iter()
            .find(|intent| intent.section_id == section_id);

        if let Some(intent) = matched_intent {
            intent_info = format!(
                "\n[INFORMACIÓN COREOGRÁFICA DE LA CANCIÓN (Music Analysis Engine)]\n\
                 - Objetivo de Densidad: {}\n\
                 - Presupuesto de Dificultad (Difficulty Budget): {}\n\
                 - Familias de Patrones Recomendados: {:?}\n\
                 - Familias de Patrones a Evitar: {:?}\n\
                 - Estrategia de Motivos: {}\n\
                 - Evidencia de Análisis: {:?}\n",
                intent.density_target,
                intent.difficulty_budget,
                intent.recommended_pattern_families,
                intent.avoid_pattern_families,
                intent.motif_strategy,
                intent.evidence
            );
        }
    }

    let start_m_val = start_m.unwrap_or(0);
    let end_m_val = end_m.unwrap_or(7);
    if end_m_val < start_m_val {
        return Err(format!(
            "El compás de fin ({}) no puede ser menor que el compás de inicio ({})",
            end_m_val, start_m_val
        ));
    }
    let num_measures = end_m_val
        .checked_sub(start_m_val)
        .and_then(|d| d.checked_add(1))
        .ok_or_else(|| "Measure range calculation overflowed".to_string())?;
    let s_type_val = s_type.unwrap_or_else(|| "Arcade".to_string());

    let play_mode_name = match play_mode {
        PlayMode::Single => "Single (5 flechas)",
        PlayMode::Double => "Double (10 flechas)",
    };

    // 4. Construct prompt
    let prompt_text = format!(
        "Eres un stepmaker experto de Pump It Up. Genera un chart coreográfico de Pump It Up para la sección '{}' de la canción. \
         Detalles de la Canción y Sección:\n\
         - Modo de Juego: {}\n\
         - Nivel de Dificultad Solicitado: {}\n\
         - Rango de Compases (Measures): {} a {} (ambos inclusive)\n\
         - Tipo de Canción: {}\n\
         {}\n\
         [CONTRATO Y FORMATO DE RESPUESTA]\n\
         Debes responder EXCLUSIVAMENTE en formato JSON, sin bloques de código markdown (como ```json) y sin explicaciones. El esquema JSON debe ser exactamente:\n\
         {{\n\
           \"section_id\": \"{}\",\n\
           \"difficulty_level\": {},\n\
           \"play_mode\": \"{:?}\",\n\
           \"biomechanical_state\": {{\n\
             \"current_twist_debt\": 0.0,\n\
             \"current_stamina_debt\": 0.0,\n\
             \"last_left_foot_lane\": null,\n\
             \"last_right_foot_lane\": null\n\
           }},\n\
           \"measures\": [\n\
             {{\n\
               \"measure_index\": {},\n\
               \"subdivision\": 4, \n\
               \"rows\": [\"10000\", \"01000\", \"00100\", \"00010\"]\n\
             }}\n\
           ]\n\
         }}\n\
         \n\
         [REGLAS BIOMECÁNICAS Y TÉCNICAS]\n\
         1. Caracteres permitidos: Solo '0' (vacío), '1' (tap), '2' (inicio de hold/congelador), '3' (fin de hold/congelador). Queda ESTRICTAMENTE PROHIBIDO el uso de minas ('M').\n\
         2. Longitud de fila: Para Single, cada fila debe medir exactamente 5 caracteres (esquina inferior-izquierda, esquina superior-izquierda, centro-amarillo, esquina superior-derecha, esquina inferior-derecha). Para Double, debe medir exactamente 10 caracteres.\n\
         3. Subdivisiones: Solo se permiten subdivisiones de 4, 8, 16 o 32 filas por compás. La cantidad de filas en 'rows' debe coincidir EXACTAMENTE con el valor de 'subdivision'.\n\
         4. Jumps (Saltos): Un jump es presionar 2 o más flechas a la vez. No coloques jumps consecutivos a alta velocidad.\n\
         5. Triple Taps: En niveles < 16, evita triples. En niveles 16+ se permiten como Brackets (puntero-talón).\n\
         6. Alternancia de pies: Evita double-steps rápidos (Jack rápido) en la misma flecha consecutivamente en streams de subdivision 16+.\n\
         7. Giros y torso (Twists): Si el chart exige cruces que rotan el torso, asegúrate de proveer un contra-giro (untwist) de retorno inmediato para neutralizar la cadera.\n\
         \n\
         Genera el array de 'measures' de tamaño exactamente {} (una por cada compás desde {} hasta {}).",
        section_id, play_mode_name, target_level, start_m_val, end_m_val, s_type_val, intent_info,
        section_id, target_level, play_mode, start_m_val,
        num_measures, start_m_val, end_m_val
    );

    // 5. Invoke Gemini API
    let audio_file_path = Path::new(audio_path);
    if !audio_file_path.exists() || !audio_file_path.is_file() {
        return Err(format!(
            "Audio file path does not exist or is not a file: {}",
            audio_path
        ));
    }

    let raw_response = client
        .process_audio_and_generate(api_key, audio_file_path, &prompt_text)
        .await?;

    // Strip markdown formatting if Gemini included it
    let clean_response = sanitize_gemini_json_payload(&raw_response);

    // 6. Parse response JSON
    let payload: GeminiChartSectionPayload =
        serde_json::from_str(&clean_response).map_err(|e| {
            let error_summary = e.to_string();
            let length = clean_response.len();
            format!(
                "Gemini returned invalid JSON: {}. Response length: {} bytes.",
                error_summary, length
            )
        })?;

    // Validate that the response matches the requested parameters
    let mut mismatch_issues = Vec::new();
    if payload.section_id != section_id {
        mismatch_issues.push(ValidationIssue {
            measure_index: 0,
            row_index: 0,
            severity: ValidationSeverity::Error,
            issue_type: ValidationIssueType::InvalidGeminiStructure,
            message: format!(
                "El ID de sección en la respuesta ({}) no coincide con el solicitado ({}).",
                payload.section_id, section_id
            ),
        });
    }
    if payload.play_mode != play_mode {
        mismatch_issues.push(ValidationIssue {
            measure_index: 0,
            row_index: 0,
            severity: ValidationSeverity::Error,
            issue_type: ValidationIssueType::InvalidGeminiStructure,
            message: format!(
                "El modo de juego en la respuesta ({:?}) no coincide con el solicitado ({:?}).",
                payload.play_mode, play_mode
            ),
        });
    }
    if payload.difficulty_level != target_level {
        mismatch_issues.push(ValidationIssue {
            measure_index: 0,
            row_index: 0,
            severity: ValidationSeverity::Error,
            issue_type: ValidationIssueType::InvalidGeminiStructure,
            message: format!(
                "El nivel de dificultad en la respuesta ({}) no coincide con el solicitado ({}).",
                payload.difficulty_level, target_level
            ),
        });
    }

    let actual_count = payload.measures.len();
    if actual_count != num_measures as usize {
        mismatch_issues.push(ValidationIssue {
            measure_index: 0,
            row_index: 0,
            severity: ValidationSeverity::Error,
            issue_type: ValidationIssueType::InvalidGeminiStructure,
            message: format!(
                "Gemini returned {} measures, expected {} for requested range {}-{}.",
                actual_count, num_measures, start_m_val, end_m_val
            ),
        });
    }

    // Validate that each measure index matches the expected absolute index in sequence
    for (i, measure) in payload.measures.iter().enumerate() {
        let expected_index = start_m_val + i as u32;
        if measure.measure_index != expected_index {
            mismatch_issues.push(ValidationIssue {
                measure_index: measure.measure_index as usize,
                row_index: 0,
                severity: ValidationSeverity::Error,
                issue_type: ValidationIssueType::InvalidGeminiStructure,
                message: format!(
                    "La medida en la posición {} tiene un índice de compás incorrecto ({}). Se esperaba el índice absoluto {}.",
                    i, measure.measure_index, expected_index
                ),
            });
        }
    }

    if !mismatch_issues.is_empty() {
        let validation_result = ValidatedChartSection {
            play_mode,
            difficulty_level: target_level,
            issues: mismatch_issues,
        };
        let current_charts = list_charts(ssc_path.to_string())?;
        return Ok(AppendChartResult {
            charts: current_charts,
            validation: validation_result,
            written: false,
            message: "Aborted: Gemini payload mismatch with requested parameters.".to_string(),
            generated_notes: None,
            raw_payload: Some(clean_response),
            backup_path: None,
        });
    }

    // 7. Validate structure
    let validation_result = match payload.validate_structure() {
        Ok(()) => {
            // Convert to SSC notes
            let notes_raw = payload.to_ssc_notes();

            // Validate and return preview only
            validate_chart(payload.play_mode, payload.difficulty_level, &notes_raw)
        }
        Err(err_msg) => {
            // Structural validation failed
            ValidatedChartSection {
                play_mode: payload.play_mode,
                difficulty_level: payload.difficulty_level,
                issues: vec![ValidationIssue {
                    measure_index: 0,
                    row_index: 0,
                    severity: ValidationSeverity::Error,
                    issue_type: ValidationIssueType::InvalidGeminiStructure,
                    message: err_msg,
                }],
            }
        }
    };

    let current_charts = list_charts(ssc_path.to_string())?;
    let written = false;

    // Determine semantic message based on errors/warnings in PreviewOnly
    let has_errors = validation_result
        .issues
        .iter()
        .any(|i| i.severity == ValidationSeverity::Error);
    let has_warnings = validation_result
        .issues
        .iter()
        .any(|i| i.severity == ValidationSeverity::Warning);

    let message = if has_errors {
        "preview generado pero inválido; no se escribió en disco".to_string()
    } else if has_warnings {
        "preview generado con advertencias".to_string()
    } else {
        "Preview content generated and validated successfully without writing to disk.".to_string()
    };

    Ok(AppendChartResult {
        charts: current_charts,
        validation: validation_result,
        written,
        message,
        generated_notes: Some(payload.to_ssc_notes()),
        raw_payload: Some(clean_response),
        backup_path: None,
    })
}

fn validate_preview_write_mode(write_mode: &str) -> Result<GeminiWriteMode, String> {
    match write_mode {
        "PreviewOnly" => Ok(GeminiWriteMode::PreviewOnly),
        _ => Err(format!(
            "generate_gemini_chart_preview only supports 'PreviewOnly' write mode. Got: {}",
            write_mode
        )),
    }
}

#[tauri::command]
pub async fn generate_gemini_chart_preview<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    ssc_path: String,
    audio_path: String,
    passphrase: String,
    play_mode: String,
    target_level: u32,
    section_id: String,
    author: String,
    write_mode: String,
    start_measure: Option<u32>,
    end_measure: Option<u32>,
    song_type: Option<String>,
) -> Result<AppendChartResult, String> {
    let play_mode_enum = match play_mode.as_str() {
        "Single" => PlayMode::Single,
        "Double" => PlayMode::Double,
        _ => return Err(format!("Unsupported play mode: {}", play_mode)),
    };

    let _w_mode = validate_preview_write_mode(&write_mode)?;

    // 2. Decrypt stored API key
    let api_key =
        crate::credentials::decrypt_stored_api_key(&app_handle, &passphrase).map_err(|e| {
            format!(
                "Error al descifrar la API Key (verifique la contraseña): {}",
                e
            )
        })?;

    // 3. Initialize Gemini Client
    #[cfg(test)]
    let client = {
        let base_url = std::env::var("AI_STEP_GEN_MOCK_BASE_URL").ok();
        GeminiClient::new(base_url)
    };
    #[cfg(not(test))]
    let client = GeminiClient::new();

    generate_gemini_chart_preview_core(
        &api_key,
        &ssc_path,
        &audio_path,
        play_mode_enum,
        target_level,
        &section_id,
        &author,
        &client,
        start_measure,
        end_measure,
        song_type,
    )
    .await
}

#[tauri::command]
pub async fn read_audio_file(path: String) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let raw_path_lower = path.to_lowercase();
    let blocked_directories = [
        "\\.ssh\\",
        "\\.aws\\",
        "\\.git\\",
        "\\.gemini\\",
        "\\appdata\\",
        "\\windows\\",
        "\\system32\\",
        "\\etc\\",
    ];
    for blocked in &blocked_directories {
        if raw_path_lower.contains(blocked) {
            return Err(
                "Acceso denegado: Ruta de archivo no permitida por razones de seguridad."
                    .to_string(),
            );
        }
    }

    let p = std::path::Path::new(&path);
    if !p.exists() || !p.is_file() {
        return Err("El archivo de audio especificado no existe o no es un archivo.".to_string());
    }

    // Canonicalize path to resolve symlinks / traversal
    let canonical = p
        .canonicalize()
        .map_err(|e| format!("Error de canonicalización: {}", e))?;
    let path_str = canonical.to_string_lossy().to_lowercase();

    for blocked in &blocked_directories {
        if path_str.contains(blocked) {
            return Err(
                "Acceso denegado: Ruta de archivo no permitida por razones de seguridad."
                    .to_string(),
            );
        }
    }

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !["mp3", "ogg", "flac", "wav"].contains(&ext.as_str()) {
        return Err("Formato de audio no soportado por el pre-análisis.".to_string());
    }

    let metadata = std::fs::metadata(p).map_err(|e| format!("Error al leer metadatos: {}", e))?;
    let size = metadata.len();
    if size > 100 * 1024 * 1024 {
        return Err("El tamaño del archivo de audio supera el límite máximo de 100 MB para el análisis en navegador.".to_string());
    }

    // Verify magic bytes / headers before reading full file
    let mut file =
        std::fs::File::open(p).map_err(|e| format!("Error al abrir archivo de audio: {}", e))?;
    let mut header = [0u8; 12];
    let bytes_read = file
        .read(&mut header)
        .map_err(|e| format!("Error al leer cabecera: {}", e))?;
    if bytes_read < 4 {
        return Err("Archivo demasiado pequeño para ser un archivo de audio válido.".to_string());
    }

    let mut valid = false;
    // MP3 (starts with "ID3" or sync word 0xFFE0 / 0xFFF0)
    if header.starts_with(b"ID3") {
        valid = true;
    } else if header[0] == 0xFF && (header[1] & 0xE0) == 0xE0 {
        valid = true;
    }
    // OGG ("OggS")
    else if &header[0..4] == b"OggS" {
        valid = true;
    }
    // FLAC ("fLaC")
    else if &header[0..4] == b"fLaC" {
        valid = true;
    }
    // WAV ("RIFF" ... "WAVE")
    else if &header[0..4] == b"RIFF" {
        if bytes_read >= 12 && &header[8..12] == b"WAVE" {
            valid = true;
        }
    }

    if !valid {
        return Err("Acceso denegado: Los bytes mágicos del archivo no corresponden a un formato de audio soportado (MP3, OGG, FLAC, WAV).".to_string());
    }

    std::fs::read(p).map_err(|e| format!("Error al leer el archivo de audio: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::path::PathBuf;

    fn get_fixture_path() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("src");
        p.push("ssc");
        p.push("test_fixtures");
        p.push("mini_sample.ssc");
        p
    }

    #[test]
    fn test_get_file_fingerprint_success_and_failure() {
        let fixture_path = get_fixture_path();
        assert!(fixture_path.exists());

        // 1. Valid file fingerprint
        let fp_result = get_file_fingerprint(fixture_path.to_string_lossy().to_string());
        assert!(fp_result.is_ok());
        let fp = fp_result.unwrap();
        assert!(fp.file_size > 0);
        assert!(!fp.sha256.is_empty());
        assert_eq!(fp.sha256.len(), 64); // SHA-256 length in hex
        assert!(fp.modified_time > 0);

        // 2. Non-existent file fingerprint
        let fake_path = "/nonexistent/path/to/some/file.ssc".to_string();
        let fp_result_err = get_file_fingerprint(fake_path);
        assert!(fp_result_err.is_err());
        assert!(fp_result_err.unwrap_err().contains("no existe"));
    }

    #[test]
    fn test_append_ai_chart_stub_single() {
        crate::settings::set_test_env(Some("dev"));
        let original_path = get_fixture_path();
        assert!(original_path.exists());

        // Create a temporary file path
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_mini_single.ssc");

        // Copy original file to temp path
        std::fs::copy(&original_path, &temp_ssc_path)
            .expect("Failed to copy Mini Sample ssc to temp");

        // Parse original to get original chart count
        let original_doc = SscDocument::parse(&temp_ssc_path).expect("Failed to parse copied ssc");
        let original_count = original_doc.charts.len();

        // Append chart stub
        let result = append_ai_chart_stub(
            temp_ssc_path.to_string_lossy().to_string(),
            "Single".to_string(),
            18,
            "AI Test Author".to_string(),
        )
        .expect("Failed to append AI chart stub");

        assert!(result.written);
        let updated_charts = result.charts;

        // Assert count incremented
        assert_eq!(updated_charts.len(), original_count + 1);

        // Verify the appended chart details
        let new_chart = &updated_charts[original_count];
        assert_eq!(new_chart.steps_type, "pump-single");
        assert_eq!(new_chart.meter, 18);
        assert_eq!(new_chart.credit, "AI Test Author");
        assert_eq!(new_chart.difficulty, "Edit");
        assert_eq!(new_chart.description, "Local Test S18");

        // Clean up
        crate::settings::set_test_env(None);
        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_append_ai_chart_stub_double_preserves_existing() {
        crate::settings::set_test_env(Some("dev"));
        let original_path = get_fixture_path();
        assert!(original_path.exists());

        // Create a temporary file path
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_mini_double.ssc");

        // Copy original file to temp path
        std::fs::copy(&original_path, &temp_ssc_path)
            .expect("Failed to copy Mini Sample ssc to temp");

        // Parse original to compare charts
        let original_doc = SscDocument::parse(&temp_ssc_path).expect("Failed to parse copied ssc");
        let original_count = original_doc.charts.len();

        // Append double chart stub
        let result = append_ai_chart_stub(
            temp_ssc_path.to_string_lossy().to_string(),
            "Double".to_string(),
            12, // Level under 16 limit for double
            "AI Test Author 2".to_string(),
        )
        .expect("Failed to append AI chart stub");

        assert!(result.written);
        let updated_charts = result.charts;
        assert_eq!(updated_charts.len(), original_count + 1);

        // Parse back the written document
        let final_doc =
            SscDocument::parse(&temp_ssc_path).expect("Failed to parse final written ssc");

        // Verify original charts remain completely identical
        for i in 0..original_count {
            assert_eq!(
                final_doc.charts[i].notes_raw,
                original_doc.charts[i].notes_raw
            );
            assert_eq!(
                final_doc.charts[i]
                    .tags
                    .iter()
                    .filter(|t| !t.is_comment)
                    .collect::<Vec<_>>(),
                original_doc.charts[i]
                    .tags
                    .iter()
                    .filter(|t| !t.is_comment)
                    .collect::<Vec<_>>()
            );
        }

        // Verify the last chart (appended one) has correct double column format (10 characters per note line)
        let final_new_chart = &final_doc.charts[original_count];
        let rows: Vec<&str> = final_new_chart
            .notes_raw
            .lines()
            .map(|l| l.trim())
            .collect();
        for row in rows {
            if row == "," || row == ";" || row.starts_with("//") || row.is_empty() {
                continue;
            }
            assert_eq!(
                row.len(),
                10,
                "Row must have exactly 10 characters for Double chart: {}",
                row
            );
        }

        // Clean up
        crate::settings::set_test_env(None);
        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_mock_gemini_payload_valid_and_invalid() {
        crate::settings::set_test_env(Some("dev"));
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_mock_gemini.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Count original charts
        let original_doc = SscDocument::parse(&temp_ssc_path).expect("Failed to parse");
        let original_count = original_doc.charts.len();

        // 1. Test VALID payload
        let valid_payload = r#"{
            "section_id": "chorus_1",
            "difficulty_level": 15,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.5,
                "last_left_foot_lane": 1,
                "last_right_foot_lane": 3
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": [
                        "10000",
                        "00100",
                        "00001",
                        "00100"
                    ]
                }
            ]
        }"#;

        let result = append_mock_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            valid_payload.to_string(),
            "Gemini Mock Tester".to_string(),
        )
        .expect("Valid payload should succeed");

        assert!(result.written);
        assert!(result.validation.issues.is_empty());
        assert_eq!(
            result.charts.last().unwrap().description,
            "AI Mock chorus_1 S15"
        );
        assert_eq!(result.charts.len(), original_count + 1);

        // 2. Test INVALID payload (Structural error: Mina 'M' Detected)
        let invalid_payload = r#"{
            "section_id": "chorus_2",
            "difficulty_level": 15,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.5,
                "last_left_foot_lane": 1,
                "last_right_foot_lane": 3
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": [
                        "10M00",
                        "00100",
                        "00001",
                        "00100"
                    ]
                }
            ]
        }"#;

        let result_invalid = append_mock_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            invalid_payload.to_string(),
            "Gemini Mock Tester".to_string(),
        )
        .expect("Command should return Ok with written = false");

        assert!(!result_invalid.written);
        assert!(!result_invalid.validation.issues.is_empty());
        assert_eq!(
            result_invalid.validation.issues[0].issue_type,
            ValidationIssueType::InvalidGeminiStructure
        );
        assert!(result_invalid.validation.issues[0]
            .message
            .contains("carácter inválido 'M'"));

        // Check file was not appended further
        let current_doc = SscDocument::parse(&temp_ssc_path).expect("Failed to parse");
        // Count should still match count after valid append (original + 1)
        assert_eq!(current_doc.charts.len(), original_count + 1);

        crate::settings::set_test_env(None);
        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[tokio::test]
    async fn test_generate_gemini_chart_preview_flow() {
        let mut server = Server::new_async().await;

        // Mock Gemini response (Valid structured response)
        let mock_post = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        // Copy ssc fixture
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_gemini_preview_core.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Create a dummy audio file
        let test_audio_path = temp_dir.join("test_audio_core.mp3");
        {
            let mut file = std::fs::File::create(&test_audio_path).unwrap();
            file.write_all(b"audio content").unwrap();
        }

        let client = GeminiClient::new(Some(server.url()));

        // Case 1: Env gate is NOT enabled -> should return error and NOT call Mock server
        crate::settings::set_test_gemini_enabled(Some(false));
        let result_gate = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            None,
            None,
            None,
        )
        .await;
        assert!(result_gate.is_err());
        assert!(result_gate.unwrap_err().contains("deshabilitada"));

        // Now enable env gate
        crate::settings::set_test_gemini_enabled(Some(true));

        // Case 2: PreviewOnly valid flow
        let result_preview = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("PreviewOnly flow failed");

        mock_post.assert_async().await;
        assert!(!result_preview.written);
        assert_eq!(
            result_preview.message,
            "Preview content generated and validated successfully without writing to disk."
        );

        // Case 3: Mismatch section_id
        let mock_post_mismatch = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"different_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_mismatch = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section", // requested
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("Core preview mismatch section_id failed");

        mock_post_mismatch.assert_async().await;
        assert!(!result_mismatch.written);
        assert!(result_mismatch
            .validation
            .issues
            .iter()
            .any(|i| i.message.contains("ID de sección")));

        // Case 4: Mismatch play_mode
        let mock_post_mismatch_pm = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Double\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"1000000000\",\n        \"0000010000\",\n        \"0000000001\",\n        \"0000010000\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_mismatch_pm = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single, // requested Single
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("Core preview mismatch play_mode failed");

        mock_post_mismatch_pm.assert_async().await;
        assert!(!result_mismatch_pm.written);
        assert!(result_mismatch_pm
            .validation
            .issues
            .iter()
            .any(|i| i.message.contains("modo de juego")));

        // Case 5: Mismatch difficulty_level
        let mock_post_mismatch_diff = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 12,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_mismatch_diff = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10, // requested 10
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("Core preview mismatch diff failed");

        mock_post_mismatch_diff.assert_async().await;
        assert!(!result_mismatch_diff.written);
        assert!(result_mismatch_diff
            .validation
            .issues
            .iter()
            .any(|i| i.message.contains("nivel de dificultad")));

        // Case 6: PreviewOnly with warning severity issue (Consecutive Jumps warning)
        let mock_post_warning = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10001\",\n        \"10100\",\n        \"00101\",\n        \"00000\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_warning = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("Core preview with warnings failed");

        mock_post_warning.assert_async().await;
        assert!(!result_warning.written);
        assert_eq!(result_warning.message, "preview generado con advertencias");

        // Case 7: PreviewOnly with error severity issue (Mina detected)
        let mock_post_error = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00M00\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_error = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await
        .expect("Core preview with error failed");

        mock_post_error.assert_async().await;
        assert!(!result_error.written);
        assert_eq!(
            result_error.message,
            "preview generado pero inválido; no se escribió en disco"
        );

        // Case 8: JSON invalid sanitization (confirm error doesn't leak raw JSON content)
        let mock_post_bad_json = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"candidates": [{"content": {"parts": [{"text": "this is not valid json {\"sensitive_info\": \"secret\"} foo bar"}]}}]}"#)
            .create_async()
            .await;

        let result_bad_json = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
        )
        .await;

        mock_post_bad_json.assert_async().await;
        assert!(result_bad_json.is_err());
        let err_msg = result_bad_json.unwrap_err();
        assert!(err_msg.contains("Gemini returned invalid JSON"));
        assert!(!err_msg.contains("sensitive_info")); // verify it was sanitized and did not leak original response content

        // Case 9: Valid preview with start_measure=32, end_measure=33 and Gemini mock returning measure_index 32 and 33
        let mock_post_valid_range = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 32,\n      \"subdivision\": 4,\n      \"rows\": [\"10000\", \"00100\", \"00001\", \"00100\"]\n    },\n    {\n      \"measure_index\": 33,\n      \"subdivision\": 4,\n      \"rows\": [\"10000\", \"00100\", \"00001\", \"00100\"]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_valid_range = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(32),
            Some(33),
            None,
        )
        .await
        .expect("Valid range preview failed");

        mock_post_valid_range.assert_async().await;
        assert!(!result_valid_range.written);
        assert!(!result_valid_range
            .validation
            .issues
            .iter()
            .any(|i| i.severity == ValidationSeverity::Error));

        // Case 10: Range 32-33 but Gemini returning only 1 measure -> should block
        let mock_post_missing_measures = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 32,\n      \"subdivision\": 4,\n      \"rows\": [\"10000\", \"00100\", \"00001\", \"00100\"]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_missing_measures = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(32),
            Some(33),
            None,
        )
        .await
        .expect("Preview core failed");

        mock_post_missing_measures.assert_async().await;
        assert!(!result_missing_measures.written);
        assert!(result_missing_measures.validation.issues.iter().any(|i| {
            i.severity == ValidationSeverity::Error && i.message.contains("expected 2")
        }));

        // Case 11: Range 32-33 but Gemini returning out-of-range/incorrect indices -> should block
        let mock_post_incorrect_indices = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 32,\n      \"subdivision\": 4,\n      \"rows\": [\"10000\", \"00100\", \"00001\", \"00100\"]\n    },\n    {\n      \"measure_index\": 34,\n      \"subdivision\": 4,\n      \"rows\": [\"10000\", \"00100\", \"00001\", \"00100\"]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_incorrect_indices = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(32),
            Some(33),
            None,
        )
        .await
        .expect("Preview core failed");

        mock_post_incorrect_indices.assert_async().await;
        assert!(!result_incorrect_indices.written);
        assert!(result_incorrect_indices.validation.issues.iter().any(|i| {
            i.severity == ValidationSeverity::Error
                && i.message.contains("índice de compás incorrecto")
        }));

        // Case 12: end_measure < start_measure -> should return error before calling Gemini
        let result_invalid_range_pre = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            Some(33),
            Some(32),
            None,
        )
        .await;

        assert!(result_invalid_range_pre.is_err());
        assert!(result_invalid_range_pre
            .unwrap_err()
            .contains("menor que el compás de inicio"));

        // Case 13: Fenced ```json markdown wrapper response in preview -> should parse and return cleaned json in raw_payload
        let mock_post_fenced = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "```json\n{\n  \"section_id\": \"preview_section\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}\n```"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let result_fenced = generate_gemini_chart_preview_core(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "preview_section",
            "AI Previewer",
            &client,
            None,
            None,
            None,
        )
        .await
        .expect("Fenced JSON preview failed");

        mock_post_fenced.assert_async().await;
        assert!(!result_fenced.written);
        let raw_payload_clean = result_fenced
            .raw_payload
            .expect("raw_payload should be present");
        assert!(!raw_payload_clean.contains("```"));
        assert!(raw_payload_clean.contains("\"section_id\""));

        // Clean up
        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_file(temp_ssc_path);
        let _ = std::fs::remove_file(test_audio_path);
    }

    #[test]
    fn test_append_approved_gemini_payload_fenced() {
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_approved_fenced.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Fenced payload JSON
        let fenced_payload = r#"```json
        {
            "section_id": "chorus_fenced",
            "difficulty_level": 12,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.3
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": [
                        "10000",
                        "00100",
                        "00001",
                        "00100"
                    ]
                }
            ]
        }
        ```"#;

        crate::settings::set_test_env(None);
        let fp = get_file_fingerprint(temp_ssc_path.to_string_lossy().to_string()).unwrap();
        let result = append_approved_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            fenced_payload.to_string(),
            "Approved Tester".to_string(),
            fp.sha256,
        )
        .expect("Command with fenced payload should succeed");

        assert!(result.written);
        assert_eq!(
            result.charts.last().unwrap().description,
            "AI chorus_fenced S12"
        );

        let _ = std::fs::remove_file(temp_ssc_path);
        if let Some(backup) = result.backup_path {
            let _ = std::fs::remove_file(backup);
        }
    }

    #[test]
    fn test_append_approved_gemini_payload_valid() {
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_approved_valid.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Valid payload JSON
        let valid_payload = r#"{
            "section_id": "chorus_approved",
            "difficulty_level": 12,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.3
            },
            "placeholder_unused_fields": {},
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": [
                        "10000",
                        "00100",
                        "00001",
                        "00100"
                    ]
                }
            ]
        }"#;

        // Call approved commit (should work even without env mode = dev)
        crate::settings::set_test_env(None);
        let fp = get_file_fingerprint(temp_ssc_path.to_string_lossy().to_string()).unwrap();
        let result = append_approved_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            valid_payload.to_string(),
            "Approved Tester".to_string(),
            fp.sha256,
        )
        .expect("Command should succeed");

        assert!(result.written);
        assert!(result.backup_path.is_some());

        // Verify backup actually exists
        let backup_file = std::path::Path::new(result.backup_path.as_ref().unwrap());
        assert!(backup_file.exists());

        // Verify backup contains original content (which has fewer charts than updated)
        let backup_doc = SscDocument::parse(backup_file).unwrap();
        let updated_doc = SscDocument::parse(&temp_ssc_path).unwrap();
        assert_eq!(backup_doc.charts.len() + 1, updated_doc.charts.len());

        // Verify description is "AI chorus_approved S12" (no "Mock")
        assert_eq!(
            result.charts.last().unwrap().description,
            "AI chorus_approved S12"
        );

        // Clean up
        let _ = std::fs::remove_file(temp_ssc_path);
        let _ = std::fs::remove_file(backup_file);
    }

    #[test]
    fn test_append_approved_gemini_payload_invalid_biomechanics() {
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_approved_invalid_bio.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Payload with severe error: Mina 'M'
        let invalid_payload = r#"{
            "section_id": "chorus_approved_err",
            "difficulty_level": 12,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.3
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": [
                        "10M00",
                        "00100",
                        "00001",
                        "00100"
                    ]
                }
            ]
        }"#;

        let fp = get_file_fingerprint(temp_ssc_path.to_string_lossy().to_string()).unwrap();
        let result = append_approved_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            invalid_payload.to_string(),
            "Approved Tester".to_string(),
            fp.sha256,
        );

        // Should return Err because of the severe biomechanical structure error
        assert!(result.is_err());

        // Clean up
        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_dev_env_protection_for_mocks_and_stubs() {
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_dev_protection.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        // Force env to production / unset
        crate::settings::set_test_env(None);

        // Calling stub should fail in prod
        let result_stub = append_ai_chart_stub(
            temp_ssc_path.to_string_lossy().to_string(),
            "Single".to_string(),
            10,
            "Stubby".to_string(),
        );
        assert!(result_stub.is_err());
        assert!(result_stub.unwrap_err().contains("development mode"));

        // Calling mock payload should fail in prod
        let result_mock = append_mock_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            r#"{"section_id": "foo", "difficulty_level": 10, "play_mode": "Single", "biomechanical_state": {"current_twist_debt": 0.0, "current_stamina_debt": 0.0}, "measures": []}"#.to_string(),
            "Mocker".to_string(),
        );
        assert!(result_mock.is_err());
        assert!(result_mock.unwrap_err().contains("development mode"));

        // Clean up
        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_validate_preview_write_mode() {
        let result_ok = validate_preview_write_mode("PreviewOnly");
        assert!(result_ok.is_ok());
        assert_eq!(result_ok.unwrap(), GeminiWriteMode::PreviewOnly);

        let result_err1 = validate_preview_write_mode("AppendIfValid");
        assert!(result_err1.is_err());
        assert!(result_err1
            .unwrap_err()
            .contains("only supports 'PreviewOnly'"));

        let result_err2 = validate_preview_write_mode("SomeOtherMode");
        assert!(result_err2.is_err());
        assert!(result_err2
            .unwrap_err()
            .contains("only supports 'PreviewOnly'"));
    }

    #[test]
    fn test_append_approved_gemini_payload_fingerprint_mismatch() {
        let original_path = get_fixture_path();
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_fingerprint_mismatch.ssc");
        std::fs::copy(&original_path, &temp_ssc_path).expect("Failed to copy");

        let valid_payload = r#"{
            "section_id": "chorus_approved",
            "difficulty_level": 12,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.3
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": ["10000", "00100", "00001", "00100"]
                }
            ]
        }"#;

        let result = append_approved_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            valid_payload.to_string(),
            "Approved Tester".to_string(),
            "incorrect_sha256_hash_value_here".to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("fingerprint del archivo .ssc ha cambiado"));

        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_get_file_fingerprint_invalid_extension() {
        let temp_dir = std::env::temp_dir();
        let temp_txt_path = temp_dir.join("test_invalid_ext.txt");
        std::fs::write(&temp_txt_path, "some content").unwrap();

        let result = get_file_fingerprint(temp_txt_path.to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Solo se permite calcular el fingerprint"));

        let _ = std::fs::remove_file(temp_txt_path);
    }

    #[test]
    fn test_validate_folder_name_rules() {
        assert!(validate_folder_name_rules("My New Song").is_ok());
        assert_eq!(
            validate_folder_name_rules("  Trimmed Song  ").unwrap(),
            "Trimmed Song"
        );

        // Windows invalid chars
        assert!(validate_folder_name_rules("Song?").is_err());
        assert!(validate_folder_name_rules("Song*").is_err());
        assert!(validate_folder_name_rules("Song/Backslash").is_err());
        assert!(validate_folder_name_rules("Song\\Backslash").is_err());

        // Windows reserved names
        assert!(validate_folder_name_rules("con").is_err());
        assert!(validate_folder_name_rules("PRN").is_err());
        assert!(validate_folder_name_rules("LPT3").is_err());

        // Trailing dot or space
        assert!(validate_folder_name_rules("Song.").is_err());
        assert!(validate_folder_name_rules("Song. ").is_err());
        assert!(validate_folder_name_rules("").is_err());
    }

    #[tokio::test]
    async fn test_create_song_project_flow() {
        let temp_dir = std::env::temp_dir();
        let target_folder = temp_dir.join("test_create_song_project_dir");
        let _ = std::fs::remove_dir_all(&target_folder); // cleanup

        // Create dummy audio file
        let dummy_audio = temp_dir.join("dummy_audio.mp3");
        std::fs::write(&dummy_audio, b"audio data").unwrap();

        // Create dummy banner file
        let dummy_banner = temp_dir.join("dummy_banner.png");
        std::fs::write(&dummy_banner, b"banner data").unwrap();

        // 1. Failure: missing audio
        let payload_no_audio = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            genre: "Original".to_string(),
            credit: "Author".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "135.000".to_string(),
            timing_bpm: 135.0,
            offset: -0.15,
            audio_path: temp_dir
                .join("nonexistent_audio.mp3")
                .to_string_lossy()
                .to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result_err = create_song_project(payload_no_audio).await;
        assert!(result_err.is_err());
        assert!(result_err
            .unwrap_err()
            .contains("Audio file does not exist"));

        // 2. Success: base creation
        let payload = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            genre: "Original".to_string(),
            credit: "Author".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "135.000".to_string(),
            timing_bpm: 135.0,
            offset: -0.15,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: Some(dummy_banner.to_string_lossy().to_string()),
            background_path: None,
            video_path: None,
        };
        let details = create_song_project(payload)
            .await
            .expect("Failed to create song project");
        assert_eq!(details.song_name, "Test Song");
        assert_eq!(details.artist, "Test Artist");
        assert_eq!(details.bpm, 135.0);
        assert_eq!(details.offset, -0.15);

        let expected_ssc = target_folder.join("test_create_song_project_dir.ssc");
        assert!(expected_ssc.exists());

        // Verify copied assets
        assert!(target_folder.join("audio.mp3").exists());
        assert!(target_folder.join("banner.png").exists());

        // Verify asset statuses
        assert_eq!(details.asset_statuses.audio.status_type, "DeclaredAndFound");
        assert_eq!(
            details.asset_statuses.banner.status_type,
            "DeclaredAndFound"
        );
        assert_eq!(details.asset_statuses.background.status_type, "NotDeclared");

        // 3. Failure: overwrite blocked
        let payload_overwrite = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Overwrite Song".to_string(),
            artist: "Test Artist".to_string(),
            genre: "Original".to_string(),
            credit: "Author".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "135.000".to_string(),
            timing_bpm: 135.0,
            offset: -0.15,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result_overwrite = create_song_project(payload_overwrite).await;
        assert!(result_overwrite.is_err());
        assert!(result_overwrite.unwrap_err().contains("Overwrite blocked"));

        // Clean up
        let _ = std::fs::remove_dir_all(&target_folder);
        let _ = std::fs::remove_file(dummy_audio);
        let _ = std::fs::remove_file(dummy_banner);
    }

    #[test]
    fn test_determine_asset_statuses() {
        let temp_dir = std::env::temp_dir();
        let folder = temp_dir.join("test_asset_statuses_dir");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();

        // Write some dummy files in the directory
        let audio_file = folder.join("my_track.mp3");
        std::fs::write(&audio_file, b"").unwrap();
        let banner_file = folder.join("cool_banner.png");
        std::fs::write(&banner_file, b"").unwrap();

        // Scan folder files
        let mut files_in_folder = Vec::new();
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_str().unwrap().to_lowercase();
                files_in_folder.push((name, path));
            }
        }

        // Case 1: Declared and found
        let audio_status =
            determine_asset_status(&folder, Some("my_track.mp3"), "audio", &files_in_folder);
        assert_eq!(audio_status.status_type, "DeclaredAndFound");
        assert_eq!(audio_status.file_name.unwrap(), "my_track.mp3");
        assert!(audio_status.file_path.is_some());

        // Case 2: Declared but missing
        let banner_status = determine_asset_status(
            &folder,
            Some("missing_banner.png"),
            "banner",
            &files_in_folder,
        );
        assert_eq!(banner_status.status_type, "DeclaredButMissing");
        assert_eq!(banner_status.file_name.unwrap(), "missing_banner.png");
        assert!(banner_status.file_path.is_none());

        // Case 3: Found but not declared (declared_name is None/empty)
        let found_banner_status = determine_asset_status(&folder, None, "banner", &files_in_folder);
        assert_eq!(found_banner_status.status_type, "FoundButNotDeclared");
        assert_eq!(found_banner_status.file_name.unwrap(), "cool_banner.png");

        // Case 4: Not declared and not found (background)
        let background_status =
            determine_asset_status(&folder, None, "background", &files_in_folder);
        assert_eq!(background_status.status_type, "NotDeclared");
        assert!(background_status.file_name.is_none());

        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn test_get_file_metadata() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_metadata.mp3");
        std::fs::write(&temp_file, b"12345").unwrap();

        let meta = get_file_metadata(temp_file.to_string_lossy().to_string()).unwrap();
        assert_eq!(meta.name, "test_metadata.mp3");
        assert_eq!(meta.extension, "mp3");
        assert_eq!(meta.size, 5);

        let _ = std::fs::remove_file(temp_file);
    }

    #[tokio::test]
    async fn test_create_song_project_collisions() {
        let temp_dir = std::env::temp_dir();
        let target_folder = temp_dir.join("test_create_song_collisions_dir");
        let _ = std::fs::remove_dir_all(&target_folder);
        std::fs::create_dir_all(&target_folder).unwrap();

        // Write pre-existing audio.mp3 in the directory to trigger collision
        let existing_audio = target_folder.join("audio.mp3");
        std::fs::write(&existing_audio, b"existing audio data").unwrap();

        let dummy_audio = temp_dir.join("colliding_audio.mp3");
        std::fs::write(&dummy_audio, b"new audio data").unwrap();

        let payload = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Collision Song".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "120".to_string(),
            timing_bpm: 120.0,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };

        let result = create_song_project(payload).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File collision detected"));

        let _ = std::fs::remove_dir_all(&target_folder);
        let _ = std::fs::remove_file(dummy_audio);
    }

    #[tokio::test]
    async fn test_create_song_project_dangerous_metadata() {
        let temp_dir = std::env::temp_dir();
        let target_folder = temp_dir.join("test_create_song_dangerous_dir");
        let _ = std::fs::remove_dir_all(&target_folder);

        let dummy_audio = temp_dir.join("dangerous_metadata_audio.mp3");
        std::fs::write(&dummy_audio, b"audio data").unwrap();

        // 1. Semicolon
        let payload_semicolon = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Dangerous; Title".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "120".to_string(),
            timing_bpm: 120.0,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result = create_song_project(payload_semicolon).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot contain semicolons"));

        // 2. Newline
        let payload_newline = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Dangerous\nTitle".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "120".to_string(),
            timing_bpm: 120.0,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result = create_song_project(payload_newline).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot contain newlines"));

        let _ = std::fs::remove_file(dummy_audio);
    }

    #[tokio::test]
    async fn test_create_song_project_invalid_optional_extensions() {
        let temp_dir = std::env::temp_dir();
        let target_folder = temp_dir.join("test_create_song_extensions_dir");
        let _ = std::fs::remove_dir_all(&target_folder);

        let dummy_audio = temp_dir.join("ext_audio.mp3");
        std::fs::write(&dummy_audio, b"audio").unwrap();

        let invalid_banner = temp_dir.join("banner.txt");
        std::fs::write(&invalid_banner, b"text data").unwrap();

        let payload = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Invalid Extension Song".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "120".to_string(),
            timing_bpm: 120.0,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: Some(invalid_banner.to_string_lossy().to_string()),
            background_path: None,
            video_path: None,
        };

        let result = create_song_project(payload).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported banner format"));

        let _ = std::fs::remove_file(dummy_audio);
        let _ = std::fs::remove_file(invalid_banner);
    }

    #[tokio::test]
    async fn test_create_song_project_bpm_range_validation() {
        let temp_dir = std::env::temp_dir();
        let target_folder = temp_dir.join("test_create_song_bpm_dir");
        let _ = std::fs::remove_dir_all(&target_folder);

        let dummy_audio = temp_dir.join("bpm_audio.mp3");
        std::fs::write(&dummy_audio, b"audio").unwrap();

        // 1. BPM too low (9.9)
        let payload_low = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "Low BPM".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "9.9".to_string(),
            timing_bpm: 9.9,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result_low = create_song_project(payload_low).await;
        assert!(result_low.is_err());
        assert!(result_low
            .unwrap_err()
            .contains("must be a reasonable number between 10.0 and 1000.0"));

        // 2. BPM too high (1000.1)
        let payload_high = CreateSongPayload {
            target_folder_path: target_folder.to_string_lossy().to_string(),
            title: "High BPM".to_string(),
            artist: "Artist".to_string(),
            genre: "Genre".to_string(),
            credit: "Credit".to_string(),
            song_type: "ARCADE".to_string(),
            display_bpm: "1000.1".to_string(),
            timing_bpm: 1000.1,
            offset: 0.0,
            audio_path: dummy_audio.to_string_lossy().to_string(),
            banner_path: None,
            background_path: None,
            video_path: None,
        };
        let result_high = create_song_project(payload_high).await;
        assert!(result_high.is_err());
        assert!(result_high
            .unwrap_err()
            .contains("must be a reasonable number between 10.0 and 1000.0"));

        let _ = std::fs::remove_file(dummy_audio);
    }

    #[test]
    fn test_determine_asset_status_path_traversal() {
        let temp_dir = std::env::temp_dir();
        let folder = temp_dir.join("test_traversal_folder");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();

        // Write a file inside the folder
        let inside_file = folder.join("inside.mp3");
        std::fs::write(&inside_file, b"inside").unwrap();

        // Write a file outside the folder
        let outside_file = temp_dir.join("outside_traversal.mp3");
        std::fs::write(&outside_file, b"outside").unwrap();

        // Scan folder files
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_str().unwrap().to_lowercase();
                files.push((name, path));
            }
        }

        // 1. Inside: DeclaredAndFound
        let inside_status = determine_asset_status(&folder, Some("inside.mp3"), "audio", &files);
        assert_eq!(inside_status.status_type, "DeclaredAndFound");

        // 2. Traversal: DeclaredButMissing (since it's outside)
        let traversal_status =
            determine_asset_status(&folder, Some("../outside_traversal.mp3"), "audio", &files);
        assert_eq!(traversal_status.status_type, "DeclaredButMissing");

        let _ = std::fs::remove_dir_all(&folder);
        let _ = std::fs::remove_file(outside_file);
    }

    #[test]
    fn test_check_destination_folder_nonexistent() {
        let result = check_destination_folder("/nonexistent/path/at/all".to_string()).unwrap();
        assert_eq!(result, "NotExist");
    }

    #[tokio::test]
    async fn test_read_audio_file_security() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_dir = manifest_dir.join("temp_security_test");
        let _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).unwrap();

        // Test 1: Invalid extension
        let txt_file = test_dir.join("test_security.txt");
        std::fs::write(&txt_file, b"some content").unwrap();
        let res = read_audio_file(txt_file.to_string_lossy().to_string()).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Formato de audio no soportado"));

        // Test 2: Invalid magic bytes on audio extension
        let fake_mp3 = test_dir.join("test_security.mp3");
        std::fs::write(&fake_mp3, b"malicious binary or config content here").unwrap();
        let res = read_audio_file(fake_mp3.to_string_lossy().to_string()).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Los bytes mágicos del archivo no corresponden"));

        // Test 3: Path blocklisting (e.g. system folder path simulating traversal)
        let res_blocked = read_audio_file("C:\\Windows\\System32\\test.mp3".to_string()).await;
        assert!(res_blocked.is_err());
        assert!(res_blocked.unwrap_err().contains("Acceso denegado"));

        // Test 4: Valid WAV audio magic bytes
        let valid_wav = test_dir.join("test_security_valid.wav");
        let mut wav_bytes = vec![0u8; 12];
        wav_bytes[0..4].copy_from_slice(b"RIFF");
        wav_bytes[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&valid_wav, &wav_bytes).unwrap();
        let res = read_audio_file(valid_wav.to_string_lossy().to_string()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), wav_bytes);

        // Test 5: Missing file
        let missing_file = test_dir.join("does_not_exist.wav");
        let res_missing = read_audio_file(missing_file.to_string_lossy().to_string()).await;
        assert!(res_missing.is_err());
        assert!(res_missing
            .unwrap_err()
            .contains("no existe o no es un archivo"));

        // Test 6: Oversized file (> 100 MB) via metadata truncation (O(1) operation)
        let oversized_file = test_dir.join("oversized.wav");
        let file = std::fs::File::create(&oversized_file).unwrap();
        file.set_len(100 * 1024 * 1024 + 1).unwrap();
        drop(file);
        let res_oversized = read_audio_file(oversized_file.to_string_lossy().to_string()).await;
        assert!(res_oversized.is_err());
        assert!(res_oversized
            .unwrap_err()
            .contains("supera el límite máximo"));

        // Test 7: Case-insensitive extension
        let case_wav = test_dir.join("test_case.WAV");
        let mut case_bytes = vec![0u8; 12];
        case_bytes[0..4].copy_from_slice(b"RIFF");
        case_bytes[8..12].copy_from_slice(b"WAVE");
        std::fs::write(&case_wav, &case_bytes).unwrap();
        let res_case = read_audio_file(case_wav.to_string_lossy().to_string()).await;
        assert!(res_case.is_ok());

        // Test 8: Valid OGG magic bytes
        let valid_ogg = test_dir.join("test_ogg.ogg");
        let mut ogg_bytes = vec![0u8; 12];
        ogg_bytes[0..4].copy_from_slice(b"OggS");
        std::fs::write(&valid_ogg, &ogg_bytes).unwrap();
        let res_ogg = read_audio_file(valid_ogg.to_string_lossy().to_string()).await;
        assert!(res_ogg.is_ok());

        // Test 9: Valid FLAC magic bytes
        let valid_flac = test_dir.join("test_flac.flac");
        let mut flac_bytes = vec![0u8; 12];
        flac_bytes[0..4].copy_from_slice(b"fLaC");
        std::fs::write(&valid_flac, &flac_bytes).unwrap();
        let res_flac = read_audio_file(valid_flac.to_string_lossy().to_string()).await;
        assert!(res_flac.is_ok());

        // Test 10: Valid MP3 magic bytes (ID3 tag)
        let valid_mp3 = test_dir.join("test_mp3.mp3");
        let mut mp3_bytes = vec![0u8; 12];
        mp3_bytes[0..3].copy_from_slice(b"ID3");
        std::fs::write(&valid_mp3, &mp3_bytes).unwrap();
        let res_mp3 = read_audio_file(valid_mp3.to_string_lossy().to_string()).await;
        assert!(res_mp3.is_ok());

        // Test 11: Valid MP3 magic bytes (raw sync word 0xFFFB)
        let valid_mp3_raw = test_dir.join("test_mp3_raw.mp3");
        let mut mp3_raw_bytes = vec![0u8; 12];
        mp3_raw_bytes[0] = 0xFF;
        mp3_raw_bytes[1] = 0xFB;
        std::fs::write(&valid_mp3_raw, &mp3_raw_bytes).unwrap();
        let res_mp3_raw = read_audio_file(valid_mp3_raw.to_string_lossy().to_string()).await;
        assert!(res_mp3_raw.is_ok());

        // Test 12: Blocklist path trigger
        let res_git_blocked = read_audio_file("C:\\my_songpack\\.git\\audio.wav".to_string()).await;
        assert!(res_git_blocked.is_err());
        assert!(res_git_blocked.unwrap_err().contains("Acceso denegado"));

        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
