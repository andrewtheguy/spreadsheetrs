// Hide the Windows console window for release builds (the GUI is the only UI).
// Debug builds keep the console so logs/panics remain visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod csv_io;
mod model;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use csv_io::pick_and_parse;
use model::{State, sort_rows};
use ui::{copy_to_clipboard, push_to_ui, rows_model};

slint::include_modules!();

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
