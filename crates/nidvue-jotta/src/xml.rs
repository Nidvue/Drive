// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Nidvue

//! JFS XML codecs.
//!
//! The JFS API is XML. Two shapes matter:
//!
//! - `mode=liststream`: a stream of flat `<file>`/`<folder>` elements plus a
//!   `<stats>` summary. Read element by element, never buffered whole.
//! - a single `<file>`/`<folder>` document with a nested `<currentRevision>`.
//!
//! Both decoders are deliberately tolerant: an unknown element is ignored, an
//! attribute may also appear as a child element (the service has done both),
//! and a malformed document is a typed error rather than a panic. Unknown
//! *values* are not guessed: a file without a size is an error, because a size
//! of zero would be a lie.

use std::collections::BTreeMap;
use std::io::BufRead;

use nidvue_core::error::{NidvueError, Result};
use nidvue_core::time::parse_jotta_time;
use nidvue_core::types::EntryKind;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// One file as reported by `liststream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    /// Path relative to the mountpoint, no leading slash.
    pub path: String,
    /// File name (the last path segment).
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// MD5 as lower-case hex, when the server reported it.
    pub md5: Option<String>,
    /// Modification time, ms since the epoch.
    pub modified_ms: i64,
    /// Creation time, ms since the epoch.
    pub created_ms: i64,
}

/// One directory as reported by `liststream`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFolder {
    /// Path relative to the mountpoint, no leading slash.
    pub path: String,
}

/// The `<stats>` trailer, used to detect a partial listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    /// Directories the server says it sent.
    pub folders: u64,
    /// Files the server says it sent.
    pub files: u64,
}

/// A parsed `liststream` response.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListStream {
    /// Directories, in stream order.
    pub folders: Vec<RemoteFolder>,
    /// Files, in stream order.
    pub files: Vec<RemoteFile>,
    /// The trailer, when present.
    pub stats: Option<Stats>,
}

impl ListStream {
    /// True when the trailer disagrees with what was parsed: the listing is
    /// partial and must not be treated as an authoritative snapshot.
    pub fn is_partial(&self) -> bool {
        match self.stats {
            Some(s) => s.files != self.files.len() as u64 || s.folders != self.folders.len() as u64,
            None => false,
        }
    }
}

/// A single entry from a `GET <path>` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    /// File or directory.
    pub kind: EntryKind,
    /// Name (last segment).
    pub name: String,
    /// Path relative to the mountpoint.
    pub path: String,
    /// Size in bytes.
    pub size: u64,
    /// MD5 as lower-case hex.
    pub md5: Option<String>,
    /// Revision state, e.g. `COMPLETED` or `INCOMPLETE`.
    pub state: Option<String>,
    /// Modification time, ms since the epoch.
    pub modified_ms: i64,
    /// Creation time, ms since the epoch.
    pub created_ms: i64,
}

impl RemoteEntry {
    /// True when the server reports an in-flight upload: the object exists but
    /// its content is not yet complete and must not be downloaded or deleted.
    pub fn is_incomplete(&self) -> bool {
        match &self.state {
            Some(s) => !s.eq_ignore_ascii_case("COMPLETED"),
            None => false,
        }
    }
}

/// A minimal element tree, enough for the JFS documents.
#[derive(Debug, Clone, Default)]
struct Node {
    name: String,
    attrs: BTreeMap<String, String>,
    text: String,
    children: Vec<Node>,
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn field(&self, name: &str) -> Option<&str> {
        if let Some(v) = self.attrs.get(name) {
            return Some(v);
        }
        self.child(name).map(|c| c.text.trim()).filter(|s| !s.is_empty())
    }

    fn child_text(&self, parent: &str, name: &str) -> Option<&str> {
        self.child(parent)?.child(name).map(|c| c.text.trim()).filter(|s| !s.is_empty())
    }
}

/// Unescape the five predefined entities and numeric character references.
fn unescape(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find('&') {
        out.push_str(&rest[..idx]);
        rest = &rest[idx..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        let replacement = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        };
        match replacement {
            Some(c) => out.push(c),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn malformed(detail: impl std::fmt::Display) -> NidvueError {
    NidvueError::Server { status: 0, message: format!("malformed JFS XML: {detail}") }
}

fn owned_attrs(e: &BytesStart<'_>) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    for attr in e.attributes().flatten() {
        let key = attr.key.local_name().as_ref().to_string();
        attrs.insert(key, unescape(attr.value.as_ref()));
    }
    attrs
}

/// Read one element, preserving nesting, and return it as a tree.
///
/// Only one top-level element is held at a time, so a stream of a million
/// `<file>` elements costs one element of memory.
fn read_node<R: BufRead>(
    reader: &mut Reader<R>,
    name: String,
    attrs: BTreeMap<String, String>,
) -> Result<Node> {
    let mut root = Node { name, attrs, ..Default::default() };
    let mut stack: Vec<Node> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                stack.push(Node {
                    name: e.local_name().as_ref().to_string(),
                    attrs: owned_attrs(&e),
                    ..Default::default()
                });
            }
            Ok(Event::Empty(e)) => {
                let child = Node {
                    name: e.local_name().as_ref().to_string(),
                    attrs: owned_attrs(&e),
                    ..Default::default()
                };
                match stack.last_mut() {
                    Some(parent) => parent.children.push(child),
                    None => root.children.push(child),
                }
            }
            Ok(Event::Text(t)) => {
                let text = unescape(t.as_ref());
                match stack.last_mut() {
                    Some(top) => top.text.push_str(&text),
                    None => root.text.push_str(&text),
                }
            }
            Ok(Event::CData(t)) => {
                let text = t.as_ref().to_string();
                match stack.last_mut() {
                    Some(top) => top.text.push_str(&text),
                    None => root.text.push_str(&text),
                }
            }
            Ok(Event::End(_)) => match stack.pop() {
                Some(node) => match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => root.children.push(node),
                },
                None => break,
            },
            Ok(Event::Eof) => return Err(malformed("unexpected end of document")),
            Ok(_) => {}
            Err(e) => return Err(malformed(e)),
        }
        buf.clear();
    }
    Ok(root)
}

/// Parse a `liststream` response body.
///
/// Unknown elements are skipped. `<file>` and `<folder>` are read as they
/// stream; nothing is buffered beyond one element.
pub fn parse_list_stream(xml: &str) -> Result<ListStream> {
    parse_list_stream_reader(xml.as_bytes())
}

/// Parse a `liststream` response from any buffered reader, so a large tree is
/// never held in memory.
pub fn parse_list_stream_reader<R: BufRead>(input: R) -> Result<ListStream> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut out = ListStream::default();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = e.local_name().as_ref().to_string();
                let attrs = owned_attrs(&e);
                if matches!(tag.as_str(), "file" | "folder" | "stats") {
                    let node = read_node(&mut reader, tag, attrs)?;
                    absorb(&mut out, &node)?;
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = e.local_name().as_ref().to_string();
                let attrs = owned_attrs(&e);
                if matches!(tag.as_str(), "file" | "folder" | "stats") {
                    let node = Node { name: tag, attrs, ..Default::default() };
                    absorb(&mut out, &node)?;
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(malformed(e)),
        }
        buf.clear();
    }
    Ok(out)
}

fn absorb(out: &mut ListStream, node: &Node) -> Result<()> {
    match node.name.as_str() {
        "file" => out.files.push(file_from(node)?),
        "folder" => {
            // A container (`<folder><folders>…<files>…`) rather than an entry:
            // recurse into it so both the streaming and the tree shape parse.
            let is_container = node.child("folders").is_some() || node.child("files").is_some();
            let entry_path = node.field("path").or_else(|| node.field("name"));
            if is_container || entry_path.is_none() {
                for child in &node.children {
                    absorb(out, child)?;
                }
            } else if let Some(path) = entry_path {
                out.folders.push(RemoteFolder { path: path.trim_matches('/').to_string() });
            }
        }
        "folders" | "files" => {
            for child in &node.children {
                absorb(out, child)?;
            }
        }
        "stats" => {
            let num = |k: &str| node.field(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            out.stats = Some(Stats { folders: num("folders"), files: num("files") });
        }
        _ => {}
    }
    Ok(())
}

fn file_from(node: &Node) -> Result<RemoteFile> {
    let raw_path = node
        .field("path")
        .ok_or_else(|| malformed("<file> without path"))?
        .trim_matches('/')
        .to_string();
    let name = node
        .field("filename")
        .or_else(|| node.field("name"))
        .map(str::to_string)
        .or_else(|| raw_path.rsplit('/').next().map(str::to_string))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| malformed("<file> without a name"))?;
    let size = node
        .field("size")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| malformed("<file> without a numeric size"))?;
    let time = |k: &str| node.field(k).and_then(|v| parse_jotta_time(v).ok()).unwrap_or(0);
    Ok(RemoteFile {
        path: raw_path,
        name,
        size,
        md5: node.field("md5").map(|s| s.to_ascii_lowercase()),
        modified_ms: time("modified"),
        created_ms: time("created"),
    })
}

/// Parse a whole document into a tree (used for single-entry responses).
fn parse_document(xml: &str) -> Result<Vec<Node>> {
    let mut reader = Reader::from_reader(xml.as_bytes());
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Node> = Vec::new();
    let mut roots: Vec<Node> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let node = Node {
                    name: e.local_name().as_ref().to_string(),
                    attrs: owned_attrs(&e),
                    ..Default::default()
                };
                stack.push(node);
            }
            Ok(Event::Empty(e)) => {
                let node = Node {
                    name: e.local_name().as_ref().to_string(),
                    attrs: owned_attrs(&e),
                    ..Default::default()
                };
                push_node(&mut stack, &mut roots, node);
            }
            Ok(Event::Text(t)) => {
                if let Some(top) = stack.last_mut() {
                    let text = unescape(t.as_ref());
                    top.text.push_str(&text);
                }
            }
            Ok(Event::CData(t)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(t.as_ref());
                }
            }
            Ok(Event::End(_)) => {
                if let Some(node) = stack.pop() {
                    push_node(&mut stack, &mut roots, node);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(malformed(e)),
        }
        buf.clear();
    }
    if !stack.is_empty() {
        return Err(malformed("unclosed element"));
    }
    Ok(roots)
}

fn push_node(stack: &mut [Node], roots: &mut Vec<Node>, node: Node) {
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None => roots.push(node),
    }
}

/// Parse a single `<file>` or `<folder>` document (the response to a `GET`).
pub fn parse_entry(xml: &str) -> Result<RemoteEntry> {
    let roots = parse_document(xml)?;
    let root = roots
        .iter()
        .find(|n| n.name == "file" || n.name == "folder")
        .ok_or_else(|| malformed("no <file> or <folder> element"))?;
    let is_file = root.name == "file";
    let name = root.field("name").or_else(|| root.field("filename")).unwrap_or("").to_string();
    let path = root.field("path").unwrap_or("").trim_matches('/').to_string();
    let size = if is_file {
        root.child_text("currentRevision", "size")
            .or_else(|| root.field("size"))
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| malformed("<file> without a size"))?
    } else {
        root.field("size").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0)
    };
    let time = |k: &str| {
        root.child_text("currentRevision", k)
            .or_else(|| root.field(k))
            .and_then(|v| parse_jotta_time(v).ok())
            .unwrap_or(0)
    };
    let md5 = root
        .child_text("currentRevision", "md5")
        .or_else(|| root.field("md5"))
        .map(|s| s.to_ascii_lowercase());
    Ok(RemoteEntry {
        kind: if is_file { EntryKind::File } else { EntryKind::Dir },
        name,
        path,
        size,
        md5,
        state: root.child_text("currentRevision", "state").map(str::to_string),
        modified_ms: time("modified"),
        created_ms: time("created"),
    })
}

/// Parse the `<error>` body the JFS API returns on failure.
pub fn parse_error(xml: &str) -> Option<(u16, String)> {
    let roots = parse_document(xml).ok()?;
    let root = roots.iter().find(|n| n.name == "error")?;
    let code = root.child("code").and_then(|c| c.text.trim().parse::<u16>().ok()).unwrap_or(0);
    let message = root
        .child("message")
        .map(|c| c.text.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| root.field("reason").map(str::to_string))
        .unwrap_or_default();
    Some((code, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTSTREAM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<folder time="2026-11-14-T10:15:00Z" host="dn-1">
  <path xml:space="preserve">/Jotta/Archive/Photos</path>
  <folders>
    <folder path="Photos/2025"/>
    <folder path="Photos/2026"/>
  </folders>
  <files>
    <file path="Photos" filename="a &amp; b.jpg" md5="1E8A7B728AB678048DF00075C9507158" size="12345"
          modified="2026-11-14T10:15:00.123Z" created="2026-11-13T09:00:00Z"/>
    <file path="Photos/2026" filename="c.png" size="7"/>
  </files>
  <stats folders="2" files="2"/>
</folder>"#;

    #[test]
    fn parses_liststream() {
        let s = parse_list_stream(LISTSTREAM).unwrap();
        assert_eq!(s.folders.len(), 2);
        assert_eq!(s.files.len(), 2);
        assert!(!s.is_partial());
        assert_eq!(s.files[0].name, "a & b.jpg");
        assert_eq!(s.files[0].size, 12345);
        assert_eq!(s.files[0].md5.as_deref(), Some("1e8a7b728ab678048df00075c9507158"));
        assert_eq!(s.files[0].path, "Photos");
        assert!(s.files[0].modified_ms > 0);
        assert_eq!(s.stats, Some(Stats { folders: 2, files: 2 }));
    }

    #[test]
    fn detects_partial_listings() {
        let xml = r#"<folder><files><file path="a" filename="x" size="1"/></files>
                     <stats folders="0" files="2"/></folder>"#;
        assert!(parse_list_stream(xml).unwrap().is_partial());
    }

    #[test]
    fn ignores_unknown_elements() {
        let xml = r#"<folder><wat><deep><deeper/></deep></wat>
                     <files><file path="a" filename="x" size="1" something="new"/></files>
                     <stats folders="0" files="1"/></folder>"#;
        let s = parse_list_stream(xml).unwrap();
        assert_eq!(s.files.len(), 1);
    }

    #[test]
    fn malformed_is_a_typed_error() {
        let err = parse_list_stream("<folder><files>").unwrap_err();
        assert!(err.to_string().contains("malformed JFS XML"));
        let err = parse_list_stream("<file path=\"a\" size=\"1\"/>trailing<").unwrap_err();
        assert!(err.to_string().contains("malformed JFS XML"));
    }

    #[test]
    fn missing_size_is_an_error_not_zero() {
        let err = parse_list_stream(r#"<file path="a" filename="x"/>"#).unwrap_err();
        assert!(err.to_string().contains("size"));
    }

    #[test]
    fn parses_single_file_entry() {
        let xml = r#"<file name="report.pdf" uuid="u1">
          <currentRevision>
            <number>3</number><state>COMPLETED</state>
            <created>2026-11-13-T09:00:00Z</created>
            <modified>2026-11-14T10:15:00Z</modified>
            <size>2048</size><md5>ABCDEF0123456789ABCDEF0123456789</md5>
          </currentRevision>
        </file>"#;
        let e = parse_entry(xml).unwrap();
        assert_eq!(e.kind, EntryKind::File);
        assert_eq!(e.size, 2048);
        assert_eq!(e.md5.as_deref(), Some("abcdef0123456789abcdef0123456789"));
        assert!(!e.is_incomplete());
        assert_eq!(e.state.as_deref(), Some("COMPLETED"));
    }

    #[test]
    fn incomplete_entries_are_flagged() {
        let xml = r#"<file name="x"><currentRevision><state>INCOMPLETE</state><size>1</size></currentRevision></file>"#;
        assert!(parse_entry(xml).unwrap().is_incomplete());
    }

    #[test]
    fn parses_directories() {
        let xml = r#"<folder name="Photos"><path>/Jotta/Archive/Photos</path><size>0</size></folder>"#;
        let e = parse_entry(xml).unwrap();
        assert_eq!(e.kind, EntryKind::Dir);
        assert_eq!(e.path, "Jotta/Archive/Photos");
    }

    #[test]
    fn parses_error_bodies() {
        let (code, msg) = parse_error(
            "<error><code>404</code><message>Not found</message><reason>x</reason></error>",
        )
        .unwrap();
        assert_eq!(code, 404);
        assert_eq!(msg, "Not found");
        assert!(parse_error("<ok/>").is_none());
    }

    #[test]
    fn unescapes_entities() {
        assert_eq!(unescape("a &amp; b &lt;c&gt; &#65; &#x42;"), "a & b <c> A B");
        assert_eq!(unescape("100% &nonsense; &"), "100% &nonsense; &");
    }
}
