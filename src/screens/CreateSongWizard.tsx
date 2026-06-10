import React, { useState, useEffect, useRef } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { useSongProjectStore } from "../store/songProjectStore";
import { useSettingsStore } from "../store/settingsStore";
import { ISongDetails } from "../types/song";
import { ArrowRight, Save, AlertTriangle, CheckCircle, Play, Pause, Music, Image, Video, Trash2 } from "lucide-react";
import { WizardFooter } from "../components/WizardFooter";
import { analyzeBrowserBpmFromArrayBuffer } from "../features/music-analysis/browser-bpm/analyzeBrowserBpmFromBuffer";
import { BrowserBpmAnalysisReport } from "../features/music-analysis/browser-bpm/browserBpmTypes";
import { reconcileBpmCandidates } from "../features/music-analysis/browser-bpm/browserBpmReconciliation";
import { getBrowserBpmSupport } from "../features/music-analysis/browser-bpm/browserBpmSupport";

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

  const isMountedRef = useRef(true);
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      invoke("clear_browser_bpm_audio_grants").catch((err) =>
        console.warn("Failed to clear audio grants:", err)
      );
    };
  }, []);

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

  // Step 2: Mini Player & Ambient Blur States & Refs
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const bannerBlurRef = useRef<HTMLDivElement | null>(null);
  const bgBlurRef = useRef<HTMLDivElement | null>(null);
  const wizardBlurRef = useRef<HTMLDivElement | null>(null);

  // Apply blurry Spotify ambient background in DOM using JavaScript to bypass "style=" JSX rules
  useEffect(() => {
    if (bannerBlurRef.current) {
      if (bannerFile) {
        const bannerUrl = convertFileSrc(bannerFile.path);
        bannerBlurRef.current.style.backgroundImage = `url("${bannerUrl}")`;
        bannerBlurRef.current.style.opacity = "0.25";
      } else {
        bannerBlurRef.current.style.backgroundImage = "";
        bannerBlurRef.current.style.opacity = "0";
      }
    }
  }, [bannerFile, currentStep]);

  useEffect(() => {
    if (bgBlurRef.current) {
      if (backgroundFile) {
        const bgUrl = convertFileSrc(backgroundFile.path);
        bgBlurRef.current.style.backgroundImage = `url("${bgUrl}")`;
        bgBlurRef.current.style.opacity = "0.25";
      } else {
        bgBlurRef.current.style.backgroundImage = "";
        bgBlurRef.current.style.opacity = "0";
      }
    }
  }, [backgroundFile, currentStep]);

  // Ambient blurry background for steps >= 3 (Metadata and Review & Create)
  useEffect(() => {
    if (wizardBlurRef.current) {
      const activeFile = bannerFile || backgroundFile;
      if (activeFile && currentStep >= 3) {
        const url = convertFileSrc(activeFile.path);
        wizardBlurRef.current.style.backgroundImage = `url("${url}")`;
        wizardBlurRef.current.style.opacity = "0.12"; // strategic soft opacity to guarantee contrast
      } else {
        wizardBlurRef.current.style.backgroundImage = "";
        wizardBlurRef.current.style.opacity = "0";
      }
    }
  }, [bannerFile, backgroundFile, currentStep]);

  // Reset play state if audio file changes
  useEffect(() => {
    setIsPlaying(false);
    setCurrentTime(0);
    setDuration(0);
  }, [audioFile]);

  const togglePlay = () => {
    if (!audioRef.current) return;
    if (isPlaying) {
      audioRef.current.pause();
      setIsPlaying(false);
    } else {
      audioRef.current.play().catch((err) => console.error("Playback error:", err));
      setIsPlaying(true);
    }
  };

  const handleSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    setCurrentTime(val);
    if (audioRef.current) {
      audioRef.current.currentTime = val;
    }
  };

  const handleTimeUpdate = () => {
    if (audioRef.current) {
      setCurrentTime(audioRef.current.currentTime);
    }
  };

  const handleLoadedMetadata = () => {
    if (audioRef.current) {
      setDuration(audioRef.current.duration);
    }
  };

  const formatTime = (secs: number) => {
    if (isNaN(secs)) return "0:00";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  };

  // Step 3: Metadata States
  const [title, setTitle] = useState("");
  const [artist, setArtist] = useState("");
  const [genre, setGenre] = useState("Original");
  const [credit, setCredit] = useState("");
  const [songType, setSongType] = useState("ARCADE");
  const [displayBpm, setDisplayBpm] = useState("120.000");
  const [timingBpm, setTimingBpm] = useState("120.000");
  const [offset, setOffset] = useState("0.000000");

  // Browser BPM Analysis States
  const [browserBpmReport, setBrowserBpmReport] = useState<BrowserBpmAnalysisReport | null>(null);
  const [isBpmAnalyzing, setIsBpmAnalyzing] = useState(false);
  const [bpmAnalysisError, setBpmAnalysisError] = useState<string | null>(null);

  const activeAudioPathRef = useRef<string | null>(null);
  const hasUserEditedBpmRef = useRef(false);
  const requestIdRef = useRef(0);

  const runBpmAnalysis = async (filePath: string, fileName: string, reqId: number) => {
    const support = getBrowserBpmSupport();
    if (!support.isSupported) {
      setBrowserBpmReport({
        source: "browser_realtime_bpm_analyzer",
        libraryName: "realtime-bpm-analyzer",
        generatedAtIso: new Date().toISOString(),
        mode: "offline_full_buffer",
        audioFileName: fileName,
        candidates: [],
        support,
        warnings: [support.reasonIfUnsupported ?? "Browser BPM unsupported"],
      });
      return;
    }

    setIsBpmAnalyzing(true);
    setBpmAnalysisError(null);
    try {
      const bytes = await invoke<number[]>("read_audio_file", { path: filePath });
      if (requestIdRef.current !== reqId || !isMountedRef.current || activeAudioPathRef.current !== filePath) {
        return;
      }
      const uint8 = new Uint8Array(bytes);
      const report = await analyzeBrowserBpmFromArrayBuffer({
        arrayBuffer: uint8.buffer,
        audioFileName: fileName,
      });

      if (requestIdRef.current !== reqId || !isMountedRef.current || activeAudioPathRef.current !== filePath) {
        return;
      }

      setBrowserBpmReport(report);

      const recon = reconcileBpmCandidates({
        sscBpms: [],
        browserCandidates: report.candidates,
        toleranceBpm: 2.0,
        minConfidence: 0.2,
        minCount: 4,
        isSupported: support.isSupported,
      });

      if (recon.suggestedBpm && !hasUserEditedBpmRef.current) {
        setTimingBpm(recon.suggestedBpm.toString());
        setDisplayBpm(recon.suggestedBpm.toString());
      }
    } catch (err: any) {
      if (requestIdRef.current === reqId && isMountedRef.current && activeAudioPathRef.current === filePath) {
        console.error("BPM preanalysis error:", err);
        setBpmAnalysisError(err.toString());
      }
    } finally {
      if (requestIdRef.current === reqId && isMountedRef.current && activeAudioPathRef.current === filePath) {
        setIsBpmAnalyzing(false);
      }
    }
  };

  useEffect(() => {
    if (audioFile) {
      activeAudioPathRef.current = audioFile.path;
      hasUserEditedBpmRef.current = false;
      const nextId = ++requestIdRef.current;
      runBpmAnalysis(audioFile.path, audioFile.name, nextId);
    } else {
      activeAudioPathRef.current = null;
      setBrowserBpmReport(null);
      setBpmAnalysisError(null);
      hasUserEditedBpmRef.current = false;
    }
    return () => {
      requestIdRef.current++;
      activeAudioPathRef.current = null;
    };
  }, [audioFile]);

  const handleTimingBpmChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTimingBpm(e.target.value);
    hasUserEditedBpmRef.current = true;
  };

  const handleDisplayBpmChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setDisplayBpm(e.target.value);
    hasUserEditedBpmRef.current = true;
  };

  const handleUseSuggestedBpm = () => {
    if (browserBpmReport?.candidates && browserBpmReport.candidates.length > 0) {
      const recon = reconcileBpmCandidates({
        sscBpms: [],
        browserCandidates: browserBpmReport.candidates,
        toleranceBpm: 2.0,
        minConfidence: 0.2,
        minCount: 4,
        isSupported: browserBpmReport.support.isSupported,
      });
      if (recon.suggestedBpm) {
        setTimingBpm(recon.suggestedBpm.toString());
        setDisplayBpm(recon.suggestedBpm.toString());
        hasUserEditedBpmRef.current = true;
      }
    }
  };

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
      if (kind === "audio") {
        await invoke("clear_browser_bpm_audio_grants").catch((err) =>
          console.warn("Failed to clear audio grants:", err)
        );
      }
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
          <div className="wizard-step-body" key="wizard-step-1">
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
                      Back to Default Songpack
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
                    Choose Custom Folder
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
        const audioUrl = audioFile ? convertFileSrc(audioFile.path) : "";
        const bannerUrl = bannerFile ? convertFileSrc(bannerFile.path) : "";
        const backgroundUrl = backgroundFile ? convertFileSrc(backgroundFile.path) : "";
        const videoUrl = videoFile ? convertFileSrc(videoFile.path) : "";

        return (
          <div className="wizard-step-body" key="wizard-step-2">
            <h3 className="wizard-step-title">2. Song Assets</h3>
            <p className="wizard-step-desc">
              Select media assets to bundle into your song folder. The audio file is mandatory.
            </p>

            <div className="bento-grid-container">
              {/* Card 1: Audio File (Required) - spans 2 columns */}
              <div className={`bento-card bento-card-wide bento-audio-card ${audioFile ? "has-asset" : "missing-asset"}`}>
                <div className="bento-card-content">
                  <div className="bento-header">
                    <span className="bento-title">Music Audio File <span className="req-star">*</span></span>
                    <span className="bento-desc">Mandatory track (.mp3, .ogg, .flac, .wav)</span>
                  </div>

                  {audioFile ? (
                    <div className="audio-player-container">
                      <div className="audio-meta">
                        <span className="audio-filename">{audioFile.name}</span>
                        <span className="audio-filesize">{formatBytes(audioFile.size)}</span>
                      </div>

                      <div className="mini-player-ui">
                        <button className="play-pause-btn" onClick={togglePlay}>
                          {isPlaying ? <Pause size={12} fill="currentColor" /> : <Play size={12} fill="currentColor" />}
                        </button>

                        <div className="player-timeline-wrapper">
                          <span className="time-text">{formatTime(currentTime)}</span>
                          <input
                            type="range"
                            className="player-slider"
                            min="0"
                            max={duration || 100}
                            value={currentTime}
                            onChange={handleSeek}
                          />
                          <span className="time-text">{formatTime(duration)}</span>
                        </div>
                      </div>

                      {/* Gemini voice spectrum / EQ waves animated indicator */}
                      <div className={`gemini-waves-container ${isPlaying ? "animating" : ""}`}>
                        <div className="gemini-wave wave1"></div>
                        <div className="gemini-wave wave2"></div>
                        <div className="gemini-wave wave3"></div>
                        <div className="gemini-wave wave4"></div>
                      </div>

                      <div className="player-actions">
                        <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("audio")}>
                          Replace Audio
                        </button>
                      </div>

                      {/* Hidden HTML audio element */}
                      <audio
                        ref={audioRef}
                        src={audioUrl}
                        onTimeUpdate={handleTimeUpdate}
                        onLoadedMetadata={handleLoadedMetadata}
                        onEnded={() => setIsPlaying(false)}
                      />
                    </div>
                  ) : (
                    <div className="bento-upload-placeholder" onClick={() => handleSelectAsset("audio")}>
                      <div className="upload-icon-wrapper">
                        <Music size={18} />
                      </div>
                      <span>Select Audio File</span>
                    </div>
                  )}
                </div>
              </div>

              {/* Card 2: Banner Image (Optional) - spans 1 column */}
              <div className={`bento-card bento-banner-card ${bannerFile ? "has-asset" : ""}`}>
                <div className="bento-card-blur-bg" ref={bannerBlurRef}></div>
                <div className="bento-card-content">
                  <div className="bento-header">
                    <span className="bento-title">Banner Image</span>
                    <span className="bento-desc">Pack cover (.png, .jpg, .jpeg)</span>
                  </div>

                  {bannerFile ? (
                    <div className="preview-container">
                      <div className="preview-image-wrapper">
                        <img src={bannerUrl} alt="Banner Preview" className="preview-img" />
                      </div>
                      <div className="preview-meta">
                        <span className="preview-filename">{bannerFile.name}</span>
                        <span className="preview-filesize">{formatBytes(bannerFile.size)}</span>
                      </div>
                      <div className="preview-actions">
                        <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("banner")}>
                          Replace
                        </button>
                        <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("banner")}>
                          <Trash2 size={12} className="icon-mr" /> Clear
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="bento-upload-placeholder" onClick={() => handleSelectAsset("banner")}>
                      <div className="upload-icon-wrapper">
                        <Image size={18} />
                      </div>
                      <span>Select Banner</span>
                    </div>
                  )}
                </div>
              </div>

              {/* Card 3: Background Image (Optional) - spans 1 column */}
              <div className={`bento-card bento-bg-card ${backgroundFile ? "has-asset" : ""}`}>
                <div className="bento-card-blur-bg" ref={bgBlurRef}></div>
                <div className="bento-card-content">
                  <div className="bento-header">
                    <span className="bento-title">Background Image</span>
                    <span className="bento-desc">Song backdrop (.png, .jpg, .jpeg)</span>
                  </div>

                  {backgroundFile ? (
                    <div className="preview-container">
                      <div className="preview-image-wrapper">
                        <img src={backgroundUrl} alt="Background Preview" className="preview-img" />
                      </div>
                      <div className="preview-meta">
                        <span className="preview-filename">{backgroundFile.name}</span>
                        <span className="preview-filesize">{formatBytes(backgroundFile.size)}</span>
                      </div>
                      <div className="preview-actions">
                        <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("background")}>
                          Replace
                        </button>
                        <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("background")}>
                          <Trash2 size={12} className="icon-mr" /> Clear
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="bento-upload-placeholder" onClick={() => handleSelectAsset("background")}>
                      <div className="upload-icon-wrapper">
                        <Image size={18} />
                      </div>
                      <span>Select Background</span>
                    </div>
                  )}
                </div>
              </div>

              {/* Card 4: Video Overlay (Optional) - spans 2 columns */}
              <div className={`bento-card bento-card-wide bento-video-card ${videoFile ? "has-asset" : ""}`}>
                <div className="bento-card-content">
                  <div className="bento-header">
                    <span className="bento-title">Background Video Overlay</span>
                    <span className="bento-desc">Optional BGA video (.mp4, .mov, .avi, .mpg)</span>
                  </div>

                  {videoFile ? (
                    <div className="preview-container">
                      <div className="preview-video-wrapper">
                        <video src={videoUrl} className="preview-video" autoPlay muted loop playsInline />
                      </div>
                      <div className="preview-meta">
                        <span className="preview-filename">{videoFile.name}</span>
                        <span className="preview-filesize">{formatBytes(videoFile.size)}</span>
                      </div>
                      <div className="preview-actions">
                        <button className="btn-ghost-pill btn-sm-contained" onClick={() => handleSelectAsset("video")}>
                          Replace
                        </button>
                        <button className="btn-ghost-pill btn-sm-contained btn-danger-text" onClick={() => handleClearAsset("video")}>
                          <Trash2 size={12} className="icon-mr" /> Clear
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="bento-upload-placeholder" onClick={() => handleSelectAsset("video")}>
                      <div className="upload-icon-wrapper">
                        <Video size={18} />
                      </div>
                      <span>Select Video</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </div>
        );
      case 3:
        return (
          <div className="wizard-step-body" key="wizard-step-3">
            <h3 className="wizard-step-title">3. Song Metadata</h3>
            <p className="wizard-step-desc">
              Specify the primary metadata parameters that will be written into the base `.ssc` configuration.
            </p>

            <div className="metadata-input-form">
              <div className="metadata-form-grid">
                {/* Left Column: Creative Fields */}
                <div className="metadata-form-column">
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

                {/* Right Column: System / Timing Fields */}
                <div className="metadata-form-column">
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
                      onChange={handleDisplayBpmChange}
                      placeholder="e.g. 120.000 or *"
                    />
                  </div>

                  <div className="form-group-contained">
                    <label className="form-label-dark">Timing BPM (Default tempo)</label>
                    <input
                      type="number"
                      step="0.001"
                      className="input-contained"
                      value={timingBpm}
                      onChange={handleTimingBpmChange}
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

              {/* Quick BPM Analysis Panel */}
              <div className="settings-card bpm-analysis-wrapper">
                <div className="bpm-analysis-header-row">
                  <h4 className="card-subtitle-obsidian">Quick BPM Analysis</h4>
                  {!getBrowserBpmSupport().isSupported && (
                    <span className="badge-status-error">Web Audio Unsupported</span>
                  )}
                </div>
                <p className="caption-text-gravel">
                  Detect tempo in your browser using <code>realtime-bpm-analyzer</code>.
                </p>

                {isBpmAnalyzing ? (
                  <div className="info-banner-gray">
                    <div className="gemini-waves-container animating">
                      <div className="gemini-wave wave1"></div>
                      <div className="gemini-wave wave2"></div>
                      <div className="gemini-wave wave3"></div>
                    </div>
                    <span className="icon-ml">Running analysis on audio buffer...</span>
                  </div>
                ) : bpmAnalysisError ? (
                  <div className="error-box">
                    <AlertTriangle size={16} className="icon-mr text-danger-icon" />
                    <span>Failed browser BPM analysis: {bpmAnalysisError}</span>
                  </div>
                ) : browserBpmReport ? (
                  <div>
                    <div className="bpm-results-grid">
                      <div className="bpm-stat-box">
                        <span className="bpm-stat-title">Suggested BPM</span>
                        <span className="bpm-stat-value">
                          {(() => {
                            const recon = reconcileBpmCandidates({
                              sscBpms: [],
                              browserCandidates: browserBpmReport.candidates,
                              toleranceBpm: 2.0,
                              minConfidence: 0.2,
                              minCount: 4,
                              isSupported: browserBpmReport.support.isSupported,
                            });
                            return recon.suggestedBpm ?? "No result (low confidence/count)";
                          })()}
                        </span>
                      </div>
                      {browserBpmReport.stableTempo && (
                        <div className="bpm-stat-box">
                          <span className="bpm-stat-title">Relative Score</span>
                          <span className="bpm-stat-value">
                            {(browserBpmReport.stableTempo.confidence * 100).toFixed(0)}%
                          </span>
                        </div>
                      )}
                    </div>

                    {browserBpmReport.warnings && browserBpmReport.warnings.length > 0 && (
                      <div className="warning-box">
                        <AlertTriangle size={16} className="icon-mr text-warning-icon" />
                        <div>
                          {browserBpmReport.warnings.map((w, idx) => (
                            <div key={idx}>{w}</div>
                          ))}
                        </div>
                      </div>
                    )}

                    {browserBpmReport.candidates && browserBpmReport.candidates.length > 0 && (
                      <div className="bpm-candidates-section">
                        <span className="bpm-stat-title">Raw Candidates</span>
                        <div className="bpm-candidates-list">
                          {browserBpmReport.candidates.map((c, i) => (
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

                    <div className="bpm-actions-row">
                      {(() => {
                        const recon = reconcileBpmCandidates({
                          sscBpms: [],
                          browserCandidates: browserBpmReport.candidates,
                          toleranceBpm: 2.0,
                          minConfidence: 0.2,
                          minCount: 4,
                          isSupported: browserBpmReport.support.isSupported,
                        });
                        const hasSuggested = recon.suggestedBpm != null;
                        return (
                          <button
                            type="button"
                            className="btn-ghost-pill btn-sm-contained"
                            onClick={handleUseSuggestedBpm}
                            disabled={!hasSuggested}
                          >
                            Use Suggested BPM
                          </button>
                        );
                      })()}
                      {audioFile && (
                        <button
                          type="button"
                          className="btn-ghost-pill btn-sm-contained"
                          onClick={() => {
                            const nextId = ++requestIdRef.current;
                            runBpmAnalysis(audioFile.path, audioFile.name, nextId);
                          }}
                        >
                          Run Browser BPM Analysis
                        </button>
                      )}
                      <span className="caption-text-gravel">
                        * Offset remains manual for now (default 0.000000)
                      </span>
                    </div>
                  </div>
                ) : (
                  <div>
                    {audioFile ? (
                      <button
                        type="button"
                        className="btn-ghost-pill btn-sm-contained"
                        onClick={() => {
                          const nextId = ++requestIdRef.current;
                          runBpmAnalysis(audioFile.path, audioFile.name, nextId);
                        }}
                      >
                        Run Browser BPM Analysis
                      </button>
                    ) : (
                      <span className="caption-text-gravel">Please select an audio asset to enable analysis.</span>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      case 4:
        return (
          <div className="wizard-step-body" key="wizard-step-4">
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
        <div className="wizard-blur-bg" ref={wizardBlurRef}></div>
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

        <WizardFooter
          onBack={currentStep === 1 ? () => onNavigate("START_MENU") : handleBack}
          backLabel={currentStep === 1 ? "Exit Wizard" : "Back"}
          onNext={currentStep === WIZARD_STEPS.length ? handleCreateProject : handleNext}
          nextLabel={currentStep === WIZARD_STEPS.length ? "Create Song Project" : "Next"}
          isNextDisabled={currentStep === 1 && !isFolderReady}
          isSubmitting={isSubmitting}
          nextIcon={
            currentStep === WIZARD_STEPS.length ? (
              <Save className="icon-ml" size={16} />
            ) : (
              <ArrowRight className="icon-ml" size={16} />
            )
          }
        />
      </div>
    </div>
  );
};
