import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSongProjectStore } from "../store/songProjectStore";
import { useSettingsStore } from "../store/settingsStore";
import {
  Music,
  Compass,
  CheckCircle,
  AlertCircle,
  AlertTriangle,
  ChevronRight,
  Layers,
  Sparkles,
  FileJson
} from "lucide-react";
import { IAppendChartResult, IFileFingerprint, IAssetStatus } from "../types/song";
import { SongAnalysisReport, AnalysisCommandResult } from "../types/musicAnalysis";
import { analyzeBrowserBpmFromArrayBuffer } from "../features/music-analysis/browser-bpm/analyzeBrowserBpmFromBuffer";
import { BrowserBpmAnalysisReport } from "../features/music-analysis/browser-bpm/browserBpmTypes";
import { reconcileBpmCandidates } from "../features/music-analysis/browser-bpm/browserBpmReconciliation";
import { getBrowserBpmSupport } from "../features/music-analysis/browser-bpm/browserBpmSupport";

const getAssetUI = (key: "audio" | "banner" | "background" | "video", status: IAssetStatus | undefined) => {
  const isRequired = key === "audio";
  const reqText = isRequired ? "Required" : "Optional";
  
  if (!status) {
    return {
      color: isRequired ? "red" : "gray",
      statusText: "Not declared",
      reqText,
      filePath: isRequired ? "Missing / Not configured" : "Optional (Missing)",
    };
  }

  let color = "gray";
  let statusText = "Not declared";

  switch (status.status_type) {
    case "DeclaredAndFound":
      color = "green";
      statusText = "Declared and found";
      break;
    case "DeclaredButMissing":
      color = isRequired ? "red" : "yellow";
      statusText = "Declared but missing";
      break;
    case "FoundButNotDeclared":
      color = "yellow";
      statusText = "Found in folder, not declared";
      break;
    case "NotDeclared":
    default:
      color = isRequired ? "red" : "gray";
      statusText = "Not declared";
      break;
  }

  let filePath = status.file_path || status.file_name || "";
  if (!filePath) {
    filePath = isRequired ? "Missing / Not configured" : "Optional (Missing)";
  }

  return {
    color,
    statusText,
    reqText,
    filePath,
  };
};

interface ProjectWorkspaceProps {
  onNavigate: (screen: string) => void;
}

export const ProjectWorkspace: React.FC<ProjectWorkspaceProps> = ({ onNavigate }) => {
  const { currentSong, setCurrentSong, isLoading, setLoading, error, setError } = useSongProjectStore();
  const { settings } = useSettingsStore();

  const [activeTab, setActiveTab] = useState<"overview" | "metadata" | "assets" | "charts" | "generate" | "analysis">("overview");

  // Generate tab states
  const [playMode, setPlayMode] = useState<"Single" | "Double">("Single");
  const [targetLevel, setTargetLevel] = useState(10);
  const [sectionId, setSectionId] = useState("chorus_1");
  const [author, setAuthor] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [startMeasure, setStartMeasure] = useState(0);
  const [endMeasure, setEndMeasure] = useState(7);
  const [songType, setSongType] = useState<"Shortcut" | "Arcade" | "Remix" | "Fullsong">("Arcade");
  const [selectedSectionKey, setSelectedSectionKey] = useState<string>("custom");

  // Preview result states
  const [previewResult, setPreviewResult] = useState<IAppendChartResult | null>(null);
  const [commitMessage, setCommitMessage] = useState<string | null>(null);

  // Fingerprint states
  const [fingerprintBefore, setFingerprintBefore] = useState<IFileFingerprint | null>(null);
  const [fingerprintAfter, setFingerprintAfter] = useState<IFileFingerprint | null>(null);

  // Music Analysis states
  const [analysisReport, setAnalysisReport] = useState<SongAnalysisReport | null>(null);
  const [reportPath, setReportPath] = useState<string | null>(null);
  const [writeReportFile, setWriteReportFile] = useState(true);
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [confirmModal, setConfirmModal] = useState<{ message: string; onConfirm: () => void } | null>(null);

  // Browser BPM Analysis States in Workspace
  const [workspaceBpmReport, setWorkspaceBpmReport] = useState<BrowserBpmAnalysisReport | null>(null);
  const [isWorkspaceBpmAnalyzing, setIsWorkspaceBpmAnalyzing] = useState(false);
  const [workspaceBpmError, setWorkspaceBpmError] = useState<string | null>(null);

  const activeWorkspaceSongIdRef = useRef<string | null>(null);
  const isMountedRef = useRef(true);
  const workspaceRequestIdRef = useRef(0);
  const activeAnalysisPathRef = useRef<string | null>(null);
  const activeSongPathRef = useRef<string | null>(null);
  const activePreviewRequestIdRef = useRef(0);
  const manualAnalysisRequestIdRef = useRef(0);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      invoke("clear_browser_bpm_audio_grants").catch((err) =>
        console.warn("Failed to clear audio grants:", err)
      );
    };
  }, []);

  const runWorkspaceBpmAnalysis = async (reqId: number) => {
    if (!currentSong || !currentSong.audio_path) return;
    const currentSongId = currentSong.song_id;
    activeWorkspaceSongIdRef.current = currentSongId;

    const support = getBrowserBpmSupport();
    if (!support.isSupported) {
      setWorkspaceBpmReport({
        source: "browser_realtime_bpm_analyzer",
        libraryName: "realtime-bpm-analyzer",
        generatedAtIso: new Date().toISOString(),
        mode: "offline_full_buffer",
        audioFileName: currentSong.audio_path.split(/[\\/]/).pop(),
        candidates: [],
        support,
        warnings: [support.reasonIfUnsupported ?? "Browser BPM unsupported"],
      });
      return;
    }

    setIsWorkspaceBpmAnalyzing(true);
    setWorkspaceBpmError(null);
    try {
      // Grant active song audio access just-in-time
      await invoke("grant_active_song_audio_access", { sscPath: currentSong.ssc_path });
      if (workspaceRequestIdRef.current !== reqId || !isMountedRef.current || activeWorkspaceSongIdRef.current !== currentSongId) return;

      const bytes = await invoke<number[]>("read_audio_file", { path: currentSong.audio_path });
      if (workspaceRequestIdRef.current !== reqId || !isMountedRef.current || activeWorkspaceSongIdRef.current !== currentSongId) return;
      const uint8 = new Uint8Array(bytes);
      const report = await analyzeBrowserBpmFromArrayBuffer({
        arrayBuffer: uint8.buffer,
        audioFileName: currentSong.audio_path.split(/[\\/]/).pop(),
      });
      if (workspaceRequestIdRef.current !== reqId || !isMountedRef.current || activeWorkspaceSongIdRef.current !== currentSongId) return;
      setWorkspaceBpmReport(report);
    } catch (err: any) {
      if (workspaceRequestIdRef.current === reqId && isMountedRef.current && activeWorkspaceSongIdRef.current === currentSongId) {
        console.error("Workspace BPM analysis error:", err);
        setWorkspaceBpmError(err.toString());
      }
    } finally {
      if (workspaceRequestIdRef.current === reqId && isMountedRef.current && activeWorkspaceSongIdRef.current === currentSongId) {
        setIsWorkspaceBpmAnalyzing(false);
      }
    }
  };

  useEffect(() => {
    setWorkspaceBpmReport(null);
    setWorkspaceBpmError(null);
    // Lazily do not run workspace BPM analysis automatically on song load.
    if (currentSong && currentSong.audio_path) {
      activeWorkspaceSongIdRef.current = currentSong.song_id;
    } else {
      activeWorkspaceSongIdRef.current = null;
    }
    return () => {
      workspaceRequestIdRef.current++;
      activeWorkspaceSongIdRef.current = null;
      invoke("clear_browser_bpm_audio_grants").catch((err) =>
        console.warn("Failed to clear audio grants:", err)
      );
    };
  }, [currentSong?.song_id]);

  // Sync default generation settings on song load
  useEffect(() => {
    if (settings) {
      setAuthor(settings.default_author || "AI Step Gen");
      setPlayMode((settings.default_play_mode as "Single" | "Double") || "Single");
      setTargetLevel(settings.default_meter || 10);
    }
  }, [settings, currentSong]);

  // Auto-load analysis report if it exists on disk, and reset preview/loading/passphrase states immediately on song change
  useEffect(() => {
    const requestedPath = currentSong?.ssc_path ?? null;

    // Sync active song path and increment request IDs immediately to invalidate pending in-flight queries
    activeSongPathRef.current = requestedPath;
    activePreviewRequestIdRef.current += 1;
    manualAnalysisRequestIdRef.current += 1;
    activeAnalysisPathRef.current = requestedPath;

    // Reset loaders and ephemeral state immediately when song changes
    setLoading(false);
    setAnalysisLoading(false);
    setPassphrase("");

    // Reset analysis and preview states immediately when song changes
    setAnalysisReport(null);
    setReportPath(null);
    setPreviewResult(null);
    setCommitMessage(null);
    setFingerprintBefore(null);
    setFingerprintAfter(null);
    setAnalysisError(null);
    setError(null);

    if (requestedPath) {
      let cancelled = false;
      invoke<SongAnalysisReport | null>("load_analysis_report", {
        sscPath: requestedPath,
      })
        .then((report) => {
          if (cancelled || activeAnalysisPathRef.current !== requestedPath || !isMountedRef.current) {
            return;
          }
          setAnalysisReport(report);
          if (report) {
            const lastSlash = requestedPath.lastIndexOf("/");
            const lastBackslash = requestedPath.lastIndexOf("\\");
            const idx = Math.max(lastSlash, lastBackslash);
            const parentDir = idx !== -1 ? requestedPath.substring(0, idx) : ".";
            setReportPath(`${parentDir}/.ai-step-gen-analysis/song-analysis-report.v1.json`);
          } else {
            setReportPath(null);
          }
        })
        .catch((err) => {
          if (cancelled || activeAnalysisPathRef.current !== requestedPath || !isMountedRef.current) {
            return;
          }
          console.warn("Failed to auto-load analysis report:", err);
          setAnalysisReport(null);
          setReportPath(null);
        });

      return () => {
        cancelled = true;
      };
    }
  }, [currentSong?.ssc_path]);

  if (!currentSong) {
    return (
      <div className="workspace-empty-state">
        <Compass size={48} className="text-slate-icon" />
        <h2 className="empty-title-waldenburg">No Song Loaded</h2>
        <p className="empty-desc-gravel">
          Please load a song from the Import screen or Create Song Wizard.
        </p>
        <button className="btn-primary-pill" onClick={() => onNavigate("IMPORT_EDIT_SONG")}>
          Go to Import
        </button>
      </div>
    );
  }

  // Difficulty limit logic
  const isGeminiBlocked = (playMode === "Single" && targetLevel > 26) || (playMode === "Double" && targetLevel > 15);

  const handleGeneratePreview = async () => {
    if (!currentSong) return;
    if (!passphrase.trim()) {
      setError("Please enter your unlock passphrase to decrypt the API Key.");
      return;
    }

    const matchedSection = analysisReport?.sections.find(s => s.section_id === selectedSectionKey);
    const startMeasureVal = matchedSection ? matchedSection.start_measure : startMeasure;
    const endMeasureVal = matchedSection ? matchedSection.end_measure : endMeasure;

    if (selectedSectionKey === "custom") {
      if (startMeasureVal < 0) {
        setError("El compás de inicio no puede ser menor a 0.");
        return;
      }
      if (endMeasureVal < startMeasureVal) {
        setError("El compás de fin debe ser mayor o igual al compás de inicio.");
        return;
      }
      const numMeasures = endMeasureVal - startMeasureVal + 1;
      if (numMeasures > 64) {
        setError(`El rango de compases solicitado (${numMeasures}) supera el límite de 64 compases permitido para MVP Alpha.`);
        return;
      }
    }

    const previewPath = currentSong.ssc_path;
    const requestId = activePreviewRequestIdRef.current + 1;
    activePreviewRequestIdRef.current = requestId;

    const isActivePreview = () =>
      isMountedRef.current &&
      activeSongPathRef.current === previewPath &&
      activePreviewRequestIdRef.current === requestId;

    setLoading(true);
    setError(null);
    setPreviewResult(null);
    setCommitMessage(null);
    setFingerprintBefore(null);
    setFingerprintAfter(null);

    try {
      // 1. Get file fingerprint BEFORE preview
      let fpBefore: IFileFingerprint | null = null;
      try {
        fpBefore = await invoke<IFileFingerprint>("get_file_fingerprint", {
          path: previewPath,
        });
        if (!isActivePreview()) return;
        setFingerprintBefore(fpBefore);
      } catch (fpErr: any) {
        console.warn("Failed to get initial fingerprint:", fpErr);
      }

      // 2. Real Gemini Preview - writeMode is "PreviewOnly"
      const matchedSectionReal = analysisReport?.sections.find(s => s.section_id === selectedSectionKey);
      const startMeasureValReal = matchedSectionReal ? matchedSectionReal.start_measure : startMeasure;
      const endMeasureValReal = matchedSectionReal ? matchedSectionReal.end_measure : endMeasure;
      const songTypeVal = matchedSectionReal ? (analysisReport?.timing_grid.song_type || songType) : songType;

      const result = await invoke<IAppendChartResult>("generate_gemini_chart_preview", {
        sscPath: previewPath,
        audioPath: currentSong.audio_path || "",
        passphrase: passphrase.trim(),
        playMode,
        targetLevel,
        sectionId: sectionId.trim() || "chorus_1",
        author: author.trim() || "Gemini Preview",
        writeMode: "PreviewOnly",
        startMeasure: startMeasureValReal,
        endMeasure: endMeasureValReal,
        songType: songTypeVal,
      });

      if (!isActivePreview()) return;
      setPreviewResult(result);

      // 3. Get file fingerprint AFTER preview
      try {
        const fpAfter = await invoke<IFileFingerprint>("get_file_fingerprint", {
          path: previewPath,
        });
        if (!isActivePreview()) return;
        setFingerprintAfter(fpAfter);
      } catch (fpErr: any) {
        console.warn("Failed to get final fingerprint:", fpErr);
      }

      const hasErrors = result.validation.issues.some((i) => i.severity === "Error");
      if (hasErrors) {
        setError("Biomechanical validation errors found. Check the report below.");
      }
    } catch (err: any) {
      if (!isActivePreview()) return;
      console.error(err);
      setError("Preview Generation failed: " + err.toString());
    } finally {
      if (isActivePreview()) {
        setPassphrase(""); // Wipe ephemeral passphrase
        setLoading(false);
      }
    }
  };

  const handleCommitChart = () => {
    if (!previewResult || !previewResult.raw_payload) {
      setError("No valid preview data found to commit.");
      return;
    }

    setConfirmModal({
      message: "This will write the generated chart directly into the .ssc file on disk. This action is permanent. Do you want to proceed?",
      onConfirm: async () => {
        setConfirmModal(null);
        await executeCommitChart();
      }
    });
  };

  const executeCommitChart = async () => {
    if (!previewResult || !previewResult.raw_payload) return;
    setLoading(true);
    setError(null);
    setCommitMessage(null);
    setBackupPath(null);

    try {
      const result = await invoke<IAppendChartResult>("append_approved_gemini_payload", {
        sscPath: currentSong.ssc_path,
        payloadJson: previewResult.raw_payload,
        author: author.trim() || "Gemini Approved",
        expectedSha256: fingerprintAfter?.sha256 || "",
      });

      if (result.written) {
        setCommitMessage("Chart successfully written to .ssc file on disk!");
        if (result.backup_path) {
          setBackupPath(result.backup_path);
        }
        setCurrentSong({
          ...currentSong,
          charts: result.charts,
        });
        setPreviewResult(null); // Clear preview once committed
        setFingerprintBefore(null);
        setFingerprintAfter(null);
      } else {
        setError("Failed to commit chart: " + result.message);
      }
    } catch (err: any) {
      console.error(err);
      setError("Error committing chart: " + err.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleDiscardPreview = () => {
    setPreviewResult(null);
    setError(null);
    setCommitMessage(null);
    setBackupPath(null);
    setFingerprintBefore(null);
    setFingerprintAfter(null);
  };

  // Developer tools handlers
  const handleDevAppendLocalStub = () => {
    setConfirmModal({
      message: "This will append a basic fixed local chart stub to the active .ssc (does not use API, writes to disk). Proceed?",
      onConfirm: async () => {
        setConfirmModal(null);
        await executeDevAppendLocalStub();
      }
    });
  };

  const executeDevAppendLocalStub = async () => {
    setLoading(true);
    setError(null);
    setCommitMessage(null);
    setBackupPath(null);

    try {
      const result = await invoke<IAppendChartResult>("append_ai_chart_stub", {
        sscPath: currentSong.ssc_path,
        playMode,
        targetLevel,
        author: author.trim() || "Dev Test",
      });

      if (result.written) {
        setCommitMessage(`Successfully appended local stub chart! (${result.message})`);
        if (result.backup_path) {
          setBackupPath(result.backup_path);
        }
        setCurrentSong({
          ...currentSong,
          charts: result.charts,
        });
      } else {
        setError("Local stub rejected: " + result.message);
      }
    } catch (err: any) {
      console.error(err);
      setError("Error appending local stub: " + err.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleDevTestMockContract = () => {
    setConfirmModal({
      message: "This will append a mock structured Gemini payload to the .ssc to test parser boundaries (does not use API, writes to disk). Proceed?",
      onConfirm: async () => {
        setConfirmModal(null);
        await executeDevTestMockContract();
      }
    });
  };

  const executeDevTestMockContract = async () => {
    setLoading(true);
    setError(null);
    setCommitMessage(null);
    setBackupPath(null);

    try {
      const mockPayload = {
        section_id: "chorus_mock",
        difficulty_level: targetLevel,
        play_mode: playMode,
        biomechanical_state: {
          current_twist_debt: 0.0,
          current_stamina_debt: 0.2,
          last_left_foot_lane: playMode === "Single" ? 1 : 1,
          last_right_foot_lane: playMode === "Single" ? 3 : 8,
        },
        measures: [
          {
            measure_index: 0,
            subdivision: 4,
            rows: playMode === "Single"
              ? ["10000", "00100", "00001", "00100"]
              : ["1000000000", "0000010000", "0000000001", "0000010000"],
          },
        ],
      };

      const result = await invoke<IAppendChartResult>("append_mock_gemini_payload", {
        sscPath: currentSong.ssc_path,
        payloadJson: JSON.stringify(mockPayload),
        author: author.trim() || "Mock Contract",
      });

      if (result.written) {
        setCommitMessage(`Successfully appended Mock Gemini Contract chart! (${result.message})`);
        if (result.backup_path) {
          setBackupPath(result.backup_path);
        }
        setCurrentSong({
          ...currentSong,
          charts: result.charts,
        });
      } else {
        setError("Mock contract write rejected: " + result.message);
      }
    } catch (err: any) {
      console.error(err);
      setError("Error appending mock contract: " + err.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleRunOfflineAnalysis = async () => {
    if (!currentSong) return;

    const analysisPath = currentSong.ssc_path;
    const requestId = manualAnalysisRequestIdRef.current + 1;
    manualAnalysisRequestIdRef.current = requestId;

    const isActiveAnalysis = () =>
      isMountedRef.current &&
      activeSongPathRef.current === analysisPath &&
      manualAnalysisRequestIdRef.current === requestId;

    setAnalysisLoading(true);
    setAnalysisError(null);
    setAnalysisReport(null);
    setReportPath(null);

    try {
      const result = await invoke<AnalysisCommandResult>("analyze_song_offline", {
        sscPath: analysisPath,
        audioPath: currentSong.audio_path || "",
        writeReport: writeReportFile,
      });

      if (!isActiveAnalysis()) return;
      setAnalysisReport(result.report);
      setReportPath(result.report_path);
    } catch (err: any) {
      if (!isActiveAnalysis()) return;
      console.error(err);
      setAnalysisError(err.toString());
    } finally {
      if (isActiveAnalysis()) {
        setAnalysisLoading(false);
      }
    }
  };

  return (
    <div className="workspace-container">
      {/* Workspace Sidebar / Navigation */}
      <aside className="workspace-sidebar">
        <div className="workspace-song-heading">
          <Music className="text-obsidian" size={24} />
          <div>
            <h2 className="song-title-waldenburg">{currentSong.song_name}</h2>
            <p className="song-artist-gravel">by {currentSong.artist}</p>
          </div>
        </div>

        <nav className="workspace-nav">
          <button
            className={`workspace-nav-btn ${activeTab === "overview" ? "active" : ""}`}
            onClick={() => setActiveTab("overview")}
          >
            Overview
          </button>
          <button
            className={`workspace-nav-btn ${activeTab === "metadata" ? "active" : ""}`}
            onClick={() => setActiveTab("metadata")}
          >
            Metadata
          </button>
          <button
            className={`workspace-nav-btn ${activeTab === "assets" ? "active" : ""}`}
            onClick={() => setActiveTab("assets")}
          >
            Assets Check
          </button>
          <button
            className={`workspace-nav-btn ${activeTab === "charts" ? "active" : ""}`}
            onClick={() => setActiveTab("charts")}
          >
            Charts List ({currentSong.charts.length})
          </button>
          <button
            className={`workspace-nav-btn ${activeTab === "generate" ? "active" : ""}`}
            onClick={() => setActiveTab("generate")}
          >
            Generate Chart
          </button>
          <button
            className={`workspace-nav-btn ${activeTab === "analysis" ? "active" : ""}`}
            onClick={() => setActiveTab("analysis")}
          >
            Music Analysis
          </button>
        </nav>

        <div className="workspace-sidebar-footer">
          <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => onNavigate("START_MENU")}>
            Close Project
          </button>
        </div>
      </aside>

      {/* Main Workspace Workspace Tab Content */}
      <main className="workspace-main-content">
        {error && (
          <div className="error-box">
            <AlertCircle size={16} className="icon-mr text-danger-icon" />
            <span>{error}</span>
          </div>
        )}

        {commitMessage && (
          <div className="success-box">
            <CheckCircle size={16} className="icon-mr text-success-icon" />
            <div className="success-box-content">
              <span>{commitMessage}</span>
              {backupPath && (
                <div className="success-backup-text">
                  Backup: {backupPath}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Tab: Overview */}
        {activeTab === "overview" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Song Overview</h3>
            <p className="section-subtitle-gravel">Quick summary of the active project and assets.</p>

            <div className="stats-grid">
              <div className="stat-card">
                <span className="stat-label">BPM</span>
                <span className="stat-value">{currentSong.bpm}</span>
              </div>
              <div className="stat-card">
                <span className="stat-label">Offset</span>
                <span className="stat-value">{currentSong.offset}s</span>
              </div>
              <div className="stat-card">
                <span className="stat-label">Existing Charts</span>
                <span className="stat-value">{currentSong.charts.length}</span>
              </div>
            </div>

            <div className="workspace-card">
              <h4 className="card-subtitle-obsidian">Project File Paths</h4>
              <div className="path-row">
                <span className="path-label">SSC File:</span>
                <code className="monospace-block">{currentSong.ssc_path}</code>
              </div>
              {currentSong.audio_path && (
                <div className="path-row">
                  <span className="path-label">Audio File:</span>
                  <code className="monospace-block">{currentSong.audio_path}</code>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Tab: Metadata */}
        {activeTab === "metadata" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Song Metadata</h3>
            <p className="section-subtitle-gravel">Technical parameters loaded from the SSC file headers.</p>

            <div className="metadata-table-card">
              <table className="workspace-table">
                <thead>
                  <tr>
                    <th>Tag Key</th>
                    <th>Value</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>TITLE</strong></td>
                    <td>{currentSong.song_name}</td>
                  </tr>
                  <tr>
                    <td><strong>ARTIST</strong></td>
                    <td>{currentSong.artist}</td>
                  </tr>
                  <tr>
                    <td><strong>BPMS</strong></td>
                    <td>{currentSong.bpm}</td>
                  </tr>
                  <tr>
                    <td><strong>OFFSET</strong></td>
                    <td>{currentSong.offset}</td>
                  </tr>
                  <tr>
                    <td><strong>SONG ID</strong></td>
                    <td className="monospace-inline">{currentSong.song_id}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Tab: Assets Check */}
        {activeTab === "assets" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Assets Verification</h3>
            <p className="section-subtitle-gravel">Simulator asset completeness and folder inspection.</p>

            <div className="assets-status-grid">
              {(() => {
                const renderRow = (key: "audio" | "banner" | "background" | "video", label: string) => {
                  const status = currentSong.asset_statuses?.[key];
                  const ui = getAssetUI(key, status);
                  return (
                    <div className="asset-status-item">
                      <div className={`status-light ${ui.color}`} />
                      <div className="asset-status-meta">
                        <div className="asset-status-row-header">
                          <span className="asset-status-name">{label}</span>
                          <div>
                            <span className={`asset-status-badge ${ui.color}`}>
                              {ui.statusText}
                            </span>
                            <span className="asset-status-req-label">
                              {ui.reqText}
                            </span>
                          </div>
                        </div>
                        <span className="asset-status-path">
                          {ui.filePath}
                        </span>
                      </div>
                    </div>
                  );
                };

                return (
                  <>
                    {renderRow("audio", "Music Audio (.mp3/.ogg/.flac/.wav)")}
                    {renderRow("banner", "Pack Banner Image (.png/.jpg/.jpeg)")}
                    {renderRow("background", "Background Image (.png/.jpg/.jpeg)")}
                    {renderRow("video", "Video Overlay (.mp4/.mov/.avi/.mpg)")}
                  </>
                );
              })()}
            </div>
          </div>
        )}

        {/* Tab: Charts List */}
        {activeTab === "charts" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Registered Charts</h3>
            <p className="section-subtitle-gravel">List of choreography charts written inside the `.ssc` file.</p>

            <div className="charts-table-container">
              <table className="workspace-table">
                <thead>
                  <tr>
                    <th>Mode</th>
                    <th>Difficulty</th>
                    <th>Level</th>
                    <th>Stepmaker</th>
                    <th>Description</th>
                  </tr>
                </thead>
                <tbody>
                  {currentSong.charts.map((chart, idx) => (
                    <tr key={idx}>
                      <td>
                        <span className={`badge-mode-${chart.steps_type.includes("double") ? "double" : "single"}`}>
                          {chart.steps_type.replace("pump-", "")}
                        </span>
                      </td>
                      <td><strong>{chart.difficulty}</strong></td>
                      <td><span className="level-badge">{chart.meter}</span></td>
                      <td>{chart.credit || "—"}</td>
                      <td className="desc-cell">{chart.description || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Tab: Generate Chart */}
        {activeTab === "generate" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Generate Stepchart Section</h3>
            <p className="section-subtitle-gravel">
              Generate choreography patterns using Gemini 3.5 Flash. Real Gemini requests run in memory and never modify disk files without explicit confirmation.
            </p>

            {/* Inputs Panel */}
            <div className="generate-form-card">
              <h4 className="card-subtitle-obsidian">Gemini Real Preview Configuration</h4>
              <p className="caption-text-gravel">
                Select target mode and difficulty parameters for real AI generation.
              </p>

              <div className="generate-grid">
                <div className="form-group-contained">
                  <label className="form-label-dark">Play Mode</label>
                  <select
                    className="input-contained"
                    value={playMode}
                    onChange={(e) => setPlayMode(e.target.value as "Single" | "Double")}
                    disabled={isLoading}
                  >
                    <option value="Single">Single (5-Key)</option>
                    <option value="Double">Double (10-Key)</option>
                  </select>
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Target Level</label>
                  <input
                    type="number"
                    min="1"
                    max="28"
                    className="input-contained"
                    value={targetLevel}
                    onChange={(e) => setTargetLevel(parseInt(e.target.value) || 10)}
                    disabled={isLoading}
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Select Section</label>
                  <select
                    className="input-contained"
                    value={selectedSectionKey}
                    onChange={(e) => {
                      const val = e.target.value;
                      setSelectedSectionKey(val);
                      if (val !== "custom" && analysisReport) {
                        const sec = analysisReport.sections.find((s) => s.section_id === val);
                        if (sec) {
                          setSectionId(sec.section_id);
                          setStartMeasure(sec.start_measure);
                          setEndMeasure(sec.end_measure);
                          if (analysisReport.timing_grid.song_type) {
                            setSongType(analysisReport.timing_grid.song_type as any);
                          }
                        }
                      }
                    }}
                    disabled={isLoading}
                  >
                    <option value="custom">Custom Section / Manual Range</option>
                    {analysisReport?.sections.map((sec) => (
                      <option key={sec.section_id} value={sec.section_id}>
                        {sec.section_id} (M.{sec.start_measure} – M.{sec.end_measure})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Stepmaker Credit</label>
                  <input
                    type="text"
                    className="input-contained"
                    value={author}
                    onChange={(e) => setAuthor(e.target.value)}
                    placeholder="e.g. Gemini Preview"
                    disabled={isLoading}
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Section Identifier</label>
                  <input
                    type="text"
                    className="input-contained"
                    value={sectionId}
                    onChange={(e) => setSectionId(e.target.value)}
                    placeholder="e.g. chorus_1"
                    disabled={isLoading || selectedSectionKey !== "custom"}
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Song Type</label>
                  <select
                    className="input-contained"
                    value={songType}
                    onChange={(e) => setSongType(e.target.value as any)}
                    disabled={isLoading || selectedSectionKey !== "custom"}
                  >
                    <option value="Arcade">Arcade</option>
                    <option value="Shortcut">Shortcut</option>
                    <option value="Remix">Remix</option>
                    <option value="Fullsong">Fullsong</option>
                  </select>
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Start Measure (Compás Inicio)</label>
                  <input
                    type="number"
                    min="0"
                    className="input-contained"
                    value={startMeasure}
                    onChange={(e) => setStartMeasure(parseInt(e.target.value) || 0)}
                    disabled={isLoading || selectedSectionKey !== "custom"}
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">End Measure (Compás Fin)</label>
                  <input
                    type="number"
                    min="0"
                    className="input-contained"
                    value={endMeasure}
                    onChange={(e) => setEndMeasure(parseInt(e.target.value) || 0)}
                    disabled={isLoading || selectedSectionKey !== "custom"}
                  />
                </div>
              </div>

              {selectedSectionKey === "custom" && (
                <p className="custom-section-hint">
                  Nota: El rango máximo de compases permitido para Custom Section es de 64 compases.
                </p>
              )}

              {(() => {
                const matchedIntent = analysisReport?.choreographic_intent.find(
                  (intent) => intent.section_id === selectedSectionKey
                );
                if (!matchedIntent || selectedSectionKey === "custom") return null;
                return (
                  <div className="accent-card">
                    <h5 className="intent-guide-title">
                      <Sparkles size={16} /> Choreographic Intent Guide for {matchedIntent.section_id}
                    </h5>
                    <div className="intent-guide-grid">
                      <div>
                        <span className="intent-guide-label">Density Target:</span>{" "}
                        <span className="level-badge">{matchedIntent.density_target}</span>
                      </div>
                      <div>
                        <span className="intent-guide-label">Difficulty Budget:</span>{" "}
                        <span className="level-badge">{matchedIntent.difficulty_budget}</span>
                      </div>
                      <div className="intent-guide-span-2">
                        <span className="intent-guide-label">Recommended Patterns:</span>{" "}
                        {matchedIntent.recommended_pattern_families.map((p, i) => (
                          <span key={i} className="analysis-badge-recommend intent-badge">
                            {p.replace('_', ' ')}
                          </span>
                        ))}
                      </div>
                      {matchedIntent.avoid_pattern_families && matchedIntent.avoid_pattern_families.length > 0 && (
                        <div className="intent-guide-span-2">
                          <span className="intent-guide-label">Patterns to Avoid:</span>{" "}
                          {matchedIntent.avoid_pattern_families.map((p, i) => (
                            <span key={i} className="analysis-badge-avoid intent-badge">
                              {p.replace('_', ' ')}
                            </span>
                          ))}
                        </div>
                      )}
                      <div className="intent-guide-span-2">
                        <span className="intent-guide-label">Motif Strategy:</span>{" "}
                        <span className="intent-guide-value">{matchedIntent.motif_strategy}</span>
                      </div>
                    </div>
                  </div>
                );
              })()}

              <div className="form-group-contained">
                <label className="form-label-dark">Decryption Passphrase</label>
                <input
                  type="password"
                  className="input-contained"
                  placeholder="Enter passphrase to authorize API key usage"
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                  disabled={isLoading}
                />
              </div>

              {isGeminiBlocked && (
                <div className="warning-box">
                  <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                  <span>
                    Gemini generation limits: Single up to Lv.26, Double up to Lv.15. Selection is blocked.
                  </span>
                </div>
              )}

              {!currentSong.audio_path && (
                <div className="warning-box">
                  <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                  <span>Audio file is required to analyze music structure.</span>
                </div>
              )}

              <div className="action-row-with-sub">
                <button
                  className="btn-primary-pill"
                  onClick={handleGeneratePreview}
                  disabled={isLoading || isGeminiBlocked || !currentSong.audio_path || !passphrase}
                >
                  {isLoading ? "Generating..." : "Generar preview con Gemini (usa API, no escribe)"}
                </button>
                <span className="caption-text-gravel">
                  * Real Gemini (uses network & API key, consumes tokens, <strong>PreviewOnly</strong> mode - does not write to disk).
                </span>
              </div>
            </div>

            {/* Pending Preview Review Area */}
            {previewResult && (
              <div className="preview-results-wrapper">
                <div className="preview-header-meta">
                  <h4 className="preview-result-title">Revisión de preview pendiente</h4>
                  <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={handleDiscardPreview}>
                    Descartar preview
                  </button>
                </div>

                {/* Fingerprint Evidence Section */}
                <div className="fingerprint-section">
                  <div className="fingerprint-title">
                    <CheckCircle size={16} className="text-success-icon" />
                    <span>Evidencia de no escritura en disco (.ssc fingerprint)</span>
                  </div>
                  <div className="fingerprint-grid">
                    <div className="fingerprint-card">
                      <span className="fingerprint-label">Antes del Preview</span>
                      {fingerprintBefore ? (
                        <div>
                          <div>Size: <span className="fingerprint-val">{fingerprintBefore.file_size} bytes</span></div>
                          <div>Hash: <span className="fingerprint-val">{fingerprintBefore.sha256.slice(0, 16)}...</span></div>
                        </div>
                      ) : (
                        <span className="caption-text-gravel">Unavailable</span>
                      )}
                    </div>
                    <div className="fingerprint-card">
                      <span className="fingerprint-label">Después del Preview</span>
                      {fingerprintAfter ? (
                        <div>
                          <div>Size: <span className="fingerprint-val">{fingerprintAfter.file_size} bytes</span></div>
                          <div>Hash: <span className="fingerprint-val">{fingerprintAfter.sha256.slice(0, 16)}...</span></div>
                        </div>
                      ) : (
                        <span className="caption-text-gravel">Unavailable</span>
                      )}
                    </div>
                  </div>

                  {fingerprintBefore && fingerprintAfter && fingerprintBefore.sha256 === fingerprintAfter.sha256 ? (
                    <div className="fingerprint-compare-banner fingerprint-match">
                      ✓ Sin cambios detectados en el archivo .ssc (Escritura prevenida con éxito)
                    </div>
                  ) : fingerprintBefore && fingerprintAfter ? (
                    <div className="fingerprint-compare-banner fingerprint-mismatch">
                      ⚠ ADVERTENCIA: El archivo .ssc ha cambiado inesperadamente. Aprobación bloqueada.
                    </div>
                  ) : null}
                </div>

                {/* Validation Report */}
                <div className={`validation-report-panel ${previewResult.validation.issues.some(i => i.severity === "Error")
                    ? "has-errors"
                    : previewResult.validation.issues.length > 0
                      ? "has-warnings"
                      : "clean"
                  }`}>
                  <div className="validation-report-header">
                    <span className="validation-report-summary">
                      Biomechanical Report ({previewResult.validation.play_mode} Lv.{previewResult.validation.difficulty_level})
                    </span>
                    <span className="validation-count-badge">
                      {previewResult.validation.issues.length === 0
                        ? "Clean (0 Issues)"
                        : `${previewResult.validation.issues.filter(i => i.severity === "Error").length} errors, ${previewResult.validation.issues.filter(i => i.severity === "Warning").length} warnings`}
                    </span>
                  </div>

                  {previewResult.validation.issues.length > 0 ? (
                    <div className="validation-issues-list">
                      {previewResult.validation.issues.map((issue, index) => (
                        <div key={index} className={`issue-row-item ${issue.severity.toLowerCase()}`}>
                          <span className="issue-meta">
                            Measure {issue.measure_index + 1}, Row {issue.row_index + 1}
                          </span>
                          <span className="issue-message">
                            [{issue.issue_type}] {issue.message}
                          </span>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="clean-validation-text">
                      ✓ Zero biomechanical or structure issues detected for this section.
                    </p>
                  )}
                </div>

                {/* Collapsible Inspectors */}
                <div className="collapsible-inspectors-stack">
                  <details className="inspector-accordion">
                    <summary className="inspector-summary">
                      <ChevronRight size={14} className="icon-mr accordion-icon" />
                      Inspect Raw Gemini Payload (JSON)
                    </summary>
                    <pre className="monospace-block">
                      <code>{previewResult.raw_payload}</code>
                    </pre>
                  </details>

                  <details className="inspector-accordion">
                    <summary className="inspector-summary">
                      <ChevronRight size={14} className="icon-mr accordion-icon" />
                      Inspect Generated SSC Notes
                    </summary>
                    <pre className="monospace-block">
                      <code>{previewResult.generated_notes}</code>
                    </pre>
                  </details>
                </div>

                {/* Committing Actions */}
                <div className="commit-action-box">
                  <div className="commit-meta-text">
                    <h5 className="commit-box-title">Apply Generated Chart</h5>
                    {previewResult.validation.issues.some(i => i.severity === "Error") ? (
                      <p className="caption-text-gravel text-missing">
                        * Committing blocked: Resolve severe biomechanical errors first.
                      </p>
                    ) : !fingerprintBefore || !fingerprintAfter ? (
                      <p className="caption-text-gravel text-missing">
                        * Committing blocked: SSC fingerprint is unavailable or unverified.
                      </p>
                    ) : fingerprintBefore.sha256 !== fingerprintAfter.sha256 ? (
                      <p className="caption-text-gravel text-missing">
                        * Committing blocked: SSC fingerprint mismatch.
                      </p>
                    ) : (
                      <p className="caption-text-gravel">
                        If the preview checks out, commit it to disk. This is a local action, does not query the API, and does not consume tokens.
                      </p>
                    )}
                  </div>

                  <button
                    className="btn-primary-pill btn-success-glow"
                    onClick={handleCommitChart}
                    disabled={
                      previewResult.validation.issues.some(i => i.severity === "Error") ||
                      !fingerprintBefore ||
                      !fingerprintAfter ||
                      fingerprintBefore.sha256 !== fingerprintAfter.sha256 ||
                      isLoading
                    }
                  >
                    Añadir preview aprobado al SSC (escribe en disco)
                  </button>
                </div>
              </div>
            )}

            {/* Collapsible Developer Tools */}
            <details className="dev-tools-collapsible">
              <summary className="dev-tools-summary">
                <ChevronRight size={14} className="accordion-icon" />
                Developer Tools
              </summary>
              <div className="dev-tools-content">
                <h4 className="dev-tools-title">Offline Test Generators</h4>
                <p className="caption-text-gravel">
                  Offline mock operations for parser testing. These write directly to the SSC file and bypass the Gemini API.
                </p>

                <div className="dev-buttons-row">
                  <button
                    className="btn-primary-pill"
                    onClick={handleDevAppendLocalStub}
                    disabled={isLoading}
                  >
                    Dev: añadir chart local fijo (no usa API, escribe)
                  </button>
                  <button
                    className="btn-ghost-pill"
                    onClick={handleDevTestMockContract}
                    disabled={isLoading}
                  >
                    Dev: probar contrato mock (no usa API, escribe)
                  </button>
                </div>
                <span className="caption-text-gravel">
                  * Warning: Both developer buttons <strong>write directly to disk</strong> (modifies the active .ssc) and bypass network API calls.
                </span>
              </div>
            </details>
          </div>
        )}

        {/* Tab: Music Analysis */}
        {activeTab === "analysis" && (
          <div className="workspace-tab">
            <h3 className="section-title-waldenburg">Music Analysis</h3>
            <p className="section-subtitle-gravel">Audio beat grid structure and phrase mapping.</p>

            {/* Quick BPM Diagnostics Panel */}
            <details className="dev-tools-collapsible" open>
              <summary className="dev-tools-summary">
                <ChevronRight size={14} className="accordion-icon" />
                Quick BPM Diagnostics Panel
              </summary>
              <div className="dev-tools-content">
                <p className="caption-text-gravel">
                  Cross-checks browser-detected tempo with SSC timing data and Python Sidecar results.
                </p>

                {isWorkspaceBpmAnalyzing ? (
                  <div className="info-banner-gray">
                    <div className="gemini-waves-container animating">
                      <div className="gemini-wave wave1"></div>
                      <div className="gemini-wave wave2"></div>
                      <div className="gemini-wave wave3"></div>
                    </div>
                    <span className="icon-ml">Analyzing audio buffer in browser...</span>
                  </div>
                ) : workspaceBpmError ? (
                  <div className="error-box">
                    <AlertTriangle size={16} className="icon-mr text-danger-icon" />
                    <span>Failed browser BPM analysis: {workspaceBpmError}</span>
                  </div>
                ) : workspaceBpmReport ? (
                  <div>
                    <div className="bpm-diagnostics-grid">
                      <div className="bpm-diagnostics-card">
                        <span className="bpm-diagnostics-label">SSC Initial BPM</span>
                        <span className="bpm-diagnostics-value">
                          {currentSong.ssc_bpms && currentSong.ssc_bpms.length > 0
                            ? currentSong.ssc_bpms.join(", ")
                            : (currentSong.bpm || "None")}
                        </span>
                      </div>
                      <div className="bpm-diagnostics-card">
                        <span className="bpm-diagnostics-label">Sidecar DSP BPM</span>
                        <span className="bpm-diagnostics-value">
                          {analysisReport?.diagnostics?.audio_bpm_detected ?? "Not analyzed"}
                        </span>
                      </div>
                      <div className="bpm-diagnostics-card">
                        <span className="bpm-diagnostics-label">Browser BPM</span>
                        <span className="bpm-diagnostics-value">
                          {workspaceBpmReport.stableTempo?.tempo ?? "None"}
                        </span>
                      </div>
                      <div className="bpm-diagnostics-card">
                        <span className="bpm-diagnostics-label">Reconciliation</span>
                        {(() => {
                          const recon = reconcileBpmCandidates({
                            sscBpms: currentSong.ssc_bpms && currentSong.ssc_bpms.length > 0
                              ? currentSong.ssc_bpms
                              : (currentSong.bpm ? [currentSong.bpm] : []),
                            sidecarDetectedBpm: analysisReport?.diagnostics?.audio_bpm_detected || undefined,
                            browserCandidates: workspaceBpmReport.candidates,
                            toleranceBpm: 2.0,
                            minConfidence: 0.2,
                            minCount: 4,
                            isSupported: workspaceBpmReport.support.isSupported,
                          });
                          let displayStatus = "";
                          let statusClass = "";
                          if (recon.reconciliationStatus === "unsupported") {
                            displayStatus = "Unsupported";
                            statusClass = "requires-review";
                          } else if (recon.reconciliationStatus === "no_browser_evidence") {
                            displayStatus = "No Browser Evidence";
                            statusClass = "caption-text-gravel";
                          } else if (recon.reconciliationStatus === "disagrees") {
                            displayStatus = "Manual Review Required";
                            statusClass = "requires-review";
                          } else {
                            displayStatus = "Agreed";
                            statusClass = "";
                          }
                          return (
                            <span className={`bpm-diagnostics-value ${statusClass}`}>
                              {displayStatus}
                            </span>
                          );
                        })()}
                      </div>
                    </div>

                    {workspaceBpmReport.warnings && workspaceBpmReport.warnings.length > 0 && (
                      <div className="warning-box">
                        <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                        <div>
                          {workspaceBpmReport.warnings.map((w, idx) => (
                            <div key={idx}>{w}</div>
                          ))}
                        </div>
                      </div>
                    )}

                    {(() => {
                      const recon = reconcileBpmCandidates({
                        sscBpms: currentSong.ssc_bpms && currentSong.ssc_bpms.length > 0
                          ? currentSong.ssc_bpms
                          : (currentSong.bpm ? [currentSong.bpm] : []),
                        sidecarDetectedBpm: analysisReport?.diagnostics?.audio_bpm_detected || undefined,
                        browserCandidates: workspaceBpmReport.candidates,
                        toleranceBpm: 2.0,
                        minConfidence: 0.2,
                        minCount: 4,
                        isSupported: workspaceBpmReport.support.isSupported,
                      });
                      return (
                        <>
                          {recon.notes && recon.notes.length > 0 && (
                            <div className="warning-box">
                              <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                              <div>
                                {recon.notes.map((note, idx) => (
                                  <div key={idx}>{note}</div>
                                ))}
                              </div>
                            </div>
                          )}
                        </>
                      );
                    })()}

                    {workspaceBpmReport.candidates && workspaceBpmReport.candidates.length > 0 && (
                      <div className="bpm-candidates-section">
                        <span className="bpm-stat-title">Browser Tempo Candidates</span>
                        <div className="bpm-candidates-list">
                          {workspaceBpmReport.candidates.map((c, i) => (
                            <div key={i} className="bpm-candidate-row">
                              <span className="bpm-candidate-tempo">
                                <strong>{c.tempo}</strong> BPM
                              </span>
                              <span className="bpm-candidate-meta">
                                Count: {c.count} | Relative Score: {(c.confidence * 100).toFixed(0)}% | Aliases: {c.aliases.join(", ")}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                ) : (
                  <div>
                    <button
                      type="button"
                      className="btn-ghost-pill btn-sm-contained"
                      onClick={() => {
                        const nextId = ++workspaceRequestIdRef.current;
                        runWorkspaceBpmAnalysis(nextId);
                      }}
                    >
                      Run Browser BPM Analysis
                    </button>
                  </div>
                )}
              </div>
            </details>

            <div className="generate-form-card">
              <div className="analysis-checkbox-container">
                <input
                  type="checkbox"
                  id="writeReportFile"
                  checked={writeReportFile}
                  onChange={(e) => setWriteReportFile(e.target.checked)}
                  disabled={analysisLoading}
                  className="analysis-checkbox"
                />
                <label htmlFor="writeReportFile" className="analysis-checkbox-label">
                  Write derived report JSON (.ai-step-gen-analysis/song-analysis-report.v1.json)
                </label>
              </div>

              {analysisError && (
                <div className="analysis-error-box">
                  <AlertCircle size={16} className="icon-mr text-danger-icon" />
                  <span>{analysisError}</span>
                </div>
              )}

              <div className="action-row-with-sub">
                <button
                  className="btn-primary-pill"
                  onClick={handleRunOfflineAnalysis}
                  disabled={analysisLoading || !currentSong.audio_path}
                >
                  {analysisLoading ? "Running Analysis..." : "Run Offline Analysis"}
                </button>
                <span className="caption-text-gravel">
                  * Runs offline DSP analysis. Does not call Gemini or write/modify .ssc files.
                </span>
              </div>
            </div>

            {analysisReport && (
              <div className="preview-results-wrapper">
                <div className="preview-header-meta">
                  <h4 className="preview-result-title">Analysis Summary</h4>
                  <div>
                    {analysisReport.audio_summary.analysis_mode === "dsp" ? (
                      <span className="badge-mode-single analysis-mode-badge">DSP Mode Active</span>
                    ) : (
                      <span className="badge-mode-double analysis-mode-badge">Fallback Mode (Metadata-Only)</span>
                    )}
                  </div>
                </div>

                {analysisReport.audio_summary.analysis_mode === "fallback" && (
                  <div className="warning-box">
                    <AlertTriangle size={16} className="icon-mr text-warning-icon warning-box-icon" />
                    <span>
                      <strong>Fallback Mode Active:</strong> Audio DSP features could not be calculated. The analysis is generated from the .ssc metadata only. Onset strength, energy metrics, and choreographic plans are estimated or zeroed.
                    </span>
                  </div>
                )}

                {reportPath && (
                  <div className="analysis-success-box">
                    <CheckCircle size={16} className="icon-mr text-success-icon" />
                    <div className="success-box-content">
                      <span>Report saved successfully!</span>
                      <div className="analysis-report-path">
                        {reportPath}
                      </div>
                    </div>
                  </div>
                )}

                <div className="analysis-stats-grid">
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Duration</span>
                    <span className="analysis-stat-value">{analysisReport.duration_seconds}s</span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Analysis Mode</span>
                    <span className={`analysis-stat-value ${analysisReport.audio_summary.analysis_mode === "fallback" ? "analysis-review-required" : ""}`}>
                      {analysisReport.audio_summary.analysis_mode === "dsp" ? "DSP" : "Fallback"}
                    </span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">SSC BPM</span>
                    <span className="analysis-stat-value">{analysisReport.diagnostics.ssc_initial_bpm}</span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Detected BPM</span>
                    <span className="analysis-stat-value">{analysisReport.diagnostics.audio_bpm_detected}</span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Confidence</span>
                    <span className="analysis-stat-value">{(analysisReport.diagnostics.timing_confidence * 100).toFixed(0)}%</span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Sections</span>
                    <span className="analysis-stat-value">{analysisReport.sections.length}</span>
                  </div>
                  <div className="analysis-stat-card">
                    <span className="analysis-stat-label">Manual Review</span>
                    <span className={`analysis-stat-value ${analysisReport.diagnostics.requires_manual_timing_review ? 'analysis-review-required' : ''}`}>
                      {analysisReport.diagnostics.requires_manual_timing_review ? "Yes" : "No"}
                    </span>
                  </div>
                </div>

                {analysisReport.diagnostics.warnings && analysisReport.diagnostics.warnings.length > 0 && (
                  <div className="analysis-warnings-box">
                    <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                    <div className="analysis-warnings-text">
                      <strong>Timing Warnings:</strong>
                      <ul>
                        {analysisReport.diagnostics.warnings.map((warn: string, idx: number) => (
                          <li key={idx}>{warn}</li>
                        ))}
                      </ul>
                    </div>
                  </div>
                )}

                {/* Sections Table */}
                <div className="workspace-card">
                  <h4 className="analysis-card-title">
                    <Layers size={18} /> Sections segmentation ({analysisReport.sections.length})
                  </h4>
                  <div className="analysis-table-container">
                    <table className="analysis-table">
                      <thead>
                        <tr>
                          <th>Section</th>
                          <th>Beats</th>
                          <th>Measures</th>
                          <th>Music Role</th>
                          <th>PIU Role</th>
                          <th>Energy Profile</th>
                        </tr>
                      </thead>
                      <tbody>
                        {analysisReport.sections.map((section: any, idx: number) => (
                          <tr key={idx}>
                            <td><strong>{section.section_id}</strong></td>
                            <td>{section.start_beat.toFixed(1)} – {section.end_beat.toFixed(1)}</td>
                            <td>M.{section.start_measure} – M.{section.end_measure}</td>
                            <td>
                              <span className="analysis-badge-music-role">
                                {section.music_role}
                              </span>
                            </td>
                            <td>
                              <span className="badge-mode-double analysis-badge-piu-role">
                                {section.piu_role.replace('_', ' ')}
                              </span>
                            </td>
                            <td>
                              <code className="analysis-energy-profile-code">{section.energy_profile}</code>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>

                {/* Choreographic Intent Map / Opportunities */}
                <div className="workspace-card">
                  <h4 className="analysis-card-title">
                    <Sparkles size={18} /> Choreographic intent & opportunities
                  </h4>
                  <div className="validation-issues-list">
                    {analysisReport.choreographic_intent.map((intent: any, idx: number) => (
                      <div key={idx} className="analysis-intent-item">
                        <div className="analysis-intent-header">
                          <span className="analysis-intent-title">
                            {intent.section_id} (M.{intent.measure_start} – M.{intent.measure_end})
                          </span>
                          <div className="analysis-intent-badges">
                            <span className="level-badge">
                              {intent.density_target}
                            </span>
                            <span className="level-badge analysis-budget-badge">
                              {intent.difficulty_budget}
                            </span>
                          </div>
                        </div>
                        
                        <div className="analysis-intent-evidence">
                          <strong>Evidence:</strong> {intent.evidence.join(", ")}
                        </div>

                        <div className="analysis-intent-patterns">
                          <span className="analysis-patterns-label">Recommend:</span>
                          {intent.recommended_pattern_families.map((fam: string, fIdx: number) => (
                            <span key={fIdx} className="analysis-badge-recommend">
                              {fam.replace('_', ' ')}
                            </span>
                          ))}
                        </div>

                        {intent.avoid_pattern_families && intent.avoid_pattern_families.length > 0 && (
                          <div className="analysis-intent-patterns">
                            <span className="analysis-patterns-label">Avoid:</span>
                            {intent.avoid_pattern_families.map((fam: string, fIdx: number) => (
                              <span key={fIdx} className="analysis-badge-avoid">
                                {fam.replace('_', ' ')}
                              </span>
                            ))}
                          </div>
                        )}

                        {intent.accent_plan && intent.accent_plan.length > 0 && (
                          <div className="analysis-accent-plan-text">
                            <strong>Accents Plan:</strong> {intent.accent_plan.length} beats marked for accents (e.g. beats {intent.accent_plan.slice(0, 5).map((a: any) => a.beat.toFixed(1)).join(", ")}
                            {intent.accent_plan.length > 5 ? "..." : ""})
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>

                {/* Raw JSON Accordion */}
                <details className="inspector-accordion">
                  <summary className="analysis-accordion-summary">
                    <ChevronRight size={14} className="icon-mr accordion-icon" />
                    <FileJson size={16} className="icon-mr" />
                    Inspect Raw Analysis Report JSON
                  </summary>
                  <pre className="analysis-accordion-pre">
                    <code>{JSON.stringify(analysisReport, null, 2)}</code>
                  </pre>
                </details>
              </div>
            )}
          </div>
        )}
      </main>

      {confirmModal && (
        <div className="custom-modal-overlay">
          <div className="custom-modal-card">
            <h4 className="custom-modal-title">Confirm Action</h4>
            <p className="custom-modal-message">{confirmModal.message}</p>
            <div className="custom-modal-actions">
              <button
                className="btn-ghost-pill btn-sm-contained"
                onClick={() => setConfirmModal(null)}
              >
                Cancel
              </button>
              <button
                className="btn-primary-pill btn-sm-contained btn-danger-text"
                onClick={confirmModal.onConfirm}
              >
                Confirm
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
