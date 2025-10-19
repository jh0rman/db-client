mod db;
mod state;

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent, Context, Window,
    WindowBounds, WindowOptions,
};
use state::AppState;

// Path to the SQLite database (hardcoded for MVP).
const DB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test.db");

// Column width for every cell in the data grid.
const CELL_W: f32 = 160.0;
// Row height for header and data rows.
const ROW_H: f32 = 36.0;

// ─── Color palette ───────────────────────────────────────────────────────────
const BG_APP: u32 = 0x0f1117;
const BG_SIDEBAR: u32 = 0x161b22;
const BG_PANEL: u32 = 0x0d1117;
const BG_ACTIVE: u32 = 0x1c2333;
const BG_HEADER: u32 = 0x161b22;
const BG_ROW_ALT: u32 = 0x0d1117;
const BORDER: u32 = 0x21262d;
const TEXT_MUTED: u32 = 0x484f58;
const TEXT_SECONDARY: u32 = 0x7d8590;
const TEXT_PRIMARY: u32 = 0xcdd9e5;
const ACCENT: u32 = 0x388bfd;

// ─── Root component ──────────────────────────────────────────────────────────
struct AppRoot {
    state: AppState,
}

impl AppRoot {
    fn new() -> Self {
        let _ = db::seed_test_db(DB_PATH);
        let tables = db::get_tables(DB_PATH).unwrap_or_default();
        Self {
            state: AppState {
                db_path: Some(DB_PATH.to_string()),
                tables,
                active_table: None,
                table_data: None,
            },
        }
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let table_listeners: Vec<_> = self
            .state
            .tables
            .iter()
            .map(|table| {
                let name = table.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    // Load data for the selected table and update state.
                    this.state.table_data = db::get_table_data(DB_PATH, &name).ok();
                    this.state.active_table = Some(name.clone());
                    cx.notify();
                })
            })
            .collect();

        let tables = self.state.tables.clone();
        let active = self.state.active_table.clone();

        div()
            .w(px(240.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(BG_SIDEBAR))
            .border_r_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .text_xs()
                    .text_color(rgb(TEXT_SECONDARY))
                    .child("TABLES"),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .py_1()
                    .children(
                        tables
                            .into_iter()
                            .zip(table_listeners)
                            .enumerate()
                            .map(|(i, (table, on_click))| {
                                let is_active = active.as_deref() == Some(&table);
                                div()
                                    .id(("table-item", i))
                                    .px_3()
                                    .py_2()
                                    .mx_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_sm()
                                    .when(is_active, |el| {
                                        el.bg(rgb(BG_ACTIVE)).text_color(rgb(ACCENT))
                                    })
                                    .when(!is_active, |el| el.text_color(rgb(TEXT_PRIMARY)))
                                    .on_click(on_click)
                                    .child(table)
                            }),
                    ),
            )
    }

    fn render_main_panel(
        active_table: Option<String>,
        table_data: Option<(Vec<String>, Vec<Vec<String>>)>,
    ) -> impl IntoElement {
        let title = active_table
            .as_deref()
            .unwrap_or("Select a table to view data")
            .to_string();

        let content = if let Some((columns, rows)) = table_data {
            Self::render_data_grid(columns, rows)
        } else {
            div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child("No table selected")
        };

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(BG_PANEL))
            .child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .text_sm()
                    .text_color(rgb(TEXT_SECONDARY))
                    .child(title),
            )
            .child(content)
    }

    fn render_data_grid(columns: Vec<String>, rows: Vec<Vec<String>>) -> gpui::Div {
        let row_count = rows.len();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            // ── Header row
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_shrink_0()
                    .h(px(ROW_H))
                    .bg(rgb(BG_HEADER))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .children(columns.iter().map(|col| {
                        div()
                            .w(px(CELL_W))
                            .flex_shrink_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .px_3()
                            .border_r_1()
                            .border_color(rgb(BORDER))
                            .text_xs()
                            .text_color(rgb(TEXT_SECONDARY))
                            .child(col.to_uppercase())
                    })),
            )
            // ── Data rows
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .children(rows.into_iter().enumerate().map(|(row_idx, cells)| {
                        let is_alt = row_idx % 2 == 1;
                        div()
                            .flex()
                            .flex_row()
                            .flex_shrink_0()
                            .h(px(ROW_H))
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .when(is_alt, |el| el.bg(rgb(BG_ROW_ALT)))
                            .children(cells.into_iter().map(|cell| {
                                div()
                                    .w(px(CELL_W))
                                    .flex_shrink_0()
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .px_3()
                                    .border_r_1()
                                    .border_color(rgb(BORDER))
                                    .text_sm()
                                    .text_color(rgb(TEXT_PRIMARY))
                                    .child(cell)
                            }))
                    })),
            )
            // ── Row count footer
            .child(
                div()
                    .flex_shrink_0()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_xs()
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!("{} row{}", row_count, if row_count == 1 { "" } else { "s" })),
            )
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_table = self.state.active_table.clone();
        let table_data = self
            .state
            .table_data
            .as_ref()
            .map(|d| (d.columns.clone(), d.rows.clone()));
        let sidebar = self.render_sidebar(cx);
        let panel = Self::render_main_panel(active_table, table_data);

        div()
            .flex()
            .w_full()
            .h_full()
            .bg(rgb(BG_APP))
            .child(sidebar)
            .child(panel)
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| AppRoot::new()),
        )
        .unwrap();
    });
}
