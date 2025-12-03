use std::sync::Arc;
use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window, UniformListScrollHandle};
use gpui_component::input::{Input, InputState};
use super::theme::*;
use super::data_grid::render_data_grid;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub fn render(
    sql_input: &gpui::Entity<InputState>,
    is_running: bool,
    query_error: Option<String>,
    results: Option<(Vec<String>, Vec<String>, Arc<Vec<Vec<Option<String>>>>, bool, bool)>,
    on_run: ClickCb,
    on_cancel: ClickCb,
    scroll_handle: &UniformListScrollHandle,
) -> gpui::Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(BG_PANEL))
        // ── SQL editor bar
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(div().flex_1().child(Input::new(sql_input)))
                // Run button — grayed out while a query is in progress
                .child(
                    div()
                        .id("btn-run")
                        .flex_shrink_0()
                        .h(px(32.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .bg(rgb(ACCENT_BG))
                        .border_1()
                        .border_color(rgb(ACCENT))
                        .text_sm()
                        .text_color(rgb(ACCENT))
                        .when(!is_running, |el| el.cursor_pointer().on_click(on_run))
                        .when(is_running, |el| {
                            el.bg(rgb(BG_CARD))
                                .border_color(rgb(BORDER))
                                .text_color(rgb(TEXT_MUTED))
                        })
                        .child(if is_running { "Running…" } else { "Run" }),
                )
                // Cancel button — only visible while running
                .when(is_running, |el| {
                    el.child(
                        div()
                            .id("btn-cancel")
                            .flex_shrink_0()
                            .h(px(32.0))
                            .px_4()
                            .flex()
                            .items_center()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(rgb(BG_CARD))
                            .border_1()
                            .border_color(rgb(BORDER_ERROR))
                            .text_sm()
                            .text_color(rgb(TEXT_ERROR))
                            .on_click(on_cancel)
                            .child("Cancel"),
                    )
                }),
        )
        // ── Query error
        .when_some(query_error, |el, err| {
            el.child(
                div()
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .mx_3()
                    .mt_2()
                    .rounded_md()
                    .bg(rgb(BG_ERROR))
                    .border_1()
                    .border_color(rgb(BORDER_ERROR))
                    .text_xs()
                    .text_color(rgb(TEXT_ERROR))
                    .child(err),
            )
        })
        // ── Results grid (or loading / empty state)
        .child(if is_running {
            div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .text_sm()
                .text_color(rgb(TEXT_SECONDARY))
                .child("Running query…")
        } else if let Some((columns, column_types, rows, loading_next, all_loaded)) = results {
            render_data_grid(columns, column_types, rows, loading_next, all_loaded, scroll_handle)
        } else {
            div()
                .flex_1()
                .flex()
                .justify_center()
                .items_center()
                .text_sm()
                .text_color(rgb(TEXT_MUTED))
                .child("Run a query or select a table from the sidebar")
        })
}
