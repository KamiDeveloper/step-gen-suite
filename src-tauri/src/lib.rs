pub mod biomechanics;
pub mod commands;
pub mod credentials;
pub mod gemini;
pub mod music_analysis;
pub mod settings;
pub mod ssc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::import_song_folder,
            commands::list_charts,
            commands::append_ai_chart_stub,
            commands::validate_chart_notes,
            commands::append_mock_gemini_payload,
            commands::append_approved_gemini_payload,
            commands::generate_gemini_chart_preview,
            commands::get_file_fingerprint,
            commands::validate_folder_name,
            commands::check_destination_folder,
            commands::create_destination_folder,
            commands::select_song_asset_file,
            commands::select_song_destination_folder,
            commands::create_song_project,
            commands::get_file_metadata,
            credentials::save_gemini_api_key,
            credentials::has_gemini_api_key,
            credentials::delete_gemini_api_key,
            settings::get_settings,
            settings::save_settings,
            settings::get_app_mode,
            settings::ensure_default_songpack,
            music_analysis::analyze_song_offline
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
