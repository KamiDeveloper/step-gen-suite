export type BpmSource =
  | "ssc_timing"
  | "python_music_analysis_engine"
  | "browser_realtime_bpm_analyzer"
  | "manual_user_input";

export interface BrowserTempoCandidate {
  readonly tempo: number;
  readonly count: number;
  readonly confidence: number;
  readonly rawConfidence: number;
  readonly aliases: readonly number[];
}

export interface BrowserBpmSupportReport {
  readonly hasWindowAudioContext: boolean;
  readonly hasAudioWorklet: boolean;
  readonly hasDecodeAudioData: boolean;
  readonly isSupported: boolean;
  readonly reasonIfUnsupported?: string;
}

export interface BrowserBpmAnalysisReport {
  readonly source: "browser_realtime_bpm_analyzer";
  readonly libraryName: "realtime-bpm-analyzer";
  readonly generatedAtIso: string;
  readonly mode: "offline_full_buffer";
  readonly audioFileName?: string;
  readonly audioDurationSeconds?: number;
  readonly candidates: readonly BrowserTempoCandidate[];
  readonly stableTempo?: BrowserTempoCandidate;
  readonly threshold?: number;
  readonly support: BrowserBpmSupportReport;
  readonly warnings: readonly string[];
}

export interface BpmReconciliationReport {
  readonly canonicalSource: BpmSource;
  readonly canonicalBpm?: number;
  readonly browserAgreesWithSsc: boolean;
  readonly browserAgreesWithSidecar: boolean;
  readonly requiresManualTimingReview: boolean;
  readonly suggestedBpm?: number;
  readonly notes: readonly string[];
  readonly reconciliationStatus: "agrees" | "disagrees" | "no_browser_evidence" | "unsupported";
}
