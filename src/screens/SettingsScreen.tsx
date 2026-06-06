import { useState, useEffect } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { open } from "@tauri-apps/plugin-dialog";
import { Key, FolderOpen, Save, Trash2, CheckCircle, AlertTriangle } from "lucide-react";

interface SettingsScreenProps {
  onNavigate: (screen: string) => void;
}

export const SettingsScreen: React.FC<SettingsScreenProps> = () => {
  const {
    settings,
    hasApiKey,
    appMode,
    loadSettings,
    saveSettings,
    saveApiKey,
    deleteApiKey,
    ensureSongpack,
  } = useSettingsStore();

  const [activeSection, setActiveSection] = useState<"api" | "game" | "defaults" | "privacy" | "dev">("api");

  // Form states
  const [songsDir, setSongsDir] = useState("");
  const [songpackMode, setSongpackMode] = useState("managed_default");
  const [defaultSongpackFolder, setDefaultSongpackFolder] = useState("");
  const [defaultAuthor, setDefaultAuthor] = useState("");
  const [defaultPlayMode, setDefaultPlayMode] = useState("Single");
  const [defaultMeter, setDefaultMeter] = useState(10);

  // API key input states
  const [apiKey, setApiKey] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [confirmPassphrase, setConfirmPassphrase] = useState("");

  // UI state feedback
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [songpackStatus, setSongpackStatus] = useState<string | null>(null);
  const [songpackError, setSongpackError] = useState<string | null>(null);

  // Custom Alert / Toast & Confirm modal state
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);
  const [confirmModal, setConfirmModal] = useState<{ message: string; onConfirm: () => void } | null>(null);

  const showToast = (message: string, type: "success" | "error") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 4000);
  };

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  useEffect(() => {
    if (settings) {
      setSongsDir(settings.songs_dir || "");
      setSongpackMode(settings.songpack_mode || "managed_default");
      setDefaultSongpackFolder(settings.default_songpack_folder || "99-AI-Step-Gen");
      setDefaultAuthor(settings.default_author || "AI Step Gen");
      setDefaultPlayMode(settings.default_play_mode || "Single");
      setDefaultMeter(settings.default_meter || 10);
    }
  }, [settings]);

  // Dev settings guard: if prod mode, redirect active section from dev to api
  useEffect(() => {
    if (appMode !== "dev" && activeSection === "dev") {
      setActiveSection("api");
    }
  }, [appMode, activeSection]);

  const handleSaveSettings = async () => {
    try {
      await saveSettings({
        songs_dir: songsDir.trim() || null,
        songpack_mode: songpackMode,
        default_songpack_folder: defaultSongpackFolder.trim() || "99-AI-Step-Gen",
        default_author: defaultAuthor.trim() || null,
        default_play_mode: defaultPlayMode || null,
        default_meter: defaultMeter || null,
      });
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err) {
      console.error(err);
      showToast("Error saving settings.", "error");
    }
  };

  const handleSaveApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiKey.trim() || !passphrase.trim()) {
      showToast("Please enter both API key and passphrase.", "error");
      return;
    }
    if (passphrase !== confirmPassphrase) {
      showToast("Passphrases do not match.", "error");
      return;
    }
    try {
      await saveApiKey(apiKey.trim(), passphrase.trim());
      setApiKey("");
      setPassphrase("");
      setConfirmPassphrase("");
      showToast("API Key successfully encrypted and stored.", "success");
    } catch (err: any) {
      showToast("Error saving API Key: " + err.toString(), "error");
    }
  };

  const handleDeleteApiKey = () => {
    setConfirmModal({
      message: "Are you sure you want to permanently delete the encrypted API Key?",
      onConfirm: async () => {
        setConfirmModal(null);
        try {
          await deleteApiKey();
          showToast("API Key deleted successfully.", "success");
        } catch (err: any) {
          showToast("Error deleting API Key: " + err.toString(), "error");
        }
      }
    });
  };

  const handleRepairSongpack = async () => {
    setSongpackStatus(null);
    setSongpackError(null);
    try {
      // Guardar primero la configuración actual en pantalla antes de reparar
      await saveSettings({
        songs_dir: songsDir.trim() || null,
        songpack_mode: songpackMode,
        default_songpack_folder: defaultSongpackFolder.trim() || "99-AI-Step-Gen",
        default_author: defaultAuthor.trim() || null,
        default_play_mode: defaultPlayMode || null,
        default_meter: defaultMeter || null,
      });
      const path = await ensureSongpack();
      setSongpackStatus(`Songpack successfully initialized/repaired at: ${path}`);
    } catch (err: any) {
      setSongpackError(err.toString());
    }
  };

  const handleBrowseSongsDir = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Games Songs Directory",
      });
      if (typeof selected === "string") {
        setSongsDir(selected);
      }
    } catch (err) {
      console.error("Failed to select Songs directory:", err);
    }
  };

  return (
    <div className="settings-container">
      {/* Settings Navigation Sidebar */}
      <aside className="settings-sidebar">
        <h2 className="settings-title-waldenburg">Settings</h2>
        <nav className="settings-nav">
          <button
            className={`settings-nav-btn ${activeSection === "api" ? "active" : ""}`}
            onClick={() => setActiveSection("api")}
          >
            API & Models
          </button>
          <button
            className={`settings-nav-btn ${activeSection === "game" ? "active" : ""}`}
            onClick={() => setActiveSection("game")}
          >
            Game & Songpacks
          </button>
          <button
            className={`settings-nav-btn ${activeSection === "defaults" ? "active" : ""}`}
            onClick={() => setActiveSection("defaults")}
          >
            Generation Defaults
          </button>
          <button
            className={`settings-nav-btn ${activeSection === "privacy" ? "active" : ""}`}
            onClick={() => setActiveSection("privacy")}
          >
            Privacy & Storage
          </button>
          {appMode === "dev" && (
            <button
              className={`settings-nav-btn ${activeSection === "dev" ? "active" : ""}`}
              onClick={() => setActiveSection("dev")}
            >
              Developer Settings
            </button>
          )}
        </nav>
      </aside>

      {/* Settings Content Area */}
      <main className="settings-main-content">
        {saveSuccess && (
          <div className="success-box">
            <CheckCircle size={16} className="icon-mr text-success-icon" />
            <span>Settings saved successfully.</span>
          </div>
        )}

        {/* Section: API & Models */}
        {activeSection === "api" && (
          <div className="settings-section">
            <h3 className="section-title-waldenburg">API & Models</h3>
            <p className="section-subtitle-gravel">
              Configure your Gemini API access. We support Bring-Your-Own-Key (BYOK). The API key is stored encrypted locally using AES-256-GCM.
            </p>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Gemini API Key Configuration</h4>
              {hasApiKey ? (
                <div className="api-key-configured-status">
                  <div className="badge-success-contained">
                    <CheckCircle size={14} className="icon-mr" />
                    API Key Configured (Encrypted)
                  </div>
                  <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={handleDeleteApiKey}>
                    <Trash2 size={14} className="icon-mr" />
                    Delete Key
                  </button>
                </div>
              ) : (
                <form onSubmit={handleSaveApiKey} className="api-key-form">
                  <div className="form-group-contained">
                    <label className="form-label-dark">Gemini API Key</label>
                    <input
                      type="password"
                      className="input-contained"
                      placeholder="AIzaSy..."
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                      required
                    />
                  </div>
                  <div className="form-group-contained">
                    <label className="form-label-dark">Passphrase (For local encryption)</label>
                    <input
                      type="password"
                      className="input-contained"
                      placeholder="Choose a strong passphrase"
                      value={passphrase}
                      onChange={(e) => setPassphrase(e.target.value)}
                      required
                    />
                  </div>
                  <div className="form-group-contained">
                    <label className="form-label-dark">Confirm Passphrase</label>
                    <input
                      type="password"
                      className="input-contained"
                      placeholder="Repeat your passphrase"
                      value={confirmPassphrase}
                      onChange={(e) => setConfirmPassphrase(e.target.value)}
                      required
                    />
                  </div>
                  <button type="submit" className="btn-primary-pill btn-self-start">
                    <Key size={14} className="icon-mr" />
                    Encrypt & Save API Key
                  </button>
                </form>
              )}
            </div>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Active Gemini Model</h4>
              <div className="model-selector-container">
                <input
                  type="text"
                  className="input-contained input-disabled"
                  value="gemini-3.5-flash"
                  disabled
                  readOnly
                />
                <span className="caption-text-gravel">
                  Current model is locked to <code>gemini-3.5-flash</code> for safety and cost-performance reasons.
                </span>
                {/* 
                  TODO: Add support for allowlisted selector for gemini-3.5-pro.
                  WARNING: Pro version should display a high-cost/paid-tier warning to the user before selection.
                */}
              </div>
            </div>
          </div>
        )}

        {/* Section: Game & Songpacks */}
        {activeSection === "game" && (
          <div className="settings-section">
            <h3 className="section-title-waldenburg">Game & Songpacks</h3>
            <p className="section-subtitle-gravel">
              Link the application to your StepF2/StepP1 Songs path to enable automatic export and songpack management.
            </p>

            <div className="settings-card">
              <div className="form-group-contained">
                <label className="form-label-dark">Game Songs Directory Path</label>
                <div className="input-group">
                  <input
                    type="text"
                    className="input-contained"
                    placeholder="e.g., C:\StepF2\Songs"
                    value={songsDir}
                    onChange={(e) => setSongsDir(e.target.value)}
                  />
                  <button
                    type="button"
                    className="btn-ghost-pill btn-sm-contained"
                    onClick={handleBrowseSongsDir}
                  >
                    Browse...
                  </button>
                </div>
                <span className="caption-text-gravel">
                  Absolute path to your simulator's <code>Songs</code> folder.
                </span>
              </div>

              <div className="form-group-contained">
                <label className="form-label-dark">Songpack Folder Management Mode</label>
                <select
                  className="input-contained"
                  value={songpackMode}
                  onChange={(e) => setSongpackMode(e.target.value)}
                >
                  <option value="managed_default">Managed Default (Auto-copies default Banner and Sound if missing)</option>
                  <option value="custom_existing">Custom Existing (Uses existing custom folder, does not write default assets)</option>
                </select>
                <span className="caption-text-gravel">
                  Choose whether the app automatically initializes/copies template assets or points to a custom directory.
                </span>
              </div>

              <div className="form-group-contained">
                <label className="form-label-dark">Default Songpack Name</label>
                <input
                  type="text"
                  className="input-contained"
                  placeholder="99-AI-Step-Gen"
                  value={defaultSongpackFolder}
                  onChange={(e) => setDefaultSongpackFolder(e.target.value)}
                />
                <span className="caption-text-gravel">
                  Folder name where generated charts will be structured.
                </span>
              </div>

              <button className="btn-primary-pill" onClick={handleSaveSettings}>
                <Save size={14} className="icon-mr" />
                Save Config
              </button>
            </div>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Songpack Actions</h4>
              <p className="section-subtitle-gravel">
                Initialize or repair the default stepchart songpack contents (creates structure, default <code>Banner.png</code>, and preview <code>info/Sound.ogg</code>).
              </p>

              {songpackStatus && (
                <div className="success-box">
                  <CheckCircle size={14} className="icon-mr text-success-icon" />
                  <span>{songpackStatus}</span>
                </div>
              )}

              {songpackError && (
                <div className="error-box">
                  <AlertTriangle size={14} className="icon-mr text-danger-icon" />
                  <span>Error: {songpackError}</span>
                </div>
              )}

              <button className="btn-ghost-pill btn-self-start" onClick={handleRepairSongpack} disabled={!songsDir}>
                <FolderOpen size={14} className="icon-mr" />
                Create / Repair Default Songpack
              </button>
            </div>
          </div>
        )}

        {/* Section: Generation Defaults */}
        {activeSection === "defaults" && (
          <div className="settings-section">
            <h3 className="section-title-waldenburg">Generation Defaults</h3>
            <p className="section-subtitle-gravel">
              Pre-configure the default author, play mode, and meter difficulty settings for new step charts.
            </p>

            <div className="settings-card">
              <div className="form-group-contained">
                <label className="form-label-dark">Default Stepchart Creator (Credit)</label>
                <input
                  type="text"
                  className="input-contained"
                  value={defaultAuthor}
                  onChange={(e) => setDefaultAuthor(e.target.value)}
                />
              </div>

              <div className="form-group-contained">
                <label className="form-label-dark">Default Play Mode</label>
                <select
                  className="input-contained"
                  value={defaultPlayMode}
                  onChange={(e) => setDefaultPlayMode(e.target.value)}
                >
                  <option value="Single">Single (5-Key)</option>
                  <option value="Double">Double (10-Key)</option>
                </select>
              </div>

              <div className="form-group-contained">
                <label className="form-label-dark">Default Target Meter (Level)</label>
                <input
                  type="number"
                  min="1"
                  max="28"
                  className="input-contained"
                  value={defaultMeter}
                  onChange={(e) => setDefaultMeter(parseInt(e.target.value) || 10)}
                />
              </div>

              <button className="btn-primary-pill" onClick={handleSaveSettings}>
                <Save size={14} className="icon-mr" />
                Save Defaults
              </button>
            </div>
          </div>
        )}

        {/* Section: Privacy & Storage */}
        {activeSection === "privacy" && (
          <div className="settings-section">
            <h3 className="section-title-waldenburg">Privacy & Storage</h3>
            <p className="section-subtitle-gravel">
              Review app local storage directories and privacy guarantees.
            </p>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Local Storage Data</h4>
              <p className="section-subtitle-gravel">
                Credentials and configuration files are stored securely in your user application data folder:
              </p>
              <code className="monospace-block text-wrap">
                %LOCALAPPDATA%\ai-step-gen-suite
              </code>
            </div>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Privacy Guarantees</h4>
              <ul className="settings-list">
                <li>Your Gemini API Key is encrypted locally and never transmitted to external servers other than Gemini's official endpoints.</li>
                <li>Your decrypted API Key is kept only in memory during requests and wiped immediately after.</li>
                <li>No telemetry is collected.</li>
              </ul>
            </div>
          </div>
        )}

        {/* Section: Developer Settings */}
        {activeSection === "dev" && (
          <div className="settings-section">
            <h3 className="section-title-waldenburg">Developer Settings</h3>
            <p className="section-subtitle-gravel">
              Diagnose application mode, environment variables, and inspect development flags.
            </p>

            <div className="settings-card">
              <div className="dev-meta-row">
                <span className="dev-meta-label">App Environment (Frontend):</span>
                <span className={`badge-dev-mode ${appMode === "dev" ? "active" : ""}`}>
                  {appMode === "dev" ? "VITE_APP_ENV=dev (Active)" : "Production"}
                </span>
              </div>

              <div className="dev-meta-row">
                <span className="dev-meta-label">App Environment (Backend):</span>
                <span className="dev-meta-value">
                  <code>AI_STEP_GEN_ENV</code>: {appMode === "dev" ? "dev" : "unset/prod"}
                </span>
              </div>
            </div>

            <div className="settings-card">
              <h4 className="card-subtitle-obsidian">Developer Access</h4>
              <p className="section-subtitle-gravel">
                To access mock generators, inspectors, and testing modules, launch the application in development mode with the environment variables set.
              </p>
            </div>
          </div>
        )}
      </main>

      {/* Custom overlays instead of native browser dialogs */}
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

      {toast && (
        <div className={`custom-toast ${toast.type}`}>
          {toast.message}
        </div>
      )}
    </div>
  );
};
