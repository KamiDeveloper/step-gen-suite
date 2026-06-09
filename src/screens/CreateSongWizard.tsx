import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSongProjectStore } from "../store/songProjectStore";
import { useSettingsStore } from "../store/settingsStore";
import { ISongDetails } from "../types/song";
import { ArrowLeft, ArrowRight, Save, AlertTriangle, CheckCircle } from "lucide-react";

interface CreateSongWizardProps {
  onNavigate: (screen: string) => void;
}

const WIZARD_STEPS = [
  { id: 1, name: "Project Destination" },
  { id: 2, name: "Song Assets" },
  { id: 3, name: "Metadata" },
  { id: 4, name: "Review & Create" },
];

interface IFileMetadata {
  name: string;
  extension: string;
  size: number;
  path: string;
}

export const CreateSongWizard: React.FC<CreateSongWizardProps> = ({ onNavigate }) => {
  const { setCurrentSong } = useSongProjectStore();
  const { settings, ensureSongpack } = useSettingsStore();

  const [currentStep, setCurrentStep] = useState(1);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Step 1: Destination States
  const [songFolder, setSongFolder] = useState("");
  const [songpackPath, setSongpackPath] = useState<string | null>(null);
  const [isCustomFolder, setIsCustomFolder] = useState(false);
  const [customFolderPath, setCustomFolderPath] = useState("");
  const [destinationPath, setDestinationPath] = useState("");
  const [folderStatus, setFolderStatus] = useState<"NotExist" | "ExistEmpty" | "ExistWithSsc" | "ExistNotEmpty" | "Unchecked">("Unchecked");
  const [sanitizationError, setSanitizationError] = useState<string | null>(null);
  const [explicitConsent, setExplicitConsent] = useState(false);
  const [isFolderReady, setIsFolderReady] = useState(false);

  // Step 2: Asset States
  const [audioFile, setAudioFile] = useState<IFileMetadata | null>(null);
  const [bannerFile, setBannerFile] = useState<IFileMetadata | null>(null);
  const [backgroundFile, setBackgroundFile] = useState<IFileMetadata | null>(null);
  const [videoFile, setVideoFile] = useState<IFileMetadata | null>(null);

  // Step 3: Metadata States
  const [title, setTitle] = useState("");
  const [artist, setArtist] = useState("");
  const [genre, setGenre] = useState("Original");
  const [credit, setCredit] = useState("");
  const [songType, setSongType] = useState("ARCADE");
  const [displayBpm, setDisplayBpm] = useState("120.000");
  const [timingBpm, setTimingBpm] = useState("120.000");
  const [offset, setOffset] = useState("0.000000");

  // Load default songpack path and preset settings
  useEffect(() => {
    const initDestination = async () => {
      try {
        if (settings && settings.songs_dir) {
          const packPath = await ensureSongpack();
          setSongpackPath(packPath);
        }
      } catch (err: any) {
        console.error("Failed to ensure songpack folder:", err);
      }
    };
    initDestination();
  }, [settings]);

  // Set default credit on load
  useEffect(() => {
    if (settings) {
      setCredit(settings.default_author || "AI Step Gen");
    }
  }, [settings]);

  // Update destination path and check folder whenever folder inputs change
  useEffect(() => {
    const updatePath = async () => {
      setErrorMsg(null);
      if (isCustomFolder) {
        setDestinationPath(customFolderPath);
        if (customFolderPath) {
          try {
            const status = await invoke<string>("check_destination_folder", { path: customFolderPath });
            setFolderStatus(status as any);
          } catch (err: any) {
            setFolderStatus("Unchecked");
          }
        } else {
          setFolderStatus("Unchecked");
        }
      } else {
        if (songpackPath && songFolder.trim()) {
          const cleanFolder = songFolder.trim();
          const target = `${songpackPath}/${cleanFolder}`.replace(/\\/g, "/");
          setDestinationPath(target);
          try {
            const status = await invoke<string>("check_destination_folder", { path: target });
            setFolderStatus(status as any);
          } catch (err: any) {
            setFolderStatus("Unchecked");
          }
        } else {
          setDestinationPath("");
          setFolderStatus("Unchecked");
        }
      }
    };
    updatePath();
  }, [songFolder, songpackPath, isCustomFolder, customFolderPath]);

  // Real-time folder name sanitization
  const handleFolderNameChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setSongFolder(val);
    setExplicitConsent(false);
    setIsFolderReady(false);
    if (!val.trim()) {
      setSanitizationError("Folder name cannot be empty.");
      return;
    }
    try {
      await invoke<string>("validate_folder_name", { name: val });
      setSanitizationError(null);
    } catch (err: any) {
      setSanitizationError(err.toString());
    }
  };

  const handleChooseCustomFolder = async () => {
    setErrorMsg(null);
    try {
      const selected = await invoke<string | null>("select_song_destination_folder");
      if (selected) {
        setCustomFolderPath(selected.replace(/\\/g, "/"));
        setIsCustomFolder(true);
        setExplicitConsent(false);
        setIsFolderReady(false);
      }
    } catch (err: any) {
      console.error(err);
      setErrorMsg("Failed to open folder dialog: " + err.toString());
    }
  };

  const handleUseDefaultSongpack = () => {
    setIsCustomFolder(false);
    setExplicitConsent(false);
    setIsFolderReady(false);
  };

  const handleCreateOrConfirmFolder = async () => {
    setErrorMsg(null);
    if (!destinationPath) {
      setErrorMsg("No target directory specified.");
      return;
    }

    if (folderStatus === "ExistWithSsc") {
      setErrorMsg("A .ssc file already exists in the destination folder. Creation is blocked.");
      return;
    }

    if (folderStatus === "ExistNotEmpty" && !explicitConsent) {
      setErrorMsg("Please explicitly confirm usage of this non-empty folder.");
      return;
    }

    try {
      await invoke("create_destination_folder", { path: destinationPath });
      setIsFolderReady(true);
      setSuccessMsg("Destination folder successfully validated and prepared.");
      setTimeout(() => setSuccessMsg(null), 3000);
      setCurrentStep(2);
    } catch (err: any) {
      setErrorMsg("Failed to create destination directory: " + err.toString());
    }
  };

  // Step 2: Selection logic
  const handleSelectAsset = async (kind: "audio" | "banner" | "background" | "video") => {
    setErrorMsg(null);
    try {
      const selectedPath = await invoke<string | null>("select_song_asset_file", { kind });
      if (selectedPath) {
        const normalizedPath = selectedPath.replace(/\\/g, "/");
        const metadata = await invoke<any>("get_file_metadata", { path: normalizedPath });
        const fileObj: IFileMetadata = {
          name: metadata.name,
          extension: metadata.extension,
          size: metadata.size,
          path: normalizedPath,
        };

        if (kind === "audio") {
          setAudioFile(fileObj);
          // Auto-derive Title from audio filename if Title is empty
          if (!title) {
            const dotIdx = metadata.name.lastIndexOf(".");
            const baseName = dotIdx !== -1 ? metadata.name.substring(0, dotIdx) : metadata.name;
            setTitle(baseName.replace(/[_-]/g, " ").trim());
          }
        } else if (kind === "banner") {
          setBannerFile(fileObj);
        } else if (kind === "background") {
          setBackgroundFile(fileObj);
        } else if (kind === "video") {
          setVideoFile(fileObj);
        }
      }
    } catch (err: any) {
      console.error(err);
      setErrorMsg("Failed to select file: " + err.toString());
    }
  };

  const handleClearAsset = (kind: "banner" | "background" | "video") => {
    if (kind === "banner") setBannerFile(null);
    if (kind === "background") setBackgroundFile(null);
    if (kind === "video") setVideoFile(null);
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 Bytes";
    const k = 1024;
    const sizes = ["Bytes", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  // Step 3: Validations & defaults
  const validateMetadata = (): boolean => {
    setErrorMsg(null);
    if (!title.trim()) {
      setErrorMsg("Song Title is required.");
      return false;
    }
    const bpmNum = parseFloat(timingBpm);
    if (isNaN(bpmNum) || bpmNum <= 0) {
      setErrorMsg("Timing BPM must be a positive number.");
      return false;
    }
    if (isNaN(parseFloat(offset))) {
      setErrorMsg("Global Offset must be a valid number.");
      return false;
    }
    return true;
  };

  // Step 4: Submission
  const handleCreateProject = async () => {
    if (!validateMetadata()) return;
    if (!audioFile) {
      setErrorMsg("Audio file is mandatory.");
      return;
    }

    setIsSubmitting(true);
    setErrorMsg(null);

    const payload = {
      target_folder_path: destinationPath,
      title: title.trim(),
      artist: artist.trim() || "Unknown Artist",
      genre: genre.trim() || "Original",
      credit: credit.trim(),
      song_type: songType,
      display_bpm: displayBpm.trim() || "120.000",
      timing_bpm: parseFloat(timingBpm) || 120.0,
      offset: parseFloat(offset) || 0.0,
      audio_path: audioFile.path,
      banner_path: bannerFile?.path || null,
      background_path: backgroundFile?.path || null,
      video_path: videoFile?.path || null,
    };

    try {
      const details = await invoke<ISongDetails>("create_song_project", { payload });
      setCurrentSong(details);
      onNavigate("WORKSPACE");
    } catch (err: any) {
      console.error(err);
      setErrorMsg("Failed to create song project: " + err.toString());
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleNext = () => {
    setErrorMsg(null);
    if (currentStep === 1) {
      if (!isFolderReady) {
        setErrorMsg("Please validate and prepare the destination folder first.");
        return;
      }
    } else if (currentStep === 2) {
      if (!audioFile) {
        setErrorMsg("Audio file selection is mandatory.");
        return;
      }
    } else if (currentStep === 3) {
      if (!validateMetadata()) return;
    }
    setCurrentStep(currentStep + 1);
  };

  const handleBack = () => {
    setErrorMsg(null);
    setCurrentStep(currentStep - 1);
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 1:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">1. Project Destination</h3>
            <p className="wizard-step-desc">
              Specify the output directory for this new StepF2/StepP1 song pack project.
            </p>

            {/* If default songpack is configured */}
            {settings && settings.songs_dir ? (
              <div className="destination-toggle-panel">
                {!isCustomFolder ? (
                  <>
                    <div className="form-group-contained">
                      <label className="form-label-dark">Songpack</label>
                      <div className="read-only-path-container">
                        <code className="monospace-block">{songpackPath || "Loading default songpack..."}</code>
                      </div>
                    </div>

                    <div className="form-group-contained">
                      <label className="form-label-dark">Song Folder Name</label>
                      <input
                        type="text"
                        className="input-contained"
                        placeholder="e.g. poseidon_special"
                        value={songFolder}
                        onChange={handleFolderNameChange}
                      />
                      {sanitizationError && (
                        <span className="sanitization-error-text">{sanitizationError}</span>
                      )}
                    </div>

                    <button className="btn-ghost-pill btn-sm-contained btn-action-margin" onClick={handleChooseCustomFolder}>
                      Choose Custom Folder
                    </button>
                  </>
                ) : (
                  <>
                    <div className="form-group-contained">
                      <label className="form-label-dark">Choose Custom Folder</label>
                      <div className="input-group">
                        <input
                          type="text"
                          className="input-contained"
                          value={customFolderPath}
                          readOnly
                          placeholder="No custom folder selected"
                        />
                        <button className="btn-ghost-pill btn-sm-contained" onClick={handleChooseCustomFolder}>
                          Choose Custom Folder
                        </button>
                      </div>
                    </div>

                    <button className="btn-ghost-pill btn-sm-contained btn-action-margin" onClick={handleUseDefaultSongpack}>
                      Back to Default Songpack Target
                    </button>
                  </>
                )}
              </div>
            ) : (
              <div className="warning-fallback-box">
                <p className="warning-text-gravel">
                  No default Songs directory is configured in Settings. Please choose a custom folder or configure it.
                </p>
                <div className="button-group-row">
                  <button className="btn-ghost-pill btn-sm-contained" onClick={handleChooseCustomFolder}>
                    Choose Custom Folder...
                  </button>
                  <button className="btn-ghost-pill btn-sm-contained" onClick={() => onNavigate("SETTINGS")}>
                    Go to Settings
                  </button>
                </div>
              </div>
            )}

            {destinationPath && (
              <div className="destination-preview-card">
                <h4 className="preview-label">Destination Preview</h4>
                <code className="monospace-block path-preview">{destinationPath}</code>

                <div className="folder-status-row">
                  {folderStatus === "NotExist" && (
                    <span className="badge-status-neutral">Folder does not exist (will be created)</span>
                  )}
                  {folderStatus === "ExistEmpty" && (
                    <span className="badge-status-ok">Folder exists and is empty (safe to use)</span>
                  )}
                  {folderStatus === "ExistWithSsc" && (
                    <span className="badge-status-error">Folder contains .ssc file (creation blocked)</span>
                  )}
                  {folderStatus === "ExistNotEmpty" && (
                    <div className="consent-required-box">
                      <span className="badge-status-warning">Folder contains files (but no .ssc file)</span>
                      <label className="consent-checkbox-label">
                        <input
                          type="checkbox"
                          checked={explicitConsent}
                          onChange={(e) => {
                            setExplicitConsent(e.target.checked);
                            setIsFolderReady(false);
                          }}
                        />
                        I explicitly agree to use this existing directory
                      </label>
                    </div>
                  )}
                </div>

                <div className="action-row-right">
                  <button
                    className="btn-primary-pill"
                    onClick={handleCreateOrConfirmFolder}
                    disabled={
                      (isCustomFolder && !customFolderPath) ||
                      (!isCustomFolder && (!songFolder || !!sanitizationError)) ||
                      folderStatus === "ExistWithSsc" ||
                      (folderStatus === "ExistNotEmpty" && !explicitConsent)
                    }
                  >
                    Create Folder
                  </button>
                </div>
              </div>
            )}
          </div>
        );
      case 2:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">2. Song Assets</h3>
            <p className="wizard-step-desc">
              Select media assets to bundle into your song folder. The audio file is mandatory.
            </p>

            <div className="assets-selection-grid">
              {/* Audio Block (Required) */}
              <div className={`asset-row-box ${audioFile ? "has-asset" : "missing-asset"}`}>
                <div className="asset-meta-info">
                  <span className="asset-meta-title">Music Audio File (.mp3 / .ogg / .flac / .wav) <span className="req-star">*</span></span>
                  {audioFile ? (
                    <div className="selected-asset-details">
                      <span className="asset-file-name">{audioFile.name}</span>
                      <span className="asset-file-size">{formatBytes(audioFile.size)}</span>
                    </div>
                  ) : (
                    <span className="no-asset-label">No audio selected (Required)</span>
                  )}
                </div>
                <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("audio")}>
                  {audioFile ? "Replace Audio" : "Choose Audio"}
                </button>
              </div>

              {/* Banner Block (Optional) */}
              <div className={`asset-row-box ${bannerFile ? "has-asset" : ""}`}>
                <div className="asset-meta-info">
                  <span className="asset-meta-title">Pack Banner Image (.png / .jpg / .jpeg)</span>
                  {bannerFile ? (
                    <div className="selected-asset-details">
                      <span className="asset-file-name">{bannerFile.name}</span>
                      <span className="asset-file-size">{formatBytes(bannerFile.size)}</span>
                    </div>
                  ) : (
                    <span className="no-asset-label">Optional Banner</span>
                  )}
                </div>
                <div className="button-group-row">
                  <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("banner")}>
                    {bannerFile ? "Replace" : "Choose Banner"}
                  </button>
                  {bannerFile && (
                    <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("banner")}>
                      Clear
                    </button>
                  )}
                </div>
              </div>

              {/* Background Block (Optional) */}
              <div className={`asset-row-box ${backgroundFile ? "has-asset" : ""}`}>
                <div className="asset-meta-info">
                  <span className="asset-meta-title">Background Image (.png / .jpg / .jpeg)</span>
                  {backgroundFile ? (
                    <div className="selected-asset-details">
                      <span className="asset-file-name">{backgroundFile.name}</span>
                      <span className="asset-file-size">{formatBytes(backgroundFile.size)}</span>
                    </div>
                  ) : (
                    <span className="no-asset-label">Optional Background</span>
                  )}
                </div>
                <div className="button-group-row">
                  <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("background")}>
                    {backgroundFile ? "Replace" : "Choose Background"}
                  </button>
                  {backgroundFile && (
                    <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("background")}>
                      Clear
                    </button>
                  )}
                </div>
              </div>

              {/* Video Block (Optional) */}
              <div className={`asset-row-box ${videoFile ? "has-asset" : ""}`}>
                <div className="asset-meta-info">
                  <span className="asset-meta-title">Background Video Overlay (.mp4 / .mov / .avi / .mpg)</span>
                  {videoFile ? (
                    <div className="selected-asset-details">
                      <span className="asset-file-name">{videoFile.name}</span>
                      <span className="asset-file-size">{formatBytes(videoFile.size)}</span>
                    </div>
                  ) : (
                    <span className="no-asset-label">Optional Video Overlay</span>
                  )}
                </div>
                <div className="button-group-row">
                  <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("video")}>
                    {videoFile ? "Replace" : "Choose Video"}
                  </button>
                  {videoFile && (
                    <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("video")}>
                      Clear
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>
        );
      case 3:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">3. Song Metadata</h3>
            <p className="wizard-step-desc">
              Specify the primary metadata parameters that will be written into the base `.ssc` configuration.
            </p>

            <div className="metadata-input-form">
              <div className="form-group-contained">
                <label className="form-label-dark">Song Title <span className="req-star">*</span></label>
                <input
                  type="text"
                  className="input-contained"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="e.g. Poseidon"
                />
              </div>

              <div className="form-group-contained">
                <label className="form-label-dark">Artist / Group</label>
                <input
                  type="text"
                  className="input-contained"
                  value={artist}
                  onChange={(e) => setArtist(e.target.value)}
                  placeholder="e.g. Banya"
                />
              </div>

              <div className="metadata-form-row">
                <div className="form-group-contained">
                  <label className="form-label-dark">Genre</label>
                  <input
                    type="text"
                    className="input-contained"
                    value={genre}
                    onChange={(e) => setGenre(e.target.value)}
                    placeholder="e.g. Original"
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Credit / Author</label>
                  <input
                    type="text"
                    className="input-contained"
                    value={credit}
                    onChange={(e) => setCredit(e.target.value)}
                    placeholder="Stepmaker Credit"
                  />
                </div>
              </div>

              <div className="metadata-form-row">
                <div className="form-group-contained">
                  <label className="form-label-dark">Song Type</label>
                  <select
                    className="input-contained"
                    value={songType}
                    onChange={(e) => setSongType(e.target.value)}
                  >
                    <option value="ARCADE">ARCADE (Standard length)</option>
                    <option value="SHORTCUT">SHORTCUT (Intro/Cut version)</option>
                    <option value="REMIX">REMIX (Longer mashup)</option>
                    <option value="FULLSONG">FULLSONG (Full track)</option>
                  </select>
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Display BPM (Interface value)</label>
                  <input
                    type="text"
                    className="input-contained"
                    value={displayBpm}
                    onChange={(e) => setDisplayBpm(e.target.value)}
                    placeholder="e.g. 120.000 or *"
                  />
                </div>
              </div>

              <div className="metadata-form-row">
                <div className="form-group-contained">
                  <label className="form-label-dark">Timing BPM (Default tempo)</label>
                  <input
                    type="number"
                    step="0.001"
                    className="input-contained"
                    value={timingBpm}
                    onChange={(e) => setTimingBpm(e.target.value)}
                    placeholder="e.g. 120.000"
                  />
                </div>

                <div className="form-group-contained">
                  <label className="form-label-dark">Global Offset (Seconds)</label>
                  <input
                    type="number"
                    step="0.000001"
                    className="input-contained"
                    value={offset}
                    onChange={(e) => setOffset(e.target.value)}
                    placeholder="e.g. 0.000000"
                  />
                </div>
              </div>
            </div>
          </div>
        );
      case 4:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">4. Review & Create</h3>
            <p className="wizard-step-desc">
              Please review your project settings and asset list before generating the StepF2 folder structure.
            </p>

            <div className="summary-card-panel">
              <div className="summary-block">
                <h4 className="summary-block-title">Target Directory</h4>
                <code className="monospace-block">{destinationPath}</code>
              </div>

              <div className="summary-grid">
                <div className="summary-block">
                  <h4 className="summary-block-title">Metadata Profile</h4>
                  <table className="summary-table">
                    <tbody>
                      <tr>
                        <td>Title:</td>
                        <td><strong>{title}</strong></td>
                      </tr>
                      <tr>
                        <td>Artist:</td>
                        <td>{artist || "—"}</td>
                      </tr>
                      <tr>
                        <td>Genre / Type:</td>
                        <td>{genre} ({songType})</td>
                      </tr>
                      <tr>
                        <td>Credit:</td>
                        <td>{credit || "—"}</td>
                      </tr>
                      <tr>
                        <td>BPM / Offset:</td>
                        <td>{timingBpm} BPM (Display: {displayBpm}) / {offset}s</td>
                      </tr>
                    </tbody>
                  </table>
                </div>

                <div className="summary-block">
                  <h4 className="summary-block-title">Associated Assets</h4>
                  <ul className="summary-assets-list">
                    <li>✓ Audio: <span className="asset-name-val">{audioFile?.name}</span> ({formatBytes(audioFile?.size || 0)})</li>
                    {bannerFile && (
                      <li>✓ Banner: <span className="asset-name-val">{bannerFile.name}</span> ({formatBytes(bannerFile.size)})</li>
                    )}
                    {backgroundFile && (
                      <li>✓ Background: <span className="asset-name-val">{backgroundFile.name}</span> ({formatBytes(backgroundFile.size)})</li>
                    )}
                    {videoFile && (
                      <li>✓ Video Overlay: <span className="asset-name-val">{videoFile.name}</span> ({formatBytes(videoFile.size)})</li>
                    )}
                  </ul>
                </div>
              </div>
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  return (
    <div className="wizard-container">
      <div className="wizard-sidebar">
        <h2 className="wizard-heading">Create Song</h2>
        <div className="wizard-steps-list">
          {WIZARD_STEPS.map((step) => (
            <div
              key={step.id}
              className={`wizard-step-item ${currentStep === step.id ? "active" : ""} ${
                currentStep > step.id ? "completed" : ""
              }`}
            >
              <div className="wizard-step-circle">{step.id}</div>
              <span className="wizard-step-name">{step.name}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="wizard-main">
        <div className="wizard-header">
          <span className="wizard-progress-info">
            Step {currentStep} of {WIZARD_STEPS.length}
          </span>
        </div>

        <div className="wizard-content">
          {errorMsg && (
            <div className="error-box">
              <AlertTriangle size={16} className="icon-mr text-danger-icon" />
              <span>{errorMsg}</span>
            </div>
          )}
          {successMsg && (
            <div className="success-box">
              <CheckCircle size={16} className="icon-mr text-success-icon" />
              <span>{successMsg}</span>
            </div>
          )}
          {renderStepContent()}
        </div>

        <div className="wizard-footer">
          <button
            className="btn-ghost-pill"
            onClick={currentStep === 1 ? () => onNavigate("START_MENU") : handleBack}
            disabled={isSubmitting}
          >
            <ArrowLeft className="icon-mr" size={16} />
            {currentStep === 1 ? "Exit Wizard" : "Back"}
          </button>

          {currentStep === WIZARD_STEPS.length ? (
            <button className="btn-primary-pill" onClick={handleCreateProject} disabled={isSubmitting}>
              {isSubmitting ? "Creating..." : "Create Song Project"}
              <Save className="icon-ml" size={16} />
            </button>
          ) : (
            <button className="btn-primary-pill" onClick={handleNext} disabled={currentStep === 1 && !isFolderReady}>
              Next
              <ArrowRight className="icon-ml" size={16} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
