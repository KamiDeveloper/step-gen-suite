export interface PreviewParams {
  targetLevel: number;
  sectionId: string;
  startMeasure: number;
  endMeasure: number;
  songType: string;
  useMusicAnalysis: boolean;
  useBrowserBpm: boolean;
  selectedSectionKey: string;
  useCalibratedPromptContext: boolean;
  patternFamilyTarget: string;
  useContinuityPlanning: boolean;
}

export interface IValidationIssue {
  measure_index: number;
  row_index: number;
  severity: "Warning" | "Error";
  issue_type: string;
  message: string;
}

export interface IFileFingerprint {
  file_size: number;
  sha256: string;
  modified_time: number;
}

/**
 * Checks if the append button should be disabled.
 */
export function isAppendDisabled(
  previewResult: { validation?: { issues: IValidationIssue[] } } | null,
  fingerprintBefore: IFileFingerprint | null,
  fingerprintAfter: IFileFingerprint | null,
  isLoading: boolean
): boolean {
  if (!previewResult) return true;
  if (isLoading) return true;
  if (!fingerprintBefore || !fingerprintAfter) return true;
  if (fingerprintBefore.sha256 !== fingerprintAfter.sha256) return true;

  const issues = previewResult.validation?.issues || [];
  const hasErrors = issues.some((i) => i.severity === "Error");
  if (hasErrors) return true;

  return false;
}

/**
 * Checks if input changes make the pending preview stale.
 */
export function isPreviewStale(
  snapshot: PreviewParams | null,
  current: PreviewParams
): boolean {
  if (!snapshot) return false;
  return (
    snapshot.targetLevel !== current.targetLevel ||
    snapshot.sectionId !== current.sectionId ||
    snapshot.startMeasure !== current.startMeasure ||
    snapshot.endMeasure !== current.endMeasure ||
    snapshot.songType !== current.songType ||
    snapshot.useMusicAnalysis !== current.useMusicAnalysis ||
    snapshot.useBrowserBpm !== current.useBrowserBpm ||
    snapshot.selectedSectionKey !== current.selectedSectionKey ||
    snapshot.useCalibratedPromptContext !== current.useCalibratedPromptContext ||
    snapshot.patternFamilyTarget !== current.patternFamilyTarget ||
    snapshot.useContinuityPlanning !== current.useContinuityPlanning
  );
}

/**
 * Validates the requested measure range.
 */
export function validateMeasureRange(
  start: number,
  end: number,
  maxMeasures: number = 16
): { isValid: boolean; error: string | null } {
  if (start < 0) {
    return { isValid: false, error: "El compás de inicio no puede ser menor a 0." };
  }
  if (end < start) {
    return { isValid: false, error: "El compás de fin debe ser mayor o igual al compás de inicio." };
  }
  const numMeasures = end - start + 1;
  if (numMeasures > maxMeasures) {
    return {
      isValid: false,
      error: `El rango de compases solicitado (${numMeasures}) supera el límite de ${maxMeasures} compases.`
    };
  }
  return { isValid: true, error: null };
}

/**
 * Splits issues into separate Error and Warning arrays.
 */
export function groupValidationIssues(issues: IValidationIssue[]): {
  errors: IValidationIssue[];
  warnings: IValidationIssue[];
} {
  const errors = issues.filter((i) => i.severity === "Error");
  const warnings = issues.filter((i) => i.severity === "Warning");
  return { errors, warnings };
}

/**
 * Returns a human-friendly label for a given pattern family name.
 */
export function getPatternFamilyLabel(family: string): string {
  const normalized = family.toLowerCase().trim();
  switch (normalized) {
    case "auto":
      return "Auto";
    case "balanced":
      return "Balanced";
    case "stream":
      return "Stream";
    case "jump_accent":
    case "jump_accents":
      return "Jump Accents";
    case "twist_technical":
      return "Twist Technical";
    case "bracket_technical":
      return "Bracket Technical";
    case "hold_control":
      return "Hold Control";
    case "center_control":
      return "Center Control";
    case "stamina":
      return "Stamina";
    default:
      // Capitalize first letters as fallback
      return family
        .split(/[_-]/)
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(" ");
  }
}

/**
 * Returns a human-friendly label for a given motif strategy.
 */
export function getMotifStrategyLabel(strategy: string): string {
  const normalized = strategy.toLowerCase().trim();
  switch (normalized) {
    case "introduce":
      return "Introduce";
    case "develop":
      return "Develop";
    case "intensify":
      return "Intensify";
    case "contrast":
      return "Contrast";
    case "rest":
      return "Rest";
    case "callback":
      return "Callback";
    case "resolve":
      return "Resolve";
    case "final_burst":
      return "Final Burst";
    default:
      return strategy.charAt(0).toUpperCase() + strategy.slice(1).toLowerCase();
  }
}

/**
 * Returns a human-friendly label for a given transition type.
 */
export function getTransitionTypeLabel(type: string): string {
  const normalized = type.toLowerCase().trim();
  switch (normalized) {
    case "smooth_continue":
      return "Smooth Continue";
    case "density_ramp_up":
      return "Density Ramp Up";
    case "density_ramp_down":
      return "Density Ramp Down";
    case "contrast_break":
      return "Contrast Break";
    case "climax_entry":
      return "Climax Entry";
    case "final_resolution":
      return "Final Resolution";
    default:
      return type
        .split(/[_-]/)
        .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
        .join(" ");
  }
}

