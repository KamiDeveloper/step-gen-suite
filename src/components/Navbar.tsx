import { useState, useEffect } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Home, Sparkles, FolderOpen, Settings, ShieldAlert, Minus, Square, Copy, X } from "lucide-react";

interface NavbarProps {
  currentScreen: string;
  onNavigate: (screen: string) => void;
}

export function Navbar({ currentScreen, onNavigate }: NavbarProps) {
  const { appMode } = useSettingsStore();
  const [isMaximized, setIsMaximized] = useState(false);
  const [inTauri, setInTauri] = useState(false);

  useEffect(() => {
    // Check if we are running inside the Tauri shell
    const checkTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ !== undefined;
    setInTauri(checkTauri);

    if (checkTauri) {
      const appWindow = getCurrentWindow();
      
      const updateMaximized = async () => {
        try {
          const maximized = await appWindow.isMaximized();
          setIsMaximized(maximized);
        } catch (error) {
          console.error("Failed to check maximized state:", error);
        }
      };

      // Initial check
      updateMaximized();

      // Listen to window resizing to update maximized state dynamically
      window.addEventListener("resize", updateMaximized);

      return () => {
        window.removeEventListener("resize", updateMaximized);
      };
    }
  }, []);

  const handleMinimize = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (inTauri) {
      try {
        await getCurrentWindow().minimize();
      } catch (error) {
        console.error("Failed to minimize window:", error);
      }
    }
  };

  const handleToggleMaximize = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (inTauri) {
      try {
        await getCurrentWindow().toggleMaximize();
      } catch (error) {
        console.error("Failed to toggle maximize window:", error);
      }
    }
  };

  const handleClose = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (inTauri) {
      try {
        await getCurrentWindow().close();
      } catch (error) {
        console.error("Failed to close window:", error);
      }
    }
  };

  // Prevent dragging when interacting with elements inside the navbar
  const handlePreventDrag = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  return (
    <header className="app-navbar" data-tauri-drag-region>
      <div 
        className="navbar-left" 
        onClick={() => onNavigate("START_MENU")}
        onMouseDown={handlePreventDrag}
      >
        <img src="/logo.png" alt="Logo" className="navbar-logo-img" />
        <span className="navbar-brand">AI STEP GEN</span>
      </div>

      <nav className="navbar-center" onMouseDown={handlePreventDrag}>
        <button
          className={`navbar-link ${currentScreen === "START_MENU" ? "active" : ""}`}
          onClick={() => onNavigate("START_MENU")}
        >
          <Home size={14} className="icon-mr" />
          Home
        </button>
        <button
          className={`navbar-link ${currentScreen === "CREATE_SONG" ? "active" : ""}`}
          onClick={() => onNavigate("CREATE_SONG")}
        >
          <Sparkles size={14} className="icon-mr" />
          Create
        </button>
        <button
          className={`navbar-link ${currentScreen === "IMPORT_EDIT_SONG" ? "active" : ""}`}
          onClick={() => onNavigate("IMPORT_EDIT_SONG")}
        >
          <FolderOpen size={14} className="icon-mr" />
          Import
        </button>
        <button
          className={`navbar-link ${currentScreen === "WORKSPACE" ? "active" : ""}`}
          onClick={() => onNavigate("WORKSPACE")}
        >
          Workspace
        </button>
      </nav>

      <div className="navbar-right" onMouseDown={handlePreventDrag}>
        <button
          className={`navbar-link ${currentScreen === "SETTINGS" ? "active" : ""}`}
          onClick={() => onNavigate("SETTINGS")}
        >
          <Settings size={14} className="icon-mr" />
          Settings
        </button>

        {appMode === "dev" && (
          <button
            className={`navbar-link navbar-dev-link ${currentScreen === "DEV_TOOLS" ? "active" : ""}`}
            onClick={() => onNavigate("DEV_TOOLS")}
          >
            <ShieldAlert size={14} className="icon-mr" />
            Dev Tools
          </button>
        )}

        {inTauri && (
          <div className="window-controls">
            <button 
              className="window-control-btn" 
              onClick={handleMinimize}
              title="Minimizar"
            >
              <Minus size={14} />
            </button>
            <button 
              className="window-control-btn" 
              onClick={handleToggleMaximize}
              title={isMaximized ? "Restaurar" : "Maximizar"}
            >
              {isMaximized ? <Copy size={12} /> : <Square size={12} />}
            </button>
            <button 
              className="window-control-btn window-control-close" 
              onClick={handleClose}
              title="Cerrar"
            >
              <X size={14} />
            </button>
          </div>
        )}
      </div>
    </header>
  );
}
