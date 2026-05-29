//! CSV data model, shared application state, and sorting.

/// Parsed contents of a CSV file: the first record as column titles, plus the
/// remaining records as data rows.
#[derive(Debug, Clone, Default)]
pub struct CsvData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Mutable application state shared across the Slint callbacks.
#[derive(Default)]
pub struct State {
    pub data: CsvData,
    /// Rows in their original load order, used to restore the unsorted view.
    pub original_rows: Vec<Vec<String>>,
    /// Column the table is currently sorted by, and whether it is ascending.
    pub sort: Option<(usize, bool)>,
    /// Currently selected cell as `(row, column)`.
    pub selected: Option<(usize, usize)>,
}

/// Sort `rows` by `col`. Numeric columns sort numerically; everything else falls
/// back to lexicographic order. `ascending` reverses the comparison when false.
pub fn sort_rows(rows: &mut [Vec<String>], col: usize, ascending: bool) {
    rows.sort_by(|a, b| {
        let left = a.get(col).map(String::as_str).unwrap_or_default();
        let right = b.get(col).map(String::as_str).unwrap_or_default();
        let ordering = match (left.parse::<f64>(), right.parse::<f64>()) {
            (Ok(l), Ok(r)) => l.total_cmp(&r),
            _ => left.cmp(right),
        };
        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}
