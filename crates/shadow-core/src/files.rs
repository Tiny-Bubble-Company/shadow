//! Masking the contents of files inside AI requests.
//!
//! This module is decoupled from the masking engine: every entry point takes a
//! [`MaskFn`] closure that masks a piece of text and reports how many values it replaced.
//!
//! Supported formats:
//! - text-like files (`text/*`, JSON, XML, CSV, Markdown, YAML, HTML, or UTF-8 files with a
//!   text extension): masked as UTF-8 text;
//! - DOCX / XLSX / PPTX: text nodes are rewritten in place and the archive is re-zipped; every
//!   other entry is copied byte-for-byte;
//! - PDF: per [`FilesConfig::pdf`], the text layer is extracted and sent as masked plain text,
//!   or the request is blocked, or the PDF is passed through;
//! - images: per [`FilesConfig::images`], passed through or blocked (no OCR);
//! - anything else: forwarded untouched ("not scanned").
//!
//! Known trade-off for Office documents: when a sensitive value is split across several runs
//! of one paragraph (`<w:p>`, `<a:p>`, a shared string `<si>`), the whole paragraph's masked
//! text is put into its first run and the other runs are emptied, so run-level formatting
//! inside that paragraph collapses to the first run's.

use std::fmt;
use std::io::{Cursor, Read, Write};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use regex::Regex;
use serde_json::{Map, Value};

use crate::config::{FileAction, FilesConfig};

/// Masks a piece of text: returns `(masked text, replacements)`.
pub type MaskFn<'a> = &'a dyn Fn(&str) -> (String, usize);

/// The result of masking one file.
#[derive(Clone, Debug)]
pub struct FileOutcome {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub filename: Option<String>,
    pub replacements: usize,
    /// Something worth logging, e.g. "converted PDF to masked text" or "not scanned".
    pub note: Option<String>,
}

/// A file Shadow refuses to forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// The reason, suitable for showing to the client.
    Blocked(String),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::Blocked(r) => write!(f, "{r}"),
        }
    }
}

impl std::error::Error for FileError {}

const CONVERTED_PDF: &str = "converted PDF to masked text";
const NO_TEXT_LAYER: &str = "this PDF has no text layer (scanned?); Shadow can't mask it";
const IMAGES_BLOCKED: &str = "images are blocked by policy: Shadow can't read text in images";

const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "text",
    "md",
    "markdown",
    "csv",
    "tsv",
    "json",
    "jsonl",
    "ndjson",
    "xml",
    "yaml",
    "yml",
    "toml",
    "ini",
    "cfg",
    "conf",
    "env",
    "log",
    "html",
    "htm",
    "rtf",
    "tex",
    "sql",
    "py",
    "rs",
    "js",
    "ts",
    "jsx",
    "tsx",
    "go",
    "java",
    "kt",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "rb",
    "php",
    "sh",
    "bash",
    "zsh",
    "ps1",
    "swift",
    "scala",
    "r",
    "lua",
    "pl",
    "css",
    "scss",
    "vue",
    "svelte",
    "properties",
    "eml",
    "vcf",
    "ics",
    "srt",
    "vtt",
    "diff",
    "patch",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Text,
    Pdf,
    Ooxml,
    Image,
    Other,
}

fn extension(filename: Option<&str>) -> Option<String> {
    let name = filename?;
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (_, ext) = base.rsplit_once('.')?;
    Some(ext.to_ascii_lowercase())
}

fn mime(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn is_text_mime(m: &str) -> bool {
    m.starts_with("text/")
        || m.ends_with("+json")
        || m.ends_with("+xml")
        || matches!(
            m,
            "application/json"
                | "application/xml"
                | "application/csv"
                | "application/x-yaml"
                | "application/yaml"
                | "application/x-ndjson"
                | "application/javascript"
                | "application/x-sh"
                | "application/sql"
                | "application/toml"
                | "application/markdown"
                | "application/rtf"
        )
}

fn is_ooxml_zip(bytes: &[u8]) -> bool {
    let Ok(archive) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    let ok = archive
        .file_names()
        .any(|n| n.starts_with("word/") || n.starts_with("xl/") || n.starts_with("ppt/"));
    ok
}

fn classify(filename: Option<&str>, content_type: &str, bytes: &[u8]) -> Format {
    // Magic bytes win over declared types.
    if bytes.starts_with(b"%PDF") {
        return Format::Pdf;
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return if is_ooxml_zip(bytes) {
            Format::Ooxml
        } else {
            Format::Other
        };
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xFF\xD8\xFF")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
    {
        return Format::Image;
    }
    let m = mime(content_type);
    let ext = extension(filename);
    if m == "application/pdf" || ext.as_deref() == Some("pdf") {
        return Format::Pdf;
    }
    if m.starts_with("image/")
        || matches!(
            ext.as_deref(),
            Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "svg")
        )
    {
        return Format::Image;
    }
    let utf8 = std::str::from_utf8(bytes).is_ok();
    let text_ext = ext.as_deref().is_some_and(|e| TEXT_EXTENSIONS.contains(&e));
    if utf8 && (is_text_mime(&m) || text_ext) {
        return Format::Text;
    }
    Format::Other
}

fn untouched(
    filename: Option<&str>,
    content_type: &str,
    bytes: &[u8],
    note: Option<String>,
) -> FileOutcome {
    FileOutcome {
        bytes: bytes.to_vec(),
        content_type: content_type.to_string(),
        filename: filename.map(str::to_string),
        replacements: 0,
        note,
    }
}

/// Mask one file by its name/content type (magic bytes are sniffed too).
pub fn mask_file(
    mask: MaskFn,
    cfg: &FilesConfig,
    filename: Option<&str>,
    content_type: &str,
    bytes: &[u8],
) -> Result<FileOutcome, FileError> {
    if !cfg.enabled {
        return Ok(untouched(filename, content_type, bytes, None));
    }
    if bytes.len() > cfg.max_bytes {
        let what = filename.unwrap_or("file");
        if cfg.block_oversize {
            return Err(FileError::Blocked(format!(
                "{what} is {} bytes, over the {} byte limit; Shadow can't scan it",
                bytes.len(),
                cfg.max_bytes
            )));
        }
        return Ok(untouched(
            filename,
            content_type,
            bytes,
            Some(format!("{what} is over the size limit; not scanned")),
        ));
    }
    match classify(filename, content_type, bytes) {
        Format::Text => {
            let text = std::str::from_utf8(bytes).unwrap_or_default();
            let (masked, n) = mask(text);
            Ok(FileOutcome {
                bytes: if n > 0 {
                    masked.into_bytes()
                } else {
                    bytes.to_vec()
                },
                content_type: content_type.to_string(),
                filename: filename.map(str::to_string),
                replacements: n,
                note: None,
            })
        }
        Format::Ooxml => match mask_ooxml(mask, bytes) {
            Ok((out, n)) => Ok(FileOutcome {
                bytes: out,
                content_type: content_type.to_string(),
                filename: filename.map(str::to_string),
                replacements: n,
                note: None,
            }),
            Err(e) => Ok(untouched(
                filename,
                content_type,
                bytes,
                Some(format!("could not read Office document ({e}); not scanned")),
            )),
        },
        Format::Pdf => match cfg.pdf {
            FileAction::Block => Err(FileError::Blocked(
                "PDFs are blocked by policy: Shadow is configured not to forward PDF files".into(),
            )),
            FileAction::Pass => Ok(untouched(
                filename,
                content_type,
                bytes,
                Some("PDF forwarded untouched (files.pdf = pass); not scanned".into()),
            )),
            FileAction::Text => {
                let text = pdf_text(bytes)?;
                let (masked, n) = mask(&text);
                Ok(FileOutcome {
                    bytes: masked.into_bytes(),
                    content_type: "text/plain".into(),
                    filename: filename.map(|f| format!("{f}.txt")),
                    replacements: n,
                    note: Some(CONVERTED_PDF.into()),
                })
            }
        },
        Format::Image => match cfg.images {
            FileAction::Block => Err(FileError::Blocked(IMAGES_BLOCKED.into())),
            FileAction::Pass | FileAction::Text => Ok(untouched(
                filename,
                content_type,
                bytes,
                Some("image forwarded untouched; Shadow can't read text in images".into()),
            )),
        },
        Format::Other => Ok(untouched(
            filename,
            content_type,
            bytes,
            Some("not scanned".into()),
        )),
    }
}

fn pdf_text(bytes: &[u8]) -> Result<String, FileError> {
    let res = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem(bytes));
    match res {
        Ok(Ok(text)) if !text.trim().is_empty() => Ok(text),
        Ok(Ok(_)) => Err(FileError::Blocked(NO_TEXT_LAYER.into())),
        Ok(Err(e)) => Err(FileError::Blocked(format!(
            "could not read this PDF ({e}); Shadow can't mask it"
        ))),
        Err(_) => Err(FileError::Blocked(
            "could not read this PDF (malformed); Shadow can't mask it".into(),
        )),
    }
}

// ---------------------------------------------------------------------------------------------
// Office Open XML

/// Which XML parts of an OOXML package hold text, and their (paragraph, text) element names.
fn ooxml_part(name: &str) -> Option<(&'static str, &'static str)> {
    let file = name.rsplit('/').next().unwrap_or(name);
    if !file.ends_with(".xml") {
        return None;
    }
    if let Some(rest) = name.strip_prefix("word/") {
        if !rest.contains('/')
            && [
                "document",
                "header",
                "footer",
                "comments",
                "footnotes",
                "endnotes",
            ]
            .iter()
            .any(|p| rest.starts_with(p))
        {
            return Some(("w:p", "w:t"));
        }
        return None;
    }
    if name == "xl/sharedStrings.xml" {
        return Some(("si", "t"));
    }
    if name.starts_with("xl/worksheets/") && !name["xl/worksheets/".len()..].contains('/') {
        return Some(("is", "t"));
    }
    if (name.starts_with("ppt/slides/") && !name["ppt/slides/".len()..].contains('/'))
        || (name.starts_with("ppt/notesSlides/") && !name["ppt/notesSlides/".len()..].contains('/'))
    {
        return Some(("a:p", "a:t"));
    }
    None
}

fn mask_ooxml(mask: MaskFn, bytes: &[u8]) -> zip::result::ZipResult<(Vec<u8>, usize)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    // First pass: compute the rewritten parts.
    let mut rewritten: Vec<Option<String>> = vec![None; archive.len()];
    let mut total = 0;
    for (i, slot) in rewritten.iter_mut().enumerate() {
        let mut f = archive.by_index(i)?;
        let Some((para, text)) = ooxml_part(f.name()) else {
            continue;
        };
        let mut xml = String::new();
        if f.read_to_string(&mut xml).is_err() {
            continue;
        }
        let (out, n) = mask_xml_runs(mask, &xml, para, text);
        if out != xml {
            total += n;
            *slot = Some(out);
        }
    }
    if rewritten.iter().all(Option::is_none) {
        return Ok((bytes.to_vec(), 0));
    }
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::with_capacity(bytes.len())));
    for (i, slot) in rewritten.into_iter().enumerate() {
        match slot {
            None => {
                let f = archive.by_index_raw(i)?;
                w.raw_copy_file(f)?;
            }
            Some(xml) => {
                let f = archive.by_index(i)?;
                let method = match f.compression() {
                    zip::CompressionMethod::Stored => zip::CompressionMethod::Stored,
                    _ => zip::CompressionMethod::Deflated,
                };
                let mut opts = zip::write::SimpleFileOptions::default().compression_method(method);
                if let Some(t) = f.last_modified() {
                    opts = opts.last_modified_time(t);
                }
                let name = f.name().to_string();
                drop(f);
                w.start_file(name, opts)?;
                w.write_all(xml.as_bytes())?;
            }
        }
    }
    Ok((w.finish()?.into_inner(), total))
}

/// One text node inside an XML part.
struct TextNode {
    /// Byte range of the whole `<t ...>...</t>` element.
    whole: std::ops::Range<usize>,
    open_tag: String,
    close_tag: String,
    raw: String,
    text: String,
}

/// Mask the text nodes `<text_tag ...>...</text_tag>` of `xml`, grouping them by enclosing
/// `<para_tag>` so that values split across runs are still found.
fn mask_xml_runs(mask: MaskFn, xml: &str, para_tag: &str, text_tag: &str) -> (String, usize) {
    let p = regex::escape(para_tag);
    let t = regex::escape(text_tag);
    let re = Regex::new(&format!(
        r"(?P<popen><{p}(?:\s[^>/]*)?>)|(?P<pclose></{p}>)|(?P<topen><{t}(?:\s[^>/]*)?>)(?P<text>[^<]*)</{t}>"
    ))
    .expect("valid regex");

    let mut nodes: Vec<TextNode> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into `groups`
    for c in re.captures_iter(xml) {
        if c.name("popen").is_some() {
            groups.push(Vec::new());
            stack.push(groups.len() - 1);
        } else if c.name("pclose").is_some() {
            stack.pop();
        } else if let (Some(open), Some(text)) = (c.name("topen"), c.name("text")) {
            let whole = c.get(0).expect("match").range();
            nodes.push(TextNode {
                whole,
                open_tag: open.as_str().to_string(),
                close_tag: format!("</{text_tag}>"),
                raw: text.as_str().to_string(),
                text: xml_unescape(text.as_str()),
            });
            let idx = nodes.len() - 1;
            match stack.last() {
                Some(&g) => groups[g].push(idx),
                None => groups.push(vec![idx]),
            }
        }
    }
    if nodes.is_empty() {
        return (xml.to_string(), 0);
    }

    // New text per node (None = unchanged).
    let mut new_text: Vec<Option<String>> = vec![None; nodes.len()];
    let mut preserve: Vec<bool> = vec![false; nodes.len()];
    let mut total = 0;
    for g in groups.iter().filter(|g| !g.is_empty()) {
        let individually: Vec<(String, usize)> = g.iter().map(|&i| mask(&nodes[i].text)).collect();
        let joined_masked: String = individually.iter().map(|(s, _)| s.as_str()).collect();
        if g.len() > 1 {
            let joined: String = g.iter().map(|&i| nodes[i].text.as_str()).collect();
            let (whole, n) = mask(&joined);
            if whole != joined_masked {
                // A value spans runs: collapse the paragraph's text into its first run.
                total += n;
                new_text[g[0]] = Some(whole);
                preserve[g[0]] = true;
                for &i in &g[1..] {
                    if !nodes[i].text.is_empty() {
                        new_text[i] = Some(String::new());
                    }
                }
                continue;
            }
        }
        for (&i, (m, n)) in g.iter().zip(individually) {
            if n > 0 && m != nodes[i].text {
                total += n;
                new_text[i] = Some(m);
            }
        }
    }

    let mut out = String::with_capacity(xml.len() + 64);
    let mut last = 0;
    for (i, node) in nodes.iter().enumerate() {
        let Some(text) = &new_text[i] else { continue };
        out.push_str(&xml[last..node.whole.start]);
        if preserve[i] && !node.open_tag.contains("xml:space") {
            out.push_str(&format!("<{text_tag} xml:space=\"preserve\">"));
        } else {
            out.push_str(&node.open_tag);
        }
        if *text == node.text {
            out.push_str(&node.raw);
        } else {
            out.push_str(&xml_escape(text));
        }
        out.push_str(&node.close_tag);
        last = node.whole.end;
    }
    out.push_str(&xml[last..]);
    (out, total)
}

fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let Some(end) = rest.find(';').filter(|&e| e <= 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => ent
                .strip_prefix("#x")
                .or_else(|| ent.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// JSON request bodies

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let cleaned: String;
    let s = if s.bytes().any(|b| b.is_ascii_whitespace()) {
        cleaned = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        cleaned.as_str()
    } else {
        s
    };
    STANDARD
        .decode(s)
        .or_else(|_| STANDARD_NO_PAD.decode(s))
        .or_else(|_| URL_SAFE.decode(s))
        .or_else(|_| URL_SAFE_NO_PAD.decode(s))
        .ok()
}

/// Split `data:<mime>[;params];base64,<data>` into (mime with params, payload).
fn parse_data_uri(s: &str) -> Option<(&str, &str)> {
    let rest = s.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    let meta = meta.strip_suffix(";base64")?;
    Some((meta, data))
}

/// True when masking turned a binary file into text (i.e. a PDF became plain text).
fn converted(orig_ct: &str, out: &FileOutcome) -> bool {
    out.note.as_deref() == Some(CONVERTED_PDF) && !mime(orig_ct).starts_with("text/")
}

fn changed(orig: &[u8], out: &FileOutcome) -> bool {
    out.bytes != orig
}

/// Decode a base64 payload, mask it, and say what happened. `None` = not decodable.
fn mask_b64(
    mask: MaskFn,
    cfg: &FilesConfig,
    filename: Option<&str>,
    ct: &str,
    data: &str,
) -> Result<Option<(Vec<u8>, FileOutcome)>, FileError> {
    let Some(bytes) = b64_decode(data) else {
        return Ok(None);
    };
    let out = mask_file(mask, cfg, filename, ct, &bytes)?;
    if let Some(n) = &out.note {
        tracing::info!(target: "shadow::files", "{}: {n}", filename.unwrap_or("embedded file"));
    }
    Ok(Some((bytes, out)))
}

fn file_text(name: Option<&str>, out: &FileOutcome) -> String {
    let text = String::from_utf8_lossy(&out.bytes);
    match name {
        Some(n) => format!("<file: {n}>\n{text}"),
        None => text.into_owned(),
    }
}

/// Walk a JSON request body and mask embedded files (OpenAI `input_file` and chat `file`
/// parts, `data:` URIs, Anthropic base64 `source`s, Gemini `inline_data`). Returns the
/// number of replacements.
pub fn mask_json_files(mask: MaskFn, cfg: &FilesConfig, v: &mut Value) -> Result<usize, FileError> {
    if !cfg.enabled {
        return Ok(0);
    }
    walk(mask, cfg, v)
}

fn walk(mask: MaskFn, cfg: &FilesConfig, v: &mut Value) -> Result<usize, FileError> {
    match v {
        Value::String(s) => mask_data_uri_string(mask, cfg, s, None),
        Value::Array(a) => {
            let mut n = 0;
            for x in a {
                n += walk(mask, cfg, x)?;
            }
            Ok(n)
        }
        Value::Object(o) => {
            if let Some(n) = mask_known_object(mask, cfg, o)? {
                return Ok(n);
            }
            let mut n = 0;
            for x in o.values_mut() {
                n += walk(mask, cfg, x)?;
            }
            Ok(n)
        }
        _ => Ok(0),
    }
}

/// Mask a string that is a base64 `data:` URI, re-encoding it in place.
fn mask_data_uri_string(
    mask: MaskFn,
    cfg: &FilesConfig,
    s: &mut String,
    filename: Option<&str>,
) -> Result<usize, FileError> {
    let Some((meta, data)) = parse_data_uri(s) else {
        return Ok(0);
    };
    if mime(meta).starts_with("image/") && cfg.images != FileAction::Block {
        return Ok(0);
    }
    let meta = meta.to_string();
    let Some((orig, out)) = mask_b64(mask, cfg, filename, &meta, data)? else {
        return Ok(0);
    };
    if changed(&orig, &out) {
        let ct = if converted(&meta, &out) {
            out.content_type.clone()
        } else {
            meta
        };
        *s = format!("data:{ct};base64,{}", STANDARD.encode(&out.bytes));
    }
    Ok(out.replacements)
}

fn str_field<'a>(o: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| o.get(*k).and_then(Value::as_str))
}

/// Handle the provider-specific file shapes. `None` = not a known shape (keep walking).
fn mask_known_object(
    mask: MaskFn,
    cfg: &FilesConfig,
    o: &mut Map<String, Value>,
) -> Result<Option<usize>, FileError> {
    let ty = o
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // OpenAI Responses: {"type":"input_file","file_data":"data:...","filename":"..."}
    if ty == "input_file" {
        if let Some(n) = mask_openai_file(mask, cfg, o, "input_text")? {
            return Ok(Some(n));
        }
    }
    // OpenAI Chat: {"type":"file","file":{"file_data":"data:...","filename":"..."}}
    if ty == "file" {
        if let Some(Value::Object(inner)) = o.get_mut("file") {
            let filename = str_field(inner, &["filename"]).map(str::to_string);
            if let Some(Value::String(fd)) = inner.get("file_data").cloned() {
                if let Some(res) = mask_file_data(mask, cfg, &fd, filename.as_deref())? {
                    let (n, replacement) = res;
                    match replacement {
                        FileDataResult::Text(text) => {
                            o.clear();
                            o.insert("type".into(), "text".into());
                            o.insert("text".into(), text.into());
                        }
                        FileDataResult::Data(d) => {
                            inner.insert("file_data".into(), d.into());
                        }
                        FileDataResult::Same => {}
                    }
                    return Ok(Some(n));
                }
            }
        }
    }
    // Anthropic: {"type":"document"|"image", "source":{"type":"base64","media_type":..,"data":..}}
    if let Some(Value::Object(src)) = o.get_mut("source") {
        if src.get("type").and_then(Value::as_str) == Some("base64") {
            let ct = str_field(src, &["media_type"])
                .unwrap_or("application/octet-stream")
                .to_string();
            if mime(&ct).starts_with("image/") && cfg.images != FileAction::Block {
                return Ok(Some(0));
            }
            let Some(data) = str_field(src, &["data"]).map(str::to_string) else {
                return Ok(Some(0));
            };
            let title = str_field(o, &["title"]).map(str::to_string);
            let Some((orig, out)) = mask_b64(mask, cfg, title.as_deref(), &ct, &data)? else {
                return Ok(Some(0));
            };
            if let Some(Value::Object(src)) = o.get_mut("source") {
                if converted(&ct, &out) {
                    src.clear();
                    src.insert("type".into(), "text".into());
                    src.insert("media_type".into(), "text/plain".into());
                    src.insert(
                        "data".into(),
                        String::from_utf8_lossy(&out.bytes).into_owned().into(),
                    );
                } else if changed(&orig, &out) {
                    src.insert("data".into(), STANDARD.encode(&out.bytes).into());
                }
            }
            return Ok(Some(out.replacements));
        }
    }
    // Gemini: {"inline_data"|"inlineData":{"mime_type"|"mimeType":..,"data":..}}
    for key in ["inline_data", "inlineData"] {
        let Some(Value::Object(inline)) = o.get(key) else {
            continue;
        };
        let ct = str_field(inline, &["mime_type", "mimeType"])
            .unwrap_or("application/octet-stream")
            .to_string();
        if mime(&ct).starts_with("image/") && cfg.images != FileAction::Block {
            return Ok(Some(0));
        }
        let Some(data) = str_field(inline, &["data"]).map(str::to_string) else {
            return Ok(Some(0));
        };
        let name = str_field(inline, &["display_name", "displayName"]).map(str::to_string);
        let Some((orig, out)) = mask_b64(mask, cfg, name.as_deref(), &ct, &data)? else {
            return Ok(Some(0));
        };
        if converted(&ct, &out) {
            o.clear();
            o.insert("text".into(), file_text(name.as_deref(), &out).into());
        } else if changed(&orig, &out) {
            if let Some(Value::Object(inline)) = o.get_mut(key) {
                inline.insert("data".into(), STANDARD.encode(&out.bytes).into());
            }
        }
        return Ok(Some(out.replacements));
    }
    Ok(None)
}

enum FileDataResult {
    /// The file became text (PDF): the masked text, prefixed with the file name.
    Text(String),
    /// New `file_data` value.
    Data(String),
    Same,
}

/// Mask an OpenAI `file_data` value (a `data:` URI or bare base64).
fn mask_file_data(
    mask: MaskFn,
    cfg: &FilesConfig,
    file_data: &str,
    filename: Option<&str>,
) -> Result<Option<(usize, FileDataResult)>, FileError> {
    let (ct, data, is_uri) = match parse_data_uri(file_data) {
        Some((m, d)) => (m.to_string(), d, true),
        None => (String::new(), file_data, false),
    };
    let Some((orig, out)) = mask_b64(mask, cfg, filename, &ct, data)? else {
        return Ok(None);
    };
    let res = if converted(&ct, &out) {
        FileDataResult::Text(file_text(filename, &out))
    } else if changed(&orig, &out) {
        let enc = STANDARD.encode(&out.bytes);
        FileDataResult::Data(if is_uri {
            format!("data:{ct};base64,{enc}")
        } else {
            enc
        })
    } else {
        FileDataResult::Same
    };
    Ok(Some((out.replacements, res)))
}

fn mask_openai_file(
    mask: MaskFn,
    cfg: &FilesConfig,
    o: &mut Map<String, Value>,
    text_type: &str,
) -> Result<Option<usize>, FileError> {
    let filename = str_field(o, &["filename"]).map(str::to_string);
    let Some(fd) = str_field(o, &["file_data"]).map(str::to_string) else {
        return Ok(None);
    };
    let Some((n, res)) = mask_file_data(mask, cfg, &fd, filename.as_deref())? else {
        return Ok(None);
    };
    match res {
        FileDataResult::Text(text) => {
            o.clear();
            o.insert("type".into(), text_type.into());
            o.insert("text".into(), text.into());
        }
        FileDataResult::Data(d) => {
            o.insert("file_data".into(), d.into());
        }
        FileDataResult::Same => {}
    }
    Ok(Some(n))
}

/// Replace raw base64 file payloads that are not `data:` URIs (Anthropic `source.data`,
/// Gemini `inline_data.data`) with placeholders, so a whole-body text masker can't mangle
/// them. Restore with [`unstash_binary`].
pub fn stash_binary(v: &mut Value) -> Vec<String> {
    fn go(v: &mut Value, out: &mut Vec<String>) {
        match v {
            Value::Array(a) => a.iter_mut().for_each(|x| go(x, out)),
            Value::Object(o) => {
                let is_b64_source = o.get("type").and_then(Value::as_str) == Some("base64")
                    && o.contains_key("data");
                let is_inline = o.contains_key("data")
                    && (o.contains_key("mime_type") || o.contains_key("mimeType"));
                if is_b64_source || is_inline {
                    if let Some(Value::String(s)) = o.get_mut("data") {
                        if !s.starts_with("data:") {
                            let placeholder = format!("data:shadow-stash;{}", out.len());
                            out.push(std::mem::replace(s, placeholder));
                        }
                    }
                }
                o.values_mut().for_each(|x| go(x, out));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    go(v, &mut out);
    out
}

/// Undo [`stash_binary`].
pub fn unstash_binary(v: &mut Value, mut stash: Vec<String>) {
    fn go(v: &mut Value, stash: &mut [String]) {
        match v {
            Value::String(s) => {
                if let Some(i) = s
                    .strip_prefix("data:shadow-stash;")
                    .and_then(|i| i.parse::<usize>().ok())
                {
                    if let Some(orig) = stash.get_mut(i) {
                        *s = std::mem::take(orig);
                    }
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| go(x, stash)),
            Value::Object(o) => o.values_mut().for_each(|x| go(x, stash)),
            _ => {}
        }
    }
    if !stash.is_empty() {
        go(v, &mut stash);
    }
}

// ---------------------------------------------------------------------------------------------
// multipart/form-data

fn header_param(value: &str, param: &str) -> Option<String> {
    // Split on ';' outside quotes.
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for c in value.chars() {
        if escaped {
            cur.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' if quoted => {
                cur.push(c);
                escaped = true;
            }
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ';' if !quoted => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    for p in parts.iter().skip(1) {
        let Some((k, v)) = p.split_once('=') else {
            continue;
        };
        if !k.trim().eq_ignore_ascii_case(param) {
            continue;
        }
        let v = v.trim();
        return Some(
            match v.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
                Some(q) => q.replace("\\\"", "\"").replace("\\\\", "\\"),
                None => v.to_string(),
            },
        );
    }
    None
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > hay.len() || needle.is_empty() {
        return None;
    }
    hay[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Mask one multipart part (headers + body, without the surrounding delimiters).
fn mask_part(mask: MaskFn, cfg: &FilesConfig, part: &[u8]) -> Result<(Vec<u8>, usize), FileError> {
    let (head, sep, body) = if let Some(p) = find(part, b"\r\n\r\n", 0) {
        (&part[..p], &b"\r\n\r\n"[..], &part[p + 4..])
    } else if let Some(p) = find(part, b"\n\n", 0) {
        (&part[..p], &b"\n\n"[..], &part[p + 2..])
    } else if let Some(rest) = part.strip_prefix(b"\r\n") {
        (&part[..0], &b"\r\n"[..], rest)
    } else {
        return Ok((part.to_vec(), 0));
    };
    let Ok(head_str) = std::str::from_utf8(head) else {
        return Ok((part.to_vec(), 0));
    };
    let nl = if head_str.contains("\r\n") || sep.len() == 4 {
        "\r\n"
    } else {
        "\n"
    };
    let lines: Vec<&str> = head_str.split(nl).collect();
    let mut disposition: Option<&str> = None;
    let mut ctype: Option<&str> = None;
    for l in &lines {
        if let Some((k, v)) = l.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-disposition") {
                disposition = Some(v.trim());
            } else if k.trim().eq_ignore_ascii_case("content-type") {
                ctype = Some(v.trim());
            }
        }
    }
    let filename = disposition.and_then(|d| header_param(d, "filename"));
    let is_file = filename.is_some() || ctype.is_some_and(|c| !mime(c).starts_with("text/plain"));
    if !is_file {
        // A text field.
        let Ok(text) = std::str::from_utf8(body) else {
            return Ok((part.to_vec(), 0));
        };
        let (masked, n) = mask(text);
        if n == 0 {
            return Ok((part.to_vec(), 0));
        }
        let mut out = Vec::with_capacity(part.len() + 16);
        out.extend_from_slice(head);
        out.extend_from_slice(sep);
        out.extend_from_slice(masked.as_bytes());
        return Ok((out, n));
    }
    let ct = ctype.unwrap_or("application/octet-stream");
    let out = mask_file(mask, cfg, filename.as_deref(), ct, body)?;
    if let Some(n) = &out.note {
        tracing::info!(target: "shadow::files", "{}: {n}", filename.as_deref().unwrap_or("upload"));
    }
    if out.bytes == body && out.filename == filename && out.content_type == ct {
        return Ok((part.to_vec(), out.replacements));
    }
    let mut new_head: Vec<String> = Vec::new();
    let mut had_ct = false;
    for l in &lines {
        let key = l
            .split_once(':')
            .map(|(k, _)| k.trim().to_ascii_lowercase());
        match key.as_deref() {
            Some("content-disposition") if out.filename != filename => {
                let name = disposition
                    .and_then(|d| header_param(d, "name"))
                    .unwrap_or_default();
                let mut h = format!("Content-Disposition: form-data; name={}", quote(&name));
                if let Some(f) = &out.filename {
                    h.push_str(&format!("; filename={}", quote(f)));
                }
                new_head.push(h);
            }
            Some("content-type") => {
                had_ct = true;
                if out.content_type != ct {
                    new_head.push(format!("Content-Type: {}", out.content_type));
                } else {
                    new_head.push(l.to_string());
                }
            }
            Some("content-length") => {}
            _ => new_head.push(l.to_string()),
        }
    }
    if !had_ct && out.content_type != ct {
        new_head.push(format!("Content-Type: {}", out.content_type));
    }
    let mut res = Vec::with_capacity(out.bytes.len() + head.len() + 64);
    res.extend_from_slice(new_head.join(nl).as_bytes());
    res.extend_from_slice(sep);
    res.extend_from_slice(&out.bytes);
    Ok((res, out.replacements))
}

/// Mask a `multipart/form-data` body: every file part via [`mask_file`] (updating its
/// Content-Type / filename when converted) and every text field via `mask`. The boundary is
/// kept. Malformed bodies are returned unchanged.
pub fn mask_multipart(
    mask: MaskFn,
    cfg: &FilesConfig,
    content_type_header: &str,
    body: &[u8],
) -> Result<(Vec<u8>, usize), FileError> {
    let Some(boundary) = header_param(content_type_header, "boundary").filter(|b| !b.is_empty())
    else {
        return Ok((body.to_vec(), 0));
    };
    let delim = format!("--{boundary}").into_bytes();
    let mut nl_delim = b"\n".to_vec();
    nl_delim.extend_from_slice(&delim);

    // First delimiter: at the very start, or after a newline.
    let first = if body.starts_with(&delim) {
        0
    } else {
        match find(body, &nl_delim, 0) {
            Some(p) => p + 1,
            None => return Ok((body.to_vec(), 0)),
        }
    };
    let mut out = Vec::with_capacity(body.len());
    out.extend_from_slice(&body[..first]);
    let mut total = 0;
    let mut cursor = first;
    loop {
        // `cursor` points at a delimiter.
        let after = cursor + delim.len();
        if body[after..].starts_with(b"--") {
            out.extend_from_slice(&body[cursor..]);
            return Ok((out, total));
        }
        // Transport padding then the line break ending the delimiter line.
        let Some(eol) = find(body, b"\n", after) else {
            return Ok((body.to_vec(), 0));
        };
        let part_start = eol + 1;
        let Some(next) = find(body, &nl_delim, part_start.saturating_sub(1)) else {
            return Ok((body.to_vec(), 0));
        };
        if next < part_start {
            // Empty part directly followed by the next delimiter.
            out.extend_from_slice(&body[cursor..next + 1]);
            cursor = next + 1;
            continue;
        }
        let part_end = if next > part_start && body[next - 1] == b'\r' {
            next - 1
        } else {
            next
        };
        out.extend_from_slice(&body[cursor..part_start]);
        let (masked, n) = mask_part(mask, cfg, &body[part_start..part_end])?;
        total += n;
        out.extend_from_slice(&masked);
        out.extend_from_slice(&body[part_end..next + 1]);
        cursor = next + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_roundtrip() {
        assert_eq!(
            xml_unescape("a &amp; b &lt;c&gt; &#65;&#x42; &bogus"),
            "a & b <c> AB &bogus"
        );
        assert_eq!(xml_escape("a & <b>"), "a &amp; &lt;b&gt;");
    }

    #[test]
    fn boundary_param() {
        assert_eq!(
            header_param("multipart/form-data; boundary=\"a;b\"", "boundary").as_deref(),
            Some("a;b")
        );
        assert_eq!(
            header_param("form-data; name=\"f\"; filename=\"x.txt\"", "filename").as_deref(),
            Some("x.txt")
        );
    }

    #[test]
    fn self_closing_text_nodes_are_ignored() {
        let id = |s: &str| (s.replace("Jane", "Hui"), s.matches("Jane").count());
        let xml = r#"<w:p><w:r><w:t xml:space="preserve"/></w:r><w:r><w:t>Jane</w:t></w:r></w:p>"#;
        let (out, n) = mask_xml_runs(&id, xml, "w:p", "w:t");
        assert_eq!(n, 1);
        assert_eq!(out, xml.replace("Jane", "Hui"));
    }
}
