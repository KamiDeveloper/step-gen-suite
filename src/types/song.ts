export interface IChartDetails {
  steps_type: string;
  difficulty: string;
  meter: number;
  description: string;
  credit: string;
}

export interface IAssetStatus {
  key: string;
  status_type: "DeclaredAndFound" | "DeclaredButMissing" | "FoundButNotDeclared" | "NotDeclared";
  file_name: string | null;
  file_path: string | null;
}

export interface ISongAssetsStatus {
  audio: IAssetStatus;
  banner: IAssetStatus;
  background: IAssetStatus;
  video: IAssetStatus;
}

export interface ISongDetails {
  song_id: string;
  song_name: string;
  artist: string;
  bpm: number;
  offset: number;
  ssc_path: string;
  audio_path: string | null;
  banner_path: string | null;
  background_path: string | null;
  video_path: string | null;
  charts: IChartDetails[];
  asset_statuses: ISongAssetsStatus;
  ssc_bpms?: number[];
}

export type PlayMode = "Single" | "Double";

export type ValidationIssueType =
  | "InvalidLength"
  | "MinaDetected"
  | "InvalidChar"
  | "TripleTap"
  | "DoubleStep"
  | "ConsecutiveJumps"
  | "InvalidGeminiStructure"
  | "CalibrationGuardrailError"
  | "CalibrationGuardrailWarning";

export interface IValidationIssue {
  measure_index: number;
  row_index: number;
  severity: "Warning" | "Error";
  issue_type: ValidationIssueType;
  message: string;
}

export interface IValidatedChartSection {
  play_mode: PlayMode;
  difficulty_level: number;
  issues: IValidationIssue[];
}

export interface ICalibrationWarning {
  issue_type: string;
  message: string;
}

export interface ICalibrationError {
  issue_type: string;
  message: string;
}

export interface ICalibrationValidationReport {
  calibration_available: boolean;
  schema_version: string | null;
  target_level: number | null;
  level_confidence: string | null;
  warnings: ICalibrationWarning[];
  errors: ICalibrationError[];
  matched_thresholds?: any | null;
  pattern_family_signals?: any | null;
  summary: string;
}

export interface IPatternFamilyCandidate {
  family: string;
  score: number;
  confidence: string;
  evidence: string[];
  warnings: string[];
}

export interface IPatternFamilyTargetingReport {
  requested_mode: string;
  primary_family: string;
  secondary_families: string[];
  avoid_families: string[];
  candidates: IPatternFamilyCandidate[];
  confidence: string;
  evidence: string[];
  warnings: string[];
}

export interface ICalibrationContextSummary {
  available: boolean;
  target_level: string;
  level_confidence: string;
  warning_count: number;
  error_count: number;
}

export interface IAppendChartResult {
  charts: IChartDetails[];
  validation: IValidatedChartSection;
  written: boolean;
  message: string;
  generated_notes?: string | null;
  raw_payload?: string | null;
  backup_path?: string | null;
  context_sources_used?: string[] | null;
  calibration_report?: ICalibrationValidationReport | null;
  calibrated_prompt_context_used?: boolean | null;
  pattern_family_targeting?: IPatternFamilyTargetingReport | null;
  calibration_context_summary?: ICalibrationContextSummary | null;
}

export interface IGeminiBiomechanicalState {
  current_twist_debt: number;
  current_stamina_debt: number;
  last_left_foot_lane?: number;
  last_right_foot_lane?: number;
}

export interface IGeminiMeasure {
  measure_index: number;
  subdivision: number;
  rows: string[];
}

export interface IGeminiPayload {
  section_id: string;
  difficulty_level: number;
  play_mode: PlayMode;
  biomechanical_state: IGeminiBiomechanicalState;
  measures: IGeminiMeasure[];
}

export interface IFileFingerprint {
  file_size: number;
  sha256: string;
  modified_time: number;
}
