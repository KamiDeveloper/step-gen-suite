use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE: &str = "settings.json";

fn default_songpack_mode() -> String {
    "managed_default".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub songs_dir: Option<String>,
    #[serde(default = "default_songpack_mode")]
    pub songpack_mode: String,
    pub default_songpack_folder: String,
    pub default_author: Option<String>,
    pub default_play_mode: Option<String>,
    pub default_meter: Option<u32>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            songs_dir: None,
            songpack_mode: default_songpack_mode(),
            default_songpack_folder: "99-AI-Step-Gen".to_string(),
            default_author: Some("AI Step Gen".to_string()),
            default_play_mode: Some("Single".to_string()),
            default_meter: Some(10),
        }
    }
}

fn get_settings_path<R: Runtime>(app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    Ok(app_dir.join(SETTINGS_FILE))
}

pub fn load_settings_internal<R: Runtime>(app_handle: &AppHandle<R>) -> AppSettings {
    let path = match get_settings_path(app_handle) {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    if !path.exists() {
        return AppSettings::default();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return AppSettings::default(),
    };
    serde_json::from_str(&content).unwrap_or_else(|_| AppSettings::default())
}

#[tauri::command]
pub fn get_settings<R: Runtime>(app_handle: AppHandle<R>) -> Result<AppSettings, String> {
    Ok(load_settings_internal(&app_handle))
}

#[tauri::command]
pub fn save_settings<R: Runtime>(
    app_handle: AppHandle<R>,
    settings: AppSettings,
) -> Result<(), String> {
    let path = get_settings_path(&app_handle)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
    }
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content).map_err(|e| format!("Failed to write settings: {}", e))?;
    fs::rename(&temp_path, &path).map_err(|e| format!("Failed to save settings: {}", e))?;
    Ok(())
}

thread_local! {
    static TEST_ENV_MODE: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    static TEST_GEMINI_ENABLED: std::cell::RefCell<Option<bool>> = std::cell::RefCell::new(None);
}

pub fn get_app_env() -> String {
    TEST_ENV_MODE.with(|v| {
        if let Some(mode) = &*v.borrow() {
            mode.clone()
        } else {
            std::env::var("AI_STEP_GEN_ENV").unwrap_or_default()
        }
    })
}

pub fn set_test_env(mode: Option<&str>) {
    TEST_ENV_MODE.with(|v| {
        *v.borrow_mut() = mode.map(|s| s.to_string());
    });
}

pub fn is_gemini_enabled() -> bool {
    TEST_GEMINI_ENABLED.with(|v| {
        if let Some(enabled) = *v.borrow() {
            enabled
        } else {
            std::env::var("AI_STEP_GEN_ENABLE_REAL_GEMINI").unwrap_or_default() == "1"
        }
    })
}

pub fn set_test_gemini_enabled(enabled: Option<bool>) {
    TEST_GEMINI_ENABLED.with(|v| {
        *v.borrow_mut() = enabled;
    });
}

#[tauri::command]
pub fn get_app_mode() -> String {
    let env_mode = get_app_env();
    if env_mode == "dev" {
        "dev".to_string()
    } else {
        "prod".to_string()
    }
}

pub fn resolve_template_dir(resource_dir: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(res_dir) = resource_dir {
        let packaged_path = res_dir.join("resources").join("songpack-template");
        if packaged_path.exists() {
            return Ok(packaged_path);
        }
        let alternative_packaged_path = res_dir.join("songpack-template");
        if alternative_packaged_path.exists() {
            return Ok(alternative_packaged_path);
        }
    }

    // Fallbacks for development / test environments
    let dev_paths = vec![
        PathBuf::from("resources/songpack-template"),
        PathBuf::from("src-tauri/resources/songpack-template"),
        PathBuf::from("../src-tauri/resources/songpack-template"),
    ];

    for path in dev_paths {
        if path.exists() {
            return Ok(path);
        }
    }

    Err("Songpack template directory not found in resources or dev fallbacks.".to_string())
}

fn get_songpack_template_dir<R: Runtime>(app_handle: &AppHandle<R>) -> Result<PathBuf, String> {
    let res_dir = app_handle.path().resource_dir().ok();
    resolve_template_dir(res_dir)
}

pub fn ensure_songpack_from_settings(
    settings: &AppSettings,
    template_dir: PathBuf,
) -> Result<String, String> {
    let songs_dir_str = settings
        .songs_dir
        .as_ref()
        .ok_or("Songs directory is not configured in settings.")?;
    let songs_dir = std::path::Path::new(songs_dir_str);
    if !songs_dir.exists() || !songs_dir.is_dir() {
        return Err(format!(
            "Songs directory does not exist or is not a directory: {}",
            songs_dir_str
        ));
    }

    let songpack_folder_name = if settings.default_songpack_folder.trim().is_empty() {
        "99-AI-Step-Gen".to_string()
    } else {
        settings.default_songpack_folder.clone()
    };

    let songpack_dir = songs_dir.join(&songpack_folder_name);

    if settings.songpack_mode == "custom_existing" {
        if !songpack_dir.exists() {
            return Err(format!(
                "Songpack directory does not exist: {}",
                songpack_dir.to_string_lossy()
            ));
        }
        return Ok(songpack_dir.to_string_lossy().to_string());
    }

    // managed_default mode: copy Banner.png and info/Sound.ogg only if they are missing.
    let template_banner = template_dir.join("Banner.png");
    if !template_banner.exists() {
        return Err("Required Banner.png template asset is missing.".to_string());
    }

    let template_sound = template_dir.join("info").join("Sound.ogg");
    if !template_sound.exists() {
        return Err("Required info/Sound.ogg template asset is missing.".to_string());
    }

    let info_dir = songpack_dir.join("info");
    fs::create_dir_all(&info_dir)
        .map_err(|e| format!("Failed to create songpack directories: {}", e))?;

    let banner_path = songpack_dir.join("Banner.png");
    if !banner_path.exists() {
        fs::copy(&template_banner, &banner_path)
            .map_err(|e| format!("Failed to copy Banner.png from template: {}", e))?;
    }

    let sound_path = info_dir.join("Sound.ogg");
    if !sound_path.exists() {
        fs::copy(&template_sound, &sound_path)
            .map_err(|e| format!("Failed to copy Sound.ogg from template: {}", e))?;
    }

    Ok(songpack_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn ensure_default_songpack<R: Runtime>(app_handle: AppHandle<R>) -> Result<String, String> {
    let settings = load_settings_internal(&app_handle);
    let template_dir = get_songpack_template_dir(&app_handle)?;
    ensure_songpack_from_settings(&settings, template_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_songpack_template_dir() {
        let template_dir = resolve_template_dir(None);
        assert!(
            template_dir.is_ok(),
            "Should resolve the template directory during tests"
        );
        let dir = template_dir.unwrap();
        assert!(dir.exists(), "Resolved template directory must exist");
        assert!(
            dir.join("Banner.png").exists(),
            "Banner.png should exist in templates"
        );
        assert!(
            dir.join("info/Sound.ogg").exists(),
            "Sound.ogg should exist in templates"
        );
    }

    #[test]
    fn test_ensure_songpack_managed_default() {
        // Setup temporary songs directory
        let temp_dir = std::env::temp_dir();
        let songs_test_dir = temp_dir.join("AI_STEP_GEN_TEST_SONGS_1");
        fs::create_dir_all(&songs_test_dir).unwrap();

        let template_dir = resolve_template_dir(None).unwrap();

        // Write custom settings with managed_default
        let settings = AppSettings {
            songs_dir: Some(songs_test_dir.to_string_lossy().to_string()),
            songpack_mode: "managed_default".to_string(),
            default_songpack_folder: "Test-Managed-Songpack".to_string(),
            default_author: Some("Test".to_string()),
            default_play_mode: Some("Single".to_string()),
            default_meter: Some(10),
        };

        let songpack_path = ensure_songpack_from_settings(&settings, template_dir).unwrap();
        let path = PathBuf::from(&songpack_path);
        assert!(path.exists());
        assert!(path.join("Banner.png").exists());
        assert!(path.join("info/Sound.ogg").exists());

        // Check file sizes are not 0 to prove we copied real assets and not dummies
        let banner_meta = fs::metadata(path.join("Banner.png")).unwrap();
        let sound_meta = fs::metadata(path.join("info/Sound.ogg")).unwrap();
        assert!(banner_meta.len() > 0);
        assert!(sound_meta.len() > 0);

        // Clean up
        let _ = fs::remove_dir_all(songs_test_dir);
    }

    #[test]
    fn test_ensure_songpack_custom_existing() {
        let temp_dir = std::env::temp_dir();
        let songs_test_dir = temp_dir.join("AI_STEP_GEN_TEST_SONGS_2");
        fs::create_dir_all(&songs_test_dir).unwrap();

        let template_dir = resolve_template_dir(None).unwrap();

        // Set up settings with custom_existing pointing to a non-existent folder
        let settings = AppSettings {
            songs_dir: Some(songs_test_dir.to_string_lossy().to_string()),
            songpack_mode: "custom_existing".to_string(),
            default_songpack_folder: "Missing-Custom-Pack".to_string(),
            default_author: Some("Test".to_string()),
            default_play_mode: Some("Single".to_string()),
            default_meter: Some(10),
        };

        // ensure_songpack_from_settings should return an error because it doesn't exist
        let result = ensure_songpack_from_settings(&settings, template_dir.clone());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));

        // Now create the custom pack manually, but leave it empty
        let custom_pack_path = songs_test_dir.join("Missing-Custom-Pack");
        fs::create_dir_all(&custom_pack_path).unwrap();

        // Should now succeed and return the path, without creating any Banner/Sound files!
        let result_ok = ensure_songpack_from_settings(&settings, template_dir).unwrap();
        assert_eq!(result_ok, custom_pack_path.to_string_lossy().to_string());
        assert!(!custom_pack_path.join("Banner.png").exists());
        assert!(!custom_pack_path.join("info/Sound.ogg").exists());

        // Clean up
        let _ = fs::remove_dir_all(songs_test_dir);
    }
}
