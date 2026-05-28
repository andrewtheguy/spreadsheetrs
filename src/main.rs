use iced::widget::{button, column, container, row, scrollable, table, text};
use iced::{Center, Element, Task};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("spreadsheetrs — CSV viewer")
        .run()
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

#[derive(Debug, Clone)]
enum Message {
    OpenPressed,
    FileLoaded(Result<CsvData, String>),
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenPressed => {
                Task::perform(pick_and_parse(), Message::FileLoaded)
            }
            Message::FileLoaded(Ok(data)) => {
                self.status = format!(
                    "Loaded {} rows × {} columns",
                    data.rows.len(),
                    data.headers.len()
                );
                self.headers = data.headers;
                self.rows = data.rows;
                Task::none()
            }
            Message::FileLoaded(Err(error)) => {
                self.status = error;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![
            button("Open CSV…").on_press(Message::OpenPressed),
            text(&self.status),
        ]
        .spacing(12)
        .align_y(Center);

        let body: Element<'_, Message> = if self.headers.is_empty() {
            text("No CSV loaded. Click \"Open CSV…\" to pick a file.").into()
        } else {
            let columns = self.headers.iter().enumerate().map(|(i, header)| {
                table::column(text(header.clone()), move |row: &Vec<String>| {
                    text(row.get(i).cloned().unwrap_or_default())
                })
            });
            scrollable(table(columns, &self.rows)).into()
        };

        container(column![controls, body].spacing(12))
            .padding(16)
            .into()
    }
}

/// Open a native file picker, read the chosen CSV, and parse it. Runs off the UI
/// thread via `Task::perform`.
async fn pick_and_parse() -> Result<CsvData, String> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("CSV", &["csv"])
        .pick_file()
        .await
        .ok_or_else(|| "No file selected".to_string())?;

    let bytes = handle.read().await;

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
