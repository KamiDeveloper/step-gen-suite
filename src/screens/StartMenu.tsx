import React from "react";
import { Sparkles, FolderOpen, Settings, ShieldAlert } from "lucide-react";
import { useSettingsStore } from "../store/settingsStore";
import { GameplayBackground } from "../components/GameplayBackground";

interface StartMenuProps {
  onNavigate: (screen: string) => void;
}

export const StartMenu: React.FC<StartMenuProps> = ({ onNavigate }) => {
  const { appMode } = useSettingsStore();

  return (
    <div className="start-menu-container">
      <GameplayBackground />
      <div className="start-menu-hero">
        <img src="/logo.png" alt="AI Step Gen Logo" className="start-menu-logo" />
        <h1 className="start-menu-title">AI Step Gen Suite</h1>
        <p className="start-menu-subtitle">Production Hardened Step Generator</p>
      </div>

      <div className="start-menu-actions">
        <button
          className="btn-primary-pill"
          onClick={() => onNavigate("CREATE_SONG")}
        >
          <Sparkles className="icon-mr" size={16} />
          Create Song
        </button>

        <button
          className="btn-ghost-pill"
          onClick={() => onNavigate("IMPORT_EDIT_SONG")}
        >
          <FolderOpen className="icon-mr" size={16} />
          Import / Edit Song
        </button>

        <button
          className="btn-ghost-pill"
          onClick={() => onNavigate("SETTINGS")}
        >
          <Settings className="icon-mr" size={16} />
          Settings
        </button>

        {appMode === "dev" && (
          <button
            className="btn-ghost-pill btn-dev-indicator"
            onClick={() => onNavigate("DEV_TOOLS")}
          >
            <ShieldAlert className="icon-mr" size={16} />
            Developer Tools
          </button>
        )}
      </div>

      <div className="start-menu-footer">
        <span className="caption-text">StepF2/StepP1 Compatibility Layer • v0.1.0</span>
      </div>
    </div>
  );
};
