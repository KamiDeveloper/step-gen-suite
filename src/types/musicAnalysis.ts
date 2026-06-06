export interface AudioSummary {
  sample_rate: number;
  detected_bpm: number;
  rms_energy_mean: number;
  rms_energy_max: number;
  spectral_centroid_mean: number;
  spectral_flatness_mean: number;
  zero_crossing_rate_mean: number;
  chroma_mean: number[] | null;
  spectral_contrast_mean: number[] | null;
  analysis_mode: string;
}

export interface TimingGrid {
  initial_offset: number;
  bpms: [number, number][];
  display_bpm: string;
  song_type: string;
}

export interface AudioEventSummary {
  onset_strength: number;
  energy: number;
}

export interface BeatFrame {
  beat: number;
  time_seconds: number;
  measure_index: number;
  beat_in_measure: number;
  bpm: number;
  confidence: number;
  audio_event_summary: AudioEventSummary;
}

export interface EventFeatures {
  beats: BeatFrame[];
}

export interface SectionFrame {
  section_id: string;
  start_beat: number;
  end_beat: number;
  start_measure: number;
  end_measure: number;
  music_role: string;
  piu_role: string;
  boundary_confidence: number;
  energy_profile: string;
}

export interface AccentFrame {
  beat: number;
  strength: number;
  suggestion: string;
}

export interface RestFrame {
  beat: number;
  strength: number;
  suggestion: string;
}

export interface ChoreographicIntentMap {
  schema_version: string;
  section_id: string;
  mode: string;
  target_level: number;
  measure_start: number;
  measure_end: number;
  density_target: string;
  difficulty_budget: number;
  recommended_pattern_families: string[];
  avoid_pattern_families: string[];
  accent_plan: AccentFrame[];
  rest_plan: RestFrame[];
  motif_strategy: string;
  evidence: string[];
  confidence: number;
}

export interface TimingDiagnostics {
  audio_bpm_detected: number;
  ssc_initial_bpm: number;
  audio_vs_ssc_tempo_agreement: boolean;
  beat_grid_error_ms_mean: number;
  timing_confidence: number;
  requires_manual_timing_review: boolean;
  warnings: string[];
  analysis_mode: string;
}

export interface Publicability {
  contains_original_audio: boolean;
  contains_full_chart: boolean;
  exportable: boolean;
}

export interface SongAnalysisReport {
  schema_version: string;
  song_id: string;
  title: string;
  artist: string;
  duration_seconds: number;
  audio_summary: AudioSummary;
  timing_grid: TimingGrid;
  event_features: EventFeatures;
  sections: SectionFrame[];
  choreographic_intent: ChoreographicIntentMap[];
  diagnostics: TimingDiagnostics;
  publicability: Publicability;
}

export interface AnalysisCommandResult {
  report: SongAnalysisReport;
  report_path: string | null;
  analysis_mode: string;
  warnings: string[];
}
