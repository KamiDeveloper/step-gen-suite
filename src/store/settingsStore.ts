import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { invoke } from "@tauri-apps/api/core";

export interface IAppSettings {
  songs_dir: string | null;
  songpack_mode: string;
  default_songpack_folder: string;
  default_author: string | null;
  default_play_mode: string | null;
  default_meter: number | null;
}

export interface ISettingsState {
  settings: IAppSettings;
  appMode: "dev" | "prod";
  hasApiKey: boolean;
  isLoading: boolean;
  error: string | null;
}

export interface ISettingsActions {
  loadSettings: () => Promise<void>;
  saveSettings: (settings: IAppSettings) => Promise<void>;
  setSongsDir: (dir: string | null) => void;
  setSongpackMode: (mode: string) => void;
  setDefaultSongpackFolder: (folder: string) => void;
  setDefaultAuthor: (author: string | null) => void;
  setDefaultPlayMode: (playMode: string | null) => void;
  setDefaultMeter: (meter: number | null) => void;
  checkApiKey: () => Promise<void>;
  saveApiKey: (apiKey: string, passphrase: string) => Promise<void>;
  deleteApiKey: () => Promise<void>;
  ensureSongpack: () => Promise<string>;
}

export const useSettingsStore = create<ISettingsState & ISettingsActions>()(
  immer((set) => ({
    settings: {
      songs_dir: null,
      songpack_mode: "managed_default",
      default_songpack_folder: "99-AI-Step-Gen",
      default_author: "AI Step Gen",
      default_play_mode: "Single",
      default_meter: 10,
    },
    appMode: "prod",
    hasApiKey: false,
    isLoading: false,
    error: null,

    loadSettings: async () => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const settings = await invoke<IAppSettings>("get_settings");
        const appMode = await invoke<string>("get_app_mode");
        const hasApiKey = await invoke<boolean>("has_gemini_api_key");
        set((state) => {
          state.settings = settings;
          state.appMode = appMode as "dev" | "prod";
          state.hasApiKey = hasApiKey;
        });
      } catch (err: any) {
        set((state) => {
          state.error = err.toString();
        });
      } finally {
        set((state) => {
          state.isLoading = false;
        });
      }
    },

    saveSettings: async (settings) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await invoke("save_settings", { settings });
        set((state) => {
          state.settings = settings;
        });
      } catch (err: any) {
        set((state) => {
          state.error = err.toString();
        });
        throw err;
      } finally {
        set((state) => {
          state.isLoading = false;
        });
      }
    },

    setSongsDir: (dir) => {
      set((state) => {
        state.settings.songs_dir = dir;
      });
    },

    setSongpackMode: (mode) => {
      set((state) => {
        state.settings.songpack_mode = mode;
      });
    },

    setDefaultSongpackFolder: (folder) => {
      set((state) => {
        state.settings.default_songpack_folder = folder;
      });
    },

    setDefaultAuthor: (author) => {
      set((state) => {
        state.settings.default_author = author;
      });
    },

    setDefaultPlayMode: (playMode) => {
      set((state) => {
        state.settings.default_play_mode = playMode;
      });
    },

    setDefaultMeter: (meter) => {
      set((state) => {
        state.settings.default_meter = meter;
      });
    },

    checkApiKey: async () => {
      try {
        const hasApiKey = await invoke<boolean>("has_gemini_api_key");
        set((state) => {
          state.hasApiKey = hasApiKey;
        });
      } catch (err: any) {
        console.error(err);
      }
    },

    saveApiKey: async (apiKey, passphrase) => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await invoke("save_gemini_api_key", { apiKey, passphrase });
        set((state) => {
          state.hasApiKey = true;
        });
      } catch (err: any) {
        set((state) => {
          state.error = err.toString();
        });
        throw err;
      } finally {
        set((state) => {
          state.isLoading = false;
        });
      }
    },

    deleteApiKey: async () => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        await invoke("delete_gemini_api_key");
        set((state) => {
          state.hasApiKey = false;
        });
      } catch (err: any) {
        set((state) => {
          state.error = err.toString();
        });
        throw err;
      } finally {
        set((state) => {
          state.isLoading = false;
        });
      }
    },

    ensureSongpack: async () => {
      set((state) => {
        state.isLoading = true;
        state.error = null;
      });
      try {
        const songpackPath = await invoke<string>("ensure_default_songpack");
        return songpackPath;
      } catch (err: any) {
        set((state) => {
          state.error = err.toString();
        });
        throw err;
      } finally {
        set((state) => {
          state.isLoading = false;
        });
      }
    },
  }))
);
