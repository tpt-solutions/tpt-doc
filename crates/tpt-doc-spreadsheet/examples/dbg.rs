fn main() {
    let rows = vec![tpt_doc_spreadsheet::Row {
        index: 1,
        cells: vec![tpt_doc_spreadsheet::Cell::String("bad\u{1}value".into())],
    }];
    let mut w = tpt_doc_spreadsheet::XlsxWriter::new();
    w.push_row(rows[0].clone());
    let bytes = w.finish().unwrap();
    // Look at the raw sheet1.xml inside the zip.
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    use std::io::Read;
    z.by_name("xl/worksheets/sheet1.xml").unwrap().read_to_string(&mut s).unwrap();
    println!("{}", s);
}
