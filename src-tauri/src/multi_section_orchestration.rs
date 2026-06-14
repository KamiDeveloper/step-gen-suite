use crate::biomechanics::{
    GeminiChartSectionPayload, PlayMode, ValidationIssue, ValidationSeverity,
};
use crate::commands::{get_file_fingerprint, FileFingerprint};
use crate::generation_context::{CalibrationContextSummary, PatternFamilyTargetingReport};
use crate::section_continuity::{
    apply_section_plan_overrides, build_song_continuity_plan, determine_motif_strategy,
    ContinuityContextSummary, NeighborSummary, NeighborSummaryGroup, SectionPlanOverride,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
pub const MAX_SECTIONS_PER_BATCH: usize = 4;
pub const MAX_MEASURES_PER_SECTION: i32 = 16;
pub const MAX_TOTAL_MEASURES_PER_BATCH: i32 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSectionGenerationRequest {
    pub ssc_path: String,
    pub audio_path: Option<String>,
    pub target_level: u8,
    pub selected_section_ids: Vec<String>,
    pub overrides: Option<Vec<SectionPlanOverride>>,
    pub use_calibrated_prompt_context: bool,
    pub use_continuity_planning: bool,
    pub pattern_family_target: Option<String>,
    pub max_sections: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSectionGenerationResult {
    pub schema_version: String,
    pub written: bool,
    pub preview_only: bool,
    pub session_id: String,
    pub target_level: u8,
    pub sections_requested: usize,
    pub sections_generated: usize,
    pub sections_failed: usize,
    pub session_status: String,
    pub fingerprint_before: Option<FileFingerprint>,
    pub fingerprint_after: Option<FileFingerprint>,
    pub fingerprint_match: bool,
    pub unsafe_to_append: bool,
    pub items: Vec<SectionPreviewQueueItem>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPreviewQueueItem {
    pub section_id: String,
    pub section_index: usize,
    pub status: String,
    pub range_summary: String,
    pub continuity_summary: Option<ContinuityContextSummary>,
    pub pattern_family_targeting: Option<PatternFamilyTargetingReport>,
    pub calibration_summary: Option<CalibrationContextSummary>,
    pub preview_payload: Option<GeminiChartSectionPayload>,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
    pub generated_abstract_summary: Option<GeneratedSectionAbstractSummary>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSectionAbstractSummary {
    pub row_count: usize,
    pub non_empty_row_count: usize,
    pub approximate_density_band: String,
    pub jump_like_event_count: usize,
    pub hold_like_event_count: usize,
    pub bracket_like_event_count: usize,
    pub primary_family: String,
    pub issue_count: usize,
}

pub fn compute_abstract_summary(
    payload: &GeminiChartSectionPayload,
    validation_issues: &[ValidationIssue],
    calibration: Option<&crate::guardrail_calibration::SingleGuardrailCalibration>,
    target_level: u8,
) -> GeneratedSectionAbstractSummary {
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

    // Approximate density band
    let p50 = if let Some(cal) = calibration {
        if let Some(threshold) = cal.level_thresholds.get(&format!("S{}", target_level)) {
            threshold
                .density
                .get("typical_p50")
                .copied()
                .unwrap_or(target_level as f64 * 2.5)
        } else {
            target_level as f64 * 2.5
        }
    } else {
        target_level as f64 * 2.5
    };

    let approximate_density_band = if density < p50 * 0.6 {
        "very_low".to_string()
    } else if density < p50 * 0.95 {
        "low".to_string()
    } else if density < p50 * 1.15 {
        "medium".to_string()
    } else if density < p50 * 1.45 {
        "high".to_string()
    } else {
        "very_high".to_string()
    };

    let initial_bpm = 120.0;
    let section_families = crate::guardrail_calibration::classify_section_families(
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

    let primary_family = section_families
        .first()
        .cloned()
        .unwrap_or_else(|| "balanced".to_string());
    let issue_count = validation_issues.len();

    GeneratedSectionAbstractSummary {
        row_count,
        non_empty_row_count: active_row_count,
        approximate_density_band,
        jump_like_event_count: jump_count,
        hold_like_event_count: hold_start_count,
        bracket_like_event_count: bracket_candidate_count,
        primary_family,
        issue_count,
    }
}

fn derive_multi_section_session_status(
    unsafe_to_append: bool,
    fingerprint_match: bool,
    sections_failed: usize,
    sections_generated: usize,
    items: &[SectionPreviewQueueItem],
) -> String {
    if unsafe_to_append || !fingerprint_match {
        "unsafe_to_append".to_string()
    } else if sections_failed > 0 {
        if sections_generated > 0 {
            "partial_success".to_string()
        } else {
            "failed".to_string()
        }
    } else {
        let has_warnings = items.iter().any(|it| it.status == "warning");
        if has_warnings {
            "warning".to_string()
        } else {
            "succeeded".to_string()
        }
    }
}

pub async fn generate_gemini_multi_section_preview_queue_core(
    api_key: &str,
    client: &crate::gemini::GeminiClient,
    request: MultiSectionGenerationRequest,
) -> Result<MultiSectionGenerationResult, String> {
    // 1. Basic validation
    if request.selected_section_ids.is_empty() {
        return Err("La selección de secciones no puede estar vacía.".to_string());
    }

    if request.selected_section_ids.len() > MAX_SECTIONS_PER_BATCH {
        return Err(format!(
            "Se ha excedido el límite máximo de secciones permitidas por lote (Límite: {}, Solicitadas: {}).",
            MAX_SECTIONS_PER_BATCH,
            request.selected_section_ids.len()
        ));
    }

    if let Some(max_sec) = request.max_sections {
        if max_sec > MAX_SECTIONS_PER_BATCH {
            return Err(format!(
                "El parámetro max_sections ({}) supera el límite estricto permitido de {}.",
                max_sec, MAX_SECTIONS_PER_BATCH
            ));
        }
    }

    // Check duplicate sections
    let mut seen = std::collections::HashSet::new();
    for id in &request.selected_section_ids {
        if !seen.insert(id) {
            return Err("No se permiten secciones duplicadas en una solicitud.".to_string());
        }
    }

    let ssc_path = Path::new(&request.ssc_path);
    if !ssc_path.exists() || !ssc_path.is_file() {
        return Err(format!(
            "El archivo .ssc especificado no existe o no es válido: {}",
            request.ssc_path
        ));
    }

    // 2. Load the Music Analysis report
    let report_opt = if let Some(dir) = ssc_path.parent() {
        let report_file = dir
            .join(".ai-step-gen-analysis")
            .join("song-analysis-report.v1.json");
        if report_file.exists() && report_file.is_file() {
            if let Ok(report_content) = fs::read_to_string(&report_file) {
                serde_json::from_str::<crate::music_analysis::SongAnalysisReport>(&report_content)
                    .ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let report = report_opt.ok_or_else(|| {
        "El informe de Análisis Musical es obligatorio para la orquestación multi-sección. Por favor, ejecute el análisis primero.".to_string()
    })?;

    // Load calibration if available
    let calibration = crate::guardrail_calibration::resolve_calibration_file(None);

    // Build the continuity plan
    let mut plan = build_song_continuity_plan(
        request.target_level as u32,
        PlayMode::Single,
        Some(&report),
        calibration.as_ref(),
        None,
        "dummy_section",
        0,
        1,
    );

    if let Some(ref ovs) = request.overrides {
        plan = apply_section_plan_overrides(plan, ovs)?;
    }

    // Validate selected sections against plan
    let mut selected_nodes = Vec::new();
    for section_id in &request.selected_section_ids {
        let node = plan
            .sections
            .iter()
            .find(|s| s.section_id == *section_id)
            .ok_or_else(|| {
                format!(
                    "La sección seleccionada '{}' no se encuentra en el plan.",
                    section_id
                )
            })?;

        if !node.enabled {
            return Err(format!(
                "La sección seleccionada '{}' está deshabilitada.",
                section_id
            ));
        }

        if node.start_measure < 0 || node.end_measure < 0 {
            return Err(format!(
                "La sección '{}' tiene límites de compás inválidos (valores negativos).",
                section_id
            ));
        }

        if node.start_measure >= node.end_measure {
            return Err(format!(
                "La sección '{}' tiene límites de compás inválidos (inicio >= fin).",
                section_id
            ));
        }

        let section_len = node.end_measure - node.start_measure + 1;
        if section_len > MAX_MEASURES_PER_SECTION {
            return Err(format!(
                "La sección '{}' supera el límite máximo de {} compases (longitud: {}).",
                section_id, MAX_MEASURES_PER_SECTION, section_len
            ));
        }

        selected_nodes.push(node.clone());
    }

    // Check chronological ordering by measure
    let mut last_start = -1;
    for node in &selected_nodes {
        if node.start_measure < last_start {
            return Err(
                "Las secciones seleccionadas deben estar ordenadas cronológicamente por compás."
                    .to_string(),
            );
        }
        last_start = node.start_measure;
    }

    // Check total measures per batch <= 64
    let total_measures: i32 = selected_nodes
        .iter()
        .map(|n| n.end_measure - n.start_measure + 1)
        .sum();
    if total_measures > MAX_TOTAL_MEASURES_PER_BATCH {
        return Err(format!(
            "El total de compases solicitado en el lote ({}) supera el límite máximo de {}.",
            total_measures, MAX_TOTAL_MEASURES_PER_BATCH
        ));
    }

    // Fail early if fingerprint calculation fails
    let fingerprint_before = get_file_fingerprint(request.ssc_path.clone()).map_err(|e| {
        format!(
            "Error al calcular el fingerprint inicial del archivo .ssc: {}",
            e
        )
    })?;

    let mut items = Vec::new();
    let mut generated_so_far_summary = Vec::new();
    let mut fingerprint_match = true;
    let mut unsafe_to_append = false;
    let mut sections_generated = 0;
    let mut sections_failed = 0;
    let mut stopped_early = false;

    let session_id = format!("ms-session-{}", chrono::Utc::now().timestamp_millis());

    for node in selected_nodes {
        if stopped_early {
            items.push(SectionPreviewQueueItem {
                section_id: node.section_id.clone(),
                section_index: node.section_index,
                status: "skipped".to_string(),
                range_summary: format!("{}-{}", node.start_measure, node.end_measure),
                continuity_summary: None,
                pattern_family_targeting: None,
                calibration_summary: None,
                preview_payload: None,
                validation_errors: Vec::new(),
                validation_warnings: Vec::new(),
                generated_abstract_summary: None,
                error_message: Some("Cancelado debido a un cambio en el fingerprint del archivo .ssc durante la sesión.".to_string()),
            });
            continue;
        }

        // Fingerprint check right before generating this item
        let fp_check = get_file_fingerprint(request.ssc_path.clone());
        match fp_check {
            Ok(fp_current) => {
                if fp_current.sha256 != fingerprint_before.sha256 {
                    fingerprint_match = false;
                    unsafe_to_append = true;
                    stopped_early = true;

                    items.push(SectionPreviewQueueItem {
                        section_id: node.section_id.clone(),
                        section_index: node.section_index,
                        status: "skipped".to_string(),
                        range_summary: format!("{}-{}", node.start_measure, node.end_measure),
                        continuity_summary: None,
                        pattern_family_targeting: None,
                        calibration_summary: None,
                        preview_payload: None,
                        validation_errors: Vec::new(),
                        validation_warnings: Vec::new(),
                        generated_abstract_summary: None,
                        error_message: Some(
                            "Fingerprint mismatch detected. Skipping remaining sections."
                                .to_string(),
                        ),
                    });
                    continue;
                }
            }
            Err(e) => {
                fingerprint_match = false;
                unsafe_to_append = true;
                stopped_early = true;

                items.push(SectionPreviewQueueItem {
                    section_id: node.section_id.clone(),
                    section_index: node.section_index,
                    status: "skipped".to_string(),
                    range_summary: format!("{}-{}", node.start_measure, node.end_measure),
                    continuity_summary: None,
                    pattern_family_targeting: None,
                    calibration_summary: None,
                    preview_payload: None,
                    validation_errors: Vec::new(),
                    validation_warnings: Vec::new(),
                    generated_abstract_summary: None,
                    error_message: Some(format!(
                        "Failed to calculate fingerprint: {}. Skipping remaining sections.",
                        e
                    )),
                });
                continue;
            }
        }

        // Build sliding window context
        let prev_neighbor = if node.section_index > 0 {
            plan.sections
                .get(node.section_index - 1)
                .map(|p| NeighborSummary {
                    section_id: p.section_id.clone(),
                    music_role: p.music_role.clone(),
                    piu_role: p.piu_role.clone(),
                    intensity_band: p.intensity_band.clone(),
                    primary_family: p.primary_pattern_family.clone(),
                })
        } else {
            None
        };

        let next_neighbor = if node.section_index + 1 < plan.sections.len() {
            plan.sections
                .get(node.section_index + 1)
                .map(|n| NeighborSummary {
                    section_id: n.section_id.clone(),
                    music_role: n.music_role.clone(),
                    piu_role: n.piu_role.clone(),
                    intensity_band: n.intensity_band.clone(),
                    primary_family: n.primary_pattern_family.clone(),
                })
        } else {
            None
        };

        let sliding_window_ctx = serde_json::json!({
            "sliding_window_context": {
                "previous_section": prev_neighbor.as_ref().map(|n| serde_json::json!({
                    "section_id": n.section_id,
                    "motif_strategy": determine_motif_strategy(Some(&n.music_role), Some(&n.piu_role)),
                    "intensity_band": n.intensity_band,
                    "primary_family": n.primary_family,
                })),
                "current_section": {
                    "section_id": node.section_id.clone(),
                    "motif_strategy": node.motif_strategy.clone(),
                    "intensity_band": node.intensity_band.clone(),
                    "primary_family": node.primary_pattern_family.clone(),
                },
                "next_section": next_neighbor.as_ref().map(|n| serde_json::json!({
                    "section_id": n.section_id,
                    "motif_strategy": determine_motif_strategy(Some(&n.music_role), Some(&n.piu_role)),
                    "intensity_band": n.intensity_band,
                    "primary_family": n.primary_family,
                })),
                "generated_so_far_summary": generated_so_far_summary
            }
        });

        // Call the core generator
        let res = crate::commands::generate_gemini_chart_preview_core_internal_with_overrides(
            api_key,
            &request.ssc_path,
            request.audio_path.as_deref().unwrap_or(""),
            PlayMode::Single,
            request.target_level as u32,
            &node.section_id,
            "author",
            client,
            Some(node.start_measure as u32),
            Some(node.end_measure as u32),
            Some(report.timing_grid.song_type.clone()),
            Some(true),
            Some(true),
            None,
            Some(request.use_calibrated_prompt_context),
            request.pattern_family_target.clone(),
            calibration.as_ref(),
            Some(request.use_continuity_planning),
            request.overrides.clone(),
            Some(sliding_window_ctx),
        )
        .await;

        match res {
            Ok(append_result) => {
                let mut parse_err = None;
                let payload_opt = if let Some(ref raw_p) = append_result.raw_payload {
                    let clean = crate::commands::sanitize_gemini_json_payload(raw_p);
                    match serde_json::from_str::<GeminiChartSectionPayload>(&clean) {
                        Ok(p) => Some(p),
                        Err(e) => {
                            parse_err = Some(e.to_string());
                            None
                        }
                    }
                } else {
                    None
                };

                let mut validation_errors = Vec::new();
                let mut validation_warnings = Vec::new();
                for issue in &append_result.validation.issues {
                    let msg = format!("[Compás {}] {}", issue.measure_index, issue.message);
                    match issue.severity {
                        ValidationSeverity::Error => validation_errors.push(msg),
                        ValidationSeverity::Warning => validation_warnings.push(msg),
                    }
                }

                let has_errors = !validation_errors.is_empty();
                let has_warnings = !validation_warnings.is_empty();
                let status = if has_errors {
                    "failed".to_string()
                } else if has_warnings {
                    "warning".to_string()
                } else {
                    "succeeded".to_string()
                };

                let abstract_summary = if let Some(ref payload) = payload_opt {
                    let summary = compute_abstract_summary(
                        payload,
                        &append_result.validation.issues,
                        calibration.as_ref(),
                        request.target_level,
                    );

                    generated_so_far_summary.push(serde_json::json!({
                        "section_id": node.section_id.clone(),
                        "density_band": summary.approximate_density_band.clone(),
                        "primary_family": summary.primary_family.clone(),
                        "issue_count": summary.issue_count,
                    }));
                    Some(summary)
                } else {
                    None
                };

                if has_errors {
                    sections_failed += 1;
                } else {
                    sections_generated += 1;
                }

                let item_err = parse_err.or_else(|| {
                    if has_errors {
                        Some("Biomechanical validation detected errors.".to_string())
                    } else {
                        None
                    }
                });

                let cont_summary = append_result.continuity_plan.as_ref().and_then(|p| {
                    p.sections
                        .iter()
                        .find(|s| s.section_id == node.section_id)
                        .map(|n| {
                            let prev_n = if n.section_index > 0 {
                                p.sections
                                    .get(n.section_index - 1)
                                    .map(|prev| NeighborSummary {
                                        section_id: prev.section_id.clone(),
                                        music_role: prev.music_role.clone(),
                                        piu_role: prev.piu_role.clone(),
                                        intensity_band: prev.intensity_band.clone(),
                                        primary_family: prev.primary_pattern_family.clone(),
                                    })
                            } else {
                                None
                            };
                            let next_n = if n.section_index + 1 < p.sections.len() {
                                p.sections
                                    .get(n.section_index + 1)
                                    .map(|next| NeighborSummary {
                                        section_id: next.section_id.clone(),
                                        music_role: next.music_role.clone(),
                                        piu_role: next.piu_role.clone(),
                                        intensity_band: next.intensity_band.clone(),
                                        primary_family: next.primary_pattern_family.clone(),
                                    })
                            } else {
                                None
                            };
                            let mut warnings = p.warnings.clone();
                            warnings.extend(n.warnings.clone());
                            ContinuityContextSummary {
                                enabled: n.enabled,
                                section_index: n.section_index,
                                section_count: p.section_count,
                                global_arc: p.global_arc.arc_type.clone(),
                                current_motif_strategy: n.motif_strategy.clone(),
                                transition_in: Some(n.transition_in.clone()),
                                transition_out: Some(n.transition_out.clone()),
                                neighbor_summary: NeighborSummaryGroup {
                                    previous: prev_n,
                                    next: next_n,
                                },
                                warnings,
                                current_primary_pattern_family: n.primary_pattern_family.clone(),
                                current_secondary_pattern_families: n
                                    .secondary_pattern_families
                                    .clone(),
                                current_avoid_pattern_families: n.avoid_pattern_families.clone(),
                                current_intensity_band: n.intensity_band.clone(),
                                current_density_intent: n.density_intent.clone(),
                                current_confidence: n.confidence.clone(),
                                current_notes: n.notes.clone(),
                            }
                        })
                });

                items.push(SectionPreviewQueueItem {
                    section_id: node.section_id.clone(),
                    section_index: node.section_index,
                    status,
                    range_summary: format!("{}-{}", node.start_measure, node.end_measure),
                    continuity_summary: cont_summary,
                    pattern_family_targeting: append_result.pattern_family_targeting.clone(),
                    calibration_summary: append_result.calibration_context_summary.clone(),
                    preview_payload: payload_opt,
                    validation_errors,
                    validation_warnings,
                    generated_abstract_summary: abstract_summary,
                    error_message: item_err,
                });
            }
            Err(err_msg) => {
                sections_failed += 1;
                items.push(SectionPreviewQueueItem {
                    section_id: node.section_id.clone(),
                    section_index: node.section_index,
                    status: "failed".to_string(),
                    range_summary: format!("{}-{}", node.start_measure, node.end_measure),
                    continuity_summary: None,
                    pattern_family_targeting: None,
                    calibration_summary: None,
                    preview_payload: None,
                    validation_errors: Vec::new(),
                    validation_warnings: Vec::new(),
                    generated_abstract_summary: None,
                    error_message: Some(err_msg),
                });
            }
        }
    }

    let fingerprint_after_res = get_file_fingerprint(request.ssc_path.clone());
    let (fingerprint_after, fp_final_match) = match fingerprint_after_res {
        Ok(fp_after) => {
            let matches = fingerprint_before.sha256 == fp_after.sha256;
            (Some(fp_after), matches)
        }
        Err(_) => (None, false),
    };

    if !fp_final_match {
        fingerprint_match = false;
        unsafe_to_append = true;
    }

    let final_status = derive_multi_section_session_status(
        unsafe_to_append,
        fingerprint_match,
        sections_failed,
        sections_generated,
        &items,
    );

    Ok(MultiSectionGenerationResult {
        schema_version: "v0".to_string(),
        written: false,
        preview_only: true,
        session_id,
        target_level: request.target_level,
        sections_requested: request.selected_section_ids.len(),
        sections_generated,
        sections_failed,
        session_status: final_status,
        fingerprint_before: Some(fingerprint_before),
        fingerprint_after,
        fingerprint_match,
        unsafe_to_append,
        items,
        warnings: Vec::new(),
        errors: Vec::new(),
    })
}

pub async fn generate_gemini_multi_section_preview_queue_impl<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    passphrase: String,
    request: MultiSectionGenerationRequest,
) -> Result<MultiSectionGenerationResult, String> {
    let api_key =
        crate::credentials::decrypt_stored_api_key(&app_handle, &passphrase).map_err(|e| {
            format!(
                "Error al descifrar la API Key (verifique la contraseña): {}",
                e
            )
        })?;

    #[cfg(test)]
    let client = {
        let base_url = std::env::var("AI_STEP_GEN_MOCK_BASE_URL").ok();
        crate::gemini::GeminiClient::new(base_url)
    };
    #[cfg(not(test))]
    let client = crate::gemini::GeminiClient::new();

    generate_gemini_multi_section_preview_queue_core(&api_key, &client, request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music_analysis::{ChoreographicIntentMap, SectionFrame, SongAnalysisReport};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn get_fixture_path() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("src");
        p.push("ssc");
        p.push("test_fixtures");
        p.push("mini_sample.ssc");
        p
    }

    fn write_mock_analysis_report(ssc_path: &Path) {
        let dir = ssc_path.parent().unwrap();
        let analysis_dir = dir.join(".ai-step-gen-analysis");
        std::fs::create_dir_all(&analysis_dir).unwrap();
        let report = SongAnalysisReport {
            schema_version: "v1".to_string(),
            song_id: "test_song".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 120.0,
            audio_summary: crate::music_analysis::AudioSummary {
                sample_rate: 44100,
                detected_bpm: 120.0,
                rms_energy_mean: 0.5,
                rms_energy_max: 0.8,
                spectral_centroid_mean: 1000.0,
                spectral_flatness_mean: 0.1,
                zero_crossing_rate_mean: 0.05,
                chroma_mean: None,
                spectral_contrast_mean: None,
                analysis_mode: "offline".to_string(),
            },
            timing_grid: crate::music_analysis::TimingGrid {
                initial_offset: 0.0,
                bpms: vec![(0.0, 120.0)],
                display_bpm: "120".to_string(),
                song_type: "Arcade".to_string(),
            },
            event_features: crate::music_analysis::EventFeatures { beats: vec![] },
            sections: vec![
                SectionFrame {
                    section_id: "sec1".to_string(),
                    start_beat: 0.0,
                    end_beat: 16.0,
                    start_measure: 0,
                    end_measure: 4,
                    music_role: "intro".to_string(),
                    piu_role: "warmup".to_string(),
                    boundary_confidence: 0.9,
                    energy_profile: "low".to_string(),
                },
                SectionFrame {
                    section_id: "sec2".to_string(),
                    start_beat: 16.0,
                    end_beat: 32.0,
                    start_measure: 4,
                    end_measure: 8,
                    music_role: "verse".to_string(),
                    piu_role: "stream_opportunity".to_string(),
                    boundary_confidence: 0.9,
                    energy_profile: "mid".to_string(),
                },
                SectionFrame {
                    section_id: "sec3".to_string(),
                    start_beat: 32.0,
                    end_beat: 48.0,
                    start_measure: 8,
                    end_measure: 12,
                    music_role: "chorus".to_string(),
                    piu_role: "climax_run".to_string(),
                    boundary_confidence: 0.9,
                    energy_profile: "high".to_string(),
                },
            ],
            choreographic_intent: vec![
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec1".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 0,
                    measure_end: 4,
                    density_target: "light".to_string(),
                    difficulty_budget: 8.0,
                    recommended_pattern_families: vec!["balanced".to_string()],
                    avoid_pattern_families: vec![],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "introduce".to_string(),
                    evidence: vec![],
                    confidence: 0.8,
                },
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec2".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 4,
                    measure_end: 8,
                    density_target: "moderate".to_string(),
                    difficulty_budget: 10.0,
                    recommended_pattern_families: vec!["stream".to_string()],
                    avoid_pattern_families: vec!["twist_technical".to_string()],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "develop".to_string(),
                    evidence: vec![],
                    confidence: 0.8,
                },
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec3".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 8,
                    measure_end: 12,
                    density_target: "heavy".to_string(),
                    difficulty_budget: 12.0,
                    recommended_pattern_families: vec!["stream".to_string(), "stamina".to_string()],
                    avoid_pattern_families: vec![],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "intensify".to_string(),
                    evidence: vec![],
                    confidence: 0.8,
                },
            ],
            diagnostics: crate::music_analysis::TimingDiagnostics {
                audio_bpm_detected: 120.0,
                ssc_initial_bpm: 120.0,
                audio_vs_ssc_tempo_agreement: true,
                beat_grid_error_ms_mean: 0.0,
                timing_confidence: 1.0,
                requires_manual_timing_review: false,
                warnings: vec![],
                analysis_mode: "offline".to_string(),
            },
            publicability: crate::music_analysis::Publicability {
                contains_original_audio: false,
                contains_full_chart: false,
                exportable: true,
            },
        };
        let content = serde_json::to_string(&report).unwrap();
        std::fs::write(analysis_dir.join("song-analysis-report.v1.json"), content).unwrap();
    }

    #[tokio::test]
    async fn test_multi_section_rejects_empty_selection() {
        let req = MultiSectionGenerationRequest {
            ssc_path: "dummy.ssc".to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec![],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };
        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("vacía"));
    }

    #[tokio::test]
    async fn test_multi_section_rejects_too_many_sections() {
        let req = MultiSectionGenerationRequest {
            ssc_path: "dummy.ssc".to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec![
                "sec1".to_string(),
                "sec2".to_string(),
                "sec3".to_string(),
                "sec4".to_string(),
                "sec5".to_string(),
            ],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: Some(4),
        };
        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("excedido el límite máximo"));
    }

    #[tokio::test]
    async fn test_multi_section_rejects_disabled_section() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_disabled_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let overrides = vec![SectionPlanOverride {
            section_id: "sec2".to_string(),
            enabled: Some(false),
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string(), "sec2".to_string()],
            overrides: Some(overrides),
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(
            res.is_err(),
            "Should fail since sec2 is disabled via override"
        );
        assert!(res.unwrap_err().contains("deshabilitada"));

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_rejects_unknown_section() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_unknown_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec_nonexistent".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("no se encuentra en el plan"));

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_rejects_invalid_ranges() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_ranges_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();

        // Let's write a report where one section has invalid bounds (start >= end)
        let dir = temp_ssc_path.parent().unwrap();
        let analysis_dir = dir.join(".ai-step-gen-analysis");
        std::fs::create_dir_all(&analysis_dir).unwrap();
        let report = SongAnalysisReport {
            schema_version: "v1".to_string(),
            song_id: "test_song".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 120.0,
            audio_summary: crate::music_analysis::AudioSummary {
                sample_rate: 44100,
                detected_bpm: 120.0,
                rms_energy_mean: 0.5,
                rms_energy_max: 0.8,
                spectral_centroid_mean: 1000.0,
                spectral_flatness_mean: 0.1,
                zero_crossing_rate_mean: 0.05,
                chroma_mean: None,
                spectral_contrast_mean: None,
                analysis_mode: "offline".to_string(),
            },
            timing_grid: crate::music_analysis::TimingGrid {
                initial_offset: 0.0,
                bpms: vec![(0.0, 120.0)],
                display_bpm: "120".to_string(),
                song_type: "Arcade".to_string(),
            },
            event_features: crate::music_analysis::EventFeatures { beats: vec![] },
            sections: vec![SectionFrame {
                section_id: "sec1".to_string(),
                start_beat: 0.0,
                end_beat: 16.0,
                start_measure: 10,
                end_measure: 5, // start > end
                music_role: "intro".to_string(),
                piu_role: "warmup".to_string(),
                boundary_confidence: 0.9,
                energy_profile: "low".to_string(),
            }],
            choreographic_intent: vec![ChoreographicIntentMap {
                schema_version: "v1".to_string(),
                section_id: "sec1".to_string(),
                mode: "Single".to_string(),
                target_level: 10,
                measure_start: 10,
                measure_end: 5,
                density_target: "light".to_string(),
                difficulty_budget: 8.0,
                recommended_pattern_families: vec!["balanced".to_string()],
                avoid_pattern_families: vec![],
                accent_plan: vec![],
                rest_plan: vec![],
                motif_strategy: "introduce".to_string(),
                evidence: vec![],
                confidence: 0.8,
            }],
            diagnostics: crate::music_analysis::TimingDiagnostics {
                audio_bpm_detected: 120.0,
                ssc_initial_bpm: 120.0,
                audio_vs_ssc_tempo_agreement: true,
                beat_grid_error_ms_mean: 0.0,
                timing_confidence: 1.0,
                requires_manual_timing_review: false,
                warnings: vec![],
                analysis_mode: "offline".to_string(),
            },
            publicability: crate::music_analysis::Publicability {
                contains_original_audio: false,
                contains_full_chart: false,
                exportable: true,
            },
        };
        let content = serde_json::to_string(&report).unwrap();
        std::fs::write(analysis_dir.join("song-analysis-report.v1.json"), content).unwrap();

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("límites de compás inválidos"));

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_runs_sequentially_with_mock_gemini() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "AI_STEP_GEN_MOCK_BASE_URL",
            format!("http://127.0.0.1:{}", port),
        );
        crate::settings::set_test_gemini_enabled(Some(true));

        let captured_requests = Arc::new(Mutex::new(Vec::new()));
        let cap_clone = captured_requests.clone();

        std::thread::spawn(move || {
            // We expect 2 sequential requests
            for _ in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0; 65536];
                    if let Ok(bytes_read) = stream.read(&mut buffer) {
                        let req_str = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
                        if let Some(body_start) = req_str.find("\r\n\r\n") {
                            let body = req_str[body_start + 4..].to_string();
                            cap_clone.lock().unwrap().push(body);
                        }
                    }
                    let last_req = cap_clone
                        .lock()
                        .unwrap()
                        .last()
                        .cloned()
                        .unwrap_or_default();
                    let is_sec2 = last_req.contains("4 a 8");
                    let section_id = if is_sec2 { "sec2" } else { "sec1" };
                    let start_measure = if section_id == "sec2" { 4 } else { 0 };
                    let end_measure = if section_id == "sec2" { 8 } else { 4 };
                    let mut measures_json = Vec::new();
                    for idx in start_measure..=end_measure {
                        measures_json.push(format!(
                            "{{\\\"measure_index\\\": {}, \\\"subdivision\\\": 4, \\\"rows\\\": [\\\"00000\\\", \\\"00000\\\", \\\"00000\\\", \\\"00000\\\"]}}",
                            idx
                        ));
                    }
                    let measures_str = measures_json.join(", ");

                    let resp_fmt = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{{\
                        \"candidates\": [\
                            {{\
                                \"content\": {{\
                                    \"parts\": [\
                                        {{\
                                             \"text\": \"{{\\n  \\\"section_id\\\": \\\"{}\\\",\\n  \\\"difficulty_level\\\": 10,\\n  \\\"play_mode\\\": \\\"Single\\\",\\n  \\\"biomechanical_state\\\": {{\\n    \\\"current_twist_debt\\\": 0.0,\\n    \\\"current_stamina_debt\\\": 0.0\\n  }},\\n  \\\"measures\\\": [ {} ]\\n}}\"\
                                         }}\
                                    ]\
                                }}\
                             }}\
                        ]\
                    }}", section_id, measures_str);

                    let _ = stream.write_all(resp_fmt.as_bytes());
                    let _ = stream.flush();
                }
            }
        });

        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_seq_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let temp_audio_path = test_root.join("dummy_audio.mp3");
        std::fs::write(&temp_audio_path, b"dummy audio contents").unwrap();

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: Some(temp_audio_path.to_string_lossy().to_string()),
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string(), "sec2".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(Some(format!("http://127.0.0.1:{}", port)));
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req)
            .await
            .unwrap();

        assert_eq!(res.sections_requested, 2);
        assert_eq!(res.sections_generated, 2);
        assert_eq!(res.sections_failed, 0);
        assert_eq!(res.session_status, "succeeded");
        assert_eq!(res.written, false);
        assert_eq!(res.preview_only, true);
        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items[0].status, "succeeded");
        assert_eq!(res.items[1].status, "succeeded");

        // Verify sliding window prompt contents
        let requests = captured_requests.lock().unwrap();
        assert_eq!(requests.len(), 2);

        // Verify that the second request includes the first generated summary in generated_so_far_summary
        let first_body = &requests[0];
        let second_body = &requests[1];

        // Check sliding window structure exists
        assert!(second_body.contains("sliding_window_context"));
        assert!(second_body.contains("generated_so_far_summary"));
        // First body's generated_so_far_summary should be empty
        assert!(first_body.contains("\\\"generated_so_far_summary\\\": []"));
        // Second body's generated_so_far_summary should contain sec1
        assert!(second_body.contains("\\\"section_id\\\": \\\"sec1\\\""));
        assert!(second_body.contains("density_band"));

        // Confirm sliding window context contains previous, current, next Neighbors
        assert!(second_body.contains("\\\"previous_section\\\": {"));
        assert!(second_body.contains("\\\"current_section\\\": {"));
        assert!(second_body.contains("\\\"next_section\\\": {"));

        // Confirm NO raw notes are leaked in generated_so_far_summary
        let second_val: serde_json::Value = serde_json::from_str(second_body).unwrap();
        let prompt_text = second_val["contents"][0]["parts"][0]["text"]
            .as_str()
            .unwrap();
        let gen_summary_start = prompt_text.find("generated_so_far_summary").unwrap();
        let gen_summary_part = &prompt_text[gen_summary_start..gen_summary_start + 300];
        assert!(!gen_summary_part.contains("rows"));
        assert!(!gen_summary_part.contains("measures"));

        // Confirm NO private paths are in prompt
        assert!(!prompt_text.contains(".ai-step-gen-private-datasets"));
        assert!(!prompt_text.contains("official_songs"));

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_returns_partial_results_on_item_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "AI_STEP_GEN_MOCK_BASE_URL",
            format!("http://127.0.0.1:{}", port),
        );
        crate::settings::set_test_gemini_enabled(Some(true));

        std::thread::spawn(move || {
            // 1st request succeeds, 2nd fails (returns error/400)
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0; 65536];
                let _ = stream.read(&mut buffer);
                let mut measures_json = Vec::new();
                for idx in 0..=4 {
                    measures_json.push(format!(
                        "{{\\\"measure_index\\\": {}, \\\"subdivision\\\": 4, \\\"rows\\\": [\\\"00000\\\", \\\"00000\\\", \\\"00000\\\", \\\"00000\\\"]}}",
                        idx
                    ));
                }
                let measures_str = measures_json.join(", ");
                let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{{\
                    \"candidates\": [\
                        {{\
                            \"content\": {{\
                                \"parts\": [\
                                    {{\
                                         \"text\": \"{{\\n  \\\"section_id\\\": \\\"sec1\\\",\\n  \\\"difficulty_level\\\": 10,\\n  \\\"play_mode\\\": \\\"Single\\\",\\n  \\\"biomechanical_state\\\": {{\\n    \\\"current_twist_debt\\\": 0.0,\\n    \\\"current_stamina_debt\\\": 0.0\\n  }},\\n  \\\"measures\\\": [ {} ]\\n}}\"\
                                     }}\
                                ]\
                            }}\
                         }}\
                    ]\
                }}", measures_str);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }

            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0; 65536];
                let _ = stream.read(&mut buffer);
                let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_partial_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let temp_audio_path = test_root.join("dummy_audio.mp3");
        std::fs::write(&temp_audio_path, b"dummy audio").unwrap();

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: Some(temp_audio_path.to_string_lossy().to_string()),
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string(), "sec2".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(Some(format!("http://127.0.0.1:{}", port)));
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req)
            .await
            .unwrap();

        assert_eq!(res.sections_requested, 2);
        assert_eq!(res.sections_generated, 1);
        assert_eq!(res.sections_failed, 1);
        assert_eq!(res.session_status, "partial_success");
        assert_eq!(res.items[0].status, "succeeded");
        assert_eq!(res.items[1].status, "failed");
        assert!(res.items[1].error_message.is_some());

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_stops_on_fingerprint_mismatch() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "AI_STEP_GEN_MOCK_BASE_URL",
            format!("http://127.0.0.1:{}", port),
        );
        crate::settings::set_test_gemini_enabled(Some(true));

        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_fingerprint_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let temp_ssc_clone = temp_ssc_path.clone();

        std::thread::spawn(move || {
            // First request succeeds
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0; 65536];
                let _ = stream.read(&mut buffer);
                let mut measures_json = Vec::new();
                for idx in 0..=4 {
                    measures_json.push(format!(
                        "{{\\\"measure_index\\\": {}, \\\"subdivision\\\": 4, \\\"rows\\\": [\\\"00000\\\", \\\"00000\\\", \\\"00000\\\", \\\"00000\\\"]}}",
                        idx
                    ));
                }
                let measures_str = measures_json.join(", ");
                let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{{\
                    \"candidates\": [\
                        {{\
                            \"content\": {{\
                                \"parts\": [\
                                    {{\
                                         \"text\": \"{{\\n  \\\"section_id\\\": \\\"sec1\\\",\\n  \\\"difficulty_level\\\": 10,\\n  \\\"play_mode\\\": \\\"Single\\\",\\n  \\\"biomechanical_state\\\": {{\\n    \\\"current_twist_debt\\\": 0.0,\\n    \\\"current_stamina_debt\\\": 0.0\\n  }},\\n  \\\"measures\\\": [ {} ]\\n}}\"\
                                     }}\
                                ]\
                            }}\
                         }}\
                    ]\
                }}", measures_str);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();

                // Mutate the ssc file dynamically behind the orchestrator's back!
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&temp_ssc_clone)
                    .unwrap();
                writeln!(file, "// some comment to change fingerprint").unwrap();
            }
        });

        let temp_audio_path = test_root.join("dummy_audio.mp3");
        std::fs::write(&temp_audio_path, b"dummy audio").unwrap();

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: Some(temp_audio_path.to_string_lossy().to_string()),
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string(), "sec2".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(Some(format!("http://127.0.0.1:{}", port)));
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req)
            .await
            .unwrap();

        assert_eq!(res.sections_requested, 2);
        assert_eq!(res.sections_generated, 1);
        assert_eq!(res.session_status, "unsafe_to_append");
        assert_eq!(res.fingerprint_match, false);
        assert_eq!(res.unsafe_to_append, true);
        assert_eq!(res.items[0].status, "succeeded");
        assert_eq!(res.items[1].status, "skipped");

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_rejects_max_sections_overflow() {
        let req = MultiSectionGenerationRequest {
            ssc_path: "dummy.ssc".to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: Some(99),
        };
        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("supera el límite"));
    }

    #[tokio::test]
    async fn test_multi_section_fails_early_on_nonexistent_file() {
        let req = MultiSectionGenerationRequest {
            ssc_path: "/nonexistent/file.ssc".to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };
        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("El archivo .ssc especificado no existe"));
    }

    #[tokio::test]
    async fn test_multi_section_fails_early_on_fingerprint_calculation_error() {
        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_fp_fail_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        // Create an existing file but with a non-.ssc extension (e.g. .txt) so that get_file_fingerprint fails.
        let temp_txt_path = test_root.join("test_song.txt");
        std::fs::write(&temp_txt_path, b"dummy content").unwrap();
        write_mock_analysis_report(&temp_txt_path);

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_txt_path.to_string_lossy().to_string(),
            audio_path: None,
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };
        let client = crate::gemini::GeminiClient::new(None);
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req).await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("Error al calcular el fingerprint inicial"));
        assert!(err_msg.contains(".ssc"));

        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[tokio::test]
    async fn test_multi_section_fingerprint_mismatch_only_at_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::env::set_var(
            "AI_STEP_GEN_MOCK_BASE_URL",
            format!("http://127.0.0.1:{}", port),
        );
        crate::settings::set_test_gemini_enabled(Some(true));

        let temp_dir = std::env::temp_dir();
        let test_root = temp_dir.join(format!(
            "multi_section_test_fp_end_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&test_root).unwrap();
        let temp_ssc_path = test_root.join("test_song.ssc");
        std::fs::copy(&get_fixture_path(), &temp_ssc_path).unwrap();
        write_mock_analysis_report(&temp_ssc_path);

        let temp_ssc_clone = temp_ssc_path.clone();

        std::thread::spawn(move || {
            // We expect 2 sequential requests.
            // Both succeed.
            for idx in 0..2 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0; 65536];
                    let _ = stream.read(&mut buffer);
                    let start_measure = if idx == 1 { 4 } else { 0 };
                    let end_measure = if idx == 1 { 8 } else { 4 };
                    let section_id = if idx == 1 { "sec2" } else { "sec1" };
                    let mut measures_json = Vec::new();
                    for m_idx in start_measure..=end_measure {
                        measures_json.push(format!(
                            "{{\\\"measure_index\\\": {}, \\\"subdivision\\\": 4, \\\"rows\\\": [\\\"00000\\\", \\\"00000\\\", \\\"00000\\\", \\\"00000\\\"]}}",
                            m_idx
                        ));
                    }
                    let measures_str = measures_json.join(", ");
                    let response = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{{\
                        \"candidates\": [\
                            {{\
                                \"content\": {{\
                                    \"parts\": [\
                                        {{\
                                             \"text\": \"{{\\n  \\\"section_id\\\": \\\"{}\\\",\\n  \\\"difficulty_level\\\": 10,\\n  \\\"play_mode\\\": \\\"Single\\\",\\n  \\\"biomechanical_state\\\": {{\\n    \\\"current_twist_debt\\\": 0.0,\\n    \\\"current_stamina_debt\\\": 0.0\\n  }},\\n  \\\"measures\\\": [ {} ]\\n}}\"\
                                         }}\
                                    ]\
                                }}\
                             }}\
                        ]\
                    }}", section_id, measures_str);
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();

                    // If this is the 2nd request, mutate the ssc file dynamically after flushing the response!
                    if idx == 1 {
                        let mut file = std::fs::OpenOptions::new()
                            .append(true)
                            .open(&temp_ssc_clone)
                            .unwrap();
                        writeln!(
                            file,
                            "// some comment to change fingerprint at the very end"
                        )
                        .unwrap();
                    }
                }
            }
        });

        let temp_audio_path = test_root.join("dummy_audio.mp3");
        std::fs::write(&temp_audio_path, b"dummy audio").unwrap();

        let req = MultiSectionGenerationRequest {
            ssc_path: temp_ssc_path.to_string_lossy().to_string(),
            audio_path: Some(temp_audio_path.to_string_lossy().to_string()),
            target_level: 10,
            selected_section_ids: vec!["sec1".to_string(), "sec2".to_string()],
            overrides: None,
            use_calibrated_prompt_context: true,
            use_continuity_planning: true,
            pattern_family_target: None,
            max_sections: None,
        };

        let client = crate::gemini::GeminiClient::new(Some(format!("http://127.0.0.1:{}", port)));
        let res = generate_gemini_multi_section_preview_queue_core("dummy_key", &client, req)
            .await
            .unwrap();

        // The mismatch should only be detected at the end of the orchestration.
        // Both items should be generated successfully.
        assert_eq!(res.sections_requested, 2);
        assert_eq!(res.sections_generated, 2);
        assert_eq!(res.sections_failed, 0);
        assert_eq!(res.session_status, "unsafe_to_append");
        assert_eq!(res.fingerprint_match, false);
        assert_eq!(res.unsafe_to_append, true);
        assert_eq!(res.written, false);
        assert_eq!(res.preview_only, true);
        assert_eq!(res.items.len(), 2);
        assert_eq!(res.items[0].status, "succeeded");
        assert_eq!(res.items[1].status, "succeeded");

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_dir_all(&test_root);
    }
}
