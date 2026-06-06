import { useState } from "react";
import { FolderOpen, AlertCircle, ArrowRight, Music } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useSongProjectStore } from "../store/songProjectStore";
import { useSettingsStore } from "../store/settingsStore";
import { ISongDetails } from "../types/song";

interface ImportEditSongProps {
  onNavigate: (screen: string) => void;
}

export const ImportEditSong: React.FC<ImportEditSongProps> = ({ onNavigate }) => {
  const { currentSong, setCurrentSong, setLoading, isLoading, error, setError } = useSongProjectStore();
  const { appMode } = useSettingsStore();
  const [folderPath, setFolderPath] = useState("");

  const handleImport = async () => {
    if (!folderPath.trim()) {
      setError("Please specify a valid folder path.");
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const details = await invoke<ISongDetails>("import_song_folder", {
        folderPath: folderPath.trim(),
      });
      setCurrentSong(details);
      onNavigate("WORKSPACE");
    } catch (err: any) {
      console.error(err);
      setError(err.toString());
    } finally {
      setLoading(false);
    }
  };

  const handleBrowseFolder = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Song Folder",
      });
      if (typeof selected === "string") {
        setFolderPath(selected);
      }
    } catch (err) {
      console.error("Failed to select folder:", err);
    }
  };

  return (
    <div className="import-screen-container">
      <div className="import-card-wrapper">
        <h2 className="section-title-waldenburg">Import Existing Song</h2>
        <p className="section-subtitle-gravel">
          Import a StepF2/StepP1 song folder containing an <code>.ssc</code> chart and its audio file.
        </p>

        {error && (
          <div className="error-box">
            <AlertCircle size={16} className="icon-mr text-danger-icon" />
            <span>{error}</span>
          </div>
        )}

        <div className="form-group-contained">
          <label className="form-label-dark">Song Folder Absolute Path</label>
          <div className="input-group">
            <input
              type="text"
              className="input-contained"
              placeholder="e.g., C:\StepF2\Songs\99-AI-Step-Gen\Poseidon"
              value={folderPath}
              onChange={(e) => setFolderPath(e.target.value)}
              disabled={isLoading}
            />
            <button
              type="button"
              className="btn-ghost-pill btn-sm-contained"
              onClick={handleBrowseFolder}
              disabled={isLoading}
            >
              Browse...
            </button>
          </div>
        </div>

        <div className="import-action-buttons">
          <button
            className="btn-primary-pill"
            onClick={handleImport}
            disabled={isLoading}
          >
            <FolderOpen className="icon-mr" size={16} />
            {isLoading ? "Importing..." : "Import Folder"}
          </button>

          {currentSong && (
            <button
              className="btn-ghost-pill"
              onClick={() => onNavigate("WORKSPACE")}
            >
              <ArrowRight className="icon-mr" size={16} />
              Open Active Workspace
            </button>
          )}
        </div>

        {appMode === "dev" && (
          <div className="demo-hint-box">
            <span className="caption-text-gravel">Need an example path to try? Use:</span>
            <code className="monospace-inline">
              docs/example_charts/example_songs_folders/1626 - Poseidon
            </code>
          </div>
        )}
      </div>

      {currentSong && (
        <div className="active-song-summary-card">
          <div className="active-song-header">
            <Music className="text-info-icon" size={24} />
            <div>
              <span className="active-song-title-sm">{currentSong.song_name}</span>
              <span className="active-song-artist-sm">by {currentSong.artist}</span>
            </div>
          </div>
          <button
            className="btn-ghost-pill btn-sm-contained"
            onClick={() => onNavigate("WORKSPACE")}
          >
            Go to Workspace
          </button>
        </div>
      )}
    </div>
  );
};
