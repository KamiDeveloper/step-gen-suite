#!/usr/bin/env python
# -*- coding: utf-8 -*-

import os
import sys
import json
import wave
import struct
import math
import tempfile
import unittest
import subprocess

class TestMusicAnalysisEngine(unittest.TestCase):
    def setUp(self):
        # Create temp directory
        self.temp_dir = tempfile.TemporaryDirectory()
        
        # Paths
        self.wav_path = os.path.join(self.temp_dir.name, "test_audio.wav")
        self.ssc_path = os.path.join(self.temp_dir.name, "test_chart.ssc")
        self.report_path = os.path.join(self.temp_dir.name, "report.json")
        
        # 1. Generate synthetic WAV
        self.generate_synthetic_wav(self.wav_path, duration=6.0, sample_rate=22050, beat_bpm=120.0)
        
        # 2. Write mock SSC
        self.ssc_content = (
            "#TITLE:Test Song;\n"
            "#ARTIST:Test Artist;\n"
            "#OFFSET:0.000000;\n"
            "#BPMS:0.000=120.000;\n"
            "#DISPLAYBPM:120.000;\n"
            "#SONGTYPE:ARCADE;\n"
            "#NOTEDATA:;\n"
            "#STEPSTYPE:pump-single;\n"
            "#METER:5;\n"
            "#CREDIT:Test Maker;\n"
            "#DIFFICULTY:Hard;\n"
            "#NOTES:\n"
            "00000\n"
            "00000\n"
            "00000\n"
            "00000\n"
            ";\n"
        )
        with open(self.ssc_path, "w", encoding="utf-8") as f:
            f.write(self.ssc_content)

    def tearDown(self):
        self.temp_dir.cleanup()

    def generate_synthetic_wav(self, path, duration=6.0, sample_rate=22050, beat_bpm=120.0):
        num_samples = int(duration * sample_rate)
        with wave.open(path, 'wb') as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(sample_rate)
            
            beat_interval = 60.0 / beat_bpm
            click_duration = 0.05
            
            for i in range(num_samples):
                time = i / sample_rate
                beat_time = round(time / beat_interval) * beat_interval
                dist_to_beat = abs(time - beat_time)
                
                if dist_to_beat < click_duration / 2:
                    val = int(10000 * math.sin(2 * math.pi * 1000.0 * time))
                else:
                    val = 0
                    
                w.writeframes(struct.pack('<h', val))

    def test_analyzer_report_fields_and_no_ssc_mod(self):
        # Read SSC before
        with open(self.ssc_path, "r", encoding="utf-8") as f:
            ssc_before = f.read()
            
        # Run analyze_song.py via subprocess
        script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "analyze_song.py"))
        
        cmd = [
            sys.executable,
            script_path,
            "--ssc-path", self.ssc_path,
            "--audio-path", self.wav_path,
            "--output", self.report_path,
            "--pretty"
        ]
        
        process = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(process.returncode, 0, f"Script failed with stderr: {process.stderr}")
        
        # Verify stdout is valid JSON
        stdout_json = json.loads(process.stdout)
        self.assertEqual(stdout_json["schema_version"], "music-analysis-report.v1")
        
        # Verify written file
        self.assertTrue(os.path.exists(self.report_path))
        with open(self.report_path, "r", encoding="utf-8") as f:
            report_data = json.load(f)
            
        # Verify mandatory schema fields
        mandatory_fields = [
            "schema_version", "song_id", "title", "artist", "duration_seconds",
            "audio_summary", "timing_grid", "event_features", "sections",
            "choreographic_intent", "diagnostics", "publicability"
        ]
        for field in mandatory_fields:
            self.assertIn(field, report_data, f"Mandatory field '{field}' is missing from report")
            
        # Verify requires_manual_timing_review is strictly boolean
        requires_review = report_data["diagnostics"]["requires_manual_timing_review"]
        self.assertIsInstance(requires_review, bool, f"requires_manual_timing_review should be bool, got {type(requires_review)}")
        
        # Verify timing_confidence is float/int
        self.assertIsInstance(report_data["diagnostics"]["timing_confidence"], (int, float))
        
        # Verify analysis_mode is present and is dsp (since valid audio is processed)
        self.assertEqual(report_data["diagnostics"]["analysis_mode"], "dsp")
        self.assertEqual(report_data["audio_summary"]["analysis_mode"], "dsp")
        
        # Verify chroma_mean and spectral_contrast_mean are present (and lists of floats or None)
        self.assertIn("chroma_mean", report_data["audio_summary"])
        self.assertIn("spectral_contrast_mean", report_data["audio_summary"])
        
        # Verify SSC wasn't modified
        with open(self.ssc_path, "r", encoding="utf-8") as f:
            ssc_after = f.read()
        self.assertEqual(ssc_before, ssc_after, "The .ssc file was modified by the analyzer!")

    def test_fallback_mode_when_audio_missing(self):
        script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "analyze_song.py"))
        nonexistent_wav = os.path.join(os.path.dirname(self.wav_path), "nonexistent_audio.wav")
        
        cmd = [
            sys.executable,
            script_path,
            "--ssc-path", self.ssc_path,
            "--audio-path", nonexistent_wav,
            "--pretty"
        ]
        
        process = subprocess.run(cmd, capture_output=True, text=True)
        self.assertEqual(process.returncode, 0, f"Script failed with stderr: {process.stderr}")
        
        report_data = json.loads(process.stdout)
        
        # Verify mode is fallback
        self.assertEqual(report_data["diagnostics"]["analysis_mode"], "fallback")
        self.assertEqual(report_data["audio_summary"]["analysis_mode"], "fallback")
        
        # Verify timing review is true
        self.assertTrue(report_data["diagnostics"]["requires_manual_timing_review"])
        
        # Verify timing confidence is low (0.1)
        self.assertEqual(report_data["diagnostics"]["timing_confidence"], 0.1)
        
        # Verify intent confidence is low (0.1)
        for intent in report_data["choreographic_intent"]:
            self.assertEqual(intent["confidence"], 0.1)
            self.assertEqual(intent["accent_plan"], [])
            self.assertEqual(intent["rest_plan"], [])
            self.assertEqual(intent["evidence"], ["metadata-only"])

        # Verify beats confidence is 0.0, onset_strength and energy are 0.0
        for beat in report_data["event_features"]["beats"]:
            self.assertEqual(beat["confidence"], 0.0)
            self.assertEqual(beat["audio_event_summary"]["onset_strength"], 0.0)
            self.assertEqual(beat["audio_event_summary"]["energy"], 0.0)
            
        # Verify fallback warning is present
        warnings = report_data["diagnostics"]["warnings"]
        self.assertTrue(any("fallback" in w.lower() for w in warnings), "Fallback warning missing")
        
        # Verify audio summary stats are zeroed
        summary = report_data["audio_summary"]
        self.assertEqual(summary["rms_energy_mean"], 0.0)
        self.assertEqual(summary["rms_energy_max"], 0.0)
        self.assertEqual(summary["spectral_centroid_mean"], 0.0)
        self.assertEqual(summary["spectral_flatness_mean"], 0.0)
        self.assertEqual(summary["zero_crossing_rate_mean"], 0.0)

    def test_invalid_output_path_fails(self):
        script_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "analyze_song.py"))
        # Using a directory name as the output file path triggers an IsADirectoryError / PermissionError on write
        invalid_output = os.path.dirname(self.wav_path)
        
        cmd = [
            sys.executable,
            script_path,
            "--ssc-path", self.ssc_path,
            "--audio-path", self.wav_path,
            "--output", invalid_output
        ]
        
        process = subprocess.run(cmd, capture_output=True, text=True)
        # Verify exit code is not 0
        self.assertNotEqual(process.returncode, 0)
        # Verify error output on stderr
        self.assertTrue("Error writing report" in process.stderr)
        # Verify no JSON printed on stdout
        self.assertEqual(process.stdout.strip(), "")

    def test_validate_report_contract_invalid_types(self):
        parent_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
        if parent_dir not in sys.path:
            sys.path.insert(0, parent_dir)
            
        try:
            from analyze_song import validate_report_contract
        except ImportError:
            sys.path.insert(0, os.path.join(parent_dir, ".."))
            from music_analysis.analyze_song import validate_report_contract

        valid_report = {
            "schema_version": "music-analysis-report.v1",
            "song_id": "test-id",
            "title": "Mock Song",
            "artist": "Mock Artist",
            "duration_seconds": 120.5,
            "audio_summary": {
                "sample_rate": 44100,
                "detected_bpm": 130.0,
                "rms_energy_mean": 0.15,
                "rms_energy_max": 0.35,
                "spectral_centroid_mean": 1500.0,
                "spectral_flatness_mean": 0.05,
                "zero_crossing_rate_mean": 0.08,
                "chroma_mean": None,
                "spectral_contrast_mean": None,
                "analysis_mode": "dsp"
            },
            "timing_grid": {
                "initial_offset": -0.123,
                "bpms": [[0.0, 130.0]]
            },
            "event_features": {
                "beats": []
            },
            "sections": [
                {
                    "section_id": "section_1",
                    "start_beat": 0.0,
                    "end_beat": 32.0,
                    "start_measure": 0,
                    "end_measure": 8,
                    "music_role": "intro",
                    "piu_role": "warmup"
                }
            ],
            "choreographic_intent": [
                {
                    "schema_version": "choreographic-intent.v1",
                    "section_id": "section_1",
                    "mode": "Single",
                    "target_level": 10,
                    "measure_start": 0,
                    "measure_end": 8,
                    "density_target": "medium",
                    "difficulty_budget": 0.5,
                    "recommended_pattern_families": ["stream"],
                    "avoid_pattern_families": ["holds"],
                    "accent_plan": [],
                    "rest_plan": [],
                    "motif_strategy": "introduce_theme",
                    "evidence": ["test"],
                    "confidence": 0.85
                }
            ],
            "diagnostics": {
                "audio_bpm_detected": 130.0,
                "ssc_initial_bpm": 130.0,
                "audio_vs_ssc_tempo_agreement": True,
                "beat_grid_error_ms_mean": 0.0,
                "timing_confidence": 1.0,
                "requires_manual_timing_review": False,
                "analysis_mode": "dsp"
            },
            "publicability": {
                "contains_original_audio": False,
                "contains_full_chart": False,
                "exportable": True
            }
        }

        # Verify it passes when valid
        validate_report_contract(valid_report)

        # Mutate schema_version to invalid type
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["schema_version"] = 123
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)

        # Mutate measure_start to invalid type (str)
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["measure_start"] = "0"
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)

        # Mutate measure_start to invalid type (bool)
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["measure_start"] = True
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)

        # Mutate difficulty_budget to invalid type (str)
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["difficulty_budget"] = "0.5"
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)

        # Mutate recommended_pattern_families to invalid type (list of int)
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["recommended_pattern_families"] = [1, 2]
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)

        # Mutate avoid_pattern_families to invalid type (str)
        bad_report = json.loads(json.dumps(valid_report))
        bad_report["choreographic_intent"][0]["avoid_pattern_families"] = "holds"
        self.assertRaises((ValueError, TypeError), validate_report_contract, bad_report)


if __name__ == "__main__":
    unittest.main()
