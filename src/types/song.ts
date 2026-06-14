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

export interface ITransitionGuidance {
  transition_type: string;
  density_delta: string;
  family_shift: string;
  recommended_bridge: string;
  warnings: string[];
}

export interface ISectionContinuityNode {
  section_id: string;
  section_index: number;
  start_measure: number;
  end_measure: number;
  music_role: string;
  piu_role: string;
  density_intent: string;
  intensity_band: string;
  primary_pattern_family: string;
  secondary_pattern_families: string[];
  avoid_pattern_families: string[];
  motif_strategy: string;
  transition_in: ITransitionGuidance;
  transition_out: ITransitionGuidance;
  confidence: string;
  evidence: string[];
  warnings: string[];
  enabled: boolean;
  notes?: string | null;
}

export interface ISectionPlanOverride {
  section_id: string;
  enabled?: boolean | null;
  primary_pattern_family?: string | null;
  secondary_pattern_families?: string[] | null;
  avoid_pattern_families?: string[] | null;
  motif_strategy?: string | null;
  intensity_band?: string | null;
  transition_in_type?: string | null;
  transition_out_type?: string | null;
  notes?: string | null;
}

export interface IGlobalArcSummary {
  arc_type: string;
  peak_section_ids: string[];
  rest_section_ids: string[];
  motif_policy: string;
  density_curve: string[];
}

export interface ISongContinuityPlan {
  schema_version: string;
  play_mode: string;
  target_level: number;
  calibration_available: boolean;
  section_count: number;
  sections: ISectionContinuityNode[];
  global_arc: IGlobalArcSummary;
  warnings: string[];
}

export interface INeighborSummary {
  section_id: string;
  music_role: string;
  piu_role: string;
  intensity_band: string;
  primary_family: string;
}

export interface INeighborSummaryGroup {
  previous?: INeighborSummary | null;
  next?: INeighborSummary | null;
}

export interface IContinuityContextSummary {
  enabled: boolean;
  section_index: number;
  section_count: number;
  global_arc: string;
  current_motif_strategy: string;
  transition_in?: ITransitionGuidance | null;
  transition_out?: ITransitionGuidance | null;
  neighbor_summary: INeighborSummaryGroup;
  warnings: string[];
  current_primary_pattern_family: string;
  current_secondary_pattern_families: string[];
  current_avoid_pattern_families: string[];
  current_intensity_band: string;
  current_density_intent: string;
  current_confidence: string;
  current_notes?: string | null;
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
  continuity_plan?: ISongContinuityPlan | null;
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
