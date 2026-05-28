use eframe::egui;
use egui_extras::{Column, TableBuilder};

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("spreadsheetrs — CSV viewer")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "spreadsheetrs",
        native_options,
        Box::new(|_cc| Ok(Box::<App>::default())),
    )
}

/// Parsed contents of a CSV file: the first record as column titles, plus the
/// remaining records as data rows.
#[derive(Debug, Clone)]
struct CsvData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Default)]
struct App {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    status: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open CSV…").clicked() {
                    match pick_and_parse() {
                        Ok(data) => {
                            self.status = format!(
                                "Loaded {} rows × {} columns",
                                data.rows.len(),
                                data.headers.len()
                            );
                            self.headers = data.headers;
                            self.rows = data.rows;
                        }
                        Err(error) => self.status = error,
                    }
                }
                ui.label(&self.status);
            });

            ui.separator();

            if self.headers.is_empty() {
                ui.label("No CSV loaded. Click \"Open CSV…\" to pick a file.");
                return;
            }

            TableBuilder::new(ui)
                .striped(true)
                .columns(Column::auto().resizable(true), self.headers.len())
                .header(20.0, |mut header| {
                    for title in &self.headers {
                        header.col(|ui| {
                            ui.strong(title);
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, self.rows.len(), |mut row| {
                        let record = &self.rows[row.index()];
                        for i in 0..self.headers.len() {
                            row.col(|ui| {
                                ui.label(record.get(i).map(String::as_str).unwrap_or_default());
                            });
                        }
                    });
                });
        });
    }
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
