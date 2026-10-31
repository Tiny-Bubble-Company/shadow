//! File-content masking: text, Office documents, PDFs, images, JSON shapes and multipart.

use std::io::{Cursor, Read, Write};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use shadow_core::config::{FileAction, FilesConfig};
use shadow_core::files::{mask_file, mask_json_files, mask_multipart, FileError};

const PAIRS: &[(&str, &str)] = &[("jane@acme.com", "hui@zhilan.cn"), ("Jane Doe", "Hui Zhao")];

fn toy(text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut n = 0;
    for (real, fake) in PAIRS {
        n += out.matches(real).count();
        out = out.replace(real, fake);
    }
    (out, n)
}

fn cfg() -> FilesConfig {
    FilesConfig::default()
}

fn zip_of(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in entries {
        w.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(body.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn read_entry(bytes: &[u8], name: &str) -> String {
    let mut a = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    a.by_name(name).unwrap().read_to_string(&mut s).unwrap();
    s
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#;

fn docx() -> Vec<u8> {
    let doc = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Dear Jane</w:t></w:r><w:r><w:t xml:space="preserve"> Doe, &amp; co</w:t></w:r></w:p>
<w:p><w:r><w:t>Mail jane@acme.com &lt;now&gt;</w:t></w:r></w:p>
<w:p><w:r><w:t>Nothing here</w:t></w:r></w:p>
</w:body></w:document>"#;
    zip_of(&[
        ("[Content_Types].xml", CONTENT_TYPES),
        ("word/document.xml", doc),
        (
            "word/styles.xml",
            "<w:styles>Jane Doe stays in styles</w:styles>",
        ),
    ])
}

fn docx_text(xml: &str) -> String {
    let re = regex::Regex::new(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>").unwrap();
    re.captures_iter(xml).map(|c| c[1].to_string()).collect()
}

#[test]
fn text_file_is_masked() {
    let out = mask_file(
        &toy,
        &cfg(),
        Some("notes.txt"),
        "text/plain",
        b"Hi Jane Doe <jane@acme.com>",
    )
    .unwrap();
    assert_eq!(out.bytes, b"Hi Hui Zhao <hui@zhilan.cn>");
    assert_eq!(out.replacements, 2);
    assert_eq!(out.content_type, "text/plain");
    // octet-stream with a text extension counts as text
    let out = mask_file(
        &toy,
        &cfg(),
        Some("data.csv"),
        "application/octet-stream",
        b"name\nJane Doe\n",
    )
    .unwrap();
    assert_eq!(out.bytes, b"name\nHui Zhao\n");
}

#[test]
fn docx_masks_split_runs_and_stays_valid() {
    let input = docx();
    let out = mask_file(
        &toy,
        &cfg(),
        Some("letter.docx"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        &input,
    )
    .unwrap();
    assert_eq!(out.replacements, 2);
    let xml = read_entry(&out.bytes, "word/document.xml");
    assert!(
        !xml.contains("Jane") && !xml.contains("jane@acme.com"),
        "{xml}"
    );
    let text = docx_text(&xml);
    assert!(text.contains("Dear Hui Zhao, &amp; co"), "{text}");
    assert!(text.contains("Mail hui@zhilan.cn &lt;now&gt;"), "{text}");
    assert!(xml.contains("<w:t>Nothing here</w:t>"));
    assert!(xml.contains("<w:b/>"), "formatting kept");
    // The XML is still well-formed (balanced tags, parses as text nodes) and other entries
    // are untouched.
    assert_eq!(xml.matches("<w:p>").count(), xml.matches("</w:p>").count());
    assert_eq!(xml.matches("<w:t").count(), xml.matches("</w:t>").count());
    assert_eq!(
        read_entry(&out.bytes, "word/styles.xml"),
        "<w:styles>Jane Doe stays in styles</w:styles>"
    );
    assert_eq!(read_entry(&out.bytes, "[Content_Types].xml"), CONTENT_TYPES);
}

#[test]
fn docx_without_matches_is_returned_byte_identical() {
    let input = zip_of(&[(
        "word/document.xml",
        "<w:p><w:r><w:t>hello</w:t></w:r></w:p>",
    )]);
    let out = mask_file(
        &toy,
        &cfg(),
        Some("a.docx"),
        "application/octet-stream",
        &input,
    )
    .unwrap();
    assert_eq!(out.bytes, input);
    assert_eq!(out.replacements, 0);
}

#[test]
fn xlsx_shared_strings_and_inline_strings() {
    let shared = r#"<sst><si><t>Jane Doe</t></si><si><r><t>jane@</t></r><r><t>acme.com</t></r></si><si><t>Total</t></si></sst>"#;
    let sheet = r#"<worksheet><sheetData><row><c t="inlineStr"><is><t>Jane Doe</t></is></c><c><v>42</v></c></row></sheetData></worksheet>"#;
    let input = zip_of(&[
        ("[Content_Types].xml", CONTENT_TYPES),
        ("xl/sharedStrings.xml", shared),
        ("xl/worksheets/sheet1.xml", sheet),
    ]);
    let out = mask_file(
        &toy,
        &cfg(),
        Some("book.xlsx"),
        "application/octet-stream",
        &input,
    )
    .unwrap();
    assert_eq!(out.replacements, 3);
    let s = read_entry(&out.bytes, "xl/sharedStrings.xml");
    assert!(s.contains("<si><t>Hui Zhao</t></si>"), "{s}");
    assert!(s.contains("hui@zhilan.cn"), "{s}");
    assert!(s.contains("<t>Total</t>"));
    let w = read_entry(&out.bytes, "xl/worksheets/sheet1.xml");
    assert!(
        w.contains("<is><t>Hui Zhao</t></is>") && w.contains("<v>42</v>"),
        "{w}"
    );
}

#[test]
fn pptx_slide_and_notes() {
    let slide =
        r#"<p:sld><p:txBody><a:p><a:r><a:t>Owner: Jane Doe</a:t></a:r></a:p></p:txBody></p:sld>"#;
    let notes = r#"<p:notes><a:p><a:r><a:t>ping jane@acme.com</a:t></a:r></a:p></p:notes>"#;
    let input = zip_of(&[
        ("[Content_Types].xml", CONTENT_TYPES),
        ("ppt/slides/slide1.xml", slide),
        ("ppt/notesSlides/notesSlide1.xml", notes),
    ]);
    let out = mask_file(&toy, &cfg(), Some("deck.pptx"), "", &input).unwrap();
    assert_eq!(out.replacements, 2);
    assert!(read_entry(&out.bytes, "ppt/slides/slide1.xml").contains("<a:t>Owner: Hui Zhao</a:t>"));
    assert!(
        read_entry(&out.bytes, "ppt/notesSlides/notesSlide1.xml").contains("ping hui@zhilan.cn")
    );
}

/// A minimal one-page PDF with a Helvetica text object.
fn pdf_with_text(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 72 720 Td ({text}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[test]
fn pdf_becomes_masked_text() {
    let pdf = pdf_with_text("Contact Jane Doe at jane@acme.com");
    let out = mask_file(&toy, &cfg(), Some("cv.pdf"), "application/pdf", &pdf).unwrap();
    let text = String::from_utf8(out.bytes).unwrap();
    assert!(
        text.contains("Hui Zhao") && text.contains("hui@zhilan.cn"),
        "{text}"
    );
    assert!(!text.contains("Jane"));
    assert_eq!(out.content_type, "text/plain");
    assert_eq!(out.filename.as_deref(), Some("cv.pdf.txt"));
    assert_eq!(out.note.as_deref(), Some("converted PDF to masked text"));
}

#[test]
fn pdf_block_pass_and_garbage() {
    let pdf = pdf_with_text("Jane Doe");
    let mut c = cfg();
    c.pdf = FileAction::Block;
    assert!(matches!(
        mask_file(&toy, &c, Some("a.pdf"), "application/pdf", &pdf),
        Err(FileError::Blocked(_))
    ));
    c.pdf = FileAction::Pass;
    let out = mask_file(&toy, &c, Some("a.pdf"), "application/pdf", &pdf).unwrap();
    assert_eq!(out.bytes, pdf);
    assert!(out.note.is_some());
    // Malformed PDFs never panic; they're blocked in text mode.
    let err = mask_file(&toy, &cfg(), None, "application/pdf", b"%PDF-1.4 garbage").unwrap_err();
    assert!(err.to_string().contains("Shadow can't mask it"), "{err}");
    // No text layer.
    let empty = pdf_with_text("");
    let err = mask_file(&toy, &cfg(), None, "application/pdf", &empty).unwrap_err();
    assert!(err.to_string().contains("no text layer"), "{err}");
}

#[test]
fn images_pass_or_block() {
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDRJane Doe";
    let out = mask_file(&toy, &cfg(), Some("x.png"), "image/png", png).unwrap();
    assert_eq!(out.bytes, png);
    let mut c = cfg();
    c.images = FileAction::Block;
    let err = mask_file(&toy, &c, None, "application/octet-stream", png).unwrap_err();
    assert_eq!(
        err,
        FileError::Blocked("images are blocked by policy: Shadow can't read text in images".into())
    );
}

#[test]
fn unknown_oversize_and_disabled() {
    let bin = b"\0\x01\x02Jane Doe";
    let out = mask_file(&toy, &cfg(), Some("x.bin"), "application/octet-stream", bin).unwrap();
    assert_eq!(out.bytes, bin);
    assert_eq!(out.note.as_deref(), Some("not scanned"));

    let mut c = cfg();
    c.max_bytes = 4;
    let out = mask_file(&toy, &c, Some("a.txt"), "text/plain", b"Jane Doe").unwrap();
    assert_eq!(out.bytes, b"Jane Doe");
    c.block_oversize = true;
    assert!(mask_file(&toy, &c, Some("a.txt"), "text/plain", b"Jane Doe").is_err());

    let c = FilesConfig {
        enabled: false,
        ..cfg()
    };
    let out = mask_file(&toy, &c, Some("a.txt"), "text/plain", b"Jane Doe").unwrap();
    assert_eq!(out.bytes, b"Jane Doe");
}

fn data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

fn decode_uri(s: &str) -> Vec<u8> {
    STANDARD
        .decode(s.split_once(";base64,").unwrap().1)
        .unwrap()
}

#[test]
fn json_openai_input_file_text_and_docx() {
    let mut v = json!({"model":"gpt-4o","input":[{"role":"user","content":[
        {"type":"input_text","text":"summarise"},
        {"type":"input_file","filename":"a.txt","file_data": data_uri("text/plain", b"from Jane Doe")},
        {"type":"input_file","filename":"b.docx","file_data": data_uri("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &docx())}
    ]}]});
    let n = mask_json_files(&toy, &cfg(), &mut v).unwrap();
    assert_eq!(n, 3);
    let parts = &v["input"][0]["content"];
    assert_eq!(parts[1]["type"], "input_file");
    assert!(parts[1]["file_data"]
        .as_str()
        .unwrap()
        .starts_with("data:text/plain;base64,"));
    assert_eq!(
        decode_uri(parts[1]["file_data"].as_str().unwrap()),
        b"from Hui Zhao"
    );
    let docx = decode_uri(parts[2]["file_data"].as_str().unwrap());
    assert!(read_entry(&docx, "word/document.xml").contains("Hui Zhao"));
}

#[test]
fn json_pdf_becomes_provider_text_blocks() {
    let pdf = pdf_with_text("Jane Doe");
    let mut v = json!({"input":[{"type":"input_file","filename":"cv.pdf","file_data": data_uri("application/pdf", &pdf)}],
        "messages":[{"role":"user","content":[{"type":"file","file":{"filename":"cv.pdf","file_data": data_uri("application/pdf", &pdf)}}]}],
        "anthropic":[{"type":"document","source":{"type":"base64","media_type":"application/pdf","data": STANDARD.encode(&pdf)}}],
        "contents":[{"parts":[{"inline_data":{"mime_type":"application/pdf","data": STANDARD.encode(&pdf)}}]}]});
    let n = mask_json_files(&toy, &cfg(), &mut v).unwrap();
    assert_eq!(n, 4);
    assert_eq!(v["input"][0]["type"], "input_text");
    assert!(v["input"][0]["text"]
        .as_str()
        .unwrap()
        .starts_with("<file: cv.pdf>\n"));
    assert!(v["input"][0]["text"].as_str().unwrap().contains("Hui Zhao"));
    assert_eq!(v["messages"][0]["content"][0]["type"], "text");
    assert!(v["messages"][0]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Hui Zhao"));
    let src = &v["anthropic"][0]["source"];
    assert_eq!(src["type"], "text");
    assert_eq!(src["media_type"], "text/plain");
    assert!(src["data"].as_str().unwrap().contains("Hui Zhao"));
    assert!(v["contents"][0]["parts"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Hui Zhao"));
    assert!(v["contents"][0]["parts"][0].get("inline_data").is_none());
}

#[test]
fn json_anthropic_text_document_and_gemini_inline() {
    let mut v = json!({"messages":[{"role":"user","content":[
        {"type":"document","source":{"type":"base64","media_type":"text/plain","data": STANDARD.encode("Jane Doe wrote")}}]}],
        "contents":[{"parts":[{"inlineData":{"mimeType":"text/csv","data": STANDARD.encode("email\njane@acme.com\n")}}]}],
        "other": data_uri("text/markdown", b"# Jane Doe")});
    let n = mask_json_files(&toy, &cfg(), &mut v).unwrap();
    assert_eq!(n, 3);
    let d = &v["messages"][0]["content"][0]["source"];
    assert_eq!(d["type"], "base64");
    assert_eq!(
        STANDARD.decode(d["data"].as_str().unwrap()).unwrap(),
        b"Hui Zhao wrote"
    );
    let g = &v["contents"][0]["parts"][0]["inlineData"];
    assert_eq!(
        STANDARD.decode(g["data"].as_str().unwrap()).unwrap(),
        b"email\nhui@zhilan.cn\n"
    );
    assert_eq!(decode_uri(v["other"].as_str().unwrap()), b"# Hui Zhao");
}

#[test]
fn json_images_pass_or_block() {
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    let mut v = json!({"messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url": data_uri("image/png", png)}}]}]});
    let before = v.clone();
    assert_eq!(mask_json_files(&toy, &cfg(), &mut v).unwrap(), 0);
    assert_eq!(v, before);
    let c = FilesConfig {
        images: FileAction::Block,
        ..cfg()
    };
    assert!(mask_json_files(&toy, &c, &mut v).is_err());
    let mut a = json!({"content":[{"type":"image","source":{"type":"base64","media_type":"image/png","data": STANDARD.encode(png)}}]});
    assert!(mask_json_files(&toy, &c, &mut a).is_err());
}

#[test]
fn multipart_text_field_and_file() {
    let body = "preamble\r\n--XyZ;b\r\n\
Content-Disposition: form-data; name=\"purpose\"\r\n\r\n\
for Jane Doe\r\n--XyZ;b\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"c.txt\"\r\n\
Content-Type: text/plain\r\n\r\n\
jane@acme.com\nline2\r\n--XyZ;b--\r\n";
    let (out, n) = mask_multipart(
        &toy,
        &cfg(),
        "multipart/form-data; boundary=\"XyZ;b\"",
        body.as_bytes(),
    )
    .unwrap();
    assert_eq!(n, 2);
    let expected = body
        .replace("Jane Doe", "Hui Zhao")
        .replace("jane@acme.com", "hui@zhilan.cn");
    assert_eq!(String::from_utf8(out).unwrap(), expected);
}

#[test]
fn multipart_pdf_is_converted_and_renamed() {
    let pdf = pdf_with_text("Jane Doe");
    let mut body = b"--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"cv.pdf\"\r\nContent-Type: application/pdf\r\n\r\n".to_vec();
    body.extend_from_slice(&pdf);
    body.extend_from_slice(b"\r\n--b--\r\n");
    let (out, n) = mask_multipart(&toy, &cfg(), "multipart/form-data; boundary=b", &body).unwrap();
    assert_eq!(n, 1);
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("filename=\"cv.pdf.txt\""), "{s}");
    assert!(s.contains("Content-Type: text/plain\r\n\r\n"), "{s}");
    assert!(
        s.contains("Hui Zhao") && s.ends_with("\r\n--b--\r\n"),
        "{s}"
    );
    let c = FilesConfig {
        pdf: FileAction::Block,
        ..cfg()
    };
    assert!(mask_multipart(&toy, &c, "multipart/form-data; boundary=b", &body).is_err());
}

#[test]
fn json_values_are_untouched_without_files() {
    let mut v: Value = json!({"model":"x","messages":[{"role":"user","content":"Jane Doe"}]});
    let before = v.clone();
    assert_eq!(mask_json_files(&toy, &cfg(), &mut v).unwrap(), 0);
    assert_eq!(v, before);
}
