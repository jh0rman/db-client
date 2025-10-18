mod db;
mod state;

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent, Context, Window,
    WindowBounds, WindowOptions,
};
use state::AppState;

// Path to the SQLite database (hardcoded for MVP).
const DB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test.db");

// ─── Color palette ───────────────────────────────────────────────────────────
const BG_APP: u32 = 0x0f1117;
const BG_SIDEBAR: u32 = 0x161b22;
const BG_PANEL: u32 = 0x0d1117;
const BG_ACTIVE: u32 = 0x1c2333;
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
        // Seed a test database if it doesn't exist, then load the table list.
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
        // Build one click listener per table before constructing the element tree.
        let table_listeners: Vec<_> = self
            .state
            .tables
            .iter()
            .enumerate()
            .map(|(_, table)| {
                let name = table.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
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
            // ── Sidebar header
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
            // ── Table list
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
                                        el.bg(rgb(BG_ACTIVE))
                                            .text_color(rgb(ACCENT))
                                    })
                                    .when(!is_active, |el| el.text_color(rgb(TEXT_PRIMARY)))
                                    .on_click(on_click)
                                    .child(table)
                            }),
                    ),
            )
    }

    fn render_main_panel(active_table: Option<String>) -> impl IntoElement {
        let title = active_table
            .as_deref()
            .unwrap_or("Select a table to view data")
            .to_string();

        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(BG_PANEL))
            // ── Toolbar / breadcrumb bar
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
            // ── Content area (Phase 3: data grid goes here)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .text_sm()
                    .text_color(rgb(TEXT_MUTED))
                    .child("Data grid coming in Phase 3"),
            )
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Extract owned data for the panel before the mutable borrow in render_sidebar.
        let active_table = self.state.active_table.clone();
        let sidebar = self.render_sidebar(cx);
        let panel = Self::render_main_panel(active_table);

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
