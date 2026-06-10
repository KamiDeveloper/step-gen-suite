import test from "node:test";
import assert from "node:assert";
import { expandTempoAliases } from "../browserBpmAliases.ts";
import { getBrowserBpmSupport } from "../browserBpmSupport.ts";
import { reconcileBpmCandidates } from "../browserBpmReconciliation.ts";
import type { BrowserTempoCandidate } from "../browserBpmTypes.ts";

test("expandTempoAliases basic testing", () => {
  const aliases = expandTempoAliases(160);
  assert.ok(aliases.includes(80), "Should include 80");
  assert.ok(aliases.includes(160), "Should include 160");
  assert.ok(aliases.includes(320), "Should include 320");
});

test("expandTempoAliases bounds limits (40-400)", () => {
  const aliases = expandTempoAliases(160);
  assert.ok(aliases.includes(40), "Should include 40");

  const aliasesLow = expandTempoAliases(60); // 60/4 = 15 (less than 40)
  assert.ok(!aliasesLow.includes(15), "Should not include 15");
  assert.ok(aliasesLow.includes(60), "Should include 60");
  assert.ok(aliasesLow.includes(120), "Should include 120");
  assert.ok(aliasesLow.includes(240), "Should include 240");
  assert.ok(!aliasesLow.includes(480), "Should not include 480");
});

test("reconcileBpmCandidates agreement with SSC using half/double", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 80, count: 10, confidence: 0.9, aliases: expandTempoAliases(80) },
  ];
  const report = reconcileBpmCandidates({
    sscBpms: [160],
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(report.browserAgreesWithSsc, true, "Browser BPM 80 (alias 160) should agree with SSC 160");
  assert.strictEqual(report.requiresManualTimingReview, false, "No review needed since they agree");
});

test("reconcileBpmCandidates flags manual review if browser does not match SSC", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 80, count: 10, confidence: 0.9, aliases: expandTempoAliases(80) },
  ];
  const report = reconcileBpmCandidates({
    sscBpms: [120],
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(report.browserAgreesWithSsc, false, "Browser BPM 80 should not agree with SSC 120");
  assert.strictEqual(report.requiresManualTimingReview, true, "Should require manual review");
});

test("reconcileBpmCandidates without SSC uses sidecar or browser as suggestion", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 120.005, count: 10, confidence: 0.9, aliases: expandTempoAliases(120.005) },
  ];
  const reportOnlyBrowser = reconcileBpmCandidates({
    sscBpms: [],
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(reportOnlyBrowser.suggestedBpm, 120, "Should suggest rounded integer 120");

  const reportWithSidecar = reconcileBpmCandidates({
    sscBpms: [],
    sidecarDetectedBpm: 128.2,
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(reportWithSidecar.suggestedBpm, 128, "Should prioritize sidecar and round to 128");
});

test("reconcileBpmCandidates raw BPM near an integer produces suggested integer BPM", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 123.8, count: 10, confidence: 0.9, aliases: expandTempoAliases(123.8) },
  ];

  const reportClose = reconcileBpmCandidates({
    sscBpms: [],
    browserCandidates,
    toleranceBpm: 1.0,
  });
  assert.strictEqual(reportClose.suggestedBpm, 124, "Should round 123.8 to 124 within tolerance");

  const reportFar = reconcileBpmCandidates({
    sscBpms: [],
    browserCandidates,
    toleranceBpm: 0.1,
  });
  assert.strictEqual(reportFar.suggestedBpm, 123.8, "Should not round if outside tolerance");
});

test("getBrowserBpmSupport returns unsupported without crashing when window is undefined", () => {
  const support = getBrowserBpmSupport();
  assert.strictEqual(support.isSupported, false, "Should be unsupported in Node");
  assert.strictEqual(support.hasWindowAudioContext, false);
});

test("reconcileBpmCandidates with empty browser candidates (no evidence)", () => {
  const report = reconcileBpmCandidates({
    sscBpms: [120],
    browserCandidates: [],
    toleranceBpm: 2.0,
  });
  assert.strictEqual(report.browserAgreesWithSsc, true, "Should assume agreement/no contradiction if no evidence");
  assert.strictEqual(report.requiresManualTimingReview, false, "Should not require review if browser is empty");
  assert.strictEqual(report.notes.length, 0, "Should have no warning notes");
});

test("reconcileBpmCandidates low confidence candidates are not suggested", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 120, count: 2, confidence: 0.05, aliases: expandTempoAliases(120) },
  ];
  const report = reconcileBpmCandidates({
    sscBpms: [],
    browserCandidates,
    toleranceBpm: 2.0,
    minConfidence: 0.2,
    minCount: 4,
  });
  assert.strictEqual(report.suggestedBpm, undefined, "Should not suggest BPM if candidate does not meet minConfidence/minCount thresholds");
});

test("reconcileBpmCandidates with multiple sscBpms matches any", () => {
  const browserCandidates: BrowserTempoCandidate[] = [
    { tempo: 150, count: 10, confidence: 0.9, aliases: expandTempoAliases(150) },
  ];
  const reportMatch = reconcileBpmCandidates({
    sscBpms: [120, 150, 180],
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(reportMatch.browserAgreesWithSsc, true, "Should agree if browser BPM matches any of the SSC BPMs");
  assert.strictEqual(reportMatch.requiresManualTimingReview, false, "No review needed");

  const reportNoMatch = reconcileBpmCandidates({
    sscBpms: [120, 130, 180],
    browserCandidates,
    toleranceBpm: 2.0,
  });
  assert.strictEqual(reportNoMatch.browserAgreesWithSsc, false, "Should not agree if browser BPM does not match any SSC BPM");
  assert.strictEqual(reportNoMatch.requiresManualTimingReview, true, "Review needed");
});
