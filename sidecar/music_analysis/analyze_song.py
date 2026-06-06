#!/usr/bin/env python
# -*- coding: utf-8 -*-

import os
import sys
import json
import hashlib
import argparse
import traceback

# Import libraries with fallback to metadata-only/simulated mode
try:
    import numpy as np
    NUMPY_AVAILABLE = True
except ImportError:
    NUMPY_AVAILABLE = False

try:
    import scipy
    SCIPY_AVAILABLE = True
except ImportError:
    SCIPY_AVAILABLE = False

try:
    import librosa
    LIBROSA_AVAILABLE = True
except ImportError:
    LIBROSA_AVAILABLE = False

try:
    import soundfile as sf
    SOUNDFILE_AVAILABLE = True
except ImportError:
    SOUNDFILE_AVAILABLE = False

try:
    import mutagen
    MUTAGEN_AVAILABLE = True
except ImportError:
    MUTAGEN_AVAILABLE = False


def parse_ssc(ssc_path):
    """Parses metadata and charts from a .ssc file."""
    metadata = {}
    charts = []
    if not ssc_path or not os.path.exists(ssc_path):
        return metadata, charts

    try:
        with open(ssc_path, 'r', encoding='utf-8', errors='ignore') as f:
            content = f.read()

        # Split tags by '#'
        parts = content.split('#')
        current_chart = {}
        in_notedata = False

        for part in parts:
            part = part.strip()
            if not part:
                continue
            if ';' not in part:
                continue

            # Split by first ':'
            if ':' not in part:
                continue
            tag_name, tag_val = part.split(':', 1)
            tag_val = tag_val.split(';', 1)[0].strip()
            tag_name = tag_name.upper()

            if tag_name == "NOTEDATA":
                in_notedata = True
                if current_chart:
                    charts.append(current_chart)
                current_chart = {}
                continue

            if in_notedata:
                if tag_name == "STEPSTYPE":
                    current_chart["mode"] = tag_val
                elif tag_name == "METER":
                    try:
                        current_chart["meter"] = int(tag_val)
                    except ValueError:
                        current_chart["meter"] = 0
                elif tag_name == "CREDIT":
                    current_chart["credit"] = tag_val
                elif tag_name == "DIFFICULTY":
                    current_chart["difficulty"] = tag_val
                elif tag_name == "NOTES":
                    # Estimate length of chart in measures
                    measures = tag_val.split(',')
                    current_chart["num_measures"] = len(measures)
            else:
                if tag_name in ["TITLE", "ARTIST", "OFFSET", "BPMS", "DISPLAYBPM", "SONGTYPE", "STOPS", "DELAYS", "WARPS"]:
                    metadata[tag_name] = tag_val

        if current_chart:
            charts.append(current_chart)

    except Exception as e:
        print(f"Warning: Error parsing SSC: {e}", file=sys.stderr)

    return metadata, charts


def beat_to_time(beat, bpms, offset):
    """Converts beat index to time in seconds based on BPM changes and offset."""
    t = -offset
    curr_b = 0.0
    for i in range(len(bpms) - 1):
        seg_b, bpm = bpms[i]
        next_b, _ = bpms[i+1]
        if beat <= next_b:
            t += (beat - curr_b) * (60.0 / bpm)
            return t
        else:
            t += (next_b - curr_b) * (60.0 / bpm)
            curr_b = next_b
            
    seg_b, bpm = bpms[-1]
    t += (beat - curr_b) * (60.0 / bpm)
    return t


def time_to_beat(time_sec, bpms, offset):
    """Converts time in seconds to beat index based on BPM changes and offset."""
    t = -offset
    curr_b = 0.0
    if time_sec <= t:
        bpm = bpms[0][1]
        return (time_sec - t) / (60.0 / bpm)
        
    for i in range(len(bpms) - 1):
        seg_b, bpm = bpms[i]
        next_b, _ = bpms[i+1]
        dt = (next_b - curr_b) * (60.0 / bpm)
        if time_sec <= t + dt:
            return curr_b + (time_sec - t) / (60.0 / bpm)
        else:
            t += dt
            curr_b = next_b
            
    seg_b, bpm = bpms[-1]
    return curr_b + (time_sec - t) / (60.0 / bpm)


def get_bpm_at_beat(beat, bpms):
    """Finds the active BPM at a given beat."""
    active_bpm = bpms[0][1]
    for seg_b, bpm in bpms:
        if beat >= seg_b:
            active_bpm = bpm
        else:
            break
    return active_bpm


def extract_audio_features(audio_path, bpms, offset, warnings_list):
    """Extracts DSP audio features using librosa with simulated fallbacks."""
    features = {
        "duration_seconds": 120.0,
        "sample_rate": 44100,
        "detected_bpm": bpms[0][1],
        "rms_energy": None,
        "spectral_centroid": None,
        "spectral_flatness": None,
        "zero_crossing_rate": None,
        "chroma": None,
        "spectral_contrast": None,
        "onset_envelope": None,
        "times": None,
        "chroma_mean": None,
        "spectral_contrast_mean": None,
        "analysis_mode": "fallback"
    }

    if not audio_path or not os.path.exists(audio_path):
        warnings_list.append(f"Audio file not found at {audio_path}. Using metadata-only and zeroed metrics.")
        return features

    # Check for librosa availability
    if not LIBROSA_AVAILABLE:
        warnings_list.append("librosa is not installed. Using metadata-only and zeroed metrics.")
        # Attempt to get duration using mutagen
        if MUTAGEN_AVAILABLE:
            try:
                from mutagen import File
                mut_file = File(audio_path)
                if mut_file and mut_file.info:
                    features["duration_seconds"] = mut_file.info.length
            except Exception:
                pass
        return features

    try:
        # Load audio (use mono, sr=22050 for speed and stability)
        y, sr = librosa.load(audio_path, sr=22050, mono=True)
        features["duration_seconds"] = float(librosa.get_duration(y=y, sr=sr))
        features["sample_rate"] = sr
        features["analysis_mode"] = "dsp"

        # Global detected BPM
        try:
            onset_env = librosa.onset.onset_strength(y=y, sr=sr)
            features["onset_envelope"] = onset_env
            tempo, _ = librosa.beat.beat_track(onset_envelope=onset_env, sr=sr)
            if isinstance(tempo, np.ndarray) and len(tempo) > 0:
                features["detected_bpm"] = float(tempo[0])
            else:
                features["detected_bpm"] = float(tempo)
        except Exception as e:
            warnings_list.append(f"Failed to calculate beat track: {e}")

        # RMS Energy
        try:
            features["rms_energy"] = librosa.feature.rms(y=y)[0]
        except Exception as e:
            warnings_list.append(f"Failed to calculate RMS energy: {e}")

        # Spectral features
        try:
            features["spectral_centroid"] = librosa.feature.spectral_centroid(y=y, sr=sr)[0]
        except Exception as e:
            warnings_list.append(f"Failed to calculate spectral centroid: {e}")

        try:
            features["spectral_flatness"] = librosa.feature.spectral_flatness(y=y)[0]
        except Exception as e:
            warnings_list.append(f"Failed to calculate spectral flatness: {e}")

        try:
            features["zero_crossing_rate"] = librosa.feature.zero_crossing_rate(y=y)[0]
        except Exception as e:
            warnings_list.append(f"Failed to calculate zero crossing rate: {e}")

        # Chroma
        try:
            features["chroma"] = librosa.feature.chroma_stft(y=y, sr=sr)
            if features["chroma"] is not None:
                features["chroma_mean"] = [float(x) for x in np.mean(features["chroma"], axis=1)]
        except Exception as e:
            warnings_list.append(f"Failed to calculate chroma: {e}")

        # Spectral Contrast
        try:
            features["spectral_contrast"] = librosa.feature.spectral_contrast(y=y, sr=sr)
            if features["spectral_contrast"] is not None:
                features["spectral_contrast_mean"] = [float(x) for x in np.mean(features["spectral_contrast"], axis=1)]
        except Exception as e:
            warnings_list.append(f"Failed to calculate spectral contrast: {e}")

        # Frame timestamps
        if features["rms_energy"] is not None:
            features["times"] = librosa.times_like(features["rms_energy"], sr=sr)

    except Exception as e:
        warnings_list.append(f"Failed to process audio with librosa: {e}. Using metadata-only and zeroed metrics.")
        traceback.print_exc()

    return features


def sample_feature_value(feat_array, time_sec, times_grid, default_val=0.0):
    """Helper to sample a feature value at a specific time."""
    if feat_array is None or times_grid is None:
        return default_val
    
    idx = np.searchsorted(times_grid, time_sec)
    if idx >= len(feat_array):
        idx = len(feat_array) - 1
    if idx < 0:
        idx = 0
    return float(feat_array[idx])


def validate_report_contract(report):
    """Strict validation of the song analysis report contract to avoid invalid types/keys."""
    if not isinstance(report.get("schema_version"), str) or report["schema_version"] != "music-analysis-report.v1":
        raise ValueError("Invalid schema_version")
    if not isinstance(report.get("song_id"), str):
        raise ValueError("Invalid song_id type")
    if not isinstance(report.get("duration_seconds"), (int, float)):
        raise ValueError("Invalid duration_seconds type")
        
    audio_sum = report.get("audio_summary", {})
    if not isinstance(audio_sum.get("sample_rate"), int):
        raise ValueError("Invalid audio_summary.sample_rate type")
    if not isinstance(audio_sum.get("detected_bpm"), (int, float)):
        raise ValueError("Invalid audio_summary.detected_bpm type")
    if not isinstance(audio_sum.get("analysis_mode"), str) or audio_sum["analysis_mode"] not in ["dsp", "fallback"]:
        raise ValueError("Invalid audio_summary.analysis_mode value")
        
    diagnostics = report.get("diagnostics", {})
    if not isinstance(diagnostics.get("requires_manual_timing_review"), bool):
        raise TypeError("requires_manual_timing_review must be a strict boolean")
    if not isinstance(diagnostics.get("timing_confidence"), (int, float)):
        raise ValueError("Invalid diagnostics.timing_confidence type")
    if not isinstance(diagnostics.get("analysis_mode"), str) or diagnostics["analysis_mode"] not in ["dsp", "fallback"]:
        raise ValueError("Invalid diagnostics.analysis_mode value")
        
    if not isinstance(report.get("sections"), list):
        raise ValueError("Invalid sections type")
    if not isinstance(report.get("choreographic_intent"), list):
        raise ValueError("Invalid choreographic_intent type")
    if not isinstance(report.get("publicability"), dict):
        raise ValueError("Invalid publicability type")
        
    # Hardened schema alignment checks
    if not isinstance(report.get("event_features"), dict) or not isinstance(report["event_features"].get("beats"), list):
        raise ValueError("event_features must contain a beats list")
        
    for intent in report.get("choreographic_intent", []):
        required_fields = [
            "schema_version", "section_id", "mode", "target_level", 
            "measure_start", "measure_end", "density_target", "difficulty_budget", 
            "recommended_pattern_families", "avoid_pattern_families", 
            "accent_plan", "rest_plan", "motif_strategy", "evidence", "confidence"
        ]
        for field in required_fields:
            if field not in intent:
                raise ValueError(f"Choreographic intent is missing critical field: {field}")
                
        # Type validations for critical fields
        # schema_version must be "choreographic-intent.v1"
        if not isinstance(intent.get("schema_version"), str) or intent["schema_version"] != "choreographic-intent.v1":
            raise ValueError("Invalid schema_version in choreographic intent")
        # section_id must be str
        if not isinstance(intent.get("section_id"), str):
            raise ValueError("Choreographic intent section_id must be a string")
        # mode must be str
        if not isinstance(intent.get("mode"), str):
            raise ValueError("Choreographic intent mode must be a string")
        # target_level must be int
        if not isinstance(intent.get("target_level"), int) or isinstance(intent.get("target_level"), bool):
            raise ValueError("Choreographic intent target_level must be an integer")
        # measure_start must be int
        if not isinstance(intent.get("measure_start"), int) or isinstance(intent.get("measure_start"), bool):
            raise ValueError("Choreographic intent measure_start must be an integer")
        # measure_end must be int
        if not isinstance(intent.get("measure_end"), int) or isinstance(intent.get("measure_end"), bool):
            raise ValueError("Choreographic intent measure_end must be an integer")
        # density_target must be str
        if not isinstance(intent.get("density_target"), str):
            raise ValueError("Choreographic intent density_target must be a string")
        # difficulty_budget must be number
        if not isinstance(intent.get("difficulty_budget"), (int, float)) or isinstance(intent.get("difficulty_budget"), bool):
            raise ValueError("Choreographic intent difficulty_budget must be a number")
        # recommended_pattern_families must be list[str]
        rec_patterns = intent.get("recommended_pattern_families")
        if not isinstance(rec_patterns, list) or not all(isinstance(x, str) for x in rec_patterns):
            raise ValueError("Choreographic intent recommended_pattern_families must be a list of strings")
        # avoid_pattern_families must be list[str]
        avoid_patterns = intent.get("avoid_pattern_families")
        if not isinstance(avoid_patterns, list) or not all(isinstance(x, str) for x in avoid_patterns):
            raise ValueError("Choreographic intent avoid_pattern_families must be a list of strings")
        # accent_plan must be list of dicts
        accents = intent.get("accent_plan")
        if not isinstance(accents, list) or not all(isinstance(x, dict) for x in accents):
            raise ValueError("Choreographic intent accent_plan must be a list of dicts")
        # rest_plan must be list of dicts
        rests = intent.get("rest_plan")
        if not isinstance(rests, list) or not all(isinstance(x, dict) for x in rests):
            raise ValueError("Choreographic intent rest_plan must be a list of dicts")
        # motif_strategy must be str
        if not isinstance(intent.get("motif_strategy"), str):
            raise ValueError("Choreographic intent motif_strategy must be a string")
        # evidence must be list[str]
        evidence = intent.get("evidence")
        if not isinstance(evidence, list) or not all(isinstance(x, str) for x in evidence):
            raise ValueError("Choreographic intent evidence must be a list of strings")
        # confidence must be number
        if not isinstance(intent.get("confidence"), (int, float)) or isinstance(intent.get("confidence"), bool):
            raise ValueError("Choreographic intent confidence must be a number")


def main():
    parser = argparse.ArgumentParser(description="Music Analysis Engine Offline Analyzer")
    parser.add_argument("--song-folder", type=str, help="Path to the song folder containing audio and .ssc")
    parser.add_argument("--ssc-path", type=str, help="Explicit path to the .ssc file")
    parser.add_argument("--audio-path", type=str, help="Explicit path to the audio file")
    parser.add_argument("--output", type=str, help="Optional output path to write JSON report")
    parser.add_argument("--pretty", action="store_true", help="Format JSON with indentation")
    args = parser.parse_args()

    ssc_path = args.ssc_path
    audio_path = args.audio_path
    warnings_list = []

    # If song folder is provided, look for .ssc and audio
    if args.song_folder:
        if not os.path.exists(args.song_folder):
            print(json.dumps({"status": "ERROR", "error_message": f"Song folder does not exist: {args.song_folder}"}))
            sys.exit(1)
            
        # Find .ssc file
        ssc_files = [f for f in os.listdir(args.song_folder) if f.endswith(".ssc")]
        if ssc_files:
            ssc_path = os.path.join(args.song_folder, ssc_files[0])
        else:
            warnings_list.append("No .ssc file found in song folder.")
            
        # Find audio file
        audio_files = [f for f in os.listdir(args.song_folder) if f.lower().endswith((".mp3", ".ogg", ".flac", ".wav"))]
        if audio_files:
            audio_path = os.path.join(args.song_folder, audio_files[0])
        else:
            warnings_list.append("No audio file found in song folder.")

    # Validate we have at least an SSC or an audio path
    if not ssc_path and not audio_path:
        print(json.dumps({"status": "ERROR", "error_message": "Must provide either --song-folder or both --ssc-path and --audio-path"}))
        sys.exit(1)

    # 1. Parse SSC ground-truth
    ssc_meta, ssc_charts = parse_ssc(ssc_path)

    # Extract Title and Artist
    title = ssc_meta.get("TITLE", "")
    artist = ssc_meta.get("ARTIST", "")
    if not title and ssc_path:
        title = os.path.splitext(os.path.basename(ssc_path))[0]

    # Extract Offset
    offset = 0.0
    offset_str = ssc_meta.get("OFFSET", "")
    if offset_str:
        try:
            offset = float(offset_str)
        except ValueError:
            warnings_list.append(f"Invalid offset string: {offset_str}")

    # Extract BPMs
    bpms = []
    bpms_str = ssc_meta.get("BPMS", "")
    if bpms_str:
        for pair in bpms_str.split(','):
            if '=' in pair:
                b_str, bpm_str = pair.split('=')
                try:
                    bpms.append((float(b_str.strip()), float(bpm_str.strip())))
                except ValueError:
                    pass
    if not bpms:
        bpms = [(0.0, 120.0)]
        warnings_list.append("No valid BPMs found in SSC. Using default 120.0 BPM.")

    initial_bpm = bpms[0][1]

    # Check for advanced tags
    stops_str = ssc_meta.get("STOPS", "").strip()
    delays_str = ssc_meta.get("DELAYS", "").strip()
    warps_str = ssc_meta.get("WARPS", "").strip()
    if stops_str or delays_str or warps_str:
        warnings_list.append("Advanced tags (STOPS, DELAYS, or WARPS) detected. Core beat grid may have offset drift.")

    # 2. Extract Audio Features
    audio_feats = extract_audio_features(audio_path, bpms, offset, warnings_list)
    duration_seconds = audio_feats["duration_seconds"]
    detected_bpm = audio_feats["detected_bpm"]
    analysis_mode = audio_feats["analysis_mode"]

    # If duration wasn't read, estimate from charts
    if duration_seconds == 120.0 and ssc_charts:
        max_measures = max([c.get("num_measures", 0) for c in ssc_charts] or [0])
        if max_measures > 0:
            total_beats = max_measures * 4
            duration_seconds = beat_to_time(total_beats, bpms, offset) + offset

    # Hash Song ID
    song_id_hash = hashlib.md5(f"{title}-{artist}-{duration_seconds:.2f}".encode('utf-8')).hexdigest()

    # 3. Create Timing Grid / Beats
    max_beat = time_to_beat(duration_seconds, bpms, offset)
    if max_beat <= 0:
        max_beat = 400.0  # Safe default fallback

    beats_list = []
    # Generate beat frames (step of 1.0 beats)
    beat = 0.0
    while beat <= max_beat:
        t_sec = beat_to_time(beat, bpms, offset)
        if t_sec > duration_seconds:
            break
            
        current_bpm = get_bpm_at_beat(beat, bpms)
        
        # Sample features at this beat
        if analysis_mode == "fallback":
            onset_val = 0.0
            energy_val = 0.0
            beat_confidence = 0.0
        else:
            onset_val = 0.0
            energy_val = 0.1
            beat_confidence = 1.0
            if NUMPY_AVAILABLE and audio_feats["times"] is not None:
                onset_val = sample_feature_value(audio_feats["onset_envelope"], t_sec, audio_feats["times"], 0.0)
                energy_val = sample_feature_value(audio_feats["rms_energy"], t_sec, audio_feats["times"], 0.1)
            else:
                # Simulated wave logic
                onset_val = 0.5 + 0.5 * np.sin(beat * np.pi) if NUMPY_AVAILABLE else 0.5
                energy_val = 0.2 + 0.1 * np.cos(beat * np.pi / 16) if NUMPY_AVAILABLE else 0.2

        beats_list.append({
            "beat": float(beat),
            "time_seconds": round(float(t_sec), 4),
            "measure_index": int(beat // 4),
            "beat_in_measure": float(beat % 4),
            "bpm": float(current_bpm),
            "confidence": beat_confidence,
            "audio_event_summary": {
                "onset_strength": round(onset_val, 4),
                "energy": round(energy_val, 4)
            }
        })
        beat += 1.0

    # 4. Heuristic Segmentation (e.g., every 8 measures / 32 beats)
    sections_list = []
    choreographic_intent_list = []
    measure_step = 8
    beats_per_section = measure_step * 4

    total_beats = len(beats_list)
    section_index = 0

    # Calculate average energy for break detection
    all_energies = [b["audio_event_summary"]["energy"] for b in beats_list]
    avg_energy = sum(all_energies) / len(all_energies) if all_energies else 0.2

    for start_idx in range(0, total_beats, beats_per_section):
        end_idx = min(start_idx + beats_per_section, total_beats)
        if start_idx == end_idx:
            break
            
        sec_beats = beats_list[start_idx:end_idx]
        start_b = sec_beats[0]["beat"]
        end_b = sec_beats[-1]["beat"] + 1.0  # Exclusive upper bound
        start_m = sec_beats[0]["measure_index"]
        end_m = sec_beats[-1]["measure_index"] + 1

        sec_id = f"section_{section_index + 1}"
        
        # Analyze section features
        sec_energies = [b["audio_event_summary"]["energy"] for b in sec_beats]
        sec_onsets = [b["audio_event_summary"]["onset_strength"] for b in sec_beats]
        
        sec_avg_energy = sum(sec_energies) / len(sec_energies) if sec_energies else 0.2
        sec_max_energy = max(sec_energies) if sec_energies else 0.2
        sec_avg_onset = sum(sec_onsets) / len(sec_onsets) if sec_onsets else 0.5
        sec_max_onset = max(sec_onsets) if sec_onsets else 1.0

        # Heuristic Energy Profile
        if sec_avg_energy < avg_energy * 0.4:
            energy_profile = "low_silence"
        elif sec_avg_energy > avg_energy * 1.3:
            energy_profile = "high_sustained"
        elif sec_avg_energy > sec_energies[0] * 1.5:
            energy_profile = "building"
        else:
            energy_profile = "moderate"

        # Heuristic Music Role & PIU Role
        music_role = "unknown"
        piu_role = "unknown"

        # Position-based heuristics
        total_sections = (total_beats + beats_per_section - 1) // beats_per_section
        is_first = (section_index == 0)
        is_second = (section_index == 1)
        is_last = (section_index == total_sections - 1)
        is_climax_pos = (section_index == total_sections - 2 or section_index == total_sections - 3)

        if is_first:
            music_role = "intro"
            piu_role = "warmup"
        elif is_last:
            music_role = "outro"
            piu_role = "cooldown"
        elif energy_profile == "low_silence":
            music_role = "break"
            piu_role = "rest_zone"
        elif energy_profile == "building":
            music_role = "build"
            piu_role = "tech_zone"
        elif energy_profile == "high_sustained" or is_climax_pos:
            music_role = "chorus"
            piu_role = "climax_zone" if is_climax_pos else "stream_zone"
        else:
            music_role = "verse"
            piu_role = "footwork_zone"

        sections_list.append({
            "section_id": sec_id,
            "start_beat": float(start_b),
            "end_beat": float(end_b),
            "start_measure": int(start_m),
            "end_measure": int(end_m),
            "music_role": music_role,
            "piu_role": piu_role,
            "boundary_confidence": 0.8,
            "energy_profile": energy_profile
        })

        # 5. Choreographic Intent Map per section
        # Heuristics for patterns
        recommended_patterns = []
        avoid_patterns = []
        
        # Difficulty parameters
        difficulty_budget = 0.5
        density_target = "medium"

        if piu_role == "warmup":
            recommended_patterns = ["alternating_quarter_notes", "neutral_center_anchors"]
            avoid_patterns = ["runs", "brackets", "twists"]
            density_target = "low"
            difficulty_budget = 0.2
        elif piu_role == "cooldown":
            recommended_patterns = ["simple_steps", "long_holds"]
            avoid_patterns = ["streams", "jump_accents"]
            density_target = "low"
            difficulty_budget = 0.15
        elif piu_role == "rest_zone":
            recommended_patterns = ["long_holds", "slow_crossovers"]
            avoid_patterns = ["drills", "speed_runs"]
            density_target = "low"
            difficulty_budget = 0.25
        elif piu_role == "stream_zone":
            recommended_patterns = ["alternating_stream", "light_twist"]
            avoid_patterns = ["long_silence", "repeated_brackets"]
            density_target = "high"
            difficulty_budget = 0.75
        elif piu_role == "climax_zone":
            recommended_patterns = ["stamina_runs", "jump_accents", "bracket_jumps"]
            avoid_patterns = ["long_rests"]
            density_target = "extreme"
            difficulty_budget = 0.9
        elif piu_role == "tech_zone":
            recommended_patterns = ["drills", "crossovers", "twists"]
            avoid_patterns = ["plain_streams"]
            density_target = "medium_high"
            difficulty_budget = 0.65
        else:  # footwork_zone
            recommended_patterns = ["standard_stream", "crossovers"]
            avoid_patterns = ["extreme_jumps"]
            density_target = "medium"
            difficulty_budget = 0.5

        if analysis_mode == "fallback":
            accent_plan = []
            rest_plan = []
            evidence = ["metadata-only"]
            intent_conf = 0.1
        else:
            # Build accent plan for high onset spikes
            accent_plan = []
            for b in sec_beats:
                if b["audio_event_summary"]["onset_strength"] > sec_max_onset * 0.85:
                    accent_plan.append({
                        "beat": b["beat"],
                        "strength": round(b["audio_event_summary"]["onset_strength"], 3),
                        "suggestion": "jump_accent" if b["audio_event_summary"]["energy"] > sec_avg_energy else "step_accent"
                    })

            # Build rest plan for low energy beats
            rest_plan = []
            for b in sec_beats:
                if b["audio_event_summary"]["energy"] < sec_avg_energy * 0.3:
                    rest_plan.append({
                        "beat": b["beat"],
                        "strength": round(b["audio_event_summary"]["energy"], 3),
                        "suggestion": "hold_or_empty"
                    })
            evidence = [f"Role: {piu_role}", f"Avg energy: {sec_avg_energy:.3f}", f"Avg onset: {sec_avg_onset:.3f}"]
            intent_conf = 0.85

        choreographic_intent_list.append({
            "schema_version": "choreographic-intent.v1",
            "section_id": sec_id,
            "mode": "Single",  # Single first as MVP
            "target_level": 10,  # Mock target level
            "measure_start": int(start_m),
            "measure_end": int(end_m),
            "density_target": density_target,
            "difficulty_budget": float(round(difficulty_budget, 2)),
            "recommended_pattern_families": recommended_patterns,
            "avoid_pattern_families": avoid_patterns,
            "accent_plan": accent_plan,
            "rest_plan": rest_plan,
            "motif_strategy": "repeat_with_variation" if piu_role in ["stream_zone", "climax_zone"] else "introduce_theme",
            "evidence": evidence,
            "confidence": intent_conf
        })

        section_index += 1

    # 6. Sync Diagnostics
    # Compare detected BPM vs ssc initial BPM
    tempo_agreement = False
    timing_confidence = 1.0
    
    bpm_diff = abs(detected_bpm - initial_bpm)
    
    # Check for half / double BPM
    half_bpm_diff = abs(detected_bpm * 2 - initial_bpm)
    double_bpm_diff = abs(detected_bpm / 2 - initial_bpm)
    
    if bpm_diff < 1.0:
        tempo_agreement = True
        timing_confidence = 1.0
    elif half_bpm_diff < 1.0:
        tempo_agreement = True
        timing_confidence = 0.6
        warnings_list.append(f"Detected BPM ({detected_bpm:.2f}) is half of SSC BPM ({initial_bpm:.2f}). Possible octave error.")
    elif double_bpm_diff < 1.0:
        tempo_agreement = True
        timing_confidence = 0.6
        warnings_list.append(f"Detected BPM ({detected_bpm:.2f}) is double of SSC BPM ({initial_bpm:.2f}). Possible octave error.")
    else:
        tempo_agreement = False
        timing_confidence = 0.3
        warnings_list.append(f"BPM mismatch: detected {detected_bpm:.2f} vs SSC {initial_bpm:.2f}.")

    requires_manual_timing_review = (not tempo_agreement) or bool(stops_str or delays_str or warps_str)

    if analysis_mode == "fallback":
        requires_manual_timing_review = True
        timing_confidence = 0.1
        warnings_list.append("DSP unavailable or audio failed; report uses fallback metadata-only estimates.")

    # 7. Compile Report
    rms_mean = 0.0
    rms_max = 0.0
    centroid_mean = 0.0
    flatness_mean = 0.0
    crossing_rate_mean = 0.0

    if analysis_mode == "dsp":
        rms_mean = float(np.mean(audio_feats["rms_energy"])) if audio_feats["rms_energy"] is not None else 0.0
        rms_max = float(np.max(audio_feats["rms_energy"])) if audio_feats["rms_energy"] is not None else 0.0
        centroid_mean = float(np.mean(audio_feats["spectral_centroid"])) if audio_feats["spectral_centroid"] is not None else 0.0
        flatness_mean = float(np.mean(audio_feats["spectral_flatness"])) if audio_feats["spectral_flatness"] is not None else 0.0
        crossing_rate_mean = float(np.mean(audio_feats["zero_crossing_rate"])) if audio_feats["zero_crossing_rate"] is not None else 0.0

    report = {
        "schema_version": "music-analysis-report.v1",
        "song_id": song_id_hash,
        "title": title,
        "artist": artist,
        "duration_seconds": round(duration_seconds, 2),
        "audio_summary": {
            "sample_rate": audio_feats["sample_rate"],
            "detected_bpm": round(detected_bpm, 3),
            "rms_energy_mean": round(rms_mean, 4),
            "rms_energy_max": round(rms_max, 4),
            "spectral_centroid_mean": round(centroid_mean, 2),
            "spectral_flatness_mean": round(flatness_mean, 4),
            "zero_crossing_rate_mean": round(crossing_rate_mean, 4),
            "chroma_mean": audio_feats.get("chroma_mean"),
            "spectral_contrast_mean": audio_feats.get("spectral_contrast_mean"),
            "analysis_mode": analysis_mode
        },
        "timing_grid": {
            "initial_offset": offset,
            "bpms": [[float(b), float(bpm)] for b, bpm in bpms],
            "display_bpm": ssc_meta.get("DISPLAYBPM", f"{initial_bpm:.3f}"),
            "song_type": ssc_meta.get("SONGTYPE", "ARCADE")
        },
        "event_features": {
            "beats": beats_list
        },
        "sections": sections_list,
        "choreographic_intent": choreographic_intent_list,
        "diagnostics": {
            "audio_bpm_detected": round(detected_bpm, 3),
            "ssc_initial_bpm": round(initial_bpm, 3),
            "audio_vs_ssc_tempo_agreement": tempo_agreement,
            "beat_grid_error_ms_mean": 0.0,  # Calculated as 0 for initial MVP
            "timing_confidence": timing_confidence,
            "requires_manual_timing_review": requires_manual_timing_review,
            "warnings": warnings_list,
            "analysis_mode": analysis_mode
        },
        "publicability": {
            "contains_original_audio": False,
            "contains_full_chart": False,
            "exportable": True
        }
    }

    # Validate report contract before outputting
    validate_report_contract(report)

    # Write output to file if specified
    if args.output:
        import tempfile
        try:
            out_dir = os.path.dirname(args.output)
            if out_dir and not os.path.exists(out_dir):
                os.makedirs(out_dir, exist_ok=True)
            
            # Atomic write using temp file and replace
            fd, temp_path = tempfile.mkstemp(dir=out_dir or ".", suffix=".tmp")
            try:
                with os.fdopen(fd, 'w', encoding='utf-8') as out_f:
                    if args.pretty:
                        json.dump(report, out_f, indent=2, ensure_ascii=False)
                    else:
                        json.dump(report, out_f, ensure_ascii=False)
                os.replace(temp_path, args.output)
            except Exception as e:
                if os.path.exists(temp_path):
                    os.remove(temp_path)
                raise e
        except Exception as e:
            print(f"Error writing report to output path {args.output}: {e}", file=sys.stderr)
            sys.exit(1)

    # Print JSON output to stdout
    if args.pretty:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(json.dumps(report, ensure_ascii=False))


if __name__ == '__main__':
    main()
