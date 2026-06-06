import { useEffect, useRef, useState } from "react";
import { BG_SONGS, IBGSongData } from "../utils/BGCharts";
import { GenerationBlurOverlay } from "./GenerationBlurOverlay";

interface ActiveNote {
  lane: number;
  hitTime: number;
  endTime?: number;
  playfield: number;
  type: "1" | "2";
  headHit?: boolean; // True when the head has crossed RECEPTOR_Y
  tailHit?: boolean; // True when the tail has crossed RECEPTOR_Y
}

interface ActiveExplosion {
  lane: number;
  hitTime: number;
  playfield: number;
  type: "1" | "2";
}

interface LoadedImages {
  dl: HTMLImageElement | HTMLCanvasElement;
  ul: HTMLImageElement | HTMLCanvasElement;
  c: HTMLImageElement | HTMLCanvasElement;
  ur: HTMLImageElement | HTMLCanvasElement;
  dr: HTMLImageElement | HTMLCanvasElement;
  basePlate: HTMLImageElement | HTMLCanvasElement;
  dlCap: HTMLImageElement | HTMLCanvasElement;
  ulCap: HTMLImageElement | HTMLCanvasElement;
  cCap: HTMLImageElement | HTMLCanvasElement;
  urCap: HTMLImageElement | HTMLCanvasElement;
  drCap: HTMLImageElement | HTMLCanvasElement;
  dlBody: HTMLImageElement | HTMLCanvasElement;
  ulBody: HTMLImageElement | HTMLCanvasElement;
  cBody: HTMLImageElement | HTMLCanvasElement;
  urBody: HTMLImageElement | HTMLCanvasElement;
  drBody: HTMLImageElement | HTMLCanvasElement;
  glow: HTMLImageElement | HTMLCanvasElement;
  explosion: HTMLImageElement | HTMLCanvasElement;
}

const RECEPTOR_Y = 0;
const LANE_SIZE = 74;
const NOTE_DRAW_SIZE = 96;

const removeBlackBackground = (img: HTMLImageElement): HTMLCanvasElement | HTMLImageElement => {
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalWidth;
  canvas.height = img.naturalHeight;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return img;

  ctx.drawImage(img, 0, 0);
  const imgData = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const data = imgData.data;

  for (let i = 0; i < data.length; i += 4) {
    const r = data[i];
    const g = data[i + 1];
    const b = data[i + 2];
    const lum = 0.299 * r + 0.587 * g + 0.114 * b;
    data[i + 3] = lum; // Set alpha to luminance (removes black entirely)
  }

  ctx.putImageData(imgData, 0, 0);
  return canvas;
};

const loadImage = (src: string, filterBlack: boolean = false): Promise<HTMLImageElement | HTMLCanvasElement> => {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.src = src;
    img.onload = () => {
      if (filterBlack) {
        resolve(removeBlackBackground(img));
      } else {
        resolve(img);
      }
    };
    img.onerror = (err) => reject(err);
  });
};

let lastSongId: string | null = null;

export function GameplayBackground() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animationFrameIdRef = useRef<number | null>(null);
  const notesRef = useRef<ActiveNote[]>([]);
  const explosionsRef = useRef<ActiveExplosion[]>([]);
  const lastNoteTimeRef = useRef<number>(performance.now());
  const imagesRef = useRef<LoadedImages | null>(null);
  const [assetsLoaded, setAssetsLoaded] = useState(false);

  const [reducedMotion, setReducedMotion] = useState(
    () => typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const listener = (e: MediaQueryListEvent) => {
      setReducedMotion(e.matches);
    };
    mediaQuery.addEventListener("change", listener);
    return () => mediaQuery.removeEventListener("change", listener);
  }, []);

  const currentSongRef = useRef<IBGSongData | null>(null);
  const scrollSpeedRef = useRef<number>(0.45);

  const appendSong = (song: IBGSongData) => {
    const rowDuration = (60000 / (song.bpm * song.subdivision)) * 3.0;

    const openHolds0: (ActiveNote | null)[] = [null, null, null, null, null];
    const openHolds1: (ActiveNote | null)[] = [null, null, null, null, null];

    const lowRows = song.charts.low.rows;
    for (let r = 0; r < lowRows.length; r++) {
      const row = lowRows[r];
      const hitTime = lastNoteTimeRef.current + r * rowDuration;
      for (let c = 0; c < 5; c++) {
        if (row[c] === "1") {
          notesRef.current.push({ lane: c, hitTime, playfield: 0, type: "1" });
        } else if (row[c] === "2") {
          const holdNode: ActiveNote = { lane: c, hitTime, playfield: 0, type: "2", endTime: hitTime };
          openHolds0[c] = holdNode;
          notesRef.current.push(holdNode);
        } else if (row[c] === "3") {
          if (openHolds0[c]) {
            openHolds0[c]!.endTime = hitTime;
            openHolds0[c] = null;
          }
        }
      }
    }

    const highRows = song.charts.high.rows;
    for (let r = 0; r < highRows.length; r++) {
      const row = highRows[r];
      const hitTime = lastNoteTimeRef.current + r * rowDuration;
      for (let c = 0; c < 5; c++) {
        if (row[c] === "1") {
          notesRef.current.push({ lane: c, hitTime, playfield: 1, type: "1" });
        } else if (row[c] === "2") {
          const holdNode: ActiveNote = { lane: c, hitTime, playfield: 1, type: "2", endTime: hitTime };
          openHolds1[c] = holdNode;
          notesRef.current.push(holdNode);
        } else if (row[c] === "3") {
          if (openHolds1[c]) {
            openHolds1[c]!.endTime = hitTime;
            openHolds1[c] = null;
          }
        }
      }
    }

    const maxRows = Math.max(lowRows.length, highRows.length);
    lastNoteTimeRef.current += maxRows * rowDuration;
  };

  useEffect(() => {
    Promise.all([
      loadImage("/noteskins/z3phoenix/DownLeft Tap Note (res 192x128) 3x2.png"),
      loadImage("/noteskins/z3phoenix/UpLeft Tap Note (res 192x128) 3x2.png"),
      loadImage("/noteskins/z3phoenix/Center Tap Note (res 192x128) 3x2.png"),
      loadImage("/noteskins/z3phoenix/UpRight Tap Note (res 192x128) 3x2.png"),
      loadImage("/noteskins/z3phoenix/DownRight Tap Note (res 192x128) 3x2.png"),
      loadImage("/noteskins/z3phoenix/BASE 1x2 - (res 320x128).png"),
      loadImage("/noteskins/z3phoenix/DownLeft Hold BottomCap Active (res 384x64) 6x1.png"),
      loadImage("/noteskins/z3phoenix/UpLeft Hold BottomCap Active (res 384x64) 6x1.png"),
      loadImage("/noteskins/z3phoenix/Center Hold BottomCap Active (res 384x64) 6x1.png"),
      loadImage("/noteskins/z3phoenix/UpRight Hold BottomCap Active (res 384x64) 6x1.png"),
      loadImage("/noteskins/z3phoenix/DownRight Hold BottomCap Active (res 384x64) 6x1.png"),
      loadImage("/noteskins/z3phoenix/DownLeft Hold Body Active (res 64x64).png"),
      loadImage("/noteskins/z3phoenix/UpLeft Hold Body Active (res 64x64).png"),
      loadImage("/noteskins/z3phoenix/Center Hold Body Active (res 64x64).png"),
      loadImage("/noteskins/z3phoenix/UpRight Hold Body Active (res 64x64).png"),
      loadImage("/noteskins/z3phoenix/DownRight Hold Body Active (res 64x64).png"),
      loadImage("/noteskins/z3phoenix/GLOW 5x2 (res 320x128).PNG", true), // Apply luminance filter
      loadImage("/noteskins/z3phoenix/_explosion 6x1.png", true) // Apply luminance filter
    ]).then((loaded) => {
      imagesRef.current = {
        dl: loaded[0], ul: loaded[1], c: loaded[2], ur: loaded[3], dr: loaded[4],
        basePlate: loaded[5],
        dlCap: loaded[6], ulCap: loaded[7], cCap: loaded[8], urCap: loaded[9], drCap: loaded[10],
        dlBody: loaded[11], ulBody: loaded[12], cBody: loaded[13], urBody: loaded[14], drBody: loaded[15],
        glow: loaded[16],
        explosion: loaded[17]
      };
      setAssetsLoaded(true);
    }).catch((err) => console.error("Critical error loading NoteSkin assets:", err));
  }, []);

  useEffect(() => {
    if (!assetsLoaded || !canvasRef.current) return;

    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // Pick a different song from the last one played on returning to the StartMenu
    const availableSongs = BG_SONGS.filter(s => s.id !== lastSongId);
    const selectedSong = availableSongs.length > 0
      ? availableSongs[Math.floor(Math.random() * availableSongs.length)]
      : BG_SONGS[Math.floor(Math.random() * BG_SONGS.length)];

    lastSongId = selectedSong.id;
    currentSongRef.current = selectedSong;

    // Calculate dynamic scroll speed using formula: AV 680 (Velocity = 680 / BPM)
    // AV = 680 / BPM is the multiplier (e.g. 3.4x for 200 BPM)
    // Scroll speed in pixels per millisecond = (BPM / 60) * AV * BASE_FACTOR
    // To match current ~0.45 px/ms baseline speed, we use a BASE_FACTOR of 0.04
    const speedMultiplier = 680 / selectedSong.bpm;
    const baseFactor = 0.04;
    const beatsPerSecond = selectedSong.bpm / 60;
    scrollSpeedRef.current = beatsPerSecond * speedMultiplier * baseFactor;

    lastNoteTimeRef.current = performance.now() + 200;
    for (let i = 0; i < 3; i++) {
      appendSong(selectedSong);
    }

    const update = (timestamp: number) => {
      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;

      if (canvas.width !== Math.floor(rect.width * dpr) || canvas.height !== Math.floor(rect.height * dpr)) {
        canvas.width = Math.floor(rect.width * dpr);
        canvas.height = Math.floor(rect.height * dpr);
      }

      ctx.save();
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, rect.width, rect.height);

      let scale = 1.0;
      if (rect.width >= 1400) {
        scale = 1.25;
      }

      if (reducedMotion) {
        // Draw static base plates only
        const BASE_WIDTH = 480;
        const BASE_HEIGHT = 96;
        const actualPlayfieldWidth = BASE_WIDTH * scale;

        const drawStaticPlayfield = (offsetX: number) => {
          if (imagesRef.current?.basePlate) {
            ctx.drawImage(
              imagesRef.current.basePlate,
              0,
              0,
              BASE_WIDTH,
              BASE_HEIGHT,
              Math.floor(offsetX),
              Math.floor(RECEPTOR_Y),
              Math.floor(BASE_WIDTH * scale),
              Math.floor(BASE_HEIGHT * scale)
            );
          }
        };

        if (rect.width >= 950) {
          const gap = 80 * scale;
          const totalContainerWidth = (actualPlayfieldWidth * 2) + gap;
          const startX = (rect.width - totalContainerWidth) / 2;
          drawStaticPlayfield(startX);
          drawStaticPlayfield(startX + actualPlayfieldWidth + gap);
        } else {
          const startX = (rect.width - actualPlayfieldWidth) / 2;
          drawStaticPlayfield(startX);
        }

        ctx.restore();
        // Pause the animation loop
        return;
      }

      if (notesRef.current.length < 40 && currentSongRef.current) {
        appendSong(currentSongRef.current);
      }

      // Hit Detection Loop (Autoplay attract mode)
      for (const note of notesRef.current) {
        if (!note.headHit && timestamp >= note.hitTime) {
          note.headHit = true;
          // Spawn an explosion for tap notes and hold heads
          explosionsRef.current.push({
            lane: note.lane,
            hitTime: note.hitTime,
            playfield: note.playfield,
            type: note.type
          });
        }

        if (note.type === "2" && note.headHit && !note.tailHit && note.endTime && timestamp >= note.endTime) {
          note.tailHit = true;
          // Spawn an explosion for hold bottomcaps (tails)
          explosionsRef.current.push({
            lane: note.lane,
            hitTime: note.endTime,
            playfield: note.playfield,
            type: "2"
          });
        }
      }

      // Garbage Collection for Notes
      notesRef.current = notesRef.current.filter((note) => {
        if (note.type === "1" && note.headHit) return false; // Purge hit tap notes immediately
        if (note.type === "2" && note.tailHit) return false; // Purge hold notes immediately once tail is hit
        if (note.type === "2") {
          const tailY = RECEPTOR_Y + ((note.endTime || note.hitTime) - timestamp) * scrollSpeedRef.current;
          // Purge once the cap has fully reached the receptor
          return tailY > RECEPTOR_Y - 50;
        }
        return true;
      });

      // Garbage Collection for Explosions (Purge after 360ms)
      explosionsRef.current = explosionsRef.current.filter((exp) => {
        return timestamp - exp.hitTime <= 360;
      });

      scale = 1.0;
      if (rect.width >= 1400) {
        scale = 1.25;
      }

      const BASE_WIDTH = 480;
      const BASE_HEIGHT = 96;
      const actualPlayfieldWidth = BASE_WIDTH * scale;

      const drawPlayfield = (offsetX: number, playfieldIndex: number) => {
        const playfieldCenterX = offsetX + (BASE_WIDTH * scale) / 2;

        // ----------------------------------------------------
        // LAYER 0 & 1: BASE PLATE & RECEPTOR BEAT GLOW
        // ----------------------------------------------------
        if (imagesRef.current?.basePlate) {
          ctx.drawImage(
            imagesRef.current.basePlate,
            0,
            0,
            BASE_WIDTH,
            BASE_HEIGHT,
            Math.floor(offsetX),
            Math.floor(RECEPTOR_Y),
            Math.floor(BASE_WIDTH * scale),
            Math.floor(BASE_HEIGHT * scale)
          );

          ctx.save();
          ctx.globalCompositeOperation = "lighter";
          const pulseAlpha = (Math.sin(timestamp / 150) + 1) / 2 * 0.6;
          ctx.globalAlpha = Math.max(0, Math.min(1, pulseAlpha));
          ctx.drawImage(
            imagesRef.current.basePlate,
            0,
            96,
            BASE_WIDTH,
            BASE_HEIGHT,
            Math.floor(offsetX),
            Math.floor(RECEPTOR_Y),
            Math.floor(BASE_WIDTH * scale),
            Math.floor(BASE_HEIGHT * scale)
          );
          ctx.restore();
        }

        // ----------------------------------------------------
        // LAYER 2 & 3: NOTES (BODIES, CAPS, UNHIT HEADS)
        // ----------------------------------------------------
        const totalCycleTime = 250;
        const frameIndex = Math.floor((timestamp / (totalCycleTime / 6))) % 6;
        const sx = Math.floor(frameIndex % 3) * 96;
        const sy = Math.floor(frameIndex / 3) * 96;

        for (const note of notesRef.current) {
          if (note.playfield !== playfieldIndex) continue;

          const timeDiff = note.hitTime - timestamp;
          const currentY = RECEPTOR_Y + timeDiff * scrollSpeedRef.current;
          const tailY = note.type === "2" ? RECEPTOR_Y + ((note.endTime || note.hitTime) - timestamp) * scrollSpeedRef.current : currentY;

          if (tailY > rect.height + 150) continue;

          let img: HTMLImageElement | HTMLCanvasElement | undefined;
          let capImg: HTMLImageElement | HTMLCanvasElement | undefined;
          let bodyImg: HTMLImageElement | HTMLCanvasElement | undefined;

          if (imagesRef.current) {
            if (note.lane === 0) { img = imagesRef.current.dl; capImg = imagesRef.current.dlCap; bodyImg = imagesRef.current.dlBody; }
            if (note.lane === 1) { img = imagesRef.current.ul; capImg = imagesRef.current.ulCap; bodyImg = imagesRef.current.ulBody; }
            if (note.lane === 2) { img = imagesRef.current.c; capImg = imagesRef.current.cCap; bodyImg = imagesRef.current.cBody; }
            if (note.lane === 3) { img = imagesRef.current.ur; capImg = imagesRef.current.urCap; bodyImg = imagesRef.current.urBody; }
            if (note.lane === 4) { img = imagesRef.current.dr; capImg = imagesRef.current.drCap; bodyImg = imagesRef.current.drBody; }
          }

          const laneCenterX = playfieldCenterX + (note.lane - 2) * LANE_SIZE * scale;
          const drawX = laneCenterX - (NOTE_DRAW_SIZE * scale) / 2;

          if (note.type === "2" && bodyImg && capImg) {
            const headY = note.headHit ? RECEPTOR_Y : currentY;
            const bodyStartY = headY + (NOTE_DRAW_SIZE * scale) / 2;
            const drawLength = tailY - bodyStartY;

            // Consume body as it scrolls
            if (drawLength > 0) {
              let bImg = bodyImg as HTMLImageElement;
              const bRawW = bImg.naturalWidth || 96;
              const bRawH = bImg.naturalHeight || 96;
              ctx.drawImage(
                bImg,
                0,
                0,
                bRawW,
                bRawH,
                Math.floor(drawX),
                Math.floor(bodyStartY),
                Math.floor(NOTE_DRAW_SIZE * scale),
                Math.floor(drawLength)
              );
            }

            const cImg = capImg as HTMLImageElement;
            const capRawW = cImg.naturalWidth / 6 || 256;
            const capRawH = cImg.naturalHeight || 256;
            const capSxDynamic = frameIndex * capRawW;
            ctx.drawImage(
              cImg,
              capSxDynamic,
              0,
              capRawW,
              capRawH,
              Math.floor(drawX),
              Math.floor(tailY),
              Math.floor(NOTE_DRAW_SIZE * scale),
              Math.floor(NOTE_DRAW_SIZE * scale)
            );
          }

          // Layer 3: Note Head
          if (img) {
            if (note.type === "1" && !note.headHit) {
              ctx.drawImage(
                img,
                sx, sy, NOTE_DRAW_SIZE, NOTE_DRAW_SIZE,
                Math.floor(drawX), Math.floor(currentY), Math.floor(NOTE_DRAW_SIZE * scale), Math.floor(NOTE_DRAW_SIZE * scale)
              );
            } else if (note.type === "2") {
              const headY = note.headHit ? RECEPTOR_Y : currentY;
              ctx.drawImage(
                img,
                sx, sy, NOTE_DRAW_SIZE, NOTE_DRAW_SIZE,
                Math.floor(drawX), Math.floor(headY), Math.floor(NOTE_DRAW_SIZE * scale), Math.floor(NOTE_DRAW_SIZE * scale)
              );
            }
          }
        }

        // ----------------------------------------------------
        // LAYER 4: EXPLOSIONS (ADDITIVE BLEND)
        // ----------------------------------------------------
        for (const exp of explosionsRef.current) {
          if (exp.playfield !== playfieldIndex) continue;

          const laneCenterX = playfieldCenterX + (exp.lane - 2) * LANE_SIZE * scale;
          const age = timestamp - exp.hitTime;
          if (age < 0) continue;

          // Phantom Note Fade & Zoom (1.0 -> 1.2, 300ms)
          if (age <= 300 && imagesRef.current) {
            let img: HTMLImageElement | HTMLCanvasElement | undefined;
            if (exp.lane === 0) img = imagesRef.current.dl;
            if (exp.lane === 1) img = imagesRef.current.ul;
            if (exp.lane === 2) img = imagesRef.current.c;
            if (exp.lane === 3) img = imagesRef.current.ur;
            if (exp.lane === 4) img = imagesRef.current.dr;

            if (img) {
              const pZoom = 1.0 + (age / 300) * 0.2;
              const pAlpha = 1.0 - (age / 300);
              const pSize = NOTE_DRAW_SIZE * scale * pZoom;
              const pX = laneCenterX - pSize / 2;
              const pY = RECEPTOR_Y + (NOTE_DRAW_SIZE * scale) / 2 - pSize / 2;

              ctx.save();
              ctx.globalCompositeOperation = "lighter";
              ctx.globalAlpha = Math.max(0, Math.min(1, pAlpha));
              ctx.drawImage(
                img,
                sx, sy, NOTE_DRAW_SIZE, NOTE_DRAW_SIZE,
                Math.floor(pX), Math.floor(pY), Math.floor(pSize), Math.floor(pSize)
              );
              ctx.restore();
            }
          }

          // GLOW 5x2 Flare
          if (age <= 200 && imagesRef.current?.glow) {
            const glowZoom = 0.9 + (age / 200) * 0.25;
            const glowAlpha = 1.0 - (age / 200);
            const glSx = exp.lane * 64;
            const glSizeRender = 128 * scale * glowZoom;
            const glX = laneCenterX - glSizeRender / 2;
            const glY = RECEPTOR_Y + (NOTE_DRAW_SIZE * scale) / 2 - glSizeRender / 2;

            ctx.save();
            ctx.globalCompositeOperation = "lighter";
            ctx.globalAlpha = Math.max(0, Math.min(1, glowAlpha));
            ctx.drawImage(
              imagesRef.current.glow,
              glSx, 0, 64, 64,
              Math.floor(glX), Math.floor(glY), Math.floor(glSizeRender), Math.floor(glSizeRender)
            );
            ctx.restore();
          }

          // _explosion 6x1 Frame Animation
          if (age <= 360 && imagesRef.current?.explosion) {
            const expFrame = Math.min(5, Math.floor(age / 60));
            const cImg = imagesRef.current.explosion as HTMLCanvasElement;
            const rawW = cImg.width / 6;
            const rawH = cImg.height;
            const exSx = expFrame * rawW;
            const exSize = 140 * scale;
            const exX = laneCenterX - exSize / 2;
            const exY = RECEPTOR_Y + (NOTE_DRAW_SIZE * scale) / 2 - exSize / 2;

            ctx.save();
            ctx.globalCompositeOperation = "lighter";
            ctx.drawImage(
              cImg,
              exSx, 0, rawW, rawH,
              Math.floor(exX), Math.floor(exY), Math.floor(exSize), Math.floor(exSize)
            );
            ctx.restore();
          }
        }
      };

      if (rect.width >= 950) {
        const gap = 80 * scale;
        const totalContainerWidth = (actualPlayfieldWidth * 2) + gap;
        const startX = (rect.width - totalContainerWidth) / 2;
        drawPlayfield(startX, 0);
        drawPlayfield(startX + actualPlayfieldWidth + gap, 1);
      } else {
        const startX = (rect.width - actualPlayfieldWidth) / 2;
        drawPlayfield(startX, 1);
      }

      ctx.restore();
      animationFrameIdRef.current = requestAnimationFrame(update);
    };

    animationFrameIdRef.current = requestAnimationFrame(update);

    return () => {
      if (animationFrameIdRef.current) cancelAnimationFrame(animationFrameIdRef.current);
    };
  }, [assetsLoaded, reducedMotion]);

  if (!assetsLoaded) return null;

  return (
    <>
      <canvas
        ref={canvasRef}
        className="gameplay-background-canvas"
      />
      <GenerationBlurOverlay variant="gameplay" />
    </>
  );
}

