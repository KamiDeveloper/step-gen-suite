import { useState, useEffect } from "react";
import { useSettingsStore } from "./store/settingsStore";
import { StartMenu } from "./screens/StartMenu";
import { CreateSongWizard } from "./screens/CreateSongWizard";
import { ImportEditSong } from "./screens/ImportEditSong";
import { SettingsScreen } from "./screens/SettingsScreen";
import { ProjectWorkspace } from "./screens/ProjectWorkspace";
import { DevToolsScreen } from "./screens/DevToolsScreen";
import { Navbar } from "./components/Navbar";
import "./App.css";

type ScreenType = "START_MENU" | "CREATE_SONG" | "IMPORT_EDIT_SONG" | "SETTINGS" | "WORKSPACE" | "DEV_TOOLS";

export default function App() {
  const [currentScreen, setCurrentScreen] = useState<ScreenType>("START_MENU");
  const { loadSettings, appMode } = useSettingsStore();

  useEffect(() => {
    // Load app settings and check API keys on startup
    loadSettings();
  }, [loadSettings]);

  const handleNavigate = (screen: string) => {
    if (screen === "DEV_TOOLS" && appMode !== "dev") {
      return;
    }
    setCurrentScreen(screen as ScreenType);
  };

  const renderActiveScreen = () => {
    switch (currentScreen) {
      case "START_MENU":
        return <StartMenu onNavigate={handleNavigate} />;
      case "CREATE_SONG":
        return <CreateSongWizard onNavigate={handleNavigate} />;
      case "IMPORT_EDIT_SONG":
        return <ImportEditSong onNavigate={handleNavigate} />;
      case "SETTINGS":
        return <SettingsScreen onNavigate={handleNavigate} />;
      case "WORKSPACE":
        return <ProjectWorkspace onNavigate={handleNavigate} />;
      case "DEV_TOOLS":
        return <DevToolsScreen onNavigate={handleNavigate} />;
      default:
        return <StartMenu onNavigate={handleNavigate} />;
    }
  };

  return (
    <div className="app-shell">
      {/* Editorial Navigation Bar */}
      <Navbar currentScreen={currentScreen} onNavigate={handleNavigate} />

      {/* Screen Container */}
      <main className="app-content-container">{renderActiveScreen()}</main>
    </div>
  );
}
