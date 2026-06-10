import type { BrowserBpmSupportReport } from "./browserBpmTypes.ts";

export function getBrowserBpmSupport(): BrowserBpmSupportReport {
  if (typeof window === "undefined") {
    return {
      hasWindowAudioContext: false,
      hasAudioWorklet: false,
      hasDecodeAudioData: false,
      isSupported: false,
      reasonIfUnsupported: "El entorno de ejecución no es un navegador (window es undefined).",
    };
  }

  const AudioContextCtor =
    window.AudioContext ??
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;

  const hasWindowAudioContext = Boolean(AudioContextCtor);
  const hasAudioWorklet = Boolean(
    AudioContextCtor &&
      AudioContextCtor.prototype &&
      "audioWorklet" in AudioContextCtor.prototype
  );
  const hasDecodeAudioData = Boolean(
    AudioContextCtor &&
      AudioContextCtor.prototype &&
      "decodeAudioData" in AudioContextCtor.prototype
  );

  const isSupported = hasWindowAudioContext && hasAudioWorklet && hasDecodeAudioData;

  return {
    hasWindowAudioContext,
    hasAudioWorklet,
    hasDecodeAudioData,
    isSupported,
    reasonIfUnsupported: isSupported
      ? undefined
      : "Web Audio API / AudioWorklet no está disponible en este entorno.",
  };
}
