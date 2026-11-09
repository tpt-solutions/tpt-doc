//! Text extraction from PDF files.
//!
//! This reads the classic PDF structure: a cross-reference table, the trailer
//! (following `/Prev` for incremental updates), the page tree, and the content
//! streams of each page. It recovers the text shown by `Tj`, `TJ`, `'` and `"`
//! and starts a new line when the text position moves vertically.
//!
//! It is deliberately limited. Cross-reference streams and object streams
//! (PDF 1.5 compressed structure) are refused with an error rather than
//! guessed at, and encrypted files are refused. Fonts are assumed to use
//! WinAnsi encoding, which matches the built-in Helvetica, Times and Courier
//! fonts this crate writes. Text in other encodings may come out wrong.
//!
//! Stream decompression is bounded by [`Limits::max_entry_bytes`].

use std::io::Read as _;

use flate2::read::ZlibDecoder;
use tpt_doc_core::{DocError, Limits, Location};

/// Maximum depth of nested objects and page-tree nodes followed.
const MAX_DEPTH: usize = 64;

/// Extract the text of every page, in page order.
///
/// # Errors
/// Returns [`DocError`] if the bytes are not a classic-structure PDF, the
/// structure is damaged in a way that cannot be followed, the file is
/// encrypted, or a stream exceeds the default limits.
pub fn extract_pages(bytes: &[u8]) -> Result<Vec<String>, DocError> {
    extract_pages_with_limits(bytes, &Limits::default())
}

/// Extract the text of every page with explicit [`Limits`].
///
/// # Errors
/// As [`extract_pages`], with the decompressed size of each stream bounded by
/// `limits.max_entry_bytes`.
pub fn extract_pages_with_limits(bytes: &[u8], limits: &Limits) -> Result<Vec<String>, DocError> {
    let file = PdfFile::open(bytes, limits)?;
    let root = file.trailer_ref("Root")?;
    let catalog = file.get(root)?;
    let pages_ref = catalog
        .dict_get("Pages")
        .ok_or_else(|| invalid("catalog has no /Pages"))?
        .clone();

    let mut pages = Vec::new();
    collect_pages(&file, &pages_ref, &mut pages, 0)?;

    let mut out = Vec::with_capacity(pages.len());
    for page in pages {
        out.push(page_text(&file, &page)?);
    }
    Ok(out)
}

/// Extract all pages and join them with a blank line between pages.
///
/// # Errors
/// As [`extract_pages`].
pub fn extract_text(bytes: &[u8]) -> Result<String, DocError> {
    Ok(extract_pages(bytes)?.join("\n\n"))
}

fn invalid(message: &str) -> DocError {
    DocError::invalid_format(message)
}

// ---------------------------------------------------------------- lexing

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Name(String),
    Str(Vec<u8>),
    ArrOpen,
    ArrClose,
    DictOpen,
    DictClose,
    Keyword(String),
    Eof,
}

struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\0')
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

impl<'a> Lexer<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    fn skip_space(&mut self) {
        while let Some(&b) = self.data.get(self.pos) {
            if is_whitespace(b) {
                self.pos += 1;
            } else if b == b'%' {
                while let Some(&c) = self.data.get(self.pos) {
                    self.pos += 1;
                    if c == b'\n' || c == b'\r' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, DocError> {
        self.skip_space();
        let Some(&b) = self.data.get(self.pos) else {
            return Ok(Token::Eof);
        };
        match b {
            b'[' => {
                self.pos += 1;
                Ok(Token::ArrOpen)
            }
            b']' => {
                self.pos += 1;
                Ok(Token::ArrClose)
            }
            b'<' if self.data.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                Ok(Token::DictOpen)
            }
            b'>' if self.data.get(self.pos + 1) == Some(&b'>') => {
                self.pos += 2;
                Ok(Token::DictClose)
            }
            b'<' => Ok(self.hex_string()),
            b'(' => self.literal_string(),
            b'/' => Ok(Token::Name(self.name())),
            b'{' | b'}' | b')' | b'>' => {
                self.pos += 1;
                Ok(Token::Keyword((b as char).to_string()))
            }
            _ => {
                let start = self.pos;
                while let Some(&c) = self.data.get(self.pos) {
                    if is_whitespace(c) || is_delimiter(c) {
                        break;
                    }
                    self.pos += 1;
                }
                let word = &self.data[start..self.pos];
                if word.is_empty() {
                    self.pos += 1;
                    return Ok(Token::Keyword(String::new()));
                }
                match parse_number(word) {
                    Some(n) => Ok(Token::Num(n)),
                    None => Ok(Token::Keyword(String::from_utf8_lossy(word).into_owned())),
                }
            }
        }
    }

    fn name(&mut self) -> String {
        self.pos += 1; // the leading slash
        let mut out = Vec::new();
        while let Some(&c) = self.data.get(self.pos) {
            if is_whitespace(c) || is_delimiter(c) {
                break;
            }
            self.pos += 1;
            if c == b'#' {
                let hex = self.data.get(self.pos..self.pos + 2);
                if let Some(value) = hex
                    .and_then(|h| std::str::from_utf8(h).ok())
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                {
                    out.push(value);
                    self.pos += 2;
                    continue;
                }
            }
            out.push(c);
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn literal_string(&mut self) -> Result<Token, DocError> {
        self.pos += 1; // opening parenthesis
        let mut depth = 1usize;
        let mut out = Vec::new();
        while let Some(&c) = self.data.get(self.pos) {
            self.pos += 1;
            match c {
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(Token::Str(out));
                    }
                    out.push(c);
                }
                b'\\' => {
                    let Some(&e) = self.data.get(self.pos) else {
                        break;
                    };
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'(' | b')' | b'\\' => out.push(e),
                        b'\r' => {
                            if self.data.get(self.pos) == Some(&b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        b'0'..=b'7' => {
                            let mut value = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.data.get(self.pos) {
                                    Some(&d @ b'0'..=b'7') => {
                                        value = value * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((value & 0xff) as u8);
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        Err(invalid("unterminated string in PDF"))
    }

    fn hex_string(&mut self) -> Token {
        self.pos += 1; // opening angle bracket
        let mut digits = Vec::new();
        while let Some(&c) = self.data.get(self.pos) {
            self.pos += 1;
            if c == b'>' {
                break;
            }
            if c.is_ascii_hexdigit() {
                digits.push(c);
            }
        }
        if digits.len() % 2 == 1 {
            digits.push(b'0');
        }
        let bytes = digits
            .chunks(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap_or("00");
                u8::from_str_radix(text, 16).unwrap_or(0)
            })
            .collect();
        Token::Str(bytes)
    }
}

fn parse_number(word: &[u8]) -> Option<f64> {
    let text = std::str::from_utf8(word).ok()?;
    let valid = text
        .bytes()
        .enumerate()
        .all(|(i, b)| b.is_ascii_digit() || b == b'.' || (i == 0 && (b == b'+' || b == b'-')));
    if !valid || !text.bytes().any(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<f64>().ok()
}

// ---------------------------------------------------------------- objects

/// A parsed PDF object.
#[derive(Debug, Clone, PartialEq)]
enum Obj {
    Null,
    Num(f64),
    Name(String),
    Str(Vec<u8>),
    Arr(Vec<Obj>),
    Dict(Vec<(String, Obj)>),
    Ref(u32),
    /// A stream dictionary with its decoded content.
    Stream(Vec<(String, Obj)>, Vec<u8>),
}

impl Obj {
    fn dict_get(&self, key: &str) -> Option<&Obj> {
        match self {
            Obj::Dict(entries) | Obj::Stream(entries, _) => {
                entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }
}

/// Parse one object whose first token has already been read.
fn parse_object(lex: &mut Lexer<'_>, first: Token, depth: usize) -> Result<Obj, DocError> {
    if depth > MAX_DEPTH {
        return Err(invalid("PDF objects nest too deeply"));
    }
    Ok(match first {
        Token::Num(n) => {
            // `N G R` is an indirect reference; anything else is a number.
            let save = lex.pos;
            if matches!(lex.next_token()?, Token::Num(_))
                && lex.next_token()? == Token::Keyword("R".into())
            {
                if let Some(number) = to_u32(n) {
                    return Ok(Obj::Ref(number));
                }
                return Err(invalid("indirect reference number is out of range"));
            }
            lex.pos = save;
            Obj::Num(n)
        }
        Token::Name(name) => Obj::Name(name),
        Token::Str(bytes) => Obj::Str(bytes),
        Token::ArrOpen => {
            let mut items = Vec::new();
            loop {
                match lex.next_token()? {
                    Token::ArrClose => break,
                    Token::Eof => return Err(invalid("unterminated array in PDF")),
                    other => items.push(parse_object(lex, other, depth + 1)?),
                }
            }
            Obj::Arr(items)
        }
        Token::DictOpen => {
            let mut entries = Vec::new();
            loop {
                match lex.next_token()? {
                    Token::DictClose => break,
                    Token::Eof => return Err(invalid("unterminated dictionary in PDF")),
                    Token::Name(key) => {
                        let value_token = lex.next_token()?;
                        let value = parse_object(lex, value_token, depth + 1)?;
                        entries.push((key, value));
                    }
                    other => {
                        return Err(invalid(&format!("unexpected {other:?} in dictionary")));
                    }
                }
            }
            Obj::Dict(entries)
        }
        Token::Keyword(word) if word == "null" => Obj::Null,
        Token::Keyword(word) if word == "true" => Obj::Name("true".into()),
        Token::Keyword(word) if word == "false" => Obj::Name("false".into()),
        other => return Err(invalid(&format!("unexpected {other:?} in PDF object"))),
    })
}

// ---------------------------------------------------------------- the file

struct PdfFile<'a> {
    data: &'a [u8],
    /// Object number to byte offset, newest entry winning.
    offsets: std::collections::HashMap<u32, usize>,
    trailers: Vec<Obj>,
    limits: Limits,
}

impl<'a> PdfFile<'a> {
    fn open(data: &'a [u8], limits: &Limits) -> Result<Self, DocError> {
        if !data.starts_with(b"%PDF-") {
            return Err(invalid("not a PDF: missing %PDF header"));
        }
        let startxref = rfind(data, b"startxref")
            .ok_or_else(|| invalid("PDF has no startxref; the file is truncated"))?;
        let mut lex = Lexer::new(data, startxref + "startxref".len());
        let offset = match lex.next_token()? {
            Token::Num(n) => to_usize(n).ok_or_else(|| invalid("startxref is not an offset"))?,
            _ => return Err(invalid("startxref is not an offset")),
        };

        let mut file = Self {
            data,
            offsets: std::collections::HashMap::new(),
            trailers: Vec::new(),
            limits: *limits,
        };
        let mut next = Some(offset);
        let mut seen = std::collections::HashSet::new();
        while let Some(at) = next.take() {
            if !seen.insert(at) || file.trailers.len() > MAX_DEPTH {
                return Err(invalid("cross-reference chain loops"));
            }
            let trailer = file.read_xref_section(at)?;
            if trailer.dict_get("Encrypt").is_some() {
                return Err(invalid("encrypted PDFs are not supported"));
            }
            next = match trailer.dict_get("Prev") {
                Some(Obj::Num(prev)) => to_usize(*prev),
                _ => None,
            };
            file.trailers.push(trailer);
        }
        Ok(file)
    }

    fn read_xref_section(&mut self, at: usize) -> Result<Obj, DocError> {
        let mut lex = Lexer::new(self.data, at);
        if lex.next_token()? != Token::Keyword("xref".into()) {
            return Err(invalid(
                "cross-reference streams and object streams are not supported",
            ));
        }
        loop {
            let token = lex.next_token()?;
            match token {
                Token::Keyword(ref word) if word == "trailer" => break,
                Token::Num(start) => {
                    let Token::Num(count) = lex.next_token()? else {
                        return Err(invalid("xref subsection has no entry count"));
                    };
                    let count =
                        to_u32(count).ok_or_else(|| invalid("xref subsection is too large"))?;
                    let start =
                        to_u32(start).ok_or_else(|| invalid("xref subsection start is invalid"))?;
                    for index in 0..count {
                        let (Token::Num(offset), Token::Num(_), Token::Keyword(kind)) =
                            (lex.next_token()?, lex.next_token()?, lex.next_token()?)
                        else {
                            return Err(invalid("malformed xref entry"));
                        };
                        let number = start + index;
                        if kind == "n"
                            && let Some(offset) = to_usize(offset).filter(|o| *o > 0)
                        {
                            self.offsets.entry(number).or_insert(offset);
                        }
                    }
                }
                _ => return Err(invalid("malformed xref table")),
            }
        }
        let trailer_token = lex.next_token()?;
        parse_object(&mut lex, trailer_token, 0)
    }

    fn trailer_ref(&self, key: &str) -> Result<u32, DocError> {
        for trailer in &self.trailers {
            if let Some(Obj::Ref(number)) = trailer.dict_get(key) {
                return Ok(*number);
            }
        }
        Err(invalid(&format!("trailer has no /{key}")))
    }

    /// Fetch and decode an indirect object.
    fn get(&self, number: u32) -> Result<Obj, DocError> {
        self.get_at_depth(number, 0)
    }

    fn get_at_depth(&self, number: u32, depth: usize) -> Result<Obj, DocError> {
        if depth > MAX_DEPTH {
            return Err(invalid("indirect objects reference each other in a loop"));
        }
        let offset = *self
            .offsets
            .get(&number)
            .ok_or_else(|| invalid(&format!("object {number} is not in the xref table")))?;
        let mut lex = Lexer::new(self.data, offset);
        let header_number = lex.next_token()?;
        let _generation = lex.next_token()?;
        let keyword = lex.next_token()?;
        if header_number != Token::Num(f64::from(number)) || keyword != Token::Keyword("obj".into())
        {
            return Err(DocError::invalid_format(format!(
                "xref points at something other than object {number}"
            ))
            .with_location(Location::Offset(offset as u64)));
        }
        let first = lex.next_token()?;
        let object = parse_object(&mut lex, first, 0)?;

        if lex.next_token()? == Token::Keyword("stream".into()) {
            let Obj::Dict(entries) = object else {
                return Err(invalid("stream without a dictionary"));
            };
            let data = self.stream_data(&entries, lex.pos, depth)?;
            return Ok(Obj::Stream(entries, data));
        }
        Ok(object)
    }

    fn stream_data(
        &self,
        entries: &[(String, Obj)],
        mut start: usize,
        depth: usize,
    ) -> Result<Vec<u8>, DocError> {
        // The keyword is followed by CRLF or LF, never a bare CR.
        if self.data.get(start) == Some(&b'\r') {
            start += 1;
        }
        if self.data.get(start) == Some(&b'\n') {
            start += 1;
        }
        let length = match entries.iter().find(|(k, _)| k == "Length").map(|(_, v)| v) {
            Some(Obj::Num(n)) => {
                to_usize(*n).ok_or_else(|| invalid("stream /Length is invalid"))?
            }
            Some(Obj::Ref(number)) => match self.get_at_depth(*number, depth + 1)? {
                Obj::Num(n) => to_usize(n).ok_or_else(|| invalid("stream /Length is invalid"))?,
                _ => return Err(invalid("stream /Length is not a number")),
            },
            _ => return Err(invalid("stream has no /Length")),
        };
        let end = start
            .checked_add(length)
            .filter(|end| *end <= self.data.len())
            .ok_or_else(|| invalid("stream runs past the end of the file"))?;
        let raw = &self.data[start..end];

        let flate = entries
            .iter()
            .find(|(k, _)| k == "Filter")
            .is_some_and(|(_, v)| matches!(v, Obj::Name(n) if n == "FlateDecode"));
        if flate {
            let mut decoded = Vec::new();
            ZlibDecoder::new(raw)
                .take(self.limits.max_entry_bytes as u64 + 1)
                .read_to_end(&mut decoded)
                .map_err(|e| invalid(&format!("cannot inflate stream: {e}")))?;
            if decoded.len() > self.limits.max_entry_bytes {
                return Err(invalid("stream inflates past max_entry_bytes"));
            }
            Ok(decoded)
        } else if entries.iter().any(|(k, _)| k == "Filter") {
            Err(invalid("stream uses a filter other than FlateDecode"))
        } else {
            Ok(raw.to_vec())
        }
    }

    fn resolve<'o>(&self, obj: &'o Obj) -> Result<std::borrow::Cow<'o, Obj>, DocError> {
        match obj {
            Obj::Ref(number) => Ok(std::borrow::Cow::Owned(self.get(*number)?)),
            other => Ok(std::borrow::Cow::Borrowed(other)),
        }
    }
}

/// Convert a PDF number to an offset or length. Negative, fractional, and
/// out-of-range values are refused rather than wrapped.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn to_usize(n: f64) -> Option<usize> {
    // `usize::MAX as f64` rounds up, so the bound is exclusive.
    (n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n < usize::MAX as f64).then_some(n as usize)
}

/// Convert a PDF number to an object number or subsection index.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_u32(n: f64) -> Option<u32> {
    (n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX)).then_some(n as u32)
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

// ---------------------------------------------------------------- pages

/// A leaf of the page tree: a page dictionary.
struct PageNode {
    obj: Obj,
}

fn collect_pages(
    file: &PdfFile<'_>,
    reference: &Obj,
    out: &mut Vec<PageNode>,
    depth: usize,
) -> Result<(), DocError> {
    if depth > MAX_DEPTH {
        return Err(invalid("page tree nests too deeply"));
    }
    let node = file.resolve(reference)?.into_owned();
    match node.dict_get("Kids") {
        Some(kids) => {
            let Obj::Arr(children) = file.resolve(kids)?.into_owned() else {
                return Err(invalid("/Kids is not an array"));
            };
            for child in &children {
                collect_pages(file, child, out, depth + 1)?;
            }
        }
        None => out.push(PageNode { obj: node }),
    }
    Ok(())
}

fn page_text(file: &PdfFile<'_>, page: &PageNode) -> Result<String, DocError> {
    let mut contents = Vec::new();
    match page.obj.dict_get("Contents") {
        Some(Obj::Arr(parts)) => {
            for part in parts {
                contents.push(stream_bytes(file, part)?);
            }
        }
        Some(other) => contents.push(stream_bytes(file, other)?),
        None => {}
    }
    let mut extractor = TextExtractor::default();
    for content in &contents {
        extractor.run(content)?;
    }
    Ok(extractor.finish())
}

fn stream_bytes(file: &PdfFile<'_>, reference: &Obj) -> Result<Vec<u8>, DocError> {
    match file.resolve(reference)?.into_owned() {
        Obj::Stream(_, data) => Ok(data),
        _ => Err(invalid("page content is not a stream")),
    }
}

// ---------------------------------------------------------------- text

/// Positions closer than this are the same line.
const LINE_EPSILON: f64 = 1e-6;

/// Walks a content stream and collects the text shown by it.
///
/// A new line starts when the vertical position of the text changes. Runs on
/// the same line are joined with a single space, so a layout that places each
/// word in its own text block still reads as a sentence.
#[derive(Default)]
struct TextExtractor {
    lines: Vec<String>,
    current: String,
    /// The y coordinate of the current text line, from `Td` and `Tm`.
    line_y: f64,
    /// The y coordinate of the last run shown.
    last_shown_y: Option<f64>,
}

impl TextExtractor {
    fn run(&mut self, content: &[u8]) -> Result<(), DocError> {
        let mut lex = Lexer::new(content, 0);
        let mut operands: Vec<Obj> = Vec::new();
        loop {
            let token = lex.next_token()?;
            match token {
                Token::Eof => break,
                Token::Keyword(op) => {
                    self.operate(&op, &operands);
                    operands.clear();
                }
                other => {
                    let obj = parse_object(&mut lex, other, 0)?;
                    operands.push(obj);
                }
            }
        }
        Ok(())
    }

    fn operate(&mut self, op: &str, operands: &[Obj]) {
        match op {
            // Text positions are relative to the origin at the start of each block.
            "BT" => self.line_y = 0.0,
            "T*" => self.newline(),
            "Td" | "TD" => {
                if let [_, Obj::Num(ty)] = operands {
                    self.line_y += ty;
                }
            }
            "Tm" => {
                if let [_, _, _, _, _, Obj::Num(f)] = operands {
                    self.line_y = *f;
                }
            }
            "Tj" => self.show(operands.last()),
            "'" | "\"" => {
                self.newline();
                self.show(operands.last());
            }
            "TJ" => {
                if let Some(Obj::Arr(items)) = operands.last() {
                    self.start_run();
                    for item in items {
                        match item {
                            Obj::Str(bytes) => self.push_bytes(bytes),
                            // A large negative adjustment is a word space.
                            Obj::Num(n) if *n < -200.0 => self.current.push(' '),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn show(&mut self, operand: Option<&Obj>) {
        if let Some(Obj::Str(bytes)) = operand {
            self.start_run();
            self.push_bytes(bytes);
        }
    }

    /// Prepare to append a run: break the line if the position moved, or
    /// separate it from the previous run on the same line.
    fn start_run(&mut self) {
        match self.last_shown_y {
            Some(y) if (y - self.line_y).abs() > LINE_EPSILON => self.newline(),
            Some(_) if !self.current.is_empty() && !self.current.ends_with(' ') => {
                self.current.push(' ');
            }
            _ => {}
        }
        self.last_shown_y = Some(self.line_y);
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.current.push(winansi(b));
        }
    }

    fn newline(&mut self) {
        if !self.current.is_empty() {
            self.lines.push(std::mem::take(&mut self.current));
        }
    }

    fn finish(mut self) -> String {
        self.newline();
        self.lines.join(
            "
",
        )
    }
}

/// Map a `WinAnsi` byte to its character. The `0x80` to `0x9F` range holds the
/// punctuation that Latin-1 leaves as control codes.
fn winansi(byte: u8) -> char {
    match byte {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        other => char::from(other),
    }
}
