export interface PreviewParams {
  targetLevel: number;
  sectionId: string;
  startMeasure: number;
  endMeasure: number;
  songType: string;
  useMusicAnalysis: boolean;
  useBrowserBpm: boolean;
  selectedSectionKey: string;
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
    snapshot.selectedSectionKey !== current.selectedSectionKey
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
