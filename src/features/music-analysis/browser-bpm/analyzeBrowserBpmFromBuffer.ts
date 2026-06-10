import { analyzeFullBuffer, Tempo } from "realtime-bpm-analyzer";
import { getBrowserBpmSupport } from "./browserBpmSupport.ts";
import { expandTempoAliases } from "./browserBpmAliases.ts";
import type {
  BrowserBpmAnalysisReport,
  BrowserTempoCandidate,
} from "./browserBpmTypes.ts";

export async function analyzeBrowserBpmFromArrayBuffer(params: {
  readonly arrayBuffer: ArrayBuffer;
  readonly audioFileName?: string;
}): Promise<BrowserBpmAnalysisReport> {
  const support = getBrowserBpmSupport();

  if (!support.isSupported) {
    return {
      source: "browser_realtime_bpm_analyzer",
      libraryName: "realtime-bpm-analyzer",
      generatedAtIso: new Date().toISOString(),
      mode: "offline_full_buffer",
      audioFileName: params.audioFileName,
      candidates: [],
      support,
      warnings: [support.reasonIfUnsupported ?? "Browser BPM unsupported"],
    };
  }

  // Create AudioContext safely supporting webkit prefix
  const AudioContextCtor =
    (window as any).AudioContext ?? (window as any).webkitAudioContext;
  const audioContext = new AudioContextCtor();

  try {
    if (audioContext.state === "suspended") {
      await audioContext.resume();
    }

    // Slice arrayBuffer since decodeAudioData transfers the buffer
    const copiedBuffer = params.arrayBuffer.slice(0);
    const audioBuffer = await audioContext.decodeAudioData(copiedBuffer);

    // Run offline analysis
    const tempos: Tempo[] = await analyzeFullBuffer(audioBuffer);

    // Map and sort candidates by confidence descending (realtime-bpm-analyzer usually ranks them already)
    const allCandidates: BrowserTempoCandidate[] = tempos.map((t) => ({
      tempo: Number(t.tempo.toFixed(3)),
      count: t.count,
      confidence: Number(t.confidence.toFixed(3)),
      aliases: expandTempoAliases(t.tempo),
    }));

    // Limit to top 8 candidates
    const candidates = allCandidates.slice(0, 8);

    return {
      source: "browser_realtime_bpm_analyzer",
      libraryName: "realtime-bpm-analyzer",
      generatedAtIso: new Date().toISOString(),
      mode: "offline_full_buffer",
      audioFileName: params.audioFileName,
      audioDurationSeconds: Number(audioBuffer.duration.toFixed(2)),
      candidates,
      stableTempo: candidates[0],
      support,
      warnings: [],
    };
  } catch (error) {
    return {
      source: "browser_realtime_bpm_analyzer",
      libraryName: "realtime-bpm-analyzer",
      generatedAtIso: new Date().toISOString(),
      mode: "offline_full_buffer",
      audioFileName: params.audioFileName,
      candidates: [],
      support,
      warnings: [
        error instanceof Error
          ? error.message
          : "Error desconocido al analizar BPM en el navegador.",
      ],
    };
  } finally {
    if (audioContext && typeof audioContext.close === "function") {
      await audioContext.close().catch(() => undefined);
    }
  }
}
