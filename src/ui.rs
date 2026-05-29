//! Bridge between the CSV data model and the generated Slint `MainWindow`.

use slint::{ModelRc, SharedString, VecModel};

use crate::MainWindow;
use crate::model::CsvData;

/// Replace the UI's headers/rows with `data` and reset the selection.
pub fn push_to_ui(ui: &MainWindow, data: &CsvData) {
    let headers: Vec<SharedString> = data.headers.iter().map(SharedString::from).collect();
    ui.set_headers(ModelRc::new(VecModel::from(headers)));
    ui.set_rows(rows_model(&data.rows));
    ui.set_has_data(!data.headers.is_empty());
    ui.set_sort_col(-1);
    ui.set_selected_row(-1);
    ui.set_selected_col(-1);
}

/// Build a Slint model-of-models (`[[string]]`) from the parsed rows.
pub fn rows_model(rows: &[Vec<String>]) -> ModelRc<ModelRc<SharedString>> {
    let rows: Vec<ModelRc<SharedString>> = rows
        .iter()
        .map(|row| {
            let cells: Vec<SharedString> = row.iter().map(SharedString::from).collect();
            ModelRc::new(VecModel::from(cells))
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Write `value` to the system clipboard, returning a human-readable error on failure.
pub fn copy_to_clipboard(value: &str) -> Result<(), SharedString> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| SharedString::from(e.to_string()))?;
    clipboard
        .set_text(value.to_owned())
        .map_err(|e| SharedString::from(e.to_string()))
}
