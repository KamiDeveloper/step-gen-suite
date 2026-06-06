import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { ISongDetails } from "../types/song";

export interface ISongProjectState {
  currentSong: ISongDetails | null;
  isLoading: boolean;
  error: string | null;
  hasApiKey: boolean;
  isApiKeyModalOpen: boolean;
  
  // Generation status
  generationStatus: "IDLE" | "IMPORTING" | "ANALYZING" | "GENERATING" | "VALIDATING" | "WRITING" | "SUCCESS" | "ERROR";
  generationProgress: number; // 0 to 100
}

export interface ISongProjectActions {
  setCurrentSong: (song: ISongDetails | null) => void;
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  setHasApiKey: (hasKey: boolean) => void;
  setApiKeyModalOpen: (isOpen: boolean) => void;
  setGenerationStatus: (status: ISongProjectState["generationStatus"]) => void;
  setGenerationProgress: (progress: number) => void;
  resetProject: () => void;
}

export const useSongProjectStore = create<ISongProjectState & ISongProjectActions>()(
  immer((set) => ({
    // Initial State
    currentSong: null,
    isLoading: false,
    error: null,
    hasApiKey: false,
    isApiKeyModalOpen: false,
    generationStatus: "IDLE",
    generationProgress: 0,

    // Actions
    setCurrentSong: (song) =>
      set((state) => {
        state.currentSong = song;
      }),
    setLoading: (isLoading) =>
      set((state) => {
        state.isLoading = isLoading;
      }),
    setError: (error) =>
      set((state) => {
        state.error = error;
      }),
    setHasApiKey: (hasKey) =>
      set((state) => {
        state.hasApiKey = hasKey;
      }),
    setApiKeyModalOpen: (isOpen) =>
      set((state) => {
        state.isApiKeyModalOpen = isOpen;
      }),
    setGenerationStatus: (status) =>
      set((state) => {
        state.generationStatus = status;
      }),
    setGenerationProgress: (progress) =>
      set((state) => {
        state.generationProgress = progress;
      }),
    resetProject: () =>
      set((state) => {
        state.currentSong = null;
        state.error = null;
        state.generationStatus = "IDLE";
        state.generationProgress = 0;
      }),
  }))
);

