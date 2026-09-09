/// The 14 standard PDF built-in fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// Helvetica (sans-serif).
    Helvetica,
    /// Helvetica-Bold.
    HelveticaBold,
    /// Helvetica-Oblique.
    HelveticaOblique,
    /// Helvetica-BoldOblique.
    HelveticaBoldOblique,
    /// Times-Roman (serif).
    TimesRoman,
    /// Times-Bold.
    TimesBold,
    /// Times-Italic.
    TimesItalic,
    /// Times-BoldItalic.
    TimesBoldItalic,
    /// Courier (monospace).
    Courier,
    /// Courier-Bold.
    CourierBold,
    /// Courier-Oblique.
    CourierOblique,
    /// Courier-BoldOblique.
    CourierBoldOblique,
    /// Symbol.
    Symbol,
    /// ZapfDingbats.
    ZapfDingbats,
}

impl Font {
    /// The PDF name string for this font (e.g. `"Helvetica"`).
    pub fn pdf_name(self) -> &'static str {
        match self {
            Self::Helvetica => "Helvetica",
            Self::HelveticaBold => "Helvetica-Bold",
            Self::HelveticaOblique => "Helvetica-Oblique",
            Self::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Self::TimesRoman => "Times-Roman",
            Self::TimesBold => "Times-Bold",
            Self::TimesItalic => "Times-Italic",
            Self::TimesBoldItalic => "Times-BoldItalic",
            Self::Courier => "Courier",
            Self::CourierBold => "Courier-Bold",
            Self::CourierOblique => "Courier-Oblique",
            Self::CourierBoldOblique => "Courier-BoldOblique",
            Self::Symbol => "Symbol",
            Self::ZapfDingbats => "ZapfDingbats",
        }
    }
}
