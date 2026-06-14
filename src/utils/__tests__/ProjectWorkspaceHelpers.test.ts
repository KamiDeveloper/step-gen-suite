import test from "node:test";
import assert from "node:assert";
import {
  isAppendDisabled,
  isPreviewStale,
  validateMeasureRange,
  groupValidationIssues,
  getPatternFamilyLabel,
  getMotifStrategyLabel,
  getTransitionTypeLabel,
  getIntensityBandLabel,
  isSectionPlanStale,
  sanitizeSectionOverrideNote,
  canRunMultiSectionBatch,
  selectedSectionCountValidation,
  isBatchStale,
  getQueueStatusLabel
} from "../ProjectWorkspaceHelpers.ts";
import type {
  PreviewParams,
  IValidationIssue,
  IFileFingerprint
} from "../ProjectWorkspaceHelpers.ts";

test("validateMeasureRange bounds validation", () => {
  // Valid range within 16 limit
  const res1 = validateMeasureRange(0, 7, 16);
  assert.strictEqual(res1.isValid, true);
  assert.strictEqual(res1.error, null);

  // Valid limit boundary
  const res2 = validateMeasureRange(10, 25, 16); // 16 measures
  assert.strictEqual(res2.isValid, true);

  // Exceeds limit
  const res3 = validateMeasureRange(10, 26, 16); // 17 measures
  assert.strictEqual(res3.isValid, false);
  assert.ok(res3.error?.includes("supera el límite"));

  // Negative start measure
  const res4 = validateMeasureRange(-1, 5, 16);
  assert.strictEqual(res4.isValid, false);
  assert.ok(res4.error?.includes("no puede ser menor a 0"));

  // Start greater than end
  const res5 = validateMeasureRange(5, 4, 16);
  assert.strictEqual(res5.isValid, false);
  assert.ok(res5.error?.includes("debe ser mayor o igual"));
});

test("groupValidationIssues separating errors and warnings", () => {
  const issues: IValidationIssue[] = [
    { measure_index: 0, row_index: 0, severity: "Error", issue_type: "MinaDetected", message: "Error message" },
    { measure_index: 1, row_index: 2, severity: "Warning", issue_type: "DoubleStep", message: "Warning message" },
    { measure_index: 2, row_index: 1, severity: "Error", issue_type: "InvalidChar", message: "Another error" }
  ];

  const grouped = groupValidationIssues(issues);
  assert.strictEqual(grouped.errors.length, 2);
  assert.strictEqual(grouped.warnings.length, 1);
  assert.strictEqual(grouped.errors[0].issue_type, "MinaDetected");
  assert.strictEqual(grouped.warnings[0].issue_type, "DoubleStep");
});

test("isAppendDisabled logic verification", () => {
  const fingerprintOk1: IFileFingerprint = { file_size: 100, sha256: "hash123", modified_time: 1 };
  const fingerprintOk2: IFileFingerprint = { file_size: 100, sha256: "hash123", modified_time: 1 };
  const fingerprintDiff: IFileFingerprint = { file_size: 100, sha256: "hash456", modified_time: 1 };

  const cleanPreview = {
    validation: {
      issues: []
    }
  };

  const warningPreview = {
    validation: {
      issues: [
        { measure_index: 0, row_index: 0, severity: "Warning" as const, issue_type: "DoubleStep", message: "Warning" }
      ]
    }
  };

  const errorPreview = {
    validation: {
      issues: [
        { measure_index: 0, row_index: 0, severity: "Error" as const, issue_type: "MinaDetected", message: "Error" }
      ]
    }
  };

  // Clean preview and matching fingerprints -> NOT disabled (can append)
  assert.strictEqual(isAppendDisabled(cleanPreview, fingerprintOk1, fingerprintOk2, false), false);

  // Warnings only and matching fingerprints -> NOT disabled (can append under review)
  assert.strictEqual(isAppendDisabled(warningPreview, fingerprintOk1, fingerprintOk2, false), false);

  // Errors present -> disabled
  assert.strictEqual(isAppendDisabled(errorPreview, fingerprintOk1, fingerprintOk2, false), true);

  // Fingerprint mismatch -> disabled
  assert.strictEqual(isAppendDisabled(cleanPreview, fingerprintOk1, fingerprintDiff, false), true);

  // Loading state -> disabled
  assert.strictEqual(isAppendDisabled(cleanPreview, fingerprintOk1, fingerprintOk2, true), true);

  // Missing fingerprint -> disabled
  assert.strictEqual(isAppendDisabled(cleanPreview, fingerprintOk1, null, false), true);

  // Null preview -> disabled
  assert.strictEqual(isAppendDisabled(null, fingerprintOk1, fingerprintOk2, false), true);

  // Session marked unsafe -> disabled
  assert.strictEqual(isAppendDisabled(cleanPreview, fingerprintOk1, fingerprintOk2, false, true), true);
});

test("isPreviewStale stale state checking", () => {
  const snapshot: PreviewParams = {
    targetLevel: 10,
    sectionId: "chorus_1",
    startMeasure: 0,
    endMeasure: 7,
    songType: "Arcade",
    useMusicAnalysis: true,
    useBrowserBpm: true,
    selectedSectionKey: "chorus_1",
    useCalibratedPromptContext: true,
    patternFamilyTarget: "auto",
    useContinuityPlanning: true
  };

  const currentSame: PreviewParams = { ...snapshot };
  const currentDiffLevel: PreviewParams = { ...snapshot, targetLevel: 11 };
  const currentDiffSection: PreviewParams = { ...snapshot, sectionId: "custom" };
  const currentDiffStart: PreviewParams = { ...snapshot, startMeasure: 1 };
  const currentDiffEnd: PreviewParams = { ...snapshot, endMeasure: 8 };
  const currentDiffSongType: PreviewParams = { ...snapshot, songType: "Shortcut" };
  const currentDiffMA: PreviewParams = { ...snapshot, useMusicAnalysis: false };
  const currentDiffBB: PreviewParams = { ...snapshot, useBrowserBpm: false };
  const currentDiffSectionKey: PreviewParams = { ...snapshot, selectedSectionKey: "custom" };
  const currentDiffCalib: PreviewParams = { ...snapshot, useCalibratedPromptContext: false };
  const currentDiffTarget: PreviewParams = { ...snapshot, patternFamilyTarget: "stream" };
  const currentDiffContinuity: PreviewParams = { ...snapshot, useContinuityPlanning: false };

  // Null snapshot -> not stale
  assert.strictEqual(isPreviewStale(null, snapshot), false);

  // Identical parameters -> not stale
  assert.strictEqual(isPreviewStale(snapshot, currentSame), false);

  // Changed targetLevel -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffLevel), true);

  // Changed sectionId -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffSection), true);

  // Changed startMeasure -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffStart), true);

  // Changed endMeasure -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffEnd), true);

  // Changed songType -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffSongType), true);

  // Changed useMusicAnalysis -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffMA), true);

  // Changed useBrowserBpm -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffBB), true);

  // Changed selectedSectionKey -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffSectionKey), true);

  // Changed useCalibratedPromptContext -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffCalib), true);

  // Changed patternFamilyTarget -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffTarget), true);

  // Changed useContinuityPlanning -> stale
  assert.strictEqual(isPreviewStale(snapshot, currentDiffContinuity), true);
});

test("getPatternFamilyLabel mappings", () => {
  assert.strictEqual(getPatternFamilyLabel("auto"), "Auto");
  assert.strictEqual(getPatternFamilyLabel("balanced"), "Balanced");
  assert.strictEqual(getPatternFamilyLabel("stream"), "Stream");
  assert.strictEqual(getPatternFamilyLabel("jump_accent"), "Jump Accents");
  assert.strictEqual(getPatternFamilyLabel("jump_accents"), "Jump Accents");
  assert.strictEqual(getPatternFamilyLabel("twist_technical"), "Twist Technical");
  assert.strictEqual(getPatternFamilyLabel("bracket_technical"), "Bracket Technical");
  assert.strictEqual(getPatternFamilyLabel("hold_control"), "Hold Control");
  assert.strictEqual(getPatternFamilyLabel("center_control"), "Center Control");
  assert.strictEqual(getPatternFamilyLabel("stamina"), "Stamina");
  assert.strictEqual(getPatternFamilyLabel("unknown_family"), "Unknown Family");
});

test("getMotifStrategyLabel mappings", () => {
  assert.strictEqual(getMotifStrategyLabel("introduce"), "Introduce");
  assert.strictEqual(getMotifStrategyLabel("develop"), "Develop");
  assert.strictEqual(getMotifStrategyLabel("final_burst"), "Final Burst");
  assert.strictEqual(getMotifStrategyLabel("unknown_strategy"), "Unknown_strategy");
});

test("getTransitionTypeLabel mappings", () => {
  assert.strictEqual(getTransitionTypeLabel("smooth_continue"), "Smooth Continue");
  assert.strictEqual(getTransitionTypeLabel("density_ramp_up"), "Density Ramp Up");
  assert.strictEqual(getTransitionTypeLabel("climax_entry"), "Climax Entry");
  assert.strictEqual(getTransitionTypeLabel("unknown_transition"), "Unknown Transition");
});

test("getIntensityBandLabel mappings", () => {
  assert.strictEqual(getIntensityBandLabel("auto"), "Auto");
  assert.strictEqual(getIntensityBandLabel("very_low"), "Very Low");
  assert.strictEqual(getIntensityBandLabel("medium"), "Medium");
  assert.strictEqual(getIntensityBandLabel("high"), "High");
  assert.strictEqual(getIntensityBandLabel("unknown"), "Unknown");
});

test("isSectionPlanStale change detection", () => {
  const baseOverrides = [
    {
      section_id: "sec1",
      enabled: true,
      primary_pattern_family: "stamina",
      secondary_pattern_families: ["stream"],
      avoid_pattern_families: ["bracket_technical"],
      motif_strategy: "develop",
      intensity_band: "medium",
      transition_in_type: "smooth_continue",
      transition_out_type: "smooth_continue",
      notes: "some notes"
    }
  ];

  // Identical overrides should not be stale
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0] }]), false);

  // Null snapshot overrides should not be stale
  assert.strictEqual(isSectionPlanStale(null, baseOverrides), false);

  // Different length should be stale
  assert.strictEqual(isSectionPlanStale(baseOverrides, []), true);

  // Changed field should be stale
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0], enabled: false }]), true);
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0], primary_pattern_family: "balanced" }]), true);
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0], notes: "different note" }]), true);

  // Changed secondary/avoid arrays should be stale
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0], secondary_pattern_families: [] }]), true);
  assert.strictEqual(isSectionPlanStale(baseOverrides, [{ ...baseOverrides[0], avoid_pattern_families: ["stamina"] }]), true);
});

test("sanitizeSectionOverrideNote security boundaries", () => {
  // Safe notes
  const res1 = sanitizeSectionOverrideNote("Please make this section slightly faster and technical");
  assert.strictEqual(res1.isValid, true);
  assert.strictEqual(res1.error, null);

  // Exceeds max length
  const longNote = "a".repeat(241);
  const res2 = sanitizeSectionOverrideNote(longNote);
  assert.strictEqual(res2.isValid, false);
  assert.ok(res2.error?.includes("exceed maximum length"));

  // Forbidden patterns
  assert.strictEqual(sanitizeSectionOverrideNote("This has #NOTEDATA inside").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("using c:\\path\\to\\ssc").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("contains d:/path/to/file").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("contains /Users/secret/file").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("using /home/user/song").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("dumping to /tmp/file").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("contains .ssc file").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("contains .mp3 extension").isValid, false);
  assert.strictEqual(sanitizeSectionOverrideNote("contains docs/official_songs inside").isValid, false);
});

test("canRunMultiSectionBatch validations", () => {
  const plan = {
    schema_version: "v1",
    play_mode: "Single",
    target_level: 10,
    calibration_available: true,
    section_count: 3,
    sections: [
      {
        section_id: "sec1",
        section_index: 0,
        start_measure: 0,
        end_measure: 4,
        music_role: "intro",
        piu_role: "warmup",
        density_intent: "light",
        intensity_band: "low",
        primary_pattern_family: "balanced",
        secondary_pattern_families: [],
        avoid_pattern_families: [],
        motif_strategy: "introduce",
        transition_in: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        transition_out: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        confidence: "high",
        evidence: [],
        warnings: [],
        enabled: true,
      },
      {
        section_id: "sec2",
        section_index: 1,
        start_measure: 4,
        end_measure: 8,
        music_role: "verse",
        piu_role: "stream_opportunity",
        density_intent: "moderate",
        intensity_band: "medium",
        primary_pattern_family: "stream",
        secondary_pattern_families: [],
        avoid_pattern_families: [],
        motif_strategy: "develop",
        transition_in: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        transition_out: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        confidence: "high",
        evidence: [],
        warnings: [],
        enabled: false, // disabled for testing
      },
      {
        section_id: "sec3",
        section_index: 2,
        start_measure: 8,
        end_measure: 12,
        music_role: "chorus",
        piu_role: "climax_run",
        density_intent: "heavy",
        intensity_band: "high",
        primary_pattern_family: "stream",
        secondary_pattern_families: [],
        avoid_pattern_families: [],
        motif_strategy: "intensify",
        transition_in: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        transition_out: { transition_type: "smooth_continue", density_delta: "none", family_shift: "none", recommended_bridge: "", warnings: [] },
        confidence: "high",
        evidence: [],
        warnings: [],
        enabled: true,
      }
    ],
    global_arc: { arc_type: "peak_climax", peak_section_ids: ["sec3"], rest_section_ids: [], motif_policy: "introduce", density_curve: [] },
    warnings: [],
  };

  // Rejects empty selection
  assert.strictEqual(canRunMultiSectionBatch([], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch([], plan).error?.includes("No hay secciones seleccionadas"));

  // Rejects when no plan
  assert.strictEqual(canRunMultiSectionBatch(["sec1"], null).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec1"], null).error?.includes("no está cargado"));

  // Rejects too many sections (limit 4)
  assert.strictEqual(canRunMultiSectionBatch(["sec1", "sec2", "sec3", "sec4", "sec5"], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec1", "sec2", "sec3", "sec4", "sec5"], plan).error?.includes("máximo de 4"));

  // Rejects disabled section
  assert.strictEqual(canRunMultiSectionBatch(["sec1", "sec2"], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec1", "sec2"], plan).error?.includes("deshabilitadas"));

  // Rejects unknown section
  assert.strictEqual(canRunMultiSectionBatch(["sec1", "sec_unknown"], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec1", "sec_unknown"], plan).error?.includes("no encontrada"));

  // Rejects non-chronological order
  assert.strictEqual(canRunMultiSectionBatch(["sec3", "sec1"], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec3", "sec1"], plan).error?.includes("cronológicamente"));

  // Rejects duplicate sections
  assert.strictEqual(canRunMultiSectionBatch(["sec1", "sec1"], plan).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec1", "sec1"], plan).error?.includes("duplicadas"));

  // Plan with invalid bounds for testing
  const planInvalidBounds = {
    ...plan,
    sections: [
      {
        ...plan.sections[0],
        section_id: "sec_negative",
        start_measure: -5,
        end_measure: 5,
        enabled: true,
      },
      {
        ...plan.sections[0],
        section_id: "sec_inverted",
        start_measure: 5,
        end_measure: 4,
        enabled: true,
      },
      {
        ...plan.sections[0],
        section_id: "sec_too_long",
        start_measure: 0,
        end_measure: 18, // 19 measures
        enabled: true,
      }
    ]
  };

  // Rejects negative measure bounds
  assert.strictEqual(canRunMultiSectionBatch(["sec_negative"], planInvalidBounds).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec_negative"], planInvalidBounds).error?.includes("valores negativos"));

  // Rejects inverted bounds (start >= end)
  assert.strictEqual(canRunMultiSectionBatch(["sec_inverted"], planInvalidBounds).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec_inverted"], planInvalidBounds).error?.includes("inicio >= fin"));

  // Rejects section > 16 measures
  assert.strictEqual(canRunMultiSectionBatch(["sec_too_long"], planInvalidBounds).isValid, false);
  assert.ok(canRunMultiSectionBatch(["sec_too_long"], planInvalidBounds).error?.includes("supera el límite máximo de 16 compases"));

  // Valid selection
  const validRes = canRunMultiSectionBatch(["sec1", "sec3"], plan);
  assert.strictEqual(validRes.isValid, true);
  assert.strictEqual(validRes.error, null);
});

test("selectedSectionCountValidation checking", () => {
  assert.strictEqual(selectedSectionCountValidation(0), false);
  assert.strictEqual(selectedSectionCountValidation(1), true);
  assert.strictEqual(selectedSectionCountValidation(4), true);
  assert.strictEqual(selectedSectionCountValidation(5), false);
});

test("isBatchStale checks", () => {
  const snapshot = {
    targetLevel: 10,
    useCalibratedPromptContext: true,
    useContinuityPlanning: true,
    patternFamilyTarget: "stream",
    selectedSectionIds: ["sec1", "sec3"],
    overrides: [
      {
        section_id: "sec1",
        enabled: true,
        primary_pattern_family: "stamina",
      }
    ],
  };

  const currentSame = { ...snapshot };
  const currentDiffLevel = { ...snapshot, targetLevel: 11 };
  const currentDiffCalib = { ...snapshot, useCalibratedPromptContext: false };
  const currentDiffIds = { ...snapshot, selectedSectionIds: ["sec1"] };

  assert.strictEqual(isBatchStale(null, snapshot), false);
  assert.strictEqual(isBatchStale(snapshot, currentSame), false);
  assert.strictEqual(isBatchStale(snapshot, currentDiffLevel), true);
  assert.strictEqual(isBatchStale(snapshot, currentDiffCalib), true);
  assert.strictEqual(isBatchStale(snapshot, currentDiffIds), true);
});

test("getQueueStatusLabel mappings", () => {
  assert.strictEqual(getQueueStatusLabel("queued"), "En cola");
  assert.strictEqual(getQueueStatusLabel("running"), "Generando...");
  assert.strictEqual(getQueueStatusLabel("succeeded"), "Completado");
  assert.strictEqual(getQueueStatusLabel("warning"), "Con Advertencias");
  assert.strictEqual(getQueueStatusLabel("failed"), "Fallido");
  assert.strictEqual(getQueueStatusLabel("skipped"), "Omitido");
  assert.strictEqual(getQueueStatusLabel("custom_status"), "custom_status");
});



