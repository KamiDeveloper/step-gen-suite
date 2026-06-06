import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSongProjectStore } from "../store/songProjectStore";
import { useSettingsStore } from "../store/settingsStore";
import {
  ShieldAlert,
  CheckSquare,
  Activity,
  Database,
  CheckCircle,
  AlertCircle
} from "lucide-react";
import { IAppendChartResult } from "../types/song";

interface DevToolsScreenProps {
  onNavigate: (screen: string) => void;
}

export const DevToolsScreen: React.FC<DevToolsScreenProps> = ({ onNavigate }) => {
  const { currentSong, setCurrentSong, isLoading, setLoading, error, setError } = useSongProjectStore();
  const { appMode } = useSettingsStore();

  const [activeLab, setActiveLab] = useState<"dashboard" | "ssc" | "gemini" | "validation" | "music" | "dataset">("dashboard");

  // Local stub options
  const [playMode, setPlayMode] = useState<"Single" | "Double">("Single");
  const [targetLevel, setTargetLevel] = useState(10);
  const [authorName, setAuthorName] = useState("Dev Stub");

  // Output messages
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [backupPath, setBackupPath] = useState<string | null>(null);
  const [confirmModal, setConfirmModal] = useState<{ message: string; onConfirm: () => void } | null>(null);

  const handleAppendLocalStub = () => {
    if (!currentSong) {
      setError("No song is loaded in the active workspace.");
      return;
    }

    setConfirmModal({
      message: "This will append a basic fixed local chart stub to the active .ssc. Proceed?",
      onConfirm: async () => {
        setConfirmModal(null);
        await executeAppendLocalStub();
      }
    });
  };

  const executeAppendLocalStub = async () => {
    if (!currentSong) return;
    setLoading(true);
    setError(null);
    setStatusMessage(null);
    setBackupPath(null);

    try {
      const result = await invoke<IAppendChartResult>("append_ai_chart_stub", {
        sscPath: currentSong.ssc_path,
        playMode,
        targetLevel,
        author: authorName.trim() || "Dev Test",
      });

      if (result.written) {
        setStatusMessage(`Successfully appended local stub chart! (${result.message})`);
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
      setError("Error appending local stub: " + err.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleTestMockContract = () => {
    if (!currentSong) {
      setError("No song is loaded in the active workspace.");
      return;
    }

    setConfirmModal({
      message: "This will append a mock structured Gemini payload to the .ssc to test parser boundaries. Proceed?",
      onConfirm: async () => {
        setConfirmModal(null);
        await executeTestMockContract();
      }
    });
  };

  const executeTestMockContract = async () => {
    if (!currentSong) return;
    setLoading(true);
    setError(null);
    setStatusMessage(null);
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
        author: authorName.trim() || "Mock Contract",
      });

      if (result.written) {
        setStatusMessage(`Successfully appended Mock Gemini Contract chart! (${result.message})`);
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
      setError("Error appending mock contract: " + err.toString());
    } finally {
      setLoading(false);
    }
  };

  if (appMode !== "dev") {
    return (
      <div className="workspace-empty-state">
        <ShieldAlert size={48} className="text-ember-icon" />
        <h2 className="empty-title-waldenburg">Access Denied</h2>
        <p className="empty-desc-gravel">
          Developer tools are only available in development mode.
        </p>
        <button className="btn-primary-pill" onClick={() => onNavigate("START_MENU")}>
          Return to Main Menu
        </button>
      </div>
    );
  }

  return (
    <div className="dev-tools-container">
      {/* Dev Navigation Sidebar */}
      <aside className="dev-sidebar">
        <div className="dev-sidebar-heading">
          <ShieldAlert className="text-ember-icon" size={24} />
          <div>
            <h2 className="dev-title-waldenburg">Developer Lab</h2>
            <span className="dev-mode-badge">Dev Mode Active</span>
          </div>
        </div>

        <nav className="dev-nav">
          <button
            className={`dev-nav-btn ${activeLab === "dashboard" ? "active" : ""}`}
            onClick={() => setActiveLab("dashboard")}
          >
            Developer Dashboard
          </button>
          <button
            className={`dev-nav-btn ${activeLab === "ssc" ? "active" : ""}`}
            onClick={() => setActiveLab("ssc")}
          >
            SSC Inspector
          </button>
          <button
            className={`dev-nav-btn ${activeLab === "gemini" ? "active" : ""}`}
            onClick={() => setActiveLab("gemini")}
          >
            Gemini Inspector (Mocks)
          </button>
          <button
            className={`dev-nav-btn ${activeLab === "validation" ? "active" : ""}`}
            onClick={() => setActiveLab("validation")}
          >
            Validation Lab
          </button>
          <button
            className={`dev-nav-btn ${activeLab === "music" ? "active" : ""}`}
            onClick={() => setActiveLab("music")}
          >
            Music Analysis Lab
          </button>
          <button
            className={`dev-nav-btn ${activeLab === "dataset" ? "active" : ""}`}
            onClick={() => setActiveLab("dataset")}
          >
            Dataset Lab
          </button>
        </nav>

        <div className="dev-sidebar-footer">
          <button className="btn-ghost-pill btn-sm-contained" onClick={() => onNavigate("START_MENU")}>
            Return to Main Menu
          </button>
        </div>
      </aside>

      {/* Dev Content Area */}
      <main className="dev-main-content">
        {error && (
          <div className="error-box">
            <AlertCircle size={16} className="icon-mr text-danger-icon" />
            <span>{error}</span>
          </div>
        )}

        {statusMessage && (
          <div className="success-box">
            <CheckCircle size={16} className="icon-mr text-success-icon" />
            <div className="success-box-content">
              <span>{statusMessage}</span>
              {backupPath && (
                <div className="success-backup-text">
                  Backup: {backupPath}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Dashboard */}
        {activeLab === "dashboard" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">Developer Dashboard</h3>
            <p className="section-subtitle-gravel">Overview of debugging context and active shell flags.</p>

            <div className="dev-grid">
              <div className="dev-card">
                <span className="dev-card-label">Active Song Project</span>
                <span className="dev-card-value">{currentSong ? currentSong.song_name : "None loaded"}</span>
              </div>
              <div className="dev-card">
                <span className="dev-card-label">Rust Env Flag</span>
                <span className="dev-card-value">AI_STEP_GEN_ENV=dev</span>
              </div>
            </div>
          </div>
        )}

        {/* SSC Inspector */}
        {activeLab === "ssc" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">SSC Structure Inspector</h3>
            <p className="section-subtitle-gravel">View and analyze the raw metadata structure of the loaded song.</p>

            {currentSong ? (
              <div className="dev-card">
                <div className="form-group-contained">
                  <label className="form-label-dark">Loaded .ssc Absolute Path</label>
                  <code className="monospace-block">{currentSong.ssc_path}</code>
                </div>
                <div className="form-group-contained">
                  <label className="form-label-dark">Technical Header Summary</label>
                  <pre className="monospace-block code-block-scrollable">
                    {JSON.stringify(
                      {
                        title: currentSong.song_name,
                        artist: currentSong.artist,
                        bpm: currentSong.bpm,
                        offset: currentSong.offset,
                        charts_count: currentSong.charts.length,
                      },
                      null,
                      2
                    )}
                  </pre>
                </div>
              </div>
            ) : (
              <div className="dev-empty-box">
                <span>Please load a song from the Workspace to use the SSC Inspector.</span>
              </div>
            )}
          </div>
        )}

        {/* Gemini Inspector (Mocks) */}
        {activeLab === "gemini" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">Gemini Mock Inspector</h3>
            <p className="section-subtitle-gravel">
              Execute offline mock generation pipelines that write fixed stubs to disk. Avoids API cost during testing.
            </p>

            {currentSong ? (
              <div className="generate-form-card">
                <h4 className="card-subtitle-obsidian">Smoke Test Options</h4>
                <div className="generate-grid">
                  <div className="form-group-contained">
                    <label className="form-label-dark">Play Mode</label>
                    <select
                      className="input-contained"
                      value={playMode}
                      onChange={(e) => setPlayMode(e.target.value as "Single" | "Double")}
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
                      className="input-contained"
                      value={targetLevel}
                      onChange={(e) => setTargetLevel(parseInt(e.target.value) || 10)}
                    />
                  </div>

                  <div className="form-group-contained">
                    <label className="form-label-dark">Creator Name</label>
                    <input
                      type="text"
                      className="input-contained"
                      value={authorName}
                      onChange={(e) => setAuthorName(e.target.value)}
                    />
                  </div>
                </div>

                <div className="dev-buttons-row">
                  <button className="btn-primary-pill" onClick={handleAppendLocalStub} disabled={isLoading}>
                    Run Local Stub Write
                  </button>
                  <button className="btn-ghost-pill" onClick={handleTestMockContract} disabled={isLoading}>
                    Run Mock Contract Write
                  </button>
                </div>
                <p className="caption-text-gravel">
                  * Warning: Both buttons <strong>will write to disk</strong> (modifies active .ssc). They do not use internet or consume tokens.
                </p>
              </div>
            ) : (
              <div className="dev-empty-box">
                <span>Please load a song from the Workspace to test mock generators.</span>
              </div>
            )}
          </div>
        )}

        {/* Validation Lab */}
        {activeLab === "validation" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">Biomechanical Validation Lab</h3>
            <p className="section-subtitle-gravel">
              Dry-run structural parsing of custom charts to debug step limits.
            </p>

            <div className="placeholder-state-card">
              <CheckSquare className="text-slate-icon" size={32} />
              <span className="placeholder-text-main">Validation sandbox</span>
              <span className="placeholder-text-sub">
                Interactive validation sandbox is planned for future dev features. Active biomechanical validations run dynamically inside the main Workspace.
              </span>
            </div>
          </div>
        )}

        {/* Music Lab */}
        {activeLab === "music" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">Music Analysis Lab</h3>
            <p className="section-subtitle-gravel">Placeholder for visual spectrograph and beat-alignment tools.</p>

            <div className="placeholder-state-card">
              <Activity className="text-slate-icon" size={32} />
              <span className="placeholder-text-main">Music Spectrograph Sandbox (Planned next engineering phase)</span>
              <span className="placeholder-text-sub">
                This lab will allow developers to test beat detection algorithms offline. Scheduled for next engineering phase.
              </span>
            </div>
          </div>
        )}

        {/* Dataset Lab */}
        {activeLab === "dataset" && (
          <div className="dev-section">
            <h3 className="section-title-waldenburg">Dataset Lab</h3>
            <p className="section-subtitle-gravel">Placeholder for indexing official collections and compiling pattern databases.</p>

            <div className="placeholder-state-card">
              <Database className="text-slate-icon" size={32} />
              <span className="placeholder-text-main">Dataset Compiler Sandbox (Planned next engineering phase)</span>
              <span className="placeholder-text-sub">
                This lab will index stepmakers, packs, and patterns to build offline databases. Scheduled for next engineering phase.
              </span>
            </div>
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
