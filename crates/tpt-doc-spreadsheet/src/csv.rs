use tpt_doc_core::DocError;

/// Streaming CSV reader backed by the `csv` crate.
pub struct CsvReader<R: std::io::Read> {
    inner: ::csv::Reader<R>,
}

impl<R: std::io::Read> CsvReader<R> {
    /// Create a reader with default settings (comma delimiter, header row).
    pub fn new(reader: R) -> Self {
        Self {
            inner: ::csv::ReaderBuilder::new()
                .flexible(true)
                .from_reader(reader),
        }
    }

    /// Iterate over records as `Vec<String>`.
    ///
    /// # Errors
    /// Returns [`DocError`] if a record cannot be parsed.
    pub fn records(&mut self) -> impl Iterator<Item = Result<Vec<String>, DocError>> + '_ {
        self.inner.records().map(|r| {
            r.map(|rec| rec.iter().map(str::to_owned).collect())
                .map_err(|e| DocError::invalid_format(e.to_string()))
        })
    }
}

/// CSV writer backed by the `csv` crate.
pub struct CsvWriter<W: std::io::Write> {
    inner: ::csv::Writer<W>,
}

impl<W: std::io::Write> CsvWriter<W> {
    /// Create a new CSV writer.
    pub fn new(writer: W) -> Self {
        Self {
            inner: ::csv::WriterBuilder::new().from_writer(writer),
        }
    }

    /// Write one record (row) to the output.
    ///
    /// # Errors
    /// Returns [`DocError`] if serialization fails.
    pub fn write_record<I, T>(&mut self, record: I) -> Result<(), DocError>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<[u8]>,
    {
        self.inner
            .write_record(record)
            .map_err(|e| DocError::invalid_format(e.to_string()))
    }

    /// Flush all buffered output.
    ///
    /// # Errors
    /// Returns [`DocError`] on I/O failure.
    pub fn flush(&mut self) -> Result<(), DocError> {
        self.inner
            .flush()
            .map_err(DocError::from)
    }
}
