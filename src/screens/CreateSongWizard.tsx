import React, { useState } from "react";
import { ArrowLeft, ArrowRight, Save, Info, AlertTriangle, ShieldCheck } from "lucide-react";

interface CreateSongWizardProps {
  onNavigate: (screen: string) => void;
}

const WIZARD_STEPS = [
  { id: 1, name: "Project Destination" },
  { id: 2, name: "Song Assets" },
  { id: 3, name: "Metadata" },
  { id: 4, name: "Timing Analysis" },
  { id: 5, name: "Music Analysis" },
  { id: 6, name: "Chart Plan" },
  { id: 7, name: "Generate Preview" },
  { id: 8, name: "Review & Export" },
];

export const CreateSongWizard: React.FC<CreateSongWizardProps> = ({ onNavigate }) => {
  const [currentStep, setCurrentStep] = useState(1);

  // Form states (placeholders)
  const [songsDir, setSongsDir] = useState("");
  const [songName, setSongName] = useState("");
  const [artist, setArtist] = useState("");
  const [bpm, setBpm] = useState("120");

  const handleNext = () => {
    if (currentStep < WIZARD_STEPS.length) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handleBack = () => {
    if (currentStep > 1) {
      setCurrentStep(currentStep - 1);
    }
  };

  const renderStepContent = () => {
    switch (currentStep) {
      case 1:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Select Project Destination</h3>
            <p className="wizard-step-desc">
              Specify where your generated song folder should be saved. By default, it will reside inside your configured StepF2/StepP1 Songs directory.
            </p>
            <div className="form-group-contained">
              <label className="form-label-dark">Songs Folder Destination</label>
              <input
                type="text"
                className="input-contained"
                placeholder="C:\StepF2\Songs\99-AI-Step-Gen\MyNewSong"
                value={songsDir}
                onChange={(e) => setSongsDir(e.target.value)}
              />
            </div>
            <div className="info-banner-gray">
              <Info size={16} className="icon-mr" />
              <span>Note: This step operates locally on your disk. No tokens consumed.</span>
            </div>
          </div>
        );
      case 2:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Upload / Link Song Assets</h3>
            <p className="wizard-step-desc">
              Attach the necessary media assets to begin the generation process. An audio file (.mp3/.ogg) is mandatory.
            </p>
            <div className="assets-placeholder-grid">
              <div className="asset-upload-card">
                <span className="asset-card-title">Audio File (.mp3 / .ogg)</span>
                <span className="asset-card-status text-missing">No file selected</span>
                <button className="btn-ghost-pill btn-sm-contained">Browse File</button>
              </div>
              <div className="asset-upload-card">
                <span className="asset-card-title">Banner Image (.png)</span>
                <span className="asset-card-status text-optional">Optional (Recommended)</span>
                <button className="btn-ghost-pill btn-sm-contained">Browse File</button>
              </div>
              <div className="asset-upload-card">
                <span className="asset-card-title">Background Image (.png)</span>
                <span className="asset-card-status text-optional">Optional</span>
                <button className="btn-ghost-pill btn-sm-contained">Browse File</button>
              </div>
              <div className="asset-upload-card">
                <span className="asset-card-title">Video File (.mp4)</span>
                <span className="asset-card-status text-optional">Optional</span>
                <button className="btn-ghost-pill btn-sm-contained">Browse File</button>
              </div>
            </div>
          </div>
        );
      case 3:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Song Metadata</h3>
            <p className="wizard-step-desc">
              Configure the metadata tags that will be serialized into the output `.ssc` file.
            </p>
            <div className="metadata-form-grid">
              <div className="form-group-contained">
                <label className="form-label-dark">Song Title</label>
                <input
                  type="text"
                  className="input-contained"
                  placeholder="e.g., Poseidon"
                  value={songName}
                  onChange={(e) => setSongName(e.target.value)}
                />
              </div>
              <div className="form-group-contained">
                <label className="form-label-dark">Artist</label>
                <input
                  type="text"
                  className="input-contained"
                  placeholder="e.g., Banya"
                  value={artist}
                  onChange={(e) => setArtist(e.target.value)}
                />
              </div>
              <div className="form-group-contained">
                <label className="form-label-dark">BPM (Tempo)</label>
                <input
                  type="number"
                  className="input-contained"
                  placeholder="e.g., 120"
                  value={bpm}
                  onChange={(e) => setBpm(e.target.value)}
                />
              </div>
            </div>
          </div>
        );
      case 4:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Timing Analysis</h3>
            <p className="wizard-step-desc">
              Align the audio beat grid with the timing offset. Accurate timing is critical for rhythm alignment.
            </p>
            <div className="placeholder-state-card">
              <AlertTriangle className="text-warning-icon" size={24} />
              <span className="placeholder-text-main">Timing analysis is a planned feature</span>
              <span className="placeholder-text-sub">
                In this version, timing will be auto-calculated using a constant BPM assumption or must be edited manually in the editor.
              </span>
            </div>
          </div>
        );
      case 5:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Music Analysis</h3>
            <p className="wizard-step-desc">
              The Music Analysis Engine segments audio into musical phrases and maps choreographic intensity.
            </p>
            <div className="placeholder-state-card">
              <Info className="text-info-icon" size={24} />
              <span className="placeholder-text-main">Feature restricted in MVP</span>
              <span className="placeholder-text-sub">
                The Music Analysis Engine is scheduled for v1.1. In this release, generation relies on user-specified segment markers.
              </span>
            </div>
          </div>
        );
      case 6:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Chart Plan</h3>
            <p className="wizard-step-desc">
              Define the generation strategy, target levels, and play modes (Single / Double).
            </p>
            <div className="placeholder-state-card">
              <ShieldCheck className="text-success-icon" size={24} />
              <span className="placeholder-text-main">Generation configuration ready</span>
              <span className="placeholder-text-sub">
                Configured to generate Single charts (Lv. 1-26). Double charts are experimental (Lv. 1-15 max).
              </span>
            </div>
          </div>
        );
      case 7:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Generate Preview</h3>
            <p className="wizard-step-desc">
              Perform secure dry-run requests to generate and review patterns before committing to disk.
            </p>
            <div className="placeholder-state-card">
              <Info className="text-info-icon" size={24} />
              <span className="placeholder-text-main">API Gateway check required</span>
              <span className="placeholder-text-sub">
                Actual generation calls are restricted to imported songs in this version. Proceed to next step to review export flows.
              </span>
            </div>
          </div>
        );
      case 8:
        return (
          <div className="wizard-step-body">
            <h3 className="wizard-step-title">Review & Export</h3>
            <p className="wizard-step-desc">
              Finalize the song metadata, package configuration, and export the file.
            </p>
            <div className="placeholder-state-card">
              <Save className="text-info-icon" size={24} />
              <span className="placeholder-text-main">Ready to build workspace</span>
              <span className="placeholder-text-sub">
                Since song-creation is in mock development, click Finish to return to the workspace to edit imported songs.
              </span>
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

        <div className="wizard-content">{renderStepContent()}</div>

        <div className="wizard-footer">
          <button
            className="btn-ghost-pill"
            onClick={currentStep === 1 ? () => onNavigate("START_MENU") : handleBack}
          >
            <ArrowLeft className="icon-mr" size={16} />
            {currentStep === 1 ? "Exit Wizard" : "Back"}
          </button>

          {currentStep === WIZARD_STEPS.length ? (
            <button className="btn-primary-pill" onClick={() => onNavigate("START_MENU")}>
              Finish
              <ShieldCheck className="icon-ml" size={16} />
            </button>
          ) : (
            <button className="btn-primary-pill" onClick={handleNext}>
              Next
              <ArrowRight className="icon-ml" size={16} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
