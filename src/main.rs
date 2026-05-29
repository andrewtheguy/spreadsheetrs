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
        Box::new(|cc| {
            install_fonts(&cc.egui_ctx);
            Ok(Box::<App>::default())
        }),
    )
}

/// Register JetBrains Mono (bundled in the binary) as the default proportional
/// and monospace font so the whole UI uses it.
fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains-mono".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains-mono-bold".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Bold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jetbrains-mono-italic".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Italic.ttf"
        ))),
    );

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "jetbrains-mono".to_owned());
    }

    // Named families for bold table headers and italic status text.
    fonts
        .families
        .insert(FontFamily::Name("bold".into()), vec!["jetbrains-mono-bold".to_owned()]);
    fonts
        .families
        .insert(FontFamily::Name("italic".into()), vec!["jetbrains-mono-italic".to_owned()]);

    ctx.set_fonts(fonts);
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
    /// Column the table is currently sorted by, and whether it is ascending.
    sort: Option<(usize, bool)>,
    /// Currently selected cell as `(row, column)`.
    selected: Option<(usize, usize)>,
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
                            self.sort = None;
                            self.selected = None;
                        }
                        Err(error) => self.status = error,
                    }
                }
                ui.label(&self.status);
            });

            ui.separator();

            if self.headers.is_empty() {
                ui.label(
                    egui::RichText::new("No CSV loaded. Click \"Open CSV…\" to pick a file.")
                        .family(egui::FontFamily::Name("italic".into())),
                );
                return;
            }

            let mut new_selection: Option<(usize, usize)> = None;
            egui::ScrollArea::horizontal().show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .columns(
                        Column::initial(140.0).at_least(40.0).clip(true).resizable(true),
                        self.headers.len(),
                    )
                    .header(20.0, |mut header| {
                        let mut clicked: Option<usize> = None;
                        for (col, title) in self.headers.iter().enumerate() {
                            header.col(|ui| {
                                if self.selected.map(|(_, c)| c == col).unwrap_or(false) {
                                    ui.painter().rect_filled(
                                        ui.max_rect(),
                                        0.0,
                                        ui.visuals().selection.bg_fill.linear_multiply(0.4),
                                    );
                                }
                                let arrow = match self.sort {
                                    Some((c, true)) if c == col => " ▲",
                                    Some((c, false)) if c == col => " ▼",
                                    _ => "",
                                };
                                let label = egui::RichText::new(format!("{title}{arrow}"))
                                    .family(egui::FontFamily::Name("bold".into()));
                                let button = egui::Button::new(label)
                                    .frame(false)
                                    .fill(egui::Color32::TRANSPARENT);
                                if ui.add_sized(ui.available_size(), button).clicked() {
                                    clicked = Some(col);
                                }
                            });
                        }
                        if let Some(col) = clicked {
                            self.toggle_sort(col);
                        }
                    })
                    .body(|body| {
                        body.rows(18.0, self.rows.len(), |mut row| {
                            let r = row.index();
                            let record = &self.rows[r];
                            for i in 0..self.headers.len() {
                                row.col(|ui| {
                                    let value = record
                                        .get(i)
                                        .map(String::as_str)
                                        .unwrap_or_default();
                                    let cell_rect = ui.max_rect();
                                    let response = ui.interact(
                                        cell_rect,
                                        ui.id().with((r, i)),
                                        egui::Sense::click(),
                                    );

                                    if self.selected == Some((r, i)) {
                                        ui.painter().rect_filled(
                                            cell_rect,
                                            0.0,
                                            ui.visuals().selection.bg_fill,
                                        );
                                    }

                                    ui.scope_builder(
                                        egui::UiBuilder::new().max_rect(cell_rect),
                                        |ui| {
                                            ui.add(egui::Label::new(value).selectable(false));
                                        },
                                    );

                                    if response.clicked() {
                                        new_selection = Some((r, i));
                                    }
                                });
                            }
                        });
                    });
            });

            if let Some(sel) = new_selection {
                self.selected = Some(sel);
            }

            let copy = ui.input(|i| {
                i.events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy))
                    || i.modifiers.command && i.key_pressed(egui::Key::C)
            });
            if copy
                && let Some((r, c)) = self.selected
                && let Some(value) = self.rows.get(r).and_then(|row| row.get(c))
            {
                ui.ctx().copy_text(value.clone());
                self.status = format!("Copied cell ({}, {})", r + 1, c + 1);
            }
        });
    }
}

impl App {
    /// Sort rows by `col`. Clicking a new column sorts ascending; clicking the
    /// already-sorted column flips the direction.
    fn toggle_sort(&mut self, col: usize) {
        let ascending = match self.sort {
            Some((c, asc)) if c == col => !asc,
            _ => true,
        };
        self.rows.sort_by(|a, b| {
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
        self.sort = Some((col, ascending));
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
