import type { ISectionPlanOverride } from "../types/song.ts";

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
  isLoading: boolean,
  isSessionUnsafe?: boolean
): boolean {
  if (isSessionUnsafe) return true;
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

/**
 * Returns a human-friendly label for a given intensity band.
 */
export function getIntensityBandLabel(band: string): string {
  const normalized = band.toLowerCase().trim();
  switch (normalized) {
    case "auto":
      return "Auto";
    case "very_low":
      return "Very Low";
    case "low":
      return "Low";
    case "medium":
      return "Medium";
    case "high":
      return "High";
    case "very_high":
      return "Very High";
    default:
      return band.charAt(0).toUpperCase() + band.slice(1).toLowerCase();
  }
}

/**
 * Checks if current overrides differ from generated/snapshot overrides.
 */
export function isSectionPlanStale(
  snapshotOverrides: ISectionPlanOverride[] | null,
  currentOverrides: ISectionPlanOverride[]
): boolean {
  if (!snapshotOverrides) return false;
  if (snapshotOverrides.length !== currentOverrides.length) return true;

  const sortedSnapshot = [...snapshotOverrides].sort((a, b) => a.section_id.localeCompare(b.section_id));
  const sortedCurrent = [...currentOverrides].sort((a, b) => a.section_id.localeCompare(b.section_id));

  for (let i = 0; i < sortedSnapshot.length; i++) {
    const s = sortedSnapshot[i];
    const c = sortedCurrent[i];
    if (s.section_id !== c.section_id) return true;
    if (s.enabled !== c.enabled) return true;
    if (s.primary_pattern_family !== c.primary_pattern_family) return true;
    if (s.motif_strategy !== c.motif_strategy) return true;
    if (s.intensity_band !== c.intensity_band) return true;
    if (s.transition_in_type !== c.transition_in_type) return true;
    if (s.transition_out_type !== c.transition_out_type) return true;
    if (s.notes !== c.notes) return true;

    const sSec = s.secondary_pattern_families || [];
    const cSec = c.secondary_pattern_families || [];
    if (sSec.length !== cSec.length || sSec.some((val, idx) => val !== cSec[idx])) return true;

    const sAvoid = s.avoid_pattern_families || [];
    const cAvoid = c.avoid_pattern_families || [];
    if (sAvoid.length !== cAvoid.length || sAvoid.some((val, idx) => val !== cAvoid[idx])) return true;
  }
  return false;
}

/**
 * Sanitizes and validates section override notes to prevent privacy violations.
 */
export function sanitizeSectionOverrideNote(note: string): { isValid: boolean; error: string | null } {
  if (note.length > 240) {
    return { isValid: false, error: "Notes exceed maximum length of 240 characters." };
  }
  const lower = note.toLowerCase();
  const forbidden = [
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

  for (const item of forbidden) {
    if (lower.includes(item)) {
      return {
        isValid: false,
        error: `Override notes contain forbidden keyword/pattern '${item}'`
      };
    }
  }

  // Windows drive letters check with both slash and backslash
  for (let i = 97; i <= 122; i++) { // a-z
    const drive = String.fromCharCode(i);
    const prefixBackslash = drive + ":\\";
    const prefixSlash = drive + ":/";
    if (lower.includes(prefixBackslash) || lower.includes(prefixSlash)) {
      return {
        isValid: false,
        error: `Override notes contain Windows path prefix (drive letter)`
      };
    }
  }

  // Common system paths check
  const forbiddenFolders = [
    "/users/",
    "/home/",
    "/var/",
    "/tmp/",
    "/etc/",
    "/opt/",
  ];
  for (const folder of forbiddenFolders) {
    if (lower.includes(folder)) {
      return {
        isValid: false,
        error: `Override notes contain system path prefix '${folder}'`
      };
    }
  }

  return { isValid: true, error: null };
}

export interface BatchSnapshot {
  targetLevel: number;
  useCalibratedPromptContext: boolean;
  useContinuityPlanning: boolean;
  patternFamilyTarget: string;
  selectedSectionIds: string[];
  overrides: ISectionPlanOverride[];
}

export function canRunMultiSectionBatch(
  selectedSectionIds: string[],
  plan: ISongContinuityPlan | null
): { isValid: boolean; error: string | null } {
  if (!plan) {
    return { isValid: false, error: "El plan de continuidad no está cargado." };
  }
  if (selectedSectionIds.length === 0) {
    return { isValid: false, error: "No hay secciones seleccionadas." };
  }
  if (selectedSectionIds.length > 4) {
    return { isValid: false, error: "Se permite un máximo de 4 secciones por lote." };
  }

  // Check duplicate IDs
  const seen = new Set<string>();
  for (const id of selectedSectionIds) {
    if (seen.has(id)) {
      return { isValid: false, error: "No se permiten secciones duplicadas en la selección." };
    }
    seen.add(id);
  }

  // Find all nodes
  const nodes: (any | undefined)[] = [];
  for (const id of selectedSectionIds) {
    const found = plan.sections.find((s) => s.section_id === id);
    if (!found) {
      return { isValid: false, error: `Sección seleccionada '${id}' no encontrada en el plan.` };
    }
    nodes.push(found);
  }

  // Check all are enabled
  if (nodes.some((n) => n && !n.enabled)) {
    return { isValid: false, error: "Una o más secciones seleccionadas están deshabilitadas." };
  }

  // Check negative bounds, start >= end, and section length > 16
  for (const node of nodes) {
    if (node) {
      if (node.start_measure < 0 || node.end_measure < 0) {
        return {
          isValid: false,
          error: `La sección '${node.section_id}' tiene límites de compás inválidos (valores negativos).`
        };
      }
      if (node.start_measure >= node.end_measure) {
        return {
          isValid: false,
          error: `La sección '${node.section_id}' tiene límites de compás inválidos (inicio >= fin).`
        };
      }
      const sectionLen = node.end_measure - node.start_measure + 1;
      if (sectionLen > 16) {
        return {
          isValid: false,
          error: `La sección '${node.section_id}' supera el límite máximo de 16 compases (longitud: ${sectionLen}).`
        };
      }
    }
  }

  // Check chronological order by measure
  let lastStart = -1;
  for (const node of nodes) {
    if (node) {
      if (node.start_measure < lastStart) {
        return { isValid: false, error: "Las secciones seleccionadas deben estar ordenadas cronológicamente por compás." };
      }
      lastStart = node.start_measure;
    }
  }

  // Check total measures <= 64
  let totalMeasures = 0;
  for (const node of nodes) {
    if (node) {
      totalMeasures += (node.end_measure - node.start_measure + 1);
    }
  }
  if (totalMeasures > 64) {
    return { isValid: false, error: `El total de compases (${totalMeasures}) supera el límite de 64.` };
  }

  return { isValid: true, error: null };
}

export function selectedSectionCountValidation(count: number): boolean {
  return count > 0 && count <= 4;
}

export function isBatchStale(
  snapshot: BatchSnapshot | null,
  current: BatchSnapshot
): boolean {
  if (!snapshot) return false;

  if (snapshot.targetLevel !== current.targetLevel) return true;
  if (snapshot.useCalibratedPromptContext !== current.useCalibratedPromptContext) return true;
  if (snapshot.useContinuityPlanning !== current.useContinuityPlanning) return true;
  if (snapshot.patternFamilyTarget !== current.patternFamilyTarget) return true;

  if (snapshot.selectedSectionIds.length !== current.selectedSectionIds.length) return true;
  const sSorted = [...snapshot.selectedSectionIds].sort();
  const cSorted = [...current.selectedSectionIds].sort();
  if (sSorted.some((id, idx) => id !== cSorted[idx])) return true;

  return isSectionPlanStale(snapshot.overrides, current.overrides);
}

export function getQueueStatusLabel(status: string): string {
  switch (status.toLowerCase()) {
    case "queued":
      return "En cola";
    case "running":
      return "Generando...";
    case "succeeded":
      return "Completado";
    case "warning":
      return "Con Advertencias";
    case "failed":
      return "Fallido";
    case "skipped":
      return "Omitido";
    default:
      return status;
  }
}

import type { ISongContinuityPlan } from "../types/song.ts";

