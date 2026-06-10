import type { BrowserTempoCandidate, BpmReconciliationReport, BpmSource } from "./browserBpmTypes.ts";

export interface BpmReconciliationInput {
  readonly sscBpms: readonly number[];
  readonly sidecarDetectedBpm?: number;
  readonly browserCandidates: readonly BrowserTempoCandidate[];
  readonly toleranceBpm: number;
  readonly minConfidence?: number;
  readonly minCount?: number;
  readonly isSupported?: boolean;
}

export function reconcileBpmCandidates(
  input: BpmReconciliationInput
): BpmReconciliationReport {
  const {
    sscBpms,
    sidecarDetectedBpm,
    browserCandidates,
    toleranceBpm,
    minConfidence = 0.2,
    minCount = 4,
    isSupported = true,
  } = input;

  const hasBrowserEvidence = browserCandidates.length > 0;

  const browserAliases = new Set(
    browserCandidates.flatMap((c) => c.aliases)
  );

  const browserAgreesWithSsc = !hasBrowserEvidence
    ? false
    : sscBpms.some((sscBpm) =>
        Array.from(browserAliases).some((candidate) =>
          Math.abs(candidate - sscBpm) <= toleranceBpm
        )
      );

  const browserAgreesWithSidecar =
    sidecarDetectedBpm == null
      ? true
      : !hasBrowserEvidence
      ? false
      : Array.from(browserAliases).some((candidate) =>
          Math.abs(candidate - sidecarDetectedBpm) <= toleranceBpm
        );

  const notes: string[] = [];

  if (sscBpms.length > 0 && hasBrowserEvidence && !browserAgreesWithSsc) {
    notes.push(
      "El BPM detectado en el navegador no coincide con #BPMS dentro de la tolerancia configurada."
    );
  }

  if (sidecarDetectedBpm != null && hasBrowserEvidence && !browserAgreesWithSidecar) {
    notes.push(
      "El BPM detectado en el navegador no coincide con el sidecar Python."
    );
  }

  let suggestedBpm: number | undefined = undefined;
  if (sscBpms.length === 0) {
    let rawTarget: number | undefined = undefined;
    if (sidecarDetectedBpm != null) {
      rawTarget = sidecarDetectedBpm;
    } else {
      const bestCandidate = browserCandidates[0];
      if (
        bestCandidate &&
        bestCandidate.confidence >= minConfidence &&
        bestCandidate.count >= minCount
      ) {
        rawTarget = bestCandidate.tempo;
      }
    }

    if (rawTarget != null) {
      const integerCandidate = Math.round(rawTarget);
      if (Math.abs(rawTarget - integerCandidate) <= toleranceBpm) {
        suggestedBpm = integerCandidate;
      } else {
        suggestedBpm = Number(rawTarget.toFixed(3));
      }
    }
  }

  let canonicalSource: BpmSource = "browser_realtime_bpm_analyzer";
  if (sscBpms.length > 0) {
    canonicalSource = "ssc_timing";
  } else if (sidecarDetectedBpm != null) {
    canonicalSource = "python_music_analysis_engine";
  }

  const canonicalBpm =
    sscBpms[0] ?? sidecarDetectedBpm ?? (browserCandidates[0] ? browserCandidates[0].tempo : undefined);

  const requiresManualTimingReview = sscBpms.length > 0 && hasBrowserEvidence && !browserAgreesWithSsc;

  let reconciliationStatus: "agrees" | "disagrees" | "no_browser_evidence" | "unsupported";
  if (isSupported === false) {
    reconciliationStatus = "unsupported";
  } else if (!hasBrowserEvidence) {
    reconciliationStatus = "no_browser_evidence";
  } else if (sscBpms.length > 0) {
    reconciliationStatus = browserAgreesWithSsc ? "agrees" : "disagrees";
  } else if (sidecarDetectedBpm != null) {
    reconciliationStatus = browserAgreesWithSidecar ? "agrees" : "disagrees";
  } else {
    reconciliationStatus = "agrees";
  }

  return {
    canonicalSource,
    canonicalBpm: canonicalBpm != null ? Number(canonicalBpm.toFixed(3)) : undefined,
    browserAgreesWithSsc,
    browserAgreesWithSidecar,
    requiresManualTimingReview,
    suggestedBpm,
    notes,
    reconciliationStatus,
  };
}
