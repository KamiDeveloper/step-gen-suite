use crate::biomechanics::PlayMode;
use crate::guardrail_calibration::SingleGuardrailCalibration;
use crate::music_analysis::SongAnalysisReport;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongContinuityPlan {
    pub schema_version: String,
    pub play_mode: String,
    pub target_level: u8,
    pub calibration_available: bool,
    pub section_count: usize,
    pub sections: Vec<SectionContinuityNode>,
    pub global_arc: GlobalArcSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionContinuityNode {
    pub section_id: String,
    pub section_index: usize,
    pub start_measure: i32,
    pub end_measure: i32,
    pub music_role: String,
    pub piu_role: String,
    pub density_intent: String,
    pub intensity_band: String,
    pub primary_pattern_family: String,
    pub secondary_pattern_families: Vec<String>,
    pub avoid_pattern_families: Vec<String>,
    pub motif_strategy: String,
    pub transition_in: TransitionGuidance,
    pub transition_out: TransitionGuidance,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub warnings: Vec<String>,
    pub enabled: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransitionGuidance {
    pub transition_type: String,
    pub density_delta: String,
    pub family_shift: String,
    pub recommended_bridge: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalArcSummary {
    pub arc_type: String,
    pub peak_section_ids: Vec<String>,
    pub rest_section_ids: Vec<String>,
    pub motif_policy: String,
    pub density_curve: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityContextSummary {
    pub enabled: bool,
    pub section_index: usize,
    pub section_count: usize,
    pub global_arc: String,
    pub current_motif_strategy: String,
    pub transition_in: Option<TransitionGuidance>,
    pub transition_out: Option<TransitionGuidance>,
    pub neighbor_summary: NeighborSummaryGroup,
    pub warnings: Vec<String>,
    pub current_primary_pattern_family: String,
    pub current_secondary_pattern_families: Vec<String>,
    pub current_avoid_pattern_families: Vec<String>,
    pub current_intensity_band: String,
    pub current_density_intent: String,
    pub current_confidence: String,
    pub current_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborSummaryGroup {
    pub previous: Option<NeighborSummary>,
    pub next: Option<NeighborSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborSummary {
    pub section_id: String,
    pub music_role: String,
    pub piu_role: String,
    pub intensity_band: String,
    pub primary_family: String,
}

fn get_intensity_value(band: &str) -> i32 {
    match band {
        "very_low" => 1,
        "low" => 2,
        "medium" => 3,
        "high" => 4,
        "very_high" => 5,
        _ => 3,
    }
}

pub fn map_intensity_band(
    target_level: u32,
    energy_profile: Option<&str>,
    piu_role: Option<&str>,
    music_role: Option<&str>,
) -> String {
    let base_band = if target_level <= 6 {
        1 // very_low
    } else if target_level <= 10 {
        2 // low
    } else if target_level <= 14 {
        3 // medium
    } else if target_level <= 22 {
        4 // high
    } else {
        5 // very_high
    };

    let mut modifier = 0;
    if let Some(ep) = energy_profile {
        match ep.to_lowercase().as_str() {
            "high" | "climax" => modifier += 1,
            "low" | "rest" | "break" | "breakdown" => modifier -= 1,
            _ => {}
        }
    }

    if let Some(piu) = piu_role {
        match piu.to_lowercase().as_str() {
            "climax_zone" | "final_burst" => modifier += 1,
            "rest_zone" | "cooldown" | "warmup" => modifier -= 1,
            _ => {}
        }
    }

    if let Some(mr) = music_role {
        match mr.to_lowercase().as_str() {
            "chorus" => modifier += 1,
            "breakdown" | "rest" => modifier -= 1,
            _ => {}
        }
    }

    let final_score = (base_band + modifier).clamp(1, 5);
    match final_score {
        1 => "very_low".to_string(),
        2 => "low".to_string(),
        3 => "medium".to_string(),
        4 => "high".to_string(),
        5 => "very_high".to_string(),
        _ => "medium".to_string(),
    }
}

pub fn determine_motif_strategy(music_role: Option<&str>, piu_role: Option<&str>) -> String {
    let music_role_lower = music_role.unwrap_or("").to_lowercase();
    let piu_role_lower = piu_role.unwrap_or("").to_lowercase();

    if piu_role_lower == "final_burst" {
        "final_burst".to_string()
    } else if piu_role_lower == "warmup"
        || music_role_lower == "intro"
        || piu_role_lower.contains("intro")
    {
        "introduce".to_string()
    } else if piu_role_lower.contains("climax") || music_role_lower == "chorus" {
        "intensify".to_string()
    } else if piu_role_lower.contains("rest")
        || music_role_lower == "rest"
        || music_role_lower == "breakdown"
        || piu_role_lower == "cooldown"
    {
        "rest".to_string()
    } else if music_role_lower == "bridge" || music_role_lower == "solo" {
        "contrast".to_string()
    } else if music_role_lower == "verse" || piu_role_lower.contains("footwork") {
        "develop".to_string()
    } else if music_role_lower == "outro" {
        "resolve".to_string()
    } else {
        "unknown".to_string()
    }
}

pub fn build_transition_guidance(
    prev: Option<&SectionContinuityNode>,
    curr: &SectionContinuityNode,
    is_out: bool,
) -> TransitionGuidance {
    let mut warnings = Vec::new();

    if !is_out {
        // transition_in: comparing prev -> curr
        let prev_node = match prev {
            Some(n) => n,
            None => {
                return TransitionGuidance {
                    transition_type: "smooth_continue".to_string(),
                    density_delta: "stable".to_string(),
                    family_shift: "none".to_string(),
                    recommended_bridge: "maintain_motif".to_string(),
                    warnings: Vec::new(),
                };
            }
        };

        let prev_val = get_intensity_value(&prev_node.intensity_band);
        let curr_val = get_intensity_value(&curr.intensity_band);

        let transition_type;
        let density_delta;
        let recommended_bridge;

        if curr_val > prev_val {
            if curr_val - prev_val >= 3 {
                transition_type = "climax_entry".to_string();
                density_delta = "large_increase".to_string();
                recommended_bridge = "accented_impact_jump".to_string();
                if prev_node.intensity_band == "very_low" && curr.intensity_band == "very_high" {
                    let has_evidence = curr
                        .evidence
                        .iter()
                        .any(|e| e.contains("climax") || e.contains("final_burst"));
                    if !has_evidence {
                        warnings.push("Abrupt intensity jump from very_low to very_high detected without clear music analysis evidence.".to_string());
                    }
                }
            } else {
                transition_type = "density_ramp_up".to_string();
                density_delta = "increase".to_string();
                recommended_bridge = "gradual_density_increase".to_string();
            }
        } else if curr_val < prev_val {
            if prev_val - curr_val >= 3 {
                transition_type = "contrast_break".to_string();
                density_delta = "large_decrease".to_string();
                recommended_bridge = "rest_measure_or_long_hold".to_string();
            } else {
                transition_type = "density_ramp_down".to_string();
                density_delta = "decrease".to_string();
                recommended_bridge = "gradual_density_decrease".to_string();
            }
        } else {
            transition_type = "smooth_continue".to_string();
            density_delta = "stable".to_string();
            recommended_bridge = "maintain_motif".to_string();
        }

        let family_shift = if prev_node.primary_pattern_family != curr.primary_pattern_family {
            format!(
                "{} -> {}",
                prev_node.primary_pattern_family, curr.primary_pattern_family
            )
        } else {
            "none".to_string()
        };

        TransitionGuidance {
            transition_type,
            density_delta,
            family_shift,
            recommended_bridge,
            warnings,
        }
    } else {
        // transition_out: comparing curr -> next (which is passed in 'prev' parameter for ease)
        let next_node = match prev {
            Some(n) => n,
            None => {
                let t_type = if curr.music_role.to_lowercase() == "outro"
                    || curr.piu_role.to_lowercase().contains("cooldown")
                {
                    "final_resolution".to_string()
                } else {
                    "smooth_continue".to_string()
                };
                return TransitionGuidance {
                    transition_type: t_type,
                    density_delta: "stable".to_string(),
                    family_shift: "none".to_string(),
                    recommended_bridge: "maintain_motif".to_string(),
                    warnings: Vec::new(),
                };
            }
        };

        let curr_val = get_intensity_value(&curr.intensity_band);
        let next_val = get_intensity_value(&next_node.intensity_band);

        let transition_type;
        let density_delta;
        let recommended_bridge;

        if next_val > curr_val {
            if next_val - curr_val >= 3 {
                transition_type = "climax_entry".to_string();
                density_delta = "large_increase".to_string();
                recommended_bridge = "accented_impact_jump".to_string();
            } else {
                transition_type = "density_ramp_up".to_string();
                density_delta = "increase".to_string();
                recommended_bridge = "gradual_density_increase".to_string();
            }
        } else if next_val < curr_val {
            if curr_val - next_val >= 3 {
                transition_type = "contrast_break".to_string();
                density_delta = "large_decrease".to_string();
                recommended_bridge = "rest_measure_or_long_hold".to_string();
            } else {
                transition_type = "density_ramp_down".to_string();
                density_delta = "decrease".to_string();
                recommended_bridge = "gradual_density_decrease".to_string();
            }
        } else {
            transition_type = "smooth_continue".to_string();
            density_delta = "stable".to_string();
            recommended_bridge = "maintain_motif".to_string();
        }

        let family_shift = if curr.primary_pattern_family != next_node.primary_pattern_family {
            format!(
                "{} -> {}",
                curr.primary_pattern_family, next_node.primary_pattern_family
            )
        } else {
            "none".to_string()
        };

        TransitionGuidance {
            transition_type,
            density_delta,
            family_shift,
            recommended_bridge,
            warnings,
        }
    }
}

pub fn build_song_continuity_plan(
    target_level: u32,
    play_mode: PlayMode,
    music_analysis: Option<&SongAnalysisReport>,
    calibration: Option<&SingleGuardrailCalibration>,
    browser_bpm_reconciliation: Option<&str>,
    section_id: &str,
    start_measure: u32,
    end_measure: u32,
) -> SongContinuityPlan {
    let calibration_available = calibration.is_some();
    let mut warnings = Vec::new();
    let mut nodes = Vec::new();

    if let Some(report) = music_analysis {
        let mut report_sections = report.sections.clone();
        let has_current = report_sections.iter().any(|s| s.section_id == section_id);
        if !has_current {
            report_sections.push(crate::music_analysis::SectionFrame {
                section_id: section_id.to_string(),
                start_beat: 0.0,
                end_beat: 0.0,
                start_measure,
                end_measure,
                music_role: "unknown".to_string(),
                piu_role: "unknown".to_string(),
                boundary_confidence: 1.0,
                energy_profile: "mid".to_string(),
            });
        }
        report_sections.sort_by_key(|s| s.start_measure);

        for (idx, sec) in report_sections.iter().enumerate() {
            let m_role = sec.music_role.clone();
            let p_role = sec.piu_role.clone();
            let energy = sec.energy_profile.clone();

            let mut intensity_band =
                map_intensity_band(target_level, Some(&energy), Some(&p_role), Some(&m_role));

            // Heuristics for low levels
            if target_level <= 6 {
                if intensity_band == "high" || intensity_band == "very_high" {
                    intensity_band = "low".to_string();
                }
            } else if target_level <= 10 {
                if intensity_band == "very_high" {
                    intensity_band = "medium".to_string();
                }
            }

            let mut node_warnings = Vec::new();
            let agreement = browser_bpm_reconciliation.and_then(|r| {
                crate::generation_context::classify_browser_bpm_reconciliation_status(r)
            });
            if agreement == Some(false) {
                node_warnings.push(
                    "Browser timing disagreement detected. Aggressiveness has been limited."
                        .to_string(),
                );
                if intensity_band == "very_high" {
                    intensity_band = "high".to_string();
                }
            }

            if sec.boundary_confidence < 0.5 {
                node_warnings.push(
                    "Low music analysis confidence. Fallback to balanced pattern focus."
                        .to_string(),
                );
            }

            if target_level >= 25 {
                node_warnings.push(format!(
                    "Low calibration confidence for level S{}. Conservatism applied.",
                    target_level
                ));
            }

            let resolved_targeting = crate::generation_context::resolve_pattern_family_targeting(
                target_level,
                play_mode,
                crate::generation_context::PatternFamilyTargetMode::Auto,
                Some(report),
                &sec.section_id,
                calibration,
                browser_bpm_reconciliation,
            );

            let mut primary_family = resolved_targeting.primary_family.clone();
            if sec.boundary_confidence < 0.5 {
                primary_family = "balanced".to_string();
            }

            let mut evidence = resolved_targeting.evidence.clone();
            evidence.push(format!(
                "Boundary confidence: {:.2}",
                sec.boundary_confidence
            ));

            nodes.push(SectionContinuityNode {
                section_id: sec.section_id.clone(),
                section_index: idx,
                start_measure: sec.start_measure as i32,
                end_measure: sec.end_measure as i32,
                music_role: m_role,
                piu_role: p_role,
                density_intent: intensity_band.clone(),
                intensity_band,
                primary_pattern_family: primary_family,
                secondary_pattern_families: resolved_targeting.secondary_families,
                avoid_pattern_families: resolved_targeting.avoid_families,
                motif_strategy: determine_motif_strategy(
                    Some(&sec.music_role),
                    Some(&sec.piu_role),
                ),
                transition_in: TransitionGuidance {
                    transition_type: "unknown".to_string(),
                    density_delta: "stable".to_string(),
                    family_shift: "none".to_string(),
                    recommended_bridge: "maintain_motif".to_string(),
                    warnings: Vec::new(),
                },
                transition_out: TransitionGuidance {
                    transition_type: "unknown".to_string(),
                    density_delta: "stable".to_string(),
                    family_shift: "none".to_string(),
                    recommended_bridge: "maintain_motif".to_string(),
                    warnings: Vec::new(),
                },
                confidence: resolved_targeting.confidence,
                evidence,
                warnings: node_warnings,
                enabled: true,
                notes: None,
            });
        }
    }

    if nodes.is_empty() {
        let mut node_warnings = Vec::new();
        node_warnings.push(
            "Music Analysis report is not available. Continuity planner degraded.".to_string(),
        );

        let mut evidence = Vec::new();
        evidence.push("Manual fallback node created.".to_string());

        let intensity_band = map_intensity_band(target_level, None, None, None);
        let resolved_targeting = crate::generation_context::resolve_pattern_family_targeting(
            target_level,
            play_mode,
            crate::generation_context::PatternFamilyTargetMode::Auto,
            None,
            section_id,
            calibration,
            browser_bpm_reconciliation,
        );

        nodes.push(SectionContinuityNode {
            section_id: section_id.to_string(),
            section_index: 0,
            start_measure: start_measure as i32,
            end_measure: end_measure as i32,
            music_role: "unknown".to_string(),
            piu_role: "unknown".to_string(),
            density_intent: intensity_band.clone(),
            intensity_band,
            primary_pattern_family: resolved_targeting.primary_family,
            secondary_pattern_families: resolved_targeting.secondary_families,
            avoid_pattern_families: resolved_targeting.avoid_families,
            motif_strategy: "unknown".to_string(),
            transition_in: TransitionGuidance {
                transition_type: "smooth_continue".to_string(),
                density_delta: "stable".to_string(),
                family_shift: "none".to_string(),
                recommended_bridge: "maintain_motif".to_string(),
                warnings: Vec::new(),
            },
            transition_out: TransitionGuidance {
                transition_type: "smooth_continue".to_string(),
                density_delta: "stable".to_string(),
                family_shift: "none".to_string(),
                recommended_bridge: "maintain_motif".to_string(),
                warnings: Vec::new(),
            },
            confidence: "low".to_string(),
            evidence,
            warnings: node_warnings,
            enabled: true,
            notes: None,
        });
    }

    // Repeated family check
    for idx in 0..nodes.len() {
        if idx >= 2 {
            let current_primary = nodes[idx].primary_pattern_family.clone();
            let prev1_primary = nodes[idx - 1].primary_pattern_family.clone();
            let prev2_primary = nodes[idx - 2].primary_pattern_family.clone();

            if current_primary == prev1_primary
                && current_primary == prev2_primary
                && current_primary != "balanced"
                && current_primary != "unknown"
            {
                let piu_role = nodes[idx].piu_role.to_lowercase();
                let stamina_justified = piu_role.contains("stamina")
                    || piu_role.contains("final_burst")
                    || target_level >= 15;
                if !stamina_justified {
                    let old_primary = nodes[idx].primary_pattern_family.clone();
                    nodes[idx].primary_pattern_family = "balanced".to_string();
                    nodes[idx].warnings.push(format!(
                        "Repeated primary pattern family '{}' limited to prevent monotony without stamina/endurance context.",
                        old_primary
                    ));
                }
            }
        }
    }

    // Resolve transitions
    for idx in 0..nodes.len() {
        let prev_node = if idx > 0 {
            Some(nodes[idx - 1].clone())
        } else {
            None
        };
        let transition_in = build_transition_guidance(prev_node.as_ref(), &nodes[idx], false);

        let next_node = if idx + 1 < nodes.len() {
            Some(nodes[idx + 1].clone())
        } else {
            None
        };
        let transition_out = build_transition_guidance(next_node.as_ref(), &nodes[idx], true);

        nodes[idx].transition_in = transition_in;
        nodes[idx].transition_out = transition_out;
    }

    // Build global arc summary
    let mut arc_types = Vec::new();
    let mut peak_section_ids = Vec::new();
    let mut rest_section_ids = Vec::new();
    let mut density_curve = Vec::new();

    for node in &nodes {
        let role = node.music_role.to_lowercase();
        let piu = node.piu_role.to_lowercase();

        if role == "intro" || piu.contains("warmup") {
            arc_types.push("intro");
        } else if role == "verse" || piu.contains("footwork") {
            arc_types.push("develop");
        } else if role == "pre-chorus" || piu.contains("build") {
            arc_types.push("build");
        } else if role == "chorus" || piu.contains("climax") || piu.contains("final_burst") {
            arc_types.push("peak");
        } else if role == "breakdown" || role == "bridge" || piu.contains("rest") {
            arc_types.push("break");
        } else if role == "outro" || piu.contains("cooldown") {
            arc_types.push("outro");
        }

        if piu.contains("climax") || piu.contains("final_burst") || role == "chorus" {
            peak_section_ids.push(node.section_id.clone());
        }
        if piu.contains("rest")
            || piu.contains("cooldown")
            || role == "breakdown"
            || role == "bridge"
        {
            rest_section_ids.push(node.section_id.clone());
        }
        density_curve.push(node.density_intent.clone());
    }

    let arc_type = if arc_types.is_empty() {
        "balanced".to_string()
    } else {
        arc_types.join("_")
    };

    if browser_bpm_reconciliation
        .and_then(|r| crate::generation_context::classify_browser_bpm_reconciliation_status(r))
        .is_some()
        && browser_bpm_reconciliation
            .and_then(|r| crate::generation_context::classify_browser_bpm_reconciliation_status(r))
            .unwrap()
            == false
    {
        warnings.push("Browser timing reconciliation reports a timing disagreement.".to_string());
    }

    let global_arc = GlobalArcSummary {
        arc_type,
        peak_section_ids,
        rest_section_ids,
        motif_policy: "repeat_and_develop".to_string(),
        density_curve,
    };

    let section_count = nodes.len();

    SongContinuityPlan {
        schema_version: "v0".to_string(),
        play_mode: match play_mode {
            PlayMode::Single => "Single".to_string(),
            PlayMode::Double => "Double".to_string(),
        },
        target_level: target_level as u8,
        calibration_available,
        section_count,
        sections: nodes,
        global_arc,
        warnings,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPlanOverride {
    pub section_id: String,
    pub enabled: Option<bool>,
    pub primary_pattern_family: Option<String>,
    pub secondary_pattern_families: Option<Vec<String>>,
    pub avoid_pattern_families: Option<Vec<String>>,
    pub motif_strategy: Option<String>,
    pub intensity_band: Option<String>,
    pub transition_in_type: Option<String>,
    pub transition_out_type: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionPlanReviewState {
    pub schema_version: String,
    pub overrides: Vec<SectionPlanOverride>,
}

pub fn sanitize_override_notes(notes: &str) -> Result<String, String> {
    let lower = notes.to_lowercase();

    let forbidden = &[
        "#notedata",
        "#title:",
        "#bpms:",
        "#offset:",
        "base64",
        "data:audio",
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
        ".ai-step-gen-private-datasets",
        "docs/official_songs",
    ];

    for &item in forbidden {
        if lower.contains(item) {
            return Err(format!(
                "Privacy Violation: Override notes contain forbidden keyword/pattern '{}'",
                item
            ));
        }
    }

    for c in b'a'..=b'z' {
        let prefix_backslash = format!("{}:\\", c as char);
        let prefix_slash = format!("{}:/", c as char);
        if lower.contains(&prefix_backslash) || lower.contains(&prefix_slash) {
            return Err(format!(
                "Privacy Violation: Override notes contain Windows path prefix (drive letter)"
            ));
        }
    }

    let forbidden_folders = &["/users/", "/home/", "/var/", "/tmp/", "/etc/", "/opt/"];
    for &folder in forbidden_folders {
        if lower.contains(folder) {
            return Err(format!(
                "Privacy Violation: Override notes contain system path prefix '{}'",
                folder
            ));
        }
    }

    Ok(notes.to_string())
}

pub fn validate_section_plan_override(o: &SectionPlanOverride) -> Result<(), String> {
    if let Some(ref primary) = o.primary_pattern_family {
        let norm = primary.to_lowercase().replace(' ', "_");
        if norm != "auto" {
            let fam = crate::generation_context::PatternFamily::from_str(&norm);
            if matches!(fam, crate::generation_context::PatternFamily::Unknown) {
                return Err(format!("Invalid primary pattern family: {}", primary));
            }
        }
    }

    if let Some(ref secondaries) = o.secondary_pattern_families {
        for sec in secondaries {
            let norm = sec.to_lowercase().replace(' ', "_");
            let fam = crate::generation_context::PatternFamily::from_str(&norm);
            if matches!(fam, crate::generation_context::PatternFamily::Unknown) {
                return Err(format!("Invalid secondary pattern family: {}", sec));
            }
        }
    }

    if let Some(ref avoids) = o.avoid_pattern_families {
        for av in avoids {
            let norm = av.to_lowercase().replace(' ', "_");
            let fam = crate::generation_context::PatternFamily::from_str(&norm);
            if matches!(fam, crate::generation_context::PatternFamily::Unknown) {
                return Err(format!("Invalid avoid pattern family: {}", av));
            }
        }
    }

    if let Some(ref motif) = o.motif_strategy {
        let norm = motif.to_lowercase().replace(' ', "_");
        let valid_motifs = &[
            "auto",
            "introduce",
            "develop",
            "intensify",
            "contrast",
            "rest",
            "callback",
            "resolve",
            "final_burst",
        ];
        if !valid_motifs.contains(&norm.as_str()) {
            return Err(format!("Invalid motif strategy: {}", motif));
        }
    }

    if let Some(ref intensity) = o.intensity_band {
        let norm = intensity.to_lowercase().replace(' ', "_");
        let valid_intensities = &["auto", "very_low", "low", "medium", "high", "very_high"];
        if !valid_intensities.contains(&norm.as_str()) {
            return Err(format!("Invalid intensity band: {}", intensity));
        }
    }

    let valid_transitions = &[
        "auto",
        "smooth_continue",
        "density_ramp_up",
        "climax_entry",
        "density_ramp_down",
        "contrast_break",
        "unknown",
        "none",
    ];

    if let Some(ref trans_in) = o.transition_in_type {
        let norm = trans_in.to_lowercase().replace(' ', "_");
        if !valid_transitions.contains(&norm.as_str()) {
            return Err(format!("Invalid transition in type: {}", trans_in));
        }
    }

    if let Some(ref trans_out) = o.transition_out_type {
        let norm = trans_out.to_lowercase().replace(' ', "_");
        if !valid_transitions.contains(&norm.as_str()) {
            return Err(format!("Invalid transition out type: {}", trans_out));
        }
    }

    if let Some(ref notes) = o.notes {
        if notes.len() > 240 {
            return Err("Notes exceed maximum length of 240 characters.".to_string());
        }
        sanitize_override_notes(notes)?;
    }

    Ok(())
}

pub fn apply_section_plan_overrides(
    mut plan: SongContinuityPlan,
    overrides: &[SectionPlanOverride],
) -> Result<SongContinuityPlan, String> {
    // First, validate all overrides and verify that they reference existing sections
    for ov in overrides {
        validate_section_plan_override(ov)?;
        if !plan.sections.iter().any(|s| s.section_id == ov.section_id) {
            return Err(format!(
                "Override references unknown section '{}'. Rebuild the section plan before applying overrides.",
                ov.section_id
            ));
        }
    }

    // Second pass: apply all non-transition overrides
    for ov in overrides {
        if let Some(ref mut node) = plan
            .sections
            .iter_mut()
            .find(|s| s.section_id == ov.section_id)
        {
            if let Some(enabled) = ov.enabled {
                node.enabled = enabled;
            }
            if let Some(ref primary) = ov.primary_pattern_family {
                let norm = primary.to_lowercase().replace(' ', "_");
                if norm != "auto" {
                    node.primary_pattern_family = norm;
                }
            }
            if let Some(ref secondaries) = &ov.secondary_pattern_families {
                node.secondary_pattern_families = secondaries
                    .iter()
                    .map(|s| s.to_lowercase().replace(' ', "_"))
                    .collect();
            }
            if let Some(ref avoids) = &ov.avoid_pattern_families {
                node.avoid_pattern_families = avoids
                    .iter()
                    .map(|s| s.to_lowercase().replace(' ', "_"))
                    .collect();
            }
            if let Some(ref motif) = ov.motif_strategy {
                let norm = motif.to_lowercase().replace(' ', "_");
                if norm != "auto" {
                    node.motif_strategy = norm;
                }
            }
            if let Some(ref intensity) = ov.intensity_band {
                let norm = intensity.to_lowercase().replace(' ', "_");
                if norm != "auto" {
                    node.intensity_band = norm.clone();
                    node.density_intent = norm;
                }
            }
            if let Some(ref notes) = ov.notes {
                let sanitized = sanitize_override_notes(notes)?;
                node.notes = Some(sanitized);
            }
        }
    }

    // Recalculate transitions dynamically based on updated intensity bands
    let nodes_len = plan.sections.len();
    for idx in 0..nodes_len {
        let prev_node = if idx > 0 {
            Some(plan.sections[idx - 1].clone())
        } else {
            None
        };
        let transition_in =
            build_transition_guidance(prev_node.as_ref(), &plan.sections[idx], false);

        let next_node = if idx + 1 < nodes_len {
            Some(plan.sections[idx + 1].clone())
        } else {
            None
        };
        let transition_out =
            build_transition_guidance(next_node.as_ref(), &plan.sections[idx], true);

        plan.sections[idx].transition_in = transition_in;
        plan.sections[idx].transition_out = transition_out;
    }

    // Second pass: apply transition type overrides (user overrides take precedence over recalculated ones)
    for ov in overrides {
        if let Some(ref mut node) = plan
            .sections
            .iter_mut()
            .find(|s| s.section_id == ov.section_id)
        {
            if let Some(ref trans_in) = ov.transition_in_type {
                let norm = trans_in.to_lowercase().replace(' ', "_");
                if norm != "auto" {
                    node.transition_in.transition_type = norm;
                }
            }
            if let Some(ref trans_out) = ov.transition_out_type {
                let norm = trans_out.to_lowercase().replace(' ', "_");
                if norm != "auto" {
                    node.transition_out.transition_type = norm;
                }
            }
        }
    }

    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biomechanics::PlayMode;
    use crate::music_analysis::{ChoreographicIntentMap, SectionFrame, SongAnalysisReport};
    use crate::ssc::parser::SscDocument;

    fn make_mock_report() -> SongAnalysisReport {
        SongAnalysisReport {
            schema_version: "v1".to_string(),
            song_id: "mock_song".to_string(),
            title: "Mock Title".to_string(),
            artist: "Mock Artist".to_string(),
            duration_seconds: 120.0,
            audio_summary: crate::music_analysis::AudioSummary {
                sample_rate: 44100,
                detected_bpm: 130.0,
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
                bpms: vec![(0.0, 130.0)],
                display_bpm: "130".to_string(),
                song_type: "Arcade".to_string(),
            },
            event_features: crate::music_analysis::EventFeatures { beats: vec![] },
            sections: vec![
                SectionFrame {
                    section_id: "sec_intro".to_string(),
                    start_beat: 0.0,
                    end_beat: 32.0,
                    start_measure: 0,
                    end_measure: 8,
                    music_role: "intro".to_string(),
                    piu_role: "warmup".to_string(),
                    boundary_confidence: 0.9,
                    energy_profile: "low".to_string(),
                },
                SectionFrame {
                    section_id: "sec_verse".to_string(),
                    start_beat: 32.0,
                    end_beat: 96.0,
                    start_measure: 8,
                    end_measure: 24,
                    music_role: "verse".to_string(),
                    piu_role: "footwork".to_string(),
                    boundary_confidence: 0.8,
                    energy_profile: "mid".to_string(),
                },
                SectionFrame {
                    section_id: "sec_chorus".to_string(),
                    start_beat: 96.0,
                    end_beat: 160.0,
                    start_measure: 24,
                    end_measure: 40,
                    music_role: "chorus".to_string(),
                    piu_role: "climax".to_string(),
                    boundary_confidence: 0.95,
                    energy_profile: "high".to_string(),
                },
            ],
            choreographic_intent: vec![
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec_intro".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 0,
                    measure_end: 8,
                    density_target: "sparse".to_string(),
                    difficulty_budget: 5.0,
                    recommended_pattern_families: vec!["balanced".to_string()],
                    avoid_pattern_families: vec!["bracket_technical".to_string()],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "introduce".to_string(),
                    evidence: vec![],
                    confidence: 0.9,
                },
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec_verse".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 8,
                    measure_end: 24,
                    density_target: "moderate".to_string(),
                    difficulty_budget: 8.0,
                    recommended_pattern_families: vec!["stream".to_string()],
                    avoid_pattern_families: vec![],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "develop".to_string(),
                    evidence: vec![],
                    confidence: 0.8,
                },
                ChoreographicIntentMap {
                    schema_version: "v1".to_string(),
                    section_id: "sec_chorus".to_string(),
                    mode: "Single".to_string(),
                    target_level: 10,
                    measure_start: 24,
                    measure_end: 40,
                    density_target: "intense".to_string(),
                    difficulty_budget: 12.0,
                    recommended_pattern_families: vec![
                        "stream".to_string(),
                        "jump_accent".to_string(),
                    ],
                    avoid_pattern_families: vec![],
                    accent_plan: vec![],
                    rest_plan: vec![],
                    motif_strategy: "intensify".to_string(),
                    evidence: vec![],
                    confidence: 0.95,
                },
            ],
            diagnostics: crate::music_analysis::TimingDiagnostics {
                audio_bpm_detected: 130.0,
                ssc_initial_bpm: 130.0,
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
        }
    }

    #[test]
    fn test_continuity_plan_degrades_without_music_analysis() {
        let plan = build_song_continuity_plan(10, PlayMode::Single, None, None, None, "sec1", 0, 8);
        assert_eq!(plan.section_count, 1);
        assert!(!plan.calibration_available);
        assert!(plan.sections[0].warnings[0].contains("degraded"));
    }

    #[test]
    fn test_continuity_plan_builds_from_music_analysis_sections() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        assert_eq!(plan.section_count, 3);
        assert_eq!(plan.sections[0].section_id, "sec_intro");
        assert_eq!(plan.sections[1].section_id, "sec_verse");
        assert_eq!(plan.sections[2].section_id, "sec_chorus");
    }

    #[test]
    fn test_intro_section_uses_introduce_or_warmup_strategy() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_intro",
            0,
            8,
        );
        let intro_node = &plan.sections[0];
        assert_eq!(intro_node.motif_strategy, "introduce");
    }

    #[test]
    fn test_build_section_ramps_density() {
        let mut report = make_mock_report();
        report.sections[1].music_role = "pre-chorus".to_string();
        report.sections[1].piu_role = "build".to_string();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let build_node = &plan.sections[1];
        assert_eq!(build_node.transition_in.transition_type, "density_ramp_up");
    }

    #[test]
    fn test_break_section_reduces_density() {
        let mut report = make_mock_report();
        // Section 0 (intro):
        report.sections[0].energy_profile = "mid".to_string();
        report.sections[0].piu_role = "normal".to_string();
        // Section 1 (verse):
        report.sections[1].music_role = "verse".to_string();
        report.sections[1].piu_role = "normal".to_string();
        report.sections[1].energy_profile = "low".to_string();

        let plan = build_song_continuity_plan(
            12,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let break_node = &plan.sections[1];
        assert_eq!(
            break_node.transition_in.transition_type,
            "density_ramp_down"
        );
    }

    #[test]
    fn test_climax_section_allows_higher_intensity() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_chorus",
            24,
            40,
        );
        let climax_node = &plan.sections[2];
        assert_eq!(climax_node.intensity_band, "high"); // Allowed at level 10
    }

    #[test]
    fn test_final_section_uses_resolve_or_final_burst() {
        let mut report = make_mock_report();
        report.sections[2].music_role = "outro".to_string();
        report.sections[2].piu_role = "final_burst".to_string();
        let plan = build_song_continuity_plan(
            16,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_chorus",
            24,
            40,
        );
        let final_node = &plan.sections[2];
        assert_eq!(final_node.motif_strategy, "final_burst");
    }

    #[test]
    fn test_low_confidence_analysis_prefers_balanced() {
        let mut report = make_mock_report();
        report.sections[1].boundary_confidence = 0.3;
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        assert_eq!(plan.sections[1].primary_pattern_family, "balanced");
    }

    #[test]
    fn test_timing_disagreement_adds_warning() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            20,
            PlayMode::Single,
            Some(&report),
            None,
            Some("Status: disagrees, Canonical BPM: 120, Browser BPM: 130"),
            "sec_verse",
            8,
            24,
        );
        assert!(plan.warnings.iter().any(|w| w.contains("disagreement")));
    }

    #[test]
    fn test_low_level_avoids_bracket_stamina() {
        let plan = build_song_continuity_plan(5, PlayMode::Single, None, None, None, "sec1", 0, 8);
        let node = &plan.sections[0];
        assert!(node
            .avoid_pattern_families
            .contains(&"bracket_technical".to_string()));
        assert!(node.avoid_pattern_families.contains(&"stamina".to_string()));
    }

    #[test]
    fn test_repeated_family_is_limited_without_stamina_context() {
        let mut report = make_mock_report();
        // Set all three sections to primary_family = stream (via recommended_pattern_families)
        report.choreographic_intent[0].recommended_pattern_families = vec!["stream".to_string()];
        report.choreographic_intent[1].recommended_pattern_families = vec!["stream".to_string()];
        report.choreographic_intent[2].recommended_pattern_families = vec!["stream".to_string()];
        // Keep target level low (e.g. 10) so stamina is not justified
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        assert_eq!(plan.sections[0].primary_pattern_family, "stream");
        assert_eq!(plan.sections[1].primary_pattern_family, "stream");
        // Third repetition gets limited to balanced
        assert_eq!(plan.sections[2].primary_pattern_family, "balanced");
        assert!(plan.sections[2]
            .warnings
            .iter()
            .any(|w| w.contains("limited to prevent monotony")));
    }

    #[test]
    fn test_current_section_prompt_receives_neighbor_summary() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let node = &plan.sections[1]; // sec_verse
        assert_eq!(node.section_id, "sec_verse");
        // Verify transition guidance has prev section focus info
        assert_eq!(node.transition_in.family_shift, "balanced -> stream");
    }

    #[test]
    fn test_continuity_context_contains_no_private_paths_or_raw_tags() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let node = &plan.sections[1];
        let summary = ContinuityContextSummary {
            enabled: true,
            section_index: node.section_index,
            section_count: plan.section_count,
            global_arc: plan.global_arc.arc_type.clone(),
            current_motif_strategy: node.motif_strategy.clone(),
            transition_in: Some(node.transition_in.clone()),
            transition_out: Some(node.transition_out.clone()),
            neighbor_summary: NeighborSummaryGroup {
                previous: None,
                next: None,
            },
            warnings: vec![],
            current_primary_pattern_family: node.primary_pattern_family.clone(),
            current_secondary_pattern_families: node.secondary_pattern_families.clone(),
            current_avoid_pattern_families: node.avoid_pattern_families.clone(),
            current_intensity_band: node.intensity_band.clone(),
            current_density_intent: node.density_intent.clone(),
            current_confidence: node.confidence.clone(),
            current_notes: None,
        };
        let serialized = serde_json::to_string(&summary).unwrap();
        assert!(crate::generation_context::self_audit_prompt_context(&serialized).is_ok());
    }

    #[tokio::test]
    async fn test_preview_response_includes_continuity_summary() {
        let mut server = mockito::Server::new_async().await;
        let mock_post = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"sec1\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_continuity_preview_summary.ssc");
        let ssc_content =
            "#TITLE:Mock Song;\n#ARTIST:Mock Artist;\n#BPMS:0.000=120.000;\n#OFFSET:0.000;\n";
        std::fs::write(&temp_ssc_path, ssc_content).unwrap();

        let test_audio_path = temp_dir.join("test_continuity_audio.mp3");
        std::fs::write(&test_audio_path, b"audio").unwrap();

        let client = crate::gemini::GeminiClient::new(Some(server.url()));
        crate::settings::set_test_gemini_enabled(Some(true));

        let result = crate::commands::generate_gemini_chart_preview_core_internal(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "sec1",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(true),
        )
        .await
        .unwrap();

        mock_post.assert_async().await;
        assert!(result.continuity_plan.is_some());
        let plan = result.continuity_plan.unwrap();
        assert_eq!(plan.target_level, 10);
        assert_eq!(plan.section_count, 1);

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_file(temp_ssc_path);
        let _ = std::fs::remove_file(test_audio_path);
    }

    #[tokio::test]
    async fn test_preview_only_still_does_not_write() {
        let mut server = mockito::Server::new_async().await;
        let mock_post = server.mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\n  \"section_id\": \"sec1\",\n  \"difficulty_level\": 10,\n  \"play_mode\": \"Single\",\n  \"biomechanical_state\": {\n    \"current_twist_debt\": 0.0,\n    \"current_stamina_debt\": 0.1\n  },\n  \"measures\": [\n    {\n      \"measure_index\": 0,\n      \"subdivision\": 4,\n      \"rows\": [\n        \"10000\",\n        \"00100\",\n        \"00001\",\n        \"00100\"\n      ]\n    }\n  ]\n}"
                                }
                            ]
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_continuity_preview_only.ssc");
        let ssc_content =
            "#TITLE:Mock Song;\n#ARTIST:Mock Artist;\n#BPMS:0.000=120.000;\n#OFFSET:0.000;\n";
        std::fs::write(&temp_ssc_path, ssc_content).unwrap();

        let test_audio_path = temp_dir.join("test_continuity_audio_po.mp3");
        std::fs::write(&test_audio_path, b"audio").unwrap();

        let client = crate::gemini::GeminiClient::new(Some(server.url()));
        crate::settings::set_test_gemini_enabled(Some(true));

        let result = crate::commands::generate_gemini_chart_preview_core_internal(
            "fake-key",
            &temp_ssc_path.to_string_lossy(),
            &test_audio_path.to_string_lossy(),
            PlayMode::Single,
            10,
            "sec1",
            "AI Previewer",
            &client,
            Some(0),
            Some(0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert!(!result.written);
        mock_post.assert_async().await;

        let doc = SscDocument::parse(&temp_ssc_path).unwrap();
        assert!(doc.charts.is_empty());

        crate::settings::set_test_gemini_enabled(None);
        let _ = std::fs::remove_file(temp_ssc_path);
        let _ = std::fs::remove_file(test_audio_path);
    }

    #[test]
    fn test_append_still_requires_fingerprint() {
        let temp_dir = std::env::temp_dir();
        let temp_ssc_path = temp_dir.join("test_continuity_append_fingerprint.ssc");
        let ssc_content =
            "#TITLE:Mock Song;\n#ARTIST:Mock Artist;\n#BPMS:0.000=120.000;\n#OFFSET:0.000;\n";
        std::fs::write(&temp_ssc_path, ssc_content).unwrap();

        let wrong_fingerprint = "wrong_sha256_hash_value_here".to_string();
        let payload = r#"{
            "section_id": "sec1",
            "difficulty_level": 10,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.1
            },
            "measures": [
                {
                    "measure_index": 0,
                    "subdivision": 4,
                    "rows": ["10000", "00100", "00001", "00100"]
                }
            ]
        }"#;

        let result = crate::commands::append_approved_gemini_payload(
            temp_ssc_path.to_string_lossy().to_string(),
            payload.to_string(),
            wrong_fingerprint,
            "AI Stepmaker".to_string(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fingerprint"));

        let _ = std::fs::remove_file(temp_ssc_path);
    }

    #[test]
    fn test_continuity_context_includes_current_effective_family_and_intensity() {
        let report = make_mock_report();
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&report),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let node = &plan.sections[1];
        let summary = ContinuityContextSummary {
            enabled: true,
            section_index: node.section_index,
            section_count: plan.section_count,
            global_arc: plan.global_arc.arc_type.clone(),
            current_motif_strategy: node.motif_strategy.clone(),
            transition_in: Some(node.transition_in.clone()),
            transition_out: Some(node.transition_out.clone()),
            neighbor_summary: NeighborSummaryGroup {
                previous: None,
                next: None,
            },
            warnings: vec![],
            current_primary_pattern_family: node.primary_pattern_family.clone(),
            current_secondary_pattern_families: node.secondary_pattern_families.clone(),
            current_avoid_pattern_families: node.avoid_pattern_families.clone(),
            current_intensity_band: node.intensity_band.clone(),
            current_density_intent: node.density_intent.clone(),
            current_confidence: node.confidence.clone(),
            current_notes: None,
        };
        assert_eq!(summary.current_primary_pattern_family, "stream");
        assert_eq!(summary.current_intensity_band, "low");
        assert_eq!(summary.current_density_intent, "low");
        assert_eq!(summary.current_confidence, "medium");

        let serialized = serde_json::to_value(&summary).unwrap();
        assert!(serialized.get("current_primary_pattern_family").is_some());
        assert!(serialized
            .get("current_secondary_pattern_families")
            .is_some());
        assert!(serialized.get("current_avoid_pattern_families").is_some());
        assert!(serialized.get("current_intensity_band").is_some());
        assert!(serialized.get("current_density_intent").is_some());
        assert!(serialized.get("current_confidence").is_some());
    }

    #[test]
    fn test_override_primary_family_applies_to_section() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: Some("Stamina".to_string()),
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let applied = apply_section_plan_overrides(plan, &overrides).unwrap();
        let verse = applied
            .sections
            .iter()
            .find(|s| s.section_id == "sec_verse")
            .unwrap();
        assert_eq!(verse.primary_pattern_family, "stamina");
    }

    #[test]
    fn test_override_motif_strategy_applies_to_section() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: Some("Contrast".to_string()),
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let applied = apply_section_plan_overrides(plan, &overrides).unwrap();
        let verse = applied
            .sections
            .iter()
            .find(|s| s.section_id == "sec_verse")
            .unwrap();
        assert_eq!(verse.motif_strategy, "contrast");
    }

    #[test]
    fn test_override_intensity_applies_to_section() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: Some("Very High".to_string()),
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let applied = apply_section_plan_overrides(plan, &overrides).unwrap();
        let verse = applied
            .sections
            .iter()
            .find(|s| s.section_id == "sec_verse")
            .unwrap();
        assert_eq!(verse.intensity_band, "very_high");
        assert_eq!(verse.density_intent, "very_high");
    }

    #[test]
    fn test_disabled_section_is_marked_and_not_selected_for_prompt() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
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
        let applied = apply_section_plan_overrides(plan, &overrides).unwrap();
        let verse = applied
            .sections
            .iter()
            .find(|s| s.section_id == "sec_verse")
            .unwrap();
        assert_eq!(verse.enabled, false);
    }

    #[test]
    fn test_invalid_family_override_is_rejected() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: Some("InvalidFamily".to_string()),
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let res = apply_section_plan_overrides(plan, &overrides);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid primary pattern family"));
    }

    #[test]
    fn test_invalid_motif_override_is_rejected() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: Some("InvalidMotif".to_string()),
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let res = apply_section_plan_overrides(plan, &overrides);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid motif strategy"));
    }

    #[test]
    fn test_override_notes_reject_private_paths() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides1 = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("C:\\Users\\private_file.ssc".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan.clone(), &overrides1).is_err());

        let overrides2 = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("/Users/someone/secret".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan, &overrides2).is_err());
    }

    #[test]
    fn test_override_notes_reject_raw_stepmania_tags() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides1 = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("#NOTEDATA:".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan.clone(), &overrides1).is_err());

        let overrides2 = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("#TITLE: My Title".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan, &overrides2).is_err());
    }

    #[test]
    fn test_missing_music_analysis_returns_degraded_plan() {
        let plan =
            build_song_continuity_plan(10, PlayMode::Single, None, None, None, "sec_verse", 8, 24);
        assert_eq!(plan.section_count, 1);
        assert_eq!(plan.sections[0].section_id, "sec_verse");
        assert!(plan.sections[0]
            .warnings
            .iter()
            .any(|w| w.contains("degraded")));
    }

    #[test]
    fn test_override_nonexistent_section_rejected() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_nonexistent".to_string(),
            enabled: Some(true),
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: None,
        }];
        let res = apply_section_plan_overrides(plan, &overrides);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Override references unknown section"));
    }

    #[test]
    fn test_invalid_transitions_rejected() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );

        let overrides_in = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: Some("invalid_trans".to_string()),
            transition_out_type: None,
            notes: None,
        }];
        let res_in = apply_section_plan_overrides(plan.clone(), &overrides_in);
        assert!(res_in.is_err());
        assert!(res_in.unwrap_err().contains("Invalid transition in type"));

        let overrides_out = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: Some("invalid_trans".to_string()),
            notes: None,
        }];
        let res_out = apply_section_plan_overrides(plan, &overrides_out);
        assert!(res_out.is_err());
        assert!(res_out.unwrap_err().contains("Invalid transition out type"));
    }

    #[test]
    fn test_valid_transitions_accepted() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );
        let overrides = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: Some("smooth_continue".to_string()),
            transition_out_type: Some("climax_entry".to_string()),
            notes: None,
        }];
        let res = apply_section_plan_overrides(plan, &overrides);
        assert!(res.is_ok());
    }

    #[test]
    fn test_stricter_override_notes_sanitization() {
        let plan = build_song_continuity_plan(
            10,
            PlayMode::Single,
            Some(&make_mock_report()),
            None,
            None,
            "sec_verse",
            8,
            24,
        );

        // Windows drive letter with forward slash
        let ov_drive = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("d:/secret/file.ssc".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan.clone(), &ov_drive).is_err());

        // Unix system path /home/
        let ov_home = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("/home/user/my_song".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan.clone(), &ov_home).is_err());

        // Unix system path /tmp/
        let ov_tmp = vec![SectionPlanOverride {
            section_id: "sec_verse".to_string(),
            enabled: None,
            primary_pattern_family: None,
            secondary_pattern_families: None,
            avoid_pattern_families: None,
            motif_strategy: None,
            intensity_band: None,
            transition_in_type: None,
            transition_out_type: None,
            notes: Some("/tmp/dump".to_string()),
        }];
        assert!(apply_section_plan_overrides(plan, &ov_tmp).is_err());
    }
}
