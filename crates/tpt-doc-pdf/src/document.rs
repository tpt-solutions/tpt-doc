use std::fmt::Write as _;
use std::io::Write as _;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use tpt_doc_core::DocError;

use crate::image::{Image, ImageFilter};
use crate::xref::XrefTable;
use crate::{Font, Page};

/// A file attached to the document (PDF 2.0 §7.11.3 associated files).
#[derive(Debug, Clone)]
pub struct Attachment {
    /// File name as shown to readers (e.g. `"factur-x.xml"`).
    pub name: String,
    /// MIME subtype encoded into `/Subtype` (e.g. `"text/xml"`); `/` becomes
    /// the PDF name-escape `#2F`.
    pub mime_subtype: String,
    /// The raw file bytes.
    pub data: Vec<u8>,
}

/// A PDF document: the root object that owns pages, fonts, images, and
/// file attachments.
#[derive(Debug, Default)]
pub struct Document {
    pages: Vec<Page>,
    fonts: Vec<Font>,
    images: Vec<Image>,
    attachments: Vec<Attachment>,
}

impl Document {
    /// Create a new empty document.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Embed one of the 14 PDF standard (built-in) fonts.
    ///
    /// Returns the font id to pass to [`Page::text`]. Duplicate fonts are
    /// embedded once.
    pub fn embed_builtin_font(&mut self, font: Font) -> usize {
        if let Some(existing) = self.fonts.iter().position(|f| *f == font) {
            return existing;
        }
        let id = self.fonts.len();
        self.fonts.push(font);
        id
    }

    /// Embed a raster image.
    ///
    /// Returns the image id to pass to [`Page::image`].
    #[must_use]
    pub fn embed_image(&mut self, image: Image) -> usize {
        let id = self.images.len();
        self.images.push(image);
        id
    }

    /// Add a page to the document.
    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    /// Attach a file to the document (e.g. a Factur-X/ZUGFeRD invoice XML).
    ///
    /// The attachment is registered in the catalog's `/AF` array and the
    /// `/Names /EmbeddedFiles` name tree, with `/AFRelationship /Data`.
    pub fn attach_file(
        &mut self,
        name: impl Into<String>,
        mime_subtype: impl Into<String>,
        data: Vec<u8>,
    ) {
        self.attachments.push(Attachment {
            name: name.into(),
            mime_subtype: mime_subtype.into(),
            data,
        });
    }

    /// Serialize the document to PDF bytes.
    ///
    /// Layout is deterministic: fixed object numbering, no timestamps, and
    /// content streams compressed with a fixed zlib level, so identical
    /// documents produce byte-identical output.
    ///
    /// # Errors
    /// Returns [`DocError`] on serialization failure.
    pub fn write(self) -> Result<Vec<u8>, DocError> {
        let mut writer = ObjectWriter::new();

        // Object numbering plan:
        //   1: Catalog, 2: Pages, 3..: fonts, then images, then two objects
        //   per attachment (embedded file + filespec), then per page: page
        //   object + content stream.
        #[allow(clippy::cast_possible_truncation)] // object counts far below u32::MAX
        let font_ids = 3u32;
        #[allow(clippy::cast_possible_truncation)]
        let image_ids = font_ids + self.fonts.len() as u32;
        #[allow(clippy::cast_possible_truncation)]
        let attachment_ids = image_ids + self.images.len() as u32;
        #[allow(clippy::cast_possible_truncation)]
        let page_ids_start = attachment_ids + 2 * self.attachments.len() as u32;

        // 1: Catalog — references the attachment filespecs deterministically.
        let catalog_body = catalog_body(&self.attachments, attachment_ids);
        writer.write_object(&catalog_body);

        // 2: Pages — the kid ids are deterministic, so this can be written
        // before the page objects themselves.
        let kids: Vec<String> = (0..self.pages.len())
            .map(|i| {
                #[allow(clippy::cast_possible_truncation)] // page counts far below u32::MAX
                let kid = page_ids_start + 2 * i as u32;
                format!("{kid} 0 R")
            })
            .collect();
        writer.write_object(
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>\n",
                kids.join(" "),
                self.pages.len()
            )
            .as_bytes(),
        );

        // Fonts: Type1 built-ins need no embedding.
        for font in &self.fonts {
            let body = format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>\n",
                font.pdf_name()
            );
            writer.write_object(body.as_bytes());
        }

        // Images: XObjects.
        for image in &self.images {
            let filter = match image.filter {
                ImageFilter::DctDecode => "/DCTDecode",
                ImageFilter::FlateDecode => "/FlateDecode",
            };
            let header = format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter {filter} /Length {} >>\nstream\n",
                image.width,
                image.height,
                image.data.len()
            );
            let mut body = header.into_bytes();
            body.extend_from_slice(&image.data);
            body.extend_from_slice(b"\nendstream");
            writer.write_object(&body);
        }

        // Attachments: embedded file stream + filespec per attachment.
        for attachment in &self.attachments {
            write_attachment(&mut writer, attachment);
        }

        // Pages and their content streams.
        for (index, page) in self.pages.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let page_id = page_ids_start + 2 * index as u32;

            let font_resources: Vec<String> = page
                .used_fonts()
                .iter()
                .map(|id| {
                    format!(
                        "/F{} {} 0 R",
                        id + 1,
                        font_ids + u32::try_from(*id).unwrap_or(u32::MAX)
                    )
                })
                .collect();
            let image_resources: Vec<String> = page
                .used_images()
                .iter()
                .map(|id| {
                    format!(
                        "/Im{} {} 0 R",
                        id + 1,
                        image_ids + u32::try_from(*id).unwrap_or(u32::MAX)
                    )
                })
                .collect();
            let body = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {} >> /XObject << {} >> >> /Contents {} 0 R >>\n",
                format_number(page.width),
                format_number(page.height),
                font_resources.join(" "),
                image_resources.join(" "),
                page_id + 1
            );
            writer.write_object(body.as_bytes());

            let content = content_stream(page.operations());
            let compressed = deflate(&content)?;
            let stream = format!(
                "<< /Length {} /Filter /FlateDecode >>\nstream\n",
                compressed.len()
            );
            let mut stream_bytes = stream.into_bytes();
            stream_bytes.extend_from_slice(&compressed);
            stream_bytes.extend_from_slice(b"\nendstream");
            writer.write_object(&stream_bytes);
        }

        // Cross-reference table and trailer.
        Ok(writer.finish())
    }
}

/// Emit the two objects of one file attachment: the embedded file stream
/// and its filespec dictionary.
fn write_attachment(writer: &mut ObjectWriter, attachment: &Attachment) {
    let subtype = attachment.mime_subtype.replace('/', "#2F");
    let header = format!(
        "<< /Type /EmbeddedFile /Subtype /{subtype} /Length {} >>
stream
",
        attachment.data.len()
    );
    let mut body = header.into_bytes();
    body.extend_from_slice(&attachment.data);
    body.extend_from_slice(
        b"
endstream",
    );
    writer.write_object(&body);

    let name = pdf_name_string(&attachment.name);
    let filespec = format!(
        "<< /Type /Filespec /F ({name}) /UF ({name}) /EF << /F {} 0 R /UF {} 0 R >> /AFRelationship /Data >>
",
        writer.next_object_id(),
        writer.next_object_id()
    );
    writer.write_object(filespec.as_bytes());
}

/// Build the catalog object body. With attachments, the catalog carries the
/// `/AF` array and the `/Names /EmbeddedFiles` name tree, both referencing
/// the deterministic filespec object ids.
fn catalog_body(attachments: &[Attachment], attachment_ids: u32) -> Vec<u8> {
    if attachments.is_empty() {
        return b"<< /Type /Catalog /Pages 2 0 R >>
"
        .to_vec();
    }
    let af: Vec<String> = (0..attachments.len())
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)]
            let filespec_id = attachment_ids + 2 * i as u32 + 1;
            format!("{filespec_id} 0 R")
        })
        .collect();
    let names: Vec<String> = (0..attachments.len())
        .map(|i| {
            #[allow(clippy::cast_possible_truncation)]
            let filespec_id = attachment_ids + 2 * i as u32 + 1;
            let name = pdf_name_string(&attachments[i].name);
            format!("({name}) {filespec_id} 0 R")
        })
        .collect();
    format!(
        "<< /Type /Catalog /Pages 2 0 R /AF [{}] /Names << /EmbeddedFiles << /Names [{}] >> >> >>
",
        af.join(" "),
        names.join(" ")
    )
    .into_bytes()
}

/// Serializes objects into the output buffer while recording byte-exact
/// xref offsets.
struct ObjectWriter {
    out: Vec<u8>,
    xref: XrefTable,
    object_count: u32,
}

impl ObjectWriter {
    /// Start a document with the fixed PDF header (no timestamps).
    fn new() -> Self {
        let mut out = Vec::new();
        out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
        Self {
            out,
            xref: XrefTable::new(),
            object_count: 0,
        }
    }

    /// The object number the next `write_object` call will assign.
    fn next_object_id(&self) -> u32 {
        self.object_count + 1
    }

    /// Write one indirect object, registering its offset in the xref.
    fn write_object(&mut self, body: &[u8]) {
        self.object_count += 1;
        self.xref.push(self.out.len() as u64);
        let id = self.object_count;
        self.out
            .extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    /// Append the byte-exact xref table, trailer, and EOF marker.
    fn finish(mut self) -> Vec<u8> {
        let xref_offset = self.out.len() as u64;
        self.out.extend_from_slice(&self.xref.serialize());
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            self.object_count + 1
        );
        self.out.extend_from_slice(trailer.as_bytes());
        self.out
    }
}

/// Render content operations to PDF content-stream text.
fn content_stream(operations: &[crate::page::ContentOp]) -> Vec<u8> {
    let mut stream = String::new();
    for op in operations {
        match op {
            crate::page::ContentOp::Text {
                content,
                font_id,
                size,
                x,
                y,
            } => {
                let _ = writeln!(
                    stream,
                    "BT /F{} {} Tf {} {} Td ({}) Tj ET",
                    font_id + 1,
                    format_number(*size),
                    format_number(*x),
                    format_number(*y),
                    escape_pdf_string(content)
                );
            }
            crate::page::ContentOp::Image {
                image_id,
                x,
                y,
                width,
                height,
            } => {
                let _ = writeln!(
                    stream,
                    "q {} 0 0 {} {} {} cm /Im{} Do Q",
                    format_number(*width),
                    format_number(*height),
                    format_number(*x),
                    format_number(*y),
                    image_id + 1
                );
            }
            crate::page::ContentOp::Rect {
                x,
                y,
                width,
                height,
            } => {
                let _ = writeln!(
                    stream,
                    "{} {} {} {} re S",
                    format_number(*x),
                    format_number(*y),
                    format_number(*width),
                    format_number(*height)
                );
            }
        }
    }
    stream.into_bytes()
}

/// zlib-compress content at a fixed level for deterministic output.
fn deflate(content: &[u8]) -> Result<Vec<u8>, DocError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(content)
        .map_err(|e| DocError::invalid_format(format!("content deflate failed: {e}")))?;
    encoder
        .finish()
        .map_err(|e| DocError::invalid_format(format!("content deflate failed: {e}")))
}

/// Format a number without a trailing `.0`: integers stay integers.
fn format_number(value: f32) -> String {
    if value.fract() == 0.0 && value.abs() < 1e10 {
        // Fractional part is intentionally discarded for whole numbers.
        #[allow(clippy::cast_possible_truncation)]
        let whole = value as i64;
        format!("{whole}")
    } else {
        let formatted = format!("{value:.2}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

/// Escape a name for inclusion in a PDF literal string (parens/backslash).
fn pdf_name_string(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\\' | '(' | ')' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Escape a PDF literal string: backslashes, parens, and non-Latin-1 bytes.
fn escape_pdf_string(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '(' => escaped.push_str("\\("),
            ')' => escaped.push_str("\\)"),
            c if (c as u32) < 0x100 => escaped.push(c),
            // Beyond Latin-1 the built-in WinAnsi fonts cannot render the
            // glyph; substitute a placeholder rather than emit mojibake.
            _ => escaped.push('?'),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_trims_fractional_zeros() {
        assert_eq!(format_number(595.0), "595");
        assert_eq!(format_number(72.5), "72.5");
        assert_eq!(format_number(72.25), "72.25");
        assert_eq!(format_number(72.20), "72.2");
    }

    #[test]
    fn pdf_strings_escape_specials_and_non_latin1() {
        assert_eq!(escape_pdf_string("a(b)c\\d"), "a\\(b\\)c\\\\d");
        assert_eq!(escape_pdf_string("café"), "café");
        assert_eq!(escape_pdf_string("emoji \u{1F600}"), "emoji ?");
    }

    #[test]
    fn empty_document_writes_valid_structure() {
        let bytes = Document::new().write().expect("write");
        assert!(bytes.starts_with(b"%PDF-1.7\n"));
        assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
        assert!(bytes.windows(7).any(|w| w == b"trailer"));
    }
}
