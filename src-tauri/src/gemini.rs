use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeminiWriteMode {
    PreviewOnly,
}

#[derive(Debug, Clone)]
pub enum GeminiAudioSource {
    Inline {
        mime_type: String,
        base64_data: String,
    },
    FileData {
        mime_type: String,
        file_uri: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    pub mime_type: String,
    pub file_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerationConfig {
    pub response_mime_type: String,
    pub response_json_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiGenerateChartRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiGenerateContentResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
}

impl GeminiGenerateContentResponse {
    pub fn get_text_content(&self) -> Result<String, String> {
        let candidates = self.candidates.as_ref().ok_or_else(|| {
            "No candidates found in Gemini response (empty candidates list).".to_string()
        })?;

        let candidate = candidates
            .first()
            .ok_or_else(|| "Candidates list is empty in Gemini response.".to_string())?;

        for part in &candidate.content.parts {
            if let GeminiPart::Text { text } = part {
                return Ok(text.clone());
            }
        }

        Err("No text part found in the first candidate.".to_string())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiErrorResponse {
    pub error: GeminiErrorDetails,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeminiErrorDetails {
    pub code: u32,
    pub message: String,
    pub status: String,
}

pub struct GeminiClient {
    client: reqwest::Client,
    base_url: String,
}

pub fn detect_mime_type(file_path: &Path) -> Result<String, String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "No file extension found on audio file.".to_string())?
        .to_lowercase();

    match ext.as_str() {
        "mp3" => Ok("audio/mpeg".to_string()),
        "ogg" => Ok("audio/ogg".to_string()),
        "wav" => Ok("audio/wav".to_string()),
        "flac" => Ok("audio/flac".to_string()),
        _ => Err(format!("Unsupported audio extension: .{}", ext)),
    }
}

pub fn get_response_schema_json() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "section_id": {
                "type": "string",
                "description": "Identifier of the song section being generated"
            },
            "difficulty_level": {
                "type": "integer",
                "description": "Difficulty level of the chart (Single: 1-26, Double: 1-15)"
            },
            "play_mode": {
                "type": "string",
                "enum": ["Single", "Double"],
                "description": "Play mode of the chart"
            },
            "biomechanical_state": {
                "type": "object",
                "properties": {
                    "current_twist_debt": {
                        "type": "number",
                        "description": "Accumulated twist debt of the player in degrees"
                    },
                    "current_stamina_debt": {
                        "type": "number",
                        "description": "Accumulated stamina debt of the player"
                    },
                    "last_left_foot_lane": {
                        "type": "integer",
                        "description": "Last lane index pressed by the left foot (0-4 for Single, 0-9 for Double)"
                    },
                    "last_right_foot_lane": {
                        "type": "integer",
                        "description": "Last lane index pressed by the right foot (0-4 for Single, 0-9 for Double)"
                    }
                },
                "required": ["current_twist_debt", "current_stamina_debt"],
                "additionalProperties": false
            },
            "measures": {
                "type": "array",
                "description": "List of compases / measures containing the choreography rows",
                "items": {
                    "type": "object",
                    "properties": {
                        "measure_index": {
                            "type": "integer",
                            "description": "Index of the measure (0-indexed)"
                        },
                        "subdivision": {
                            "type": "integer",
                            "enum": [4, 8, 16, 32],
                            "description": "Subdivision of the measure (4: quarter notes, 8: eighth notes, 16: sixteenth notes, 32: thirty-second notes)"
                        },
                        "rows": {
                            "type": "array",
                            "description": "Note rows matching the subdivision count. Each row must be a string of length 5 (Single) or 10 (Double), composed only of characters '0' (empty), '1' (normal step), '2' (hold start), '3' (hold release). No mines 'M' or other characters allowed.",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "required": ["measure_index", "subdivision", "rows"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["section_id", "difficulty_level", "play_mode", "biomechanical_state", "measures"],
        "additionalProperties": false
    })
}

impl GeminiClient {
    #[cfg(test)]
    pub fn new(base_url: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let url =
            base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string());

        Self {
            client,
            base_url: url,
        }
    }

    #[cfg(not(test))]
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
        }
    }

    pub async fn generate_chart_content(
        &self,
        api_key: &str,
        audio_source: GeminiAudioSource,
        prompt_text: &str,
    ) -> Result<String, String> {
        let audio_part = match audio_source {
            GeminiAudioSource::Inline {
                mime_type,
                base64_data,
            } => GeminiPart::InlineData {
                inline_data: InlineData {
                    mime_type,
                    data: base64_data,
                },
            },
            GeminiAudioSource::FileData {
                mime_type,
                file_uri,
            } => GeminiPart::FileData {
                file_data: FileData {
                    mime_type,
                    file_uri,
                },
            },
        };

        let text_part = GeminiPart::Text {
            text: prompt_text.to_string(),
        };
        let schema_json = get_response_schema_json();

        let request_body = GeminiGenerateChartRequest {
            contents: vec![GeminiContent {
                parts: vec![text_part, audio_part],
            }],
            generation_config: GeminiGenerationConfig {
                response_mime_type: "application/json".to_string(),
                response_json_schema: schema_json,
            },
        };

        let url = format!(
            "{}/v1beta/models/gemini-3.5-flash:generateContent",
            self.base_url
        );

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let err_text = response.text().await.unwrap_or_default();
            if let Ok(api_err) = serde_json::from_str::<GeminiErrorResponse>(&err_text) {
                return Err(format!(
                    "Gemini API Error ({}): {}",
                    api_err.error.status, api_err.error.message
                ));
            } else {
                return Err(format!(
                    "Gemini API returned status {}: {}",
                    status, err_text
                ));
            }
        }

        let resp_body: GeminiGenerateContentResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response JSON: {}", e))?;

        resp_body.get_text_content()
    }

    pub async fn upload_file_resumable(
        &self,
        api_key: &str,
        file_path: &Path,
        mime_type: &str,
        file_size: u64,
    ) -> Result<String, String> {
        let display_name = file_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("audio_file")
            .to_string();

        // Phase 1: Initiation
        let init_url = format!("{}/upload/v1beta/files", self.base_url);

        let init_body = serde_json::json!({
            "file": {
                "display_name": display_name
            }
        });

        let init_response = self
            .client
            .post(&init_url)
            .header("x-goog-api-key", api_key)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", file_size.to_string())
            .header("X-Goog-Upload-Header-Content-Type", mime_type)
            .json(&init_body)
            .send()
            .await
            .map_err(|e| format!("Files API upload initiation failed: {}", e))?;

        let init_status = init_response.status();
        if !init_status.is_success() {
            let err_text = init_response.text().await.unwrap_or_default();
            return Err(format!(
                "Files API initiation returned status {}: {}",
                init_status, err_text
            ));
        }

        let upload_url = init_response
            .headers()
            .get("X-Goog-Upload-URL")
            .or_else(|| init_response.headers().get("x-goog-upload-url"))
            .ok_or_else(|| {
                "X-Goog-Upload-URL header missing from upload initiation response.".to_string()
            })?
            .to_str()
            .map_err(|e| format!("Failed to parse upload URL header: {}", e))?
            .to_string();

        // Phase 2: Upload streaming (not loading entire file in memory)
        let file = tokio::fs::File::open(file_path)
            .await
            .map_err(|e| format!("Failed to open audio file for upload streaming: {}", e))?;
        let body = reqwest::Body::from(file);

        let upload_response = self
            .client
            .put(&upload_url)
            .header("x-goog-api-key", api_key)
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Files API upload execution failed: {}", e))?;

        let upload_status = upload_response.status();
        if !upload_status.is_success() {
            let err_text = upload_response.text().await.unwrap_or_default();
            return Err(format!(
                "Files API upload execution returned status {}: {}",
                upload_status, err_text
            ));
        }

        #[derive(Debug, Deserialize)]
        struct FilesApiResponse {
            file: FileMetadata,
        }

        #[derive(Debug, Deserialize)]
        struct FileMetadata {
            uri: String,
        }

        let result_body: FilesApiResponse = upload_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse upload result JSON: {}", e))?;

        Ok(result_body.file.uri)
    }

    pub async fn process_audio_and_generate(
        &self,
        api_key: &str,
        audio_path: &Path,
        prompt_text: &str,
    ) -> Result<String, String> {
        let metadata = std::fs::metadata(audio_path)
            .map_err(|e| format!("Failed to read audio file size: {}", e))?;
        let file_size = metadata.len();
        let mime_type = detect_mime_type(audio_path)?;

        // Estimate total request size including base64 overhead (roughly 1.35x raw size)
        // and prompt/schema overhead (estimated around 50KB)
        let base64_estimated_size = (file_size as f64 * 1.35) as u64;
        let prompt_estimated_size = prompt_text.len() as u64;
        let total_request_estimated_size = base64_estimated_size + prompt_estimated_size + 50_000;

        let source = if total_request_estimated_size <= 20 * 1024 * 1024 {
            // Under or equal to 20 MB estimated request size -> Inline base64
            let file_bytes = std::fs::read(audio_path)
                .map_err(|e| format!("Failed to read audio bytes: {}", e))?;
            let base64_data = STANDARD.encode(&file_bytes);
            GeminiAudioSource::Inline {
                mime_type,
                base64_data,
            }
        } else {
            // Exceeds 20 MB estimated request size -> Files API upload
            let file_uri = self
                .upload_file_resumable(api_key, audio_path, &mime_type, file_size)
                .await?;
            GeminiAudioSource::FileData {
                mime_type,
                file_uri,
            }
        };

        self.generate_chart_content(api_key, source, prompt_text)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::io::Write;

    #[test]
    fn test_schema_json_generation() {
        let schema = get_response_schema_json();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["section_id"].is_object());
        assert!(schema["properties"]["biomechanical_state"].is_object());
        assert_eq!(
            schema["properties"]["biomechanical_state"]["additionalProperties"],
            false
        );
        assert_eq!(
            schema["properties"]["measures"]["items"]["additionalProperties"],
            false
        );
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn test_detect_mime_type() {
        assert_eq!(
            detect_mime_type(Path::new("song.mp3")).unwrap(),
            "audio/mpeg"
        );
        assert_eq!(
            detect_mime_type(Path::new("song.OGG")).unwrap(),
            "audio/ogg"
        );
        assert_eq!(
            detect_mime_type(Path::new("song.wav")).unwrap(),
            "audio/wav"
        );
        assert_eq!(
            detect_mime_type(Path::new("song.flac")).unwrap(),
            "audio/flac"
        );
        assert!(detect_mime_type(Path::new("song.txt")).is_err());
    }

    #[tokio::test]
    async fn test_mock_http_generate_content_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .match_header("x-goog-api-key", "test-key-1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "candidates": [
                    {
                        "content": {
                            "parts": [
                                {
                                    "text": "{\"section_id\": \"chorus_test\"}"
                                }
                            ]
                        }
                    }
                ]
            }"#,
            )
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));
        let source = GeminiAudioSource::Inline {
            mime_type: "audio/mp3".to_string(),
            base64_data: "AAAA".to_string(),
        };

        let result = client
            .generate_chart_content("test-key-1", source, "Generate chart")
            .await;
        mock.assert_async().await;

        if let Err(e) = &result {
            panic!("Test failed with error: {}", e);
        }
        let text = result.unwrap();
        assert_eq!(text, "{\"section_id\": \"chorus_test\"}");
    }

    #[tokio::test]
    async fn test_mock_http_generate_content_invalid_json() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"candidates": [{"content": {"parts": [{"text": "invalid_json_content"}]}}]}"#,
            )
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));
        let source = GeminiAudioSource::Inline {
            mime_type: "audio/mp3".to_string(),
            base64_data: "AAAA".to_string(),
        };

        let result = client
            .generate_chart_content("test-key", source, "Generate")
            .await;
        mock.assert_async().await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "invalid_json_content");
    }

    #[tokio::test]
    async fn test_mock_http_generate_content_no_candidates() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"candidates": []}"#)
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));
        let source = GeminiAudioSource::Inline {
            mime_type: "audio/mp3".to_string(),
            base64_data: "AAAA".to_string(),
        };

        let result = client
            .generate_chart_content("test-key", source, "Generate")
            .await;
        mock.assert_async().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Candidates list is empty"));
    }

    #[tokio::test]
    async fn test_mock_http_401_unauthorized() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "error": {
                    "code": 401,
                    "message": "API key not valid",
                    "status": "UNAUTHENTICATED"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));
        let source = GeminiAudioSource::Inline {
            mime_type: "audio/mp3".to_string(),
            base64_data: "AAAA".to_string(),
        };

        let result = client
            .generate_chart_content("wrong-key", source, "Generate")
            .await;
        mock.assert_async().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("UNAUTHENTICATED"));
        assert!(err.contains("API key not valid"));
    }

    #[tokio::test]
    async fn test_mock_http_429_rate_limit() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1beta/models/gemini-3.5-flash:generateContent")
            .with_status(429)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "error": {
                    "code": 429,
                    "message": "Resource has been exhausted",
                    "status": "RESOURCE_EXHAUSTED"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));
        let source = GeminiAudioSource::Inline {
            mime_type: "audio/mp3".to_string(),
            base64_data: "AAAA".to_string(),
        };

        let result = client
            .generate_chart_content("key", source, "Generate")
            .await;
        mock.assert_async().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("RESOURCE_EXHAUSTED"));
        assert!(err.contains("Resource has been exhausted"));
    }

    #[tokio::test]
    async fn test_mock_http_files_api_resumable_flow() {
        let mut server = Server::new_async().await;

        // 1. Mock Initiation POST
        let upload_start_url = "/upload/v1beta/files";
        let mock_init = server
            .mock("POST", upload_start_url)
            .match_header("x-goog-api-key", "my-key")
            .match_header("X-Goog-Upload-Protocol", "resumable")
            .match_header("X-Goog-Upload-Command", "start")
            .match_header("X-Goog-Upload-Header-Content-Length", "4")
            .match_header("X-Goog-Upload-Header-Content-Type", "audio/mp3")
            .with_status(200)
            .with_header(
                "X-Goog-Upload-URL",
                &format!("{}/upload-session-id-123", server.url()),
            )
            .create_async()
            .await;

        // 2. Mock Put raw bytes
        let mock_put = server
            .mock("PUT", "/upload-session-id-123")
            .match_header("x-goog-api-key", "my-key")
            .match_header("X-Goog-Upload-Offset", "0")
            .match_header("X-Goog-Upload-Command", "upload, finalize")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "file": {
                    "name": "files/test-file-123",
                    "displayName": "test.mp3",
                    "mimeType": "audio/mp3",
                    "sizeBytes": "4",
                    "uri": "https://generativelanguage.googleapis.com/v1beta/files/test-file-123"
                }
            }"#,
            )
            .create_async()
            .await;

        let client = GeminiClient::new(Some(server.url()));

        // Create a dummy file in temp dir
        let temp_dir = std::env::temp_dir();
        let test_file_path = temp_dir.join("test.mp3");
        {
            let mut file = std::fs::File::create(&test_file_path).unwrap();
            file.write_all(b"test").unwrap();
        }

        let file_uri = client
            .upload_file_resumable("my-key", &test_file_path, "audio/mp3", 4)
            .await;

        mock_init.assert_async().await;
        mock_put.assert_async().await;

        assert!(file_uri.is_ok());
        assert_eq!(
            file_uri.unwrap(),
            "https://generativelanguage.googleapis.com/v1beta/files/test-file-123"
        );

        let _ = std::fs::remove_file(test_file_path);
    }
}
