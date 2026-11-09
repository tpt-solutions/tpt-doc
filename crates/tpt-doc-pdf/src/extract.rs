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

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::rc::Rc;

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

/// Where an object lives: at a byte offset, or as a member of an object stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loc {
    Offset(usize),
    InStream { stream: u32, index: u32 },
}

/// A decoded object stream: its bytes, where its objects start, and the number
/// and offset (relative to `first`) of each member.
struct ObjectStream {
    data: Vec<u8>,
    first: usize,
    members: Vec<(u32, usize)>,
}

struct PdfFile<'a> {
    data: &'a [u8],
    /// Object number to location, newest entry winning.
    offsets: HashMap<u32, Loc>,
    /// Object streams, decoded once each.
    object_streams: RefCell<HashMap<u32, Rc<ObjectStream>>>,
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
            offsets: HashMap::new(),
            object_streams: RefCell::new(HashMap::new()),
            trailers: Vec::new(),
            limits: *limits,
        };
        let mut next = Some(offset);
        let mut seen = HashSet::new();
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

    /// Read the cross-reference section at `at`: a classic table, or a
    /// cross-reference stream. Returns its trailer dictionary.
    fn read_xref_section(&mut self, at: usize) -> Result<Obj, DocError> {
        let mut lex = Lexer::new(self.data, at);
        match lex.next_token()? {
            Token::Keyword(word) if word == "xref" => {}
            Token::Num(_) => return self.read_xref_stream(at),
            _ => return Err(invalid("cross-reference table is missing")),
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
                        let number = start
                            .checked_add(index)
                            .ok_or_else(|| invalid("xref object number overflows"))?;
                        if kind == "n"
                            && let Some(offset) = to_usize(offset).filter(|o| *o > 0)
                        {
                            self.offsets.entry(number).or_insert(Loc::Offset(offset));
                        }
                    }
                }
                _ => return Err(invalid("malformed xref table")),
            }
        }
        let trailer_token = lex.next_token()?;
        let trailer = parse_object(&mut lex, trailer_token, 0)?;
        // A hybrid file keeps some entries in a cross-reference stream as well.
        if let Some(Obj::Num(xref_stm)) = trailer.dict_get("XRefStm")
            && let Some(offset) = to_usize(*xref_stm)
        {
            self.read_xref_stream(offset)?;
        }
        Ok(trailer)
    }

    /// Read a cross-reference stream (PDF 1.5). Its entries add locations, and
    /// its dictionary is the trailer.
    fn read_xref_stream(&mut self, at: usize) -> Result<Obj, DocError> {
        let Obj::Stream(entries, data) = self.read_indirect(at, None, 0)? else {
            return Err(invalid("cross-reference stream is not a stream"));
        };
        let dict = Obj::Dict(entries);
        let widths: Vec<usize> = match dict.dict_get("W") {
            Some(Obj::Arr(items)) if items.len() == 3 => items
                .iter()
                .map(|w| match w {
                    Obj::Num(n) => to_usize(*n),
                    _ => None,
                })
                .collect::<Option<_>>()
                .ok_or_else(|| invalid("cross-reference /W is invalid"))?,
            _ => return Err(invalid("cross-reference stream has no /W")),
        };
        if widths.iter().any(|w| *w > 8) {
            return Err(invalid("cross-reference field is too wide"));
        }
        let size = match dict.dict_get("Size") {
            Some(Obj::Num(n)) => {
                to_u32(*n).ok_or_else(|| invalid("cross-reference /Size is invalid"))?
            }
            _ => return Err(invalid("cross-reference stream has no /Size")),
        };
        let index: Vec<u32> = match dict.dict_get("Index") {
            Some(Obj::Arr(items)) => items
                .iter()
                .map(|i| match i {
                    Obj::Num(n) => to_u32(*n),
                    _ => None,
                })
                .collect::<Option<_>>()
                .ok_or_else(|| invalid("cross-reference /Index is invalid"))?,
            _ => vec![0, size],
        };
        if !index.len().is_multiple_of(2) {
            return Err(invalid(
                "cross-reference /Index has an odd number of entries",
            ));
        }
        let row: usize = widths.iter().sum();
        if row == 0 {
            return Err(invalid("cross-reference rows are empty"));
        }
        let mut pos = 0usize;
        for pair in index.chunks(2) {
            let (start, count) = (pair[0], pair[1]);
            for i in 0..count {
                let end = pos
                    .checked_add(row)
                    .filter(|end| *end <= data.len())
                    .ok_or_else(|| invalid("cross-reference stream is truncated"))?;
                let fields = &data[pos..end];
                pos = end;
                let (w0, w1) = (widths[0], widths[1]);
                // A type field of width zero means type 1 (a plain offset).
                let kind = if w0 == 0 { 1 } else { be(&fields[..w0]) };
                let primary = be(&fields[w0..w0 + w1]);
                let tertiary = be(&fields[w0 + w1..]);
                let number = start
                    .checked_add(i)
                    .ok_or_else(|| invalid("cross-reference object number overflows"))?;
                match kind {
                    1 => {
                        if let (Ok(offset), true) = (usize::try_from(primary), primary > 0) {
                            self.offsets.entry(number).or_insert(Loc::Offset(offset));
                        }
                    }
                    2 => {
                        if let (Ok(stream), Ok(index)) =
                            (u32::try_from(primary), u32::try_from(tertiary))
                        {
                            self.offsets
                                .entry(number)
                                .or_insert(Loc::InStream { stream, index });
                        }
                    }
                    // Type 0 is free, and unknown types are ignored, as the spec asks.
                    _ => {}
                }
            }
        }
        Ok(dict)
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
        let loc = *self
            .offsets
            .get(&number)
            .ok_or_else(|| invalid(&format!("object {number} is not in the xref table")))?;
        match loc {
            Loc::Offset(offset) => self.read_indirect(offset, Some(number), depth),
            Loc::InStream { stream, index } => self.object_in_stream(stream, index, number, depth),
        }
    }

    /// Read the indirect object at `offset`. With `expected`, the object number
    /// in the header must match it.
    fn read_indirect(
        &self,
        offset: usize,
        expected: Option<u32>,
        depth: usize,
    ) -> Result<Obj, DocError> {
        let mut lex = Lexer::new(self.data, offset);
        let header_number = lex.next_token()?;
        let _generation = lex.next_token()?;
        let keyword = lex.next_token()?;
        let number_ok = match expected {
            Some(number) => header_number == Token::Num(f64::from(number)),
            None => true,
        };
        if !number_ok || keyword != Token::Keyword("obj".into()) {
            return Err(DocError::invalid_format(
                "cross-reference points at something other than the expected object",
            )
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

    /// The object stream `stm`, decoded and indexed (cached after the first read).
    fn object_stream(&self, stm: u32, depth: usize) -> Result<Rc<ObjectStream>, DocError> {
        if let Some(found) = self.object_streams.borrow().get(&stm) {
            return Ok(Rc::clone(found));
        }
        let Obj::Stream(entries, data) = self.get_at_depth(stm, depth + 1)? else {
            return Err(invalid("object stream is not a stream"));
        };
        let dict = Obj::Dict(entries);
        let count = match dict.dict_get("N") {
            Some(Obj::Num(n)) => {
                to_usize(*n).ok_or_else(|| invalid("object stream /N is invalid"))?
            }
            _ => return Err(invalid("object stream has no /N")),
        };
        let first = match dict.dict_get("First") {
            Some(Obj::Num(n)) => {
                to_usize(*n).ok_or_else(|| invalid("object stream /First is invalid"))?
            }
            _ => return Err(invalid("object stream has no /First")),
        };
        let mut lex = Lexer::new(&data, 0);
        let mut members = Vec::with_capacity(count.min(data.len()));
        for _ in 0..count {
            let (Token::Num(number), Token::Num(offset)) = (lex.next_token()?, lex.next_token()?)
            else {
                return Err(invalid("malformed object stream header"));
            };
            let number =
                to_u32(number).ok_or_else(|| invalid("object stream member number is invalid"))?;
            let offset = to_usize(offset)
                .ok_or_else(|| invalid("object stream member offset is invalid"))?;
            members.push((number, offset));
        }
        let stream = Rc::new(ObjectStream {
            data,
            first,
            members,
        });
        self.object_streams
            .borrow_mut()
            .insert(stm, Rc::clone(&stream));
        Ok(stream)
    }

    /// Read object `number`, which is member `index` of object stream `stm`.
    fn object_in_stream(
        &self,
        stm: u32,
        index: u32,
        number: u32,
        depth: usize,
    ) -> Result<Obj, DocError> {
        let stream = self.object_stream(stm, depth)?;
        let (member, offset) = usize::try_from(index)
            .ok()
            .and_then(|i| stream.members.get(i).copied())
            .ok_or_else(|| invalid("object stream index is out of range"))?;
        if member != number {
            return Err(invalid(
                "object stream member does not match the cross-reference",
            ));
        }
        let start = stream
            .first
            .checked_add(offset)
            .ok_or_else(|| invalid("object stream offset overflows"))?;
        let mut lex = Lexer::new(&stream.data, start);
        let first = lex.next_token()?;
        parse_object(&mut lex, first, 0)
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
        let mut data = self.data[start..end].to_vec();

        for filter in filter_names(entries) {
            data = match filter.as_str() {
                "FlateDecode" | "Fl" => inflate(&data, self.limits.max_entry_bytes)?,
                other => return Err(invalid(&format!("stream filter {other} is not supported"))),
            };
        }
        if let Some(parms) = decode_parms(entries) {
            let predictor = parm_num(parms, "Predictor").unwrap_or(1);
            if predictor >= 10 {
                let colors = parm_num(parms, "Colors").unwrap_or(1);
                let bits = parm_num(parms, "BitsPerComponent").unwrap_or(8);
                let columns = parm_num(parms, "Columns").unwrap_or(1);
                let overflow = || invalid("predictor parameters are too large");
                let bits_per_pixel = colors.checked_mul(bits).ok_or_else(overflow)?;
                let bits_per_row = bits_per_pixel.checked_mul(columns).ok_or_else(overflow)?;
                let bpp = bits_per_pixel.div_ceil(8).max(1);
                let row_len = bits_per_row.div_ceil(8);
                data = png_unfilter(&data, bpp, row_len)?;
            } else if predictor != 1 {
                return Err(invalid("TIFF predictors are not supported"));
            }
        }
        Ok(data)
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

/// A big-endian unsigned integer of up to eight bytes.
fn be(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte))
}

/// Inflate a zlib stream, refusing output larger than `max`.
fn inflate(raw: &[u8], max: usize) -> Result<Vec<u8>, DocError> {
    let mut decoded = Vec::new();
    ZlibDecoder::new(raw)
        .take(u64::try_from(max).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut decoded)
        .map_err(|e| invalid(&format!("cannot inflate stream: {e}")))?;
    if decoded.len() > max {
        return Err(invalid("stream inflates past max_entry_bytes"));
    }
    Ok(decoded)
}

/// The filter names of a stream, in the order they were applied.
fn filter_names(entries: &[(String, Obj)]) -> Vec<String> {
    match entries.iter().find(|(k, _)| k == "Filter").map(|(_, v)| v) {
        Some(Obj::Name(name)) => vec![name.clone()],
        Some(Obj::Arr(items)) => items
            .iter()
            .filter_map(|item| match item {
                Obj::Name(name) => Some(name.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The decode parameters of a stream: a dictionary, or the first one of an array.
fn decode_parms(entries: &[(String, Obj)]) -> Option<&Obj> {
    match entries
        .iter()
        .find(|(k, _)| k == "DecodeParms")
        .map(|(_, v)| v)?
    {
        dict @ Obj::Dict(_) => Some(dict),
        Obj::Arr(items) => items.iter().find(|item| matches!(item, Obj::Dict(_))),
        _ => None,
    }
}

/// An integer decode parameter.
fn parm_num(parms: &Obj, key: &str) -> Option<usize> {
    match parms.dict_get(key) {
        Some(Obj::Num(n)) => to_usize(*n),
        _ => None,
    }
}

/// Undo a PNG predictor (PDF predictors 10 to 15). Each row starts with its
/// filter type. `bpp` is bytes per complete pixel, and `row_len` the bytes in a
/// row without the filter byte.
fn png_unfilter(data: &[u8], bpp: usize, row_len: usize) -> Result<Vec<u8>, DocError> {
    let stride = row_len
        .checked_add(1)
        .ok_or_else(|| invalid("predictor row is too wide"))?;
    // A row longer than the whole input cannot be complete, so refuse it before
    // allocating a row buffer of that size.
    if row_len == 0 || stride > data.len() || !data.len().is_multiple_of(stride) {
        return Err(invalid("predictor data is not a whole number of rows"));
    }
    let rows = data.len() / stride;
    let mut out = Vec::with_capacity(rows * row_len);
    let mut previous = vec![0u8; row_len];
    for row in data.chunks_exact(stride) {
        let filter = row[0];
        let line = &row[1..];
        let mut current = vec![0u8; row_len];
        for i in 0..row_len {
            let left = if i >= bpp { current[i - bpp] } else { 0 };
            let up = previous[i];
            let up_left = if i >= bpp { previous[i - bpp] } else { 0 };
            current[i] = match filter {
                0 => line[i],
                1 => line[i].wrapping_add(left),
                2 => line[i].wrapping_add(up),
                3 => line[i].wrapping_add(
                    u8::try_from(u16::midpoint(u16::from(left), u16::from(up))).unwrap_or(u8::MAX),
                ),
                4 => line[i].wrapping_add(paeth(left, up, up_left)),
                _ => return Err(invalid("unknown PNG filter type in predictor data")),
            };
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    Ok(out)
}

/// The Paeth predictor of PNG filter type 4.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let (a, b, c) = (i16::from(left), i16::from(up), i16::from(up_left));
    let p = a + b - c;
    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        up_left
    }
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
