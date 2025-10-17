mod db;
mod state;

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Window,
    WindowBounds, WindowOptions,
};
use state::AppState;

// ─── Color palette ───────────────────────────────────────────────────────────
const BG_APP: u32 = 0x0f1117;
const BG_SIDEBAR: u32 = 0x161b22;
const BG_PANEL: u32 = 0x0d1117;
const BORDER: u32 = 0x21262d;
const TEXT_MUTED: u32 = 0x484f58;
const TEXT_SECONDARY: u32 = 0x7d8590;

// ─── Root component ──────────────────────────────────────────────────────────
struct AppRoot {
    state: AppState,
}

impl AppRoot {
    fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }

    fn render_sidebar(&self) -> impl IntoElement {
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
            // ── Empty state
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .text_color(rgb(TEXT_MUTED))
                    .child("No database open"),
            )
    }

    fn render_main_panel(&self) -> impl IntoElement {
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
                    .child("Select a table to view data"),
            )
            // ── Empty content area
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .items_center()
                    .text_sm()
                    .text_color(rgb(TEXT_MUTED))
                    .child("No table selected"),
            )
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .h_full()
            .bg(rgb(BG_APP))
            .child(self.render_sidebar())
            .child(self.render_main_panel())
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
