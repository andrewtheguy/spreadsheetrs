use std::cell::RefCell;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

/// Parsed contents of a CSV file: the first record as column titles, plus the
/// remaining records as data rows.
#[derive(Debug, Clone, Default)]
struct CsvData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

/// Mutable application state shared across the Slint callbacks.
#[derive(Default)]
struct State {
    data: CsvData,
    /// Rows in their original load order, used to restore the unsorted view.
    original_rows: Vec<Vec<String>>,
    /// Column the table is currently sorted by, and whether it is ascending.
    sort: Option<(usize, bool)>,
    /// Currently selected cell as `(row, column)`.
    selected: Option<(usize, usize)>,
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    ui.set_status("No CSV loaded".into());
    ui.set_version(env!("BUILD_TAG").into());

    let state = Rc::new(RefCell::new(State::default()));

    // Open a file picker, parse it, and replace the table contents.
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_csv(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            match pick_and_parse() {
                Ok(data) => {
                    let status = format!(
                        "Loaded {} rows × {} columns",
                        data.rows.len(),
                        data.headers.len()
                    );
                    let mut state = state.borrow_mut();
                    state.original_rows = data.rows.clone();
                    state.data = data;
                    state.sort = None;
                    state.selected = None;
                    push_to_ui(&ui, &state.data);
                    ui.set_status(status.into());
                }
                Err(error) => ui.set_status(error.into()),
            }
        });
    }

    // Click a header to sort by that column, toggling direction on repeat clicks.
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_header_clicked(move |col| {
            let Some(ui) = ui_weak.upgrade() else { return };
            let col = col as usize;
            let mut state = state.borrow_mut();
            // Cycle the clicked column: unsorted → ascending → descending → unsorted.
            let next = match state.sort {
                Some((c, true)) if c == col => Some((col, false)),
                Some((c, false)) if c == col => None,
                _ => Some((col, true)),
            };
            match next {
                Some((col, ascending)) => sort_rows(&mut state.data.rows, col, ascending),
                None => state.data.rows = state.original_rows.clone(),
            }
            state.sort = next;
            state.selected = None;
            ui.set_sort_col(next.map_or(-1, |(c, _)| c as i32));
            ui.set_sort_ascending(next.is_some_and(|(_, asc)| asc));
            ui.set_selected_row(-1);
            ui.set_selected_col(-1);
            ui.set_rows(rows_model(&state.data.rows));
        });
    }

    // Click a cell to select it.
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_cell_clicked(move |row, col| {
            let Some(ui) = ui_weak.upgrade() else { return };
            let (row, col) = (row as usize, col as usize);
            state.borrow_mut().selected = Some((row, col));
            ui.set_selected_row(row as i32);
            ui.set_selected_col(col as i32);
        });
    }

    // Copy the selected cell to the system clipboard.
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_copy_selection(move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let state = state.borrow();
            let Some((r, c)) = state.selected else { return };
            let Some(value) = state.data.rows.get(r).and_then(|row| row.get(c)) else {
                return;
            };
            match copy_to_clipboard(value) {
                Ok(()) => ui.set_status(format!("Copied cell ({}, {})", r + 1, c + 1).into()),
                Err(error) => ui.set_status(error),
            }
        });
    }

    ui.run()
}

/// Sort `rows` by `col`. Numeric columns sort numerically; everything else falls
/// back to lexicographic order. `ascending` reverses the comparison when false.
fn sort_rows(rows: &mut [Vec<String>], col: usize, ascending: bool) {
    rows.sort_by(|a, b| {
        let left = a.get(col).map(String::as_str).unwrap_or_default();
        let right = b.get(col).map(String::as_str).unwrap_or_default();
        let ordering = match (left.parse::<f64>(), right.parse::<f64>()) {
            (Ok(l), Ok(r)) => l.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal),
            _ => left.cmp(right),
        };
        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}

/// Replace the UI's headers/rows with `data` and reset the selection.
fn push_to_ui(ui: &MainWindow, data: &CsvData) {
    let headers: Vec<SharedString> = data.headers.iter().map(SharedString::from).collect();
    ui.set_headers(ModelRc::new(VecModel::from(headers)));
    ui.set_rows(rows_model(&data.rows));
    ui.set_has_data(!data.headers.is_empty());
    ui.set_sort_col(-1);
    ui.set_selected_row(-1);
    ui.set_selected_col(-1);
}

/// Build a Slint model-of-models (`[[string]]`) from the parsed rows.
fn rows_model(rows: &[Vec<String>]) -> ModelRc<ModelRc<SharedString>> {
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
fn copy_to_clipboard(value: &str) -> Result<(), SharedString> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| SharedString::from(e.to_string()))?;
    clipboard
        .set_text(value.to_owned())
        .map_err(|e| SharedString::from(e.to_string()))
}

/// Open a native file picker, read the chosen CSV, and parse it.
fn pick_and_parse() -> Result<CsvData, String> {
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
