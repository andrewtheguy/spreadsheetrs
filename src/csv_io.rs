//! Loading CSV files from disk via a native file picker.

use crate::model::CsvData;

/// Open a native file picker, read the chosen CSV, and parse it.
pub fn pick_and_parse() -> Result<CsvData, String> {
    let path = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .pick_file()
        .ok_or_else(|| "No file selected".to_string())?;

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes.as_slice());

    let mut records = reader.records();

    let headers: Vec<String> = records
        .next()
        .ok_or_else(|| "CSV file is empty".to_string())?
        .map_err(|e| e.to_string())?
        .iter()
        .map(str::to_string)
        .collect();

    let rows: Vec<Vec<String>> = records
        .map(|record| record.map(|rec| rec.iter().map(str::to_string).collect()))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;

    Ok(CsvData { headers, rows })
}
