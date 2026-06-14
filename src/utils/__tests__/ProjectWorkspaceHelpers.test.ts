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
  sanitizeSectionOverrideNote
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


