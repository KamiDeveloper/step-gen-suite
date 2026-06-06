use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SscTag {
    pub key: Option<String>, // None for comments or empty lines
    pub value: String, // The value of the tag (without starting '#' and ending ';'), or the comment line
    pub is_comment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SscChart {
    pub tags: Vec<SscTag>, // Tags inside the chart (e.g. STEPSTYPE, DIFFICULTY, METER, BPMS, NOTES)
    pub notes_raw: String, // The raw notes content, including rows, measures, and comments, ending with ';'
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SscDocument {
    pub global_tags: Vec<SscTag>,
    pub charts: Vec<SscChart>,
    pub trailing_comments: Vec<SscTag>,
}

enum ParserState {
    Normal,
    ReadingTag {
        key: String,
        accumulated_value: String,
    },
    ReadingNotes,
}

impl SscDocument {
    pub fn parse<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(Self::parse_str(&contents))
    }

    pub fn parse_str(contents: &str) -> Self {
        let mut global_tags = Vec::new();
        let mut charts = Vec::new();
        let mut trailing_comments = Vec::new();
        let mut pending_comments = Vec::new();

        let mut state = ParserState::Normal;

        // Split by lines, but keep track of raw line endings
        let lines: Vec<&str> = contents.lines().collect();

        for line in lines {
            let trimmed = line.trim();

            match &mut state {
                ParserState::Normal => {
                    if trimmed.starts_with("//") || trimmed.is_empty() {
                        pending_comments.push(SscTag {
                            key: None,
                            value: line.to_string(),
                            is_comment: true,
                        });
                    } else if trimmed.starts_with('#') {
                        // Start of a tag
                        // Extract key and value
                        let without_hash = &trimmed[1..];
                        if let Some(colon_idx) = without_hash.find(':') {
                            let key = without_hash[..colon_idx].to_string();
                            let after_colon = &without_hash[colon_idx + 1..];

                            if key == "NOTEDATA" {
                                // Start a new chart!
                                // Flush pending comments to the new chart's tags
                                let mut tags = std::mem::take(&mut pending_comments);
                                // Check if this is a tag with a semicolon
                                let value = if after_colon.ends_with(';') {
                                    after_colon[..after_colon.len() - 1].to_string()
                                } else {
                                    after_colon.to_string()
                                };
                                tags.push(SscTag {
                                    key: Some(key.clone()),
                                    value,
                                    is_comment: false,
                                });
                                charts.push(SscChart {
                                    tags,
                                    notes_raw: String::new(),
                                });
                            } else if key == "NOTES" {
                                // Transition to notes reading
                                // Flush pending comments to last chart
                                if let Some(last_chart) = charts.last_mut() {
                                    last_chart.tags.append(&mut pending_comments);
                                    last_chart.tags.push(SscTag {
                                        key: Some(key),
                                        value: after_colon.to_string(),
                                        is_comment: false,
                                    });
                                } else {
                                    // Ssc syntax error: NOTES outside of chart. Add to global.
                                    let mut tags = std::mem::take(&mut pending_comments);
                                    tags.push(SscTag {
                                        key: Some(key),
                                        value: after_colon.to_string(),
                                        is_comment: false,
                                    });
                                    global_tags.append(&mut tags);
                                }
                                state = ParserState::ReadingNotes;
                            } else {
                                // Normal tag
                                if after_colon.ends_with(';') {
                                    let val = after_colon[..after_colon.len() - 1].to_string();
                                    let tag = SscTag {
                                        key: Some(key),
                                        value: val,
                                        is_comment: false,
                                    };
                                    if charts.is_empty() {
                                        global_tags.append(&mut pending_comments);
                                        global_tags.push(tag);
                                    } else if let Some(last_chart) = charts.last_mut() {
                                        last_chart.tags.append(&mut pending_comments);
                                        last_chart.tags.push(tag);
                                    }
                                } else {
                                    // Semicolon is missing, spans multiple lines
                                    state = ParserState::ReadingTag {
                                        key,
                                        accumulated_value: after_colon.to_string(),
                                    };
                                }
                            }
                        } else {
                            // Syntax error or tag without colon, skip or treat as comment
                            pending_comments.push(SscTag {
                                key: None,
                                value: line.to_string(),
                                is_comment: true,
                            });
                        }
                    } else {
                        // Lines not starting with # or // inside Normal state
                        pending_comments.push(SscTag {
                            key: None,
                            value: line.to_string(),
                            is_comment: true,
                        });
                    }
                }
                ParserState::ReadingTag {
                    key,
                    accumulated_value,
                } => {
                    if trimmed.ends_with(';') {
                        accumulated_value.push('\n');
                        accumulated_value.push_str(&line[..line.len() - 1]); // exclude ';'
                        let tag = SscTag {
                            key: Some(key.clone()),
                            value: accumulated_value.clone(),
                            is_comment: false,
                        };
                        if charts.is_empty() {
                            global_tags.append(&mut pending_comments);
                            global_tags.push(tag);
                        } else if let Some(last_chart) = charts.last_mut() {
                            last_chart.tags.append(&mut pending_comments);
                            last_chart.tags.push(tag);
                        }
                        state = ParserState::Normal;
                    } else {
                        accumulated_value.push('\n');
                        accumulated_value.push_str(line);
                    }
                }
                ParserState::ReadingNotes => {
                    if let Some(last_chart) = charts.last_mut() {
                        if !last_chart.notes_raw.is_empty() {
                            last_chart.notes_raw.push('\n');
                        }
                        last_chart.notes_raw.push_str(line);
                    }

                    // Check if notes block ends on this line
                    // The block ends with a semicolon ';' that is not part of a comment
                    let has_semicolon = if let Some(comment_start) = trimmed.find("//") {
                        trimmed[..comment_start].contains(';')
                    } else {
                        trimmed.contains(';')
                    };

                    if has_semicolon {
                        state = ParserState::Normal;
                    }
                }
            }
        }

        // Flush any remaining comments
        if !pending_comments.is_empty() {
            trailing_comments.append(&mut pending_comments);
        }

        Self {
            global_tags,
            charts,
            trailing_comments,
        }
    }

    pub fn serialize<W: Write>(&self, mut writer: W) -> io::Result<()> {
        // 1. Write Global Tags
        for tag in &self.global_tags {
            if tag.is_comment {
                writeln!(writer, "{}", tag.value)?;
            } else if let Some(key) = &tag.key {
                writeln!(writer, "#{}:{};", key, tag.value)?;
            }
        }

        // 2. Write Charts
        for chart in &self.charts {
            for tag in &chart.tags {
                if tag.is_comment {
                    writeln!(writer, "{}", tag.value)?;
                } else if let Some(key) = &tag.key {
                    if key == "NOTES" {
                        writeln!(writer, "#NOTES:")?;
                    } else {
                        writeln!(writer, "#{}:{};", key, tag.value)?;
                    }
                }
            }
            writeln!(writer, "{}", chart.notes_raw)?;
        }

        // 3. Write Trailing Comments
        for tag in &self.trailing_comments {
            writeln!(writer, "{}", tag.value)?;
        }

        Ok(())
    }

    pub fn serialize_to_string(&self) -> String {
        let mut buf = Vec::new();
        self.serialize(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_fixture_path() -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("src");
        p.push("ssc");
        p.push("test_fixtures");
        p.push("mini_sample.ssc");
        p
    }

    #[test]
    fn test_parse_mini_sample_ssc() {
        let path = get_fixture_path();
        assert!(
            path.exists(),
            "Mini sample .ssc file should exist at {:?}",
            path
        );

        let doc = SscDocument::parse(&path).expect("Should parse Mini Sample .ssc successfully");
        assert!(
            !doc.global_tags.is_empty(),
            "Should have parsed global tags"
        );
        assert!(
            !doc.charts.is_empty(),
            "Should have parsed at least one chart"
        );

        // Verify some metadata tag is correct
        let title_tag = doc
            .global_tags
            .iter()
            .find(|t| t.key.as_deref() == Some("TITLE"));
        assert!(title_tag.is_some());
        assert_eq!(title_tag.unwrap().value.trim(), "Mini Sample");

        // Roundtrip check
        let serialized = doc.serialize_to_string();
        let doc2 = SscDocument::parse_str(&serialized);
        assert_eq!(
            doc.charts.len(),
            doc2.charts.len(),
            "Roundtrip charts count should match"
        );
        assert_eq!(
            doc.global_tags.len(),
            doc2.global_tags.len(),
            "Roundtrip global tags count should match"
        );
    }

    #[test]
    fn test_add_chart_preserves_existing() {
        let path = get_fixture_path();
        let mut doc = SscDocument::parse(&path).expect("Should parse Mini Sample .ssc");
        let original_charts_count = doc.charts.len();

        // Capture original charts to compare later
        let original_charts = doc.charts.clone();

        // Create a new chart
        let mut new_chart_tags = Vec::new();
        new_chart_tags.push(SscTag {
            key: Some("NOTEDATA".to_string()),
            value: "".to_string(),
            is_comment: false,
        });
        new_chart_tags.push(SscTag {
            key: Some("STEPSTYPE".to_string()),
            value: "pump-single".to_string(),
            is_comment: false,
        });
        new_chart_tags.push(SscTag {
            key: Some("DESCRIPTION".to_string()),
            value: "AI Generated S99".to_string(),
            is_comment: false,
        });
        new_chart_tags.push(SscTag {
            key: Some("METER".to_string()),
            value: "99".to_string(),
            is_comment: false,
        });
        new_chart_tags.push(SscTag {
            key: Some("DIFFICULTY".to_string()),
            value: "Edit".to_string(),
            is_comment: false,
        });
        new_chart_tags.push(SscTag {
            key: Some("NOTES".to_string()),
            value: "".to_string(),
            is_comment: false,
        });

        let new_chart = SscChart {
            tags: new_chart_tags,
            notes_raw: "00000\n00000\n00000\n00000\n;".to_string(),
        };

        // Append chart
        doc.charts.push(new_chart);

        // Serialize and parse back
        let serialized = doc.serialize_to_string();
        let parsed_back = SscDocument::parse_str(&serialized);

        assert_eq!(parsed_back.charts.len(), original_charts_count + 1);

        // Verify original charts remain identical
        for i in 0..original_charts_count {
            assert_eq!(
                parsed_back.charts[i].notes_raw,
                original_charts[i].notes_raw
            );
            assert_eq!(
                parsed_back.charts[i]
                    .tags
                    .iter()
                    .filter(|t| !t.is_comment)
                    .collect::<Vec<_>>(),
                original_charts[i]
                    .tags
                    .iter()
                    .filter(|t| !t.is_comment)
                    .collect::<Vec<_>>()
            );
        }

        // Verify new chart is at the end
        let parsed_new_chart = parsed_back.charts.last().unwrap();
        let meter_tag = parsed_new_chart
            .tags
            .iter()
            .find(|t| t.key.as_deref() == Some("METER"));
        assert!(meter_tag.is_some());
        assert_eq!(meter_tag.unwrap().value, "99");
        assert_eq!(parsed_new_chart.notes_raw, "00000\n00000\n00000\n00000\n;");
    }
}
