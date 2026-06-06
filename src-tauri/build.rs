fn main() {
    println!("cargo::rustc-check-cfg=cfg(mobile)");
    if std::env::var("AI_STEP_GEN_TESTS").is_err() {
        tauri_build::build();
    }
}
