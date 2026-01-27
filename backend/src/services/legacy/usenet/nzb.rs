//! NZB file parser
//!
//! NZB is an XML-based format that describes how to download files from Usenet.
//! It contains a list of files, each with multiple segments (articles) to download.
//!
//! # NZB Structure
//!
//! ```xml
//! <?xml version="1.0" encoding="utf-8"?>
//! <!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
//! <nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
//!   <file poster="user@example.com" date="1234567890" subject="My File (1/10)">
//!     <groups>
//!       <group>alt.binaries.example</group>
//!     </groups>
//!     <segments>
//!       <segment bytes="123456" number="1">article-id-1@example.com</segment>
//!       <segment bytes="123456" number="2">article-id-2@example.com</segment>
//!     </segments>
//!   </file>
//! </nzb>
//! ```

use anyhow::{Result, anyhow};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Parsed NZB file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NzbFile {
    /// Files contained in this NZB
    pub files: Vec<NzbFileEntry>,
    /// Total size in bytes
    pub total_size: u64,
    /// NZB metadata (if present)
    pub metadata: NzbMetadata,
}

/// NZB metadata from head section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NzbMetadata {
    pub title: Option<String>,
    pub password: Option<String>,
    pub tag: Option<String>,
    pub category: Option<String>,
}

/// A file entry in an NZB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NzbFileEntry {
    /// Subject line (usually contains filename)
    pub subject: String,
    /// Extracted filename from subject
    pub filename: String,
    /// Poster email/name
    pub poster: String,
    /// Post timestamp
    pub date: i64,
    /// Newsgroups where this file was posted
    pub groups: Vec<String>,
    /// Segments (articles) to download
    pub segments: Vec<NzbSegment>,
    /// Total size in bytes (sum of segments)
    pub size: u64,
}

impl NzbFileEntry {
    /// Extract filename from subject line
    ///
    /// Common patterns:
    /// - `"filename.ext" yEnc (1/10)`
    /// - `[001/100] - "filename.ext" yEnc (1/10)`
    /// - `filename.ext (1/10)`
    pub fn extract_filename(subject: &str) -> String {
        // Try to find quoted filename first
        if let Some(start) = subject.find('"') {
            if let Some(end) = subject[start + 1..].find('"') {
                return subject[start + 1..start + 1 + end].to_string();
            }
        }

        // Try to find filename before yEnc marker
        if let Some(yenc_pos) = subject.to_lowercase().find("yenc") {
            let before = subject[..yenc_pos].trim();
            // Get last space-separated word
            if let Some(last_word) = before.split_whitespace().last() {
                if last_word.contains('.') {
                    return last_word.to_string();
                }
            }
        }

        // Fall back to using subject as filename
        subject
            .chars()
            .filter(|c| !['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(c))
            .collect()
    }

    /// Check if this is a par2 file
    pub fn is_par2(&self) -> bool {
        self.filename.to_lowercase().ends_with(".par2")
    }

    /// Check if this is a rar file
    pub fn is_rar(&self) -> bool {
        let lower = self.filename.to_lowercase();
        lower.ends_with(".rar") || lower.contains(".r") // .r00, .r01, etc.
    }
}

/// A segment (article) of a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NzbSegment {
    /// Message-ID for NNTP retrieval
    pub message_id: String,
    /// Size in bytes
    pub bytes: u64,
    /// Segment number (1-based)
    pub number: u32,
}

impl NzbFile {
    /// Parse an NZB from XML bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let xml = String::from_utf8_lossy(data);
        Self::parse_str(&xml)
    }

    /// Parse an NZB from XML string
    pub fn parse_str(xml: &str) -> Result<Self> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut files = Vec::new();
        let mut metadata = NzbMetadata::default();
        let mut current_file: Option<NzbFileEntryBuilder> = None;
        let mut current_segment: Option<NzbSegmentBuilder> = None;
        let mut current_groups: Vec<String> = Vec::new();
        let mut in_groups = false;
        let mut in_segments = false;
        let mut in_head = false;
        let mut current_tag = String::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    current_tag = tag_name.clone();

                    match tag_name.as_str() {
                        "head" => in_head = true,
                        "file" => {
                            let mut builder = NzbFileEntryBuilder::new();
                            
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                let val = String::from_utf8_lossy(&attr.value).to_string();

                                match key.as_str() {
                                    "subject" => builder.subject = Some(val),
                                    "poster" => builder.poster = Some(val),
                                    "date" => {
                                        if let Ok(ts) = val.parse::<i64>() {
                                            builder.date = ts;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            current_file = Some(builder);
                            current_groups.clear();
                        }
                        "groups" => in_groups = true,
                        "segments" => in_segments = true,
                        "segment" => {
                            if in_segments {
                                let mut seg_builder = NzbSegmentBuilder::new();

                                for attr in e.attributes().flatten() {
                                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                    let val = String::from_utf8_lossy(&attr.value).to_string();

                                    match key.as_str() {
                                        "bytes" => {
                                            if let Ok(b) = val.parse::<u64>() {
                                                seg_builder.bytes = b;
                                            }
                                        }
                                        "number" => {
                                            if let Ok(n) = val.parse::<u32>() {
                                                seg_builder.number = n;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                current_segment = Some(seg_builder);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();

                    if in_head && !text.is_empty() {
                        // Handle head metadata
                        match current_tag.as_str() {
                            "meta" | "title" => metadata.title = Some(text),
                            "password" => metadata.password = Some(text),
                            "tag" => metadata.tag = Some(text),
                            "category" => metadata.category = Some(text),
                            _ => {}
                        }
                    } else if in_groups && current_tag == "group" && !text.is_empty() {
                        current_groups.push(text);
                    } else if let Some(ref mut seg) = current_segment {
                        if !text.is_empty() {
                            seg.message_id = text;
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    match tag_name.as_str() {
                        "head" => in_head = false,
                        "file" => {
                            if let Some(mut builder) = current_file.take() {
                                builder.groups = std::mem::take(&mut current_groups);
                                if let Some(file_entry) = builder.build() {
                                    files.push(file_entry);
                                }
                            }
                        }
                        "groups" => in_groups = false,
                        "segments" => in_segments = false,
                        "segment" => {
                            if let Some(seg_builder) = current_segment.take() {
                                if let Some(segment) = seg_builder.build() {
                                    if let Some(ref mut file) = current_file {
                                        file.segments.push(segment);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    current_tag.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow!("Error parsing NZB XML: {}", e));
                }
                _ => {}
            }
        }

        if files.is_empty() {
            return Err(anyhow!("NZB contains no files"));
        }

        let total_size: u64 = files.iter().map(|f| f.size).sum();

        debug!(
            file_count = files.len(),
            total_size = total_size,
            "Parsed NZB file"
        );

        Ok(NzbFile {
            files,
            total_size,
            metadata,
        })
    }

    /// Get all unique newsgroups referenced in this NZB
    pub fn all_groups(&self) -> Vec<String> {
        let mut groups: Vec<String> = self
            .files
            .iter()
            .flat_map(|f| f.groups.iter().cloned())
            .collect();
        groups.sort();
        groups.dedup();
        groups
    }

    /// Get total segment count
    pub fn total_segments(&self) -> usize {
        self.files.iter().map(|f| f.segments.len()).sum()
    }

    /// Get all message IDs in order
    pub fn all_message_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        for file in &self.files {
            for seg in &file.segments {
                ids.push(&seg.message_id);
            }
        }
        ids
    }
}

/// Builder for NzbFileEntry during parsing
struct NzbFileEntryBuilder {
    subject: Option<String>,
    poster: Option<String>,
    date: i64,
    groups: Vec<String>,
    segments: Vec<NzbSegment>,
}

impl NzbFileEntryBuilder {
    fn new() -> Self {
        Self {
            subject: None,
            poster: None,
            date: 0,
            groups: Vec::new(),
            segments: Vec::new(),
        }
    }

    fn build(self) -> Option<NzbFileEntry> {
        let subject = self.subject?;
        let filename = NzbFileEntry::extract_filename(&subject);
        let size: u64 = self.segments.iter().map(|s| s.bytes).sum();

        Some(NzbFileEntry {
            subject,
            filename,
            poster: self.poster.unwrap_or_default(),
            date: self.date,
            groups: self.groups,
            segments: self.segments,
            size,
        })
    }
}

/// Builder for NzbSegment during parsing
struct NzbSegmentBuilder {
    message_id: String,
    bytes: u64,
    number: u32,
}

impl NzbSegmentBuilder {
    fn new() -> Self {
        Self {
            message_id: String::new(),
            bytes: 0,
            number: 0,
        }
    }

    fn build(self) -> Option<NzbSegment> {
        if self.message_id.is_empty() {
            warn!("Segment with empty message ID");
            return None;
        }

        Some(NzbSegment {
            message_id: self.message_id,
            bytes: self.bytes,
            number: self.number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_NZB: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">
  <file poster="user@example.com" date="1609459200" subject="My.Movie.2021.1080p.BluRay.x264.nzb (1/5) &quot;movie.part1.rar&quot; yEnc (1/100)">
    <groups>
      <group>alt.binaries.movies</group>
      <group>alt.binaries.multimedia</group>
    </groups>
    <segments>
      <segment bytes="512000" number="1">article1@example.com</segment>
      <segment bytes="512000" number="2">article2@example.com</segment>
    </segments>
  </file>
</nzb>"#;

    #[test]
    fn test_parse_nzb() {
        let nzb = NzbFile::parse(SAMPLE_NZB.as_bytes()).unwrap();
        assert_eq!(nzb.files.len(), 1);
        assert_eq!(nzb.total_size, 1024000);

        let file = &nzb.files[0];
        assert!(file.subject.contains("My.Movie"));
        assert_eq!(file.filename, "movie.part1.rar");
        assert_eq!(file.groups.len(), 2);
        assert_eq!(file.segments.len(), 2);
    }

    #[test]
    fn test_extract_filename() {
        assert_eq!(
            NzbFileEntry::extract_filename(r#""movie.mkv" yEnc (1/10)"#),
            "movie.mkv"
        );
        assert_eq!(
            NzbFileEntry::extract_filename(r#"[001/100] - "file.rar" yEnc (1/50)"#),
            "file.rar"
        );
    }
}
