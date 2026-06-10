use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayMode {
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationIssueType {
    InvalidLength,
    MinaDetected,
    InvalidChar,
    TripleTap,
    DoubleStep,
    ConsecutiveJumps,
    InvalidGeminiStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub measure_index: usize,
    pub row_index: usize, // 0-indexed row within the measure
    pub severity: ValidationSeverity,
    pub issue_type: ValidationIssueType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedChartSection {
    pub play_mode: PlayMode,
    pub difficulty_level: u32,
    pub issues: Vec<ValidationIssue>,
}

// Structures for structured Gemini AI payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeminiBiomechanicalState {
    pub current_twist_debt: f64,
    pub current_stamina_debt: f64,
    pub last_left_foot_lane: Option<u32>,
    pub last_right_foot_lane: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeminiMeasure {
    pub measure_index: u32,
    pub subdivision: u32,
    pub rows: Vec<String>,
}

pub const MAX_SECTION_MEASURES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeminiChartSectionPayload {
    pub section_id: String,
    pub difficulty_level: u32,
    pub play_mode: PlayMode,
    pub biomechanical_state: GeminiBiomechanicalState,
    pub measures: Vec<GeminiMeasure>,
}

impl GeminiChartSectionPayload {
    pub fn validate_structure(&self) -> Result<(), String> {
        // 1. Difficulty limit validations
        match self.play_mode {
            PlayMode::Single => {
                if self.difficulty_level < 1 || self.difficulty_level > 26 {
                    return Err(format!(
                        "Nivel de dificultad Single ({}) fuera de rango (1-26).",
                        self.difficulty_level
                    ));
                }
            }
            PlayMode::Double => {
                return Err("Play mode Double is not supported in this phase.".to_string());
            }
        }

        // 2. Reject empty measures list
        if self.measures.is_empty() {
            return Err(
                "El payload de Gemini no contiene medidas (measures está vacío).".to_string(),
            );
        }

        // 2b. Reject oversized measures list (hard backend cap)
        if self.measures.len() > MAX_SECTION_MEASURES {
            return Err(format!(
                "El payload de Gemini supera el límite máximo de {} medidas (recibidas: {}).",
                MAX_SECTION_MEASURES,
                self.measures.len()
            ));
        }

        let expected_len = match self.play_mode {
            PlayMode::Single => 5,
            PlayMode::Double => 10,
        };

        for (m_idx, measure) in self.measures.iter().enumerate() {
            // 3. subdivision validation (only 4, 8, 16, 32 allowed)
            let sub = measure.subdivision;
            if sub != 4 && sub != 8 && sub != 16 && sub != 32 {
                return Err(format!(
                    "Medida {}: subdivisión {} no es válida. Solo se permiten 4, 8, 16 o 32.",
                    m_idx, sub
                ));
            }

            // 4. rows length must match subdivision
            if measure.rows.len() as u32 != sub {
                return Err(format!(
                    "Medida {}: el número de filas ({}) no coincide con la subdivisión ({}).",
                    m_idx,
                    measure.rows.len(),
                    sub
                ));
            }

            for (r_idx, row) in measure.rows.iter().enumerate() {
                // 5. row string length check
                if row.len() != expected_len {
                    return Err(format!(
                        "Medida {}, fila {}: longitud incorrecta ({}). Debe ser {}.",
                        m_idx,
                        r_idx,
                        row.len(),
                        expected_len
                    ));
                }

                // 6. Gemini payloads cannot emit 'M' (only 0, 1, 2, 3 allowed)
                for c in row.chars() {
                    if c != '0' && c != '1' && c != '2' && c != '3' {
                        return Err(format!(
                            "Medida {}, fila {}: carácter inválido '{}' en payload Gemini. Solo se permiten '0', '1', '2', '3'.",
                            m_idx, r_idx, c
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn to_ssc_notes(&self) -> String {
        let mut notes_parts = Vec::new();
        for measure in &self.measures {
            let measure_str = measure.rows.join("\n");
            notes_parts.push(measure_str);
        }
        notes_parts.join(",\n") + "\n;"
    }
}

pub fn validate_chart(
    play_mode: PlayMode,
    difficulty_level: u32,
    notes_raw: &str,
) -> ValidatedChartSection {
    let mut issues = Vec::new();

    // Split the notes raw text into measures by comma.
    // Clean trailing semicolons or comments.
    let clean_notes = notes_raw.trim().trim_end_matches(';');
    let measures_raw: Vec<&str> = clean_notes.split(',').collect();

    // Track last tap event details for double-step checks
    // We store (beat, Vec<panel_indices>, is_subdivision_16_plus)
    let mut last_tap_event: Option<(f64, Vec<usize>, bool)> = None;

    // Track consecutive jumps
    let mut consecutive_jumps_count = 0;

    for (measure_idx, measure_content) in measures_raw.iter().enumerate() {
        // Filter lines to get valid note rows (ignore comments, empty lines)
        let rows_lines: Vec<&str> = measure_content
            .lines()
            .map(|l| {
                let trimmed = l.trim();
                // strip trailing comment
                if let Some(comment_start) = trimmed.find("//") {
                    trimmed[..comment_start].trim()
                } else {
                    trimmed
                }
            })
            .filter(|l| !l.is_empty())
            .collect();

        let rows_count = rows_lines.len();
        let is_subdivision_16_plus = rows_count >= 16;

        for (row_idx, row_str) in rows_lines.iter().enumerate() {
            let row_beat =
                (measure_idx as f64) * 4.0 + (row_idx as f64) * (4.0 / rows_count as f64);
            let expected_len = match play_mode {
                PlayMode::Single => 5,
                PlayMode::Double => 10,
            };

            // 1. Length check
            if row_str.len() != expected_len {
                issues.push(ValidationIssue {
                    measure_index: measure_idx,
                    row_index: row_idx,
                    severity: ValidationSeverity::Error,
                    issue_type: ValidationIssueType::InvalidLength,
                    message: format!(
                        "La fila mide {} caracteres; debe medir exactamente {}.",
                        row_str.len(),
                        expected_len
                    ),
                });
                continue; // Cannot validate contents if length is invalid
            }

            // 2. Character validation
            let mut has_invalid_char = false;
            let mut has_mina = false;
            for c in row_str.chars() {
                if c == 'M' {
                    has_mina = true;
                } else if c != '0' && c != '1' && c != '2' && c != '3' {
                    has_invalid_char = true;
                }
            }

            if has_mina {
                issues.push(ValidationIssue {
                    measure_index: measure_idx,
                    row_index: row_idx,
                    severity: ValidationSeverity::Error,
                    issue_type: ValidationIssueType::MinaDetected,
                    message: "Se detectó una mina ('M'), lo cual está prohibido en StepF2."
                        .to_string(),
                });
            }

            if has_invalid_char {
                issues.push(ValidationIssue {
                    measure_index: measure_idx,
                    row_index: row_idx,
                    severity: ValidationSeverity::Error,
                    issue_type: ValidationIssueType::InvalidChar,
                    message: "Fila contiene caracteres inválidos. Solo se permite 0/1/2/3."
                        .to_string(),
                });
            }

            // Extract tapped panels in this row
            let mut current_taps = Vec::new();
            for (col_idx, c) in row_str.chars().enumerate() {
                if c == '1' || c == '2' {
                    current_taps.push(col_idx);
                }
            }

            if current_taps.is_empty() {
                // If it is an empty row (or only hold releases '3'), reset consecutive jumps
                // Note: hold releases are not taps, so they don't count as taps.
                consecutive_jumps_count = 0;
                last_tap_event = None; // Reset last tap event on silence
                continue;
            }

            // 3. Triple tap checks
            if difficulty_level < 16 && current_taps.len() >= 3 {
                issues.push(ValidationIssue {
                    measure_index: measure_idx,
                    row_index: row_idx,
                    severity: ValidationSeverity::Warning,
                    issue_type: ValidationIssueType::TripleTap,
                    message: format!(
                        "Nivel {} (menor a 16) contiene {} pulsaciones simultáneas.",
                        difficulty_level,
                        current_taps.len()
                    ),
                });
            }

            // 4. Consecutive jumps checks
            let is_jump = current_taps.len() >= 2;
            if is_jump {
                consecutive_jumps_count += 1;
                if consecutive_jumps_count >= 3 {
                    issues.push(ValidationIssue {
                        measure_index: measure_idx,
                        row_index: row_idx,
                        severity: ValidationSeverity::Warning,
                        issue_type: ValidationIssueType::ConsecutiveJumps,
                        message: format!(
                            "Se detectaron {} saltos (jumps) consecutivos.",
                            consecutive_jumps_count
                        ),
                    });
                }
            } else {
                consecutive_jumps_count = 0;
            }

            // 5. Rapid Double-step checks in stream of subdivision 16+
            if is_subdivision_16_plus {
                if let Some((last_beat, last_taps, last_was_16_plus)) = &last_tap_event {
                    if *last_was_16_plus && (row_beat - last_beat) <= 0.2501 {
                        // Hitting notes in consecutive rows at high speed
                        if current_taps.len() == 1 && last_taps.len() == 1 {
                            let curr_panel = current_taps[0];
                            let prev_panel = last_taps[0];

                            if curr_panel == prev_panel {
                                // Double-step: same panel hit consecutively
                                issues.push(ValidationIssue {
                                    measure_index: measure_idx,
                                    row_index: row_idx,
                                    severity: ValidationSeverity::Warning,
                                    issue_type: ValidationIssueType::DoubleStep,
                                    message: format!(
                                        "Double-step (Jack rápido) en panel {}: pulsaciones consecutivas sin alternar pies.",
                                        curr_panel
                                    ),
                                });
                            } else {
                                match play_mode {
                                    PlayMode::Single => {
                                        let is_left = |p| p == 0 || p == 1;
                                        let is_right = |p| p == 3 || p == 4;

                                        if is_left(curr_panel) && is_left(prev_panel) {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas consecutivas en el lado izquierdo (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        } else if is_right(curr_panel) && is_right(prev_panel) {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas consecutivas en el lado derecho (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        }
                                    }
                                    PlayMode::Double => {
                                        let is_left_pad_left = |p| p == 0 || p == 1;
                                        let is_left_pad_right = |p| p == 3 || p == 4;
                                        let is_right_pad_left = |p| p == 5 || p == 6;
                                        let is_right_pad_right = |p| p == 8 || p == 9;

                                        if is_left_pad_left(curr_panel)
                                            && is_left_pad_left(prev_panel)
                                        {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas en lado izquierdo de pad izquierdo (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        } else if is_left_pad_right(curr_panel)
                                            && is_left_pad_right(prev_panel)
                                        {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas en lado derecho de pad izquierdo (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        } else if is_right_pad_left(curr_panel)
                                            && is_right_pad_left(prev_panel)
                                        {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas en lado izquierdo de pad derecho (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        } else if is_right_pad_right(curr_panel)
                                            && is_right_pad_right(prev_panel)
                                        {
                                            issues.push(ValidationIssue {
                                                measure_index: measure_idx,
                                                row_index: row_idx,
                                                severity: ValidationSeverity::Warning,
                                                issue_type: ValidationIssueType::DoubleStep,
                                                message: format!(
                                                    "Posible double-step: pulsaciones rápidas en lado derecho de pad derecho (paneles {} -> {}).",
                                                    prev_panel, curr_panel
                                                ),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Update last tap event
            last_tap_event = Some((row_beat, current_taps, is_subdivision_16_plus));
        }
    }

    ValidatedChartSection {
        play_mode,
        difficulty_level,
        issues,
    }
}

pub fn validate_single_chart(notes_raw: &str, difficulty_level: u32) -> ValidatedChartSection {
    validate_chart(PlayMode::Single, difficulty_level, notes_raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_row_contents() {
        let notes = "10000\n01000\n00100\n00010\n00001\n;";
        let validation = validate_single_chart(notes, 10);
        assert!(
            validation.issues.is_empty(),
            "Valid chart should have zero issues, got {:?}",
            validation.issues
        );
    }

    #[test]
    fn test_invalid_row_length() {
        let notes = "1000\n100000\n00100\n;";
        let validation = validate_single_chart(notes, 10);
        assert_eq!(validation.issues.len(), 2);
        assert_eq!(
            validation.issues[0].issue_type,
            ValidationIssueType::InvalidLength
        );
        assert_eq!(
            validation.issues[1].issue_type,
            ValidationIssueType::InvalidLength
        );
    }

    #[test]
    fn test_mina_and_invalid_chars() {
        let notes = "10M00\n100A0\n00100\n;";
        let validation = validate_single_chart(notes, 10);
        assert_eq!(validation.issues.len(), 2);

        let issue_types: Vec<ValidationIssueType> =
            validation.issues.iter().map(|i| i.issue_type).collect();
        assert!(issue_types.contains(&ValidationIssueType::MinaDetected));
        assert!(issue_types.contains(&ValidationIssueType::InvalidChar));
    }

    #[test]
    fn test_triple_tap_under_level_16() {
        let notes = "11100\n;";
        let validation_low = validate_single_chart(notes, 10);
        assert_eq!(validation_low.issues.len(), 1);
        assert_eq!(
            validation_low.issues[0].issue_type,
            ValidationIssueType::TripleTap
        );

        // High levels are allowed to have triple taps (brackets)
        let validation_high = validate_single_chart(notes, 18);
        assert!(validation_high.issues.is_empty());
    }

    #[test]
    fn test_consecutive_jumps() {
        // 3 consecutive jumps
        let notes = "10001\n10100\n00101\n00000\n;";
        let validation = validate_single_chart(notes, 15);
        assert_eq!(validation.issues.len(), 1);
        assert_eq!(
            validation.issues[0].issue_type,
            ValidationIssueType::ConsecutiveJumps
        );
    }

    #[test]
    fn test_consecutive_jumps_reset_by_empty_or_releases() {
        // jump, jump, empty, jump -> should NOT report ConsecutiveJumps
        let notes = "10001\n10100\n00000\n00101\n;";
        let validation = validate_single_chart(notes, 15);
        assert!(
            validation.issues.is_empty(),
            "Consecutive jumps count should be reset by empty row, got {:?}",
            validation.issues
        );

        // jump, jump, hold release (only 3s), jump -> should NOT report ConsecutiveJumps
        let notes_release = "10001\n10100\n00300\n00101\n;";
        let validation_release = validate_single_chart(notes_release, 15);
        assert!(
            validation_release.issues.is_empty(),
            "Consecutive jumps count should be reset by hold release row, got {:?}",
            validation_release.issues
        );
    }

    #[test]
    fn test_rapid_double_steps_in_streams() {
        // Measure with 16 rows (subdivision 16+)
        let notes = "10000\n10000\n00100\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n;";
        let validation = validate_single_chart(notes, 15);
        assert_eq!(validation.issues.len(), 1);
        assert_eq!(
            validation.issues[0].issue_type,
            ValidationIssueType::DoubleStep
        );
        assert!(validation.issues[0].message.contains("Jack"));

        // Left side double step
        let notes_left = "10000\n01000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n;";
        let validation_left = validate_single_chart(notes_left, 15);
        assert_eq!(validation_left.issues.len(), 1);
        assert_eq!(
            validation_left.issues[0].issue_type,
            ValidationIssueType::DoubleStep
        );
        assert!(validation_left.issues[0].message.contains("izquierdo"));
    }

    #[test]
    fn test_stream_silence_prevents_double_step() {
        // Stream of subdivision 16+ with middle silence
        // Row 0 has note, Row 1 is silence, Row 2 has same note
        let notes = "10000\n00000\n10000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n00000\n;";
        let validation = validate_single_chart(notes, 15);
        assert!(
            validation.issues.is_empty(),
            "Silence row should prevent double-step detection, got {:?}",
            validation.issues
        );
    }

    #[test]
    fn test_gemini_payload_difficulty_limits() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: None,
            last_right_foot_lane: None,
        };
        let measure = GeminiMeasure {
            measure_index: 0,
            subdivision: 4,
            rows: vec!["10000".to_string(); 4],
        };

        // 1. Single valid difficulty: 15
        let payload_single_ok = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 15,
            play_mode: PlayMode::Single,
            biomechanical_state: state.clone(),
            measures: vec![measure.clone()],
        };
        assert!(payload_single_ok.validate_structure().is_ok());

        // 2. Single invalid difficulty: 27
        let payload_single_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 27,
            play_mode: PlayMode::Single,
            biomechanical_state: state.clone(),
            measures: vec![measure.clone()],
        };
        assert!(payload_single_err.validate_structure().is_err());

        // 3. Double is rejected completely in this phase
        let payload_double_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Double,
            biomechanical_state: state.clone(),
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 4,
                rows: vec!["1000000000".to_string(); 4],
            }],
        };
        assert!(payload_double_err.validate_structure().is_err());
    }

    #[test]
    fn test_gemini_payload_rejects_timing_gimmick_tags() {
        // If the JSON contains unknown fields like bpms or scrolls, it must fail deserialization because of deny_unknown_fields
        let json_with_extra = r#"{
            "section_id": "chorus_1",
            "difficulty_level": 12,
            "play_mode": "Single",
            "biomechanical_state": {
                "current_twist_debt": 0.0,
                "current_stamina_debt": 0.0,
                "last_left_foot_lane": 1,
                "last_right_foot_lane": 3
            },
            "measures": [
                {
                    "measure_index": 32,
                    "subdivision": 4,
                    "rows": ["00000", "00000", "00000", "00000"]
                }
            ],
            "bpms": [[0.0, 150.0]]
        }"#;
        let parsed: Result<GeminiChartSectionPayload, serde_json::Error> =
            serde_json::from_str(json_with_extra);
        assert!(
            parsed.is_err(),
            "Should fail deserialization due to unknown field 'bpms'"
        );
    }

    #[test]
    fn test_gemini_payload_oversized_measures_rejected() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: Some(1),
            last_right_foot_lane: Some(3),
        };
        let measure = GeminiMeasure {
            measure_index: 0,
            subdivision: 4,
            rows: vec!["10000".to_string(); 4],
        };
        // Create 17 measures to exceed the MAX_SECTION_MEASURES = 16 limit
        let payload_oversized = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Single,
            biomechanical_state: state.clone(),
            measures: vec![measure.clone(); 17],
        };
        let err = payload_oversized.validate_structure().unwrap_err();
        assert!(err.contains("supera el límite máximo"));
    }

    #[test]
    fn test_gemini_payload_subdivision() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: None,
            last_right_foot_lane: None,
        };

        // Invalid subdivision (12)
        let payload_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Single,
            biomechanical_state: state,
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 12,
                rows: vec!["10000".to_string(); 12],
            }],
        };
        let err = payload_err.validate_structure().unwrap_err();
        assert!(err.contains("subdivisión 12 no es válida"));
    }

    #[test]
    fn test_gemini_payload_empty_measures() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: None,
            last_right_foot_lane: None,
        };
        let payload_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Single,
            biomechanical_state: state,
            measures: vec![],
        };
        let err = payload_err.validate_structure().unwrap_err();
        assert!(err.contains("measures está vacío"));
    }

    #[test]
    fn test_gemini_payload_row_count_mismatch() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: None,
            last_right_foot_lane: None,
        };
        let payload_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Single,
            biomechanical_state: state,
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 8,
                rows: vec!["10000".to_string(); 4], // only 4 rows, expected 8
            }],
        };
        let err = payload_err.validate_structure().unwrap_err();
        assert!(err.contains("no coincide con la subdivisión"));
    }

    #[test]
    fn test_gemini_payload_mina_rejected() {
        let state = GeminiBiomechanicalState {
            current_twist_debt: 0.0,
            current_stamina_debt: 0.0,
            last_left_foot_lane: None,
            last_right_foot_lane: None,
        };
        let payload_err = GeminiChartSectionPayload {
            section_id: "test".to_string(),
            difficulty_level: 10,
            play_mode: PlayMode::Single,
            biomechanical_state: state,
            measures: vec![GeminiMeasure {
                measure_index: 0,
                subdivision: 4,
                rows: vec![
                    "10000".to_string(),
                    "00M00".to_string(), // contains 'M'
                    "00010".to_string(),
                    "00001".to_string(),
                ],
            }],
        };
        let err = payload_err.validate_structure().unwrap_err();
        assert!(err.contains("carácter inválido 'M' en payload Gemini"));
    }
}
