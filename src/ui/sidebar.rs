use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window};
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A 2×2 grid icon used to represent a table.
fn table_icon() -> gpui::Div {
    div()
        .w(px(16.0))
        .h(px(16.0))
        .flex()
        .flex_wrap()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(3.0))
        .bg(rgb(0x1c3a5e))
        .child(div().w(px(5.0)).h(px(5.0)).rounded(px(1.0)).bg(rgb(0x4493f8)))
        .child(div().w(px(5.0)).h(px(5.0)).rounded(px(1.0)).bg(rgb(0x4493f8)))
        .child(div().w(px(5.0)).h(px(5.0)).rounded(px(1.0)).bg(rgb(0x4493f8)))
        .child(div().w(px(5.0)).h(px(5.0)).rounded(px(1.0)).bg(rgb(0x4493f8)))
}

/// A small decorative icon button (no action).
fn icon_btn(label: &'static str) -> gpui::Div {
    div()
        .w(px(24.0))
        .h(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_xs()
        .text_color(rgb(TEXT_SECONDARY))
        .child(label)
}

pub fn render(
    conn_name: String,
    tables: Vec<String>,
    active_table: Option<String>,
    on_table_click: Vec<ClickCb>,
) -> impl gpui::IntoElement {
    let table_count = tables.len();

    div()
        .w(px(240.0))
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(BG_SIDEBAR))
        .border_r_1()
        .border_color(rgb(BORDER))
        // ── Traffic-light spacer
        .child(div().h(px(36.0)))
        // ── Connection header
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .pb_3()
                .border_b_1()
                .border_color(rgb(BORDER))
                // DB icon
                .child(
                    div()
                        .w(px(24.0))
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.0))
                        .bg(rgb(0x1c3a5e))
                        .text_xs()
                        .text_color(rgb(0x4493f8))
                        .child("db"),
                )
                // Connection name
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT_PRIMARY))
                        .child(conn_name),
                )
                // Decorative action icons
                .child(icon_btn("⊕"))
                .child(icon_btn("≡"))
                .child(icon_btn("⌕")),
        )
        // ── Schema + table list
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    // Schema header row
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .px_3()
                        .py_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("public"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(table_count.to_string()),
                        )
                        .child(div().flex_1())
                        .child(icon_btn("↻"))
                        .child(icon_btn("…")),
                )
                // Table rows
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .children(
                            tables
                                .into_iter()
                                .zip(on_table_click)
                                .enumerate()
                                .map(|(i, (table, on_click))| {
                                    let is_active = active_table.as_deref() == Some(&table);
                                    div()
                                        .id(("table-item", i))
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .px_3()
                                        .py(px(5.0))
                                        .mx_1()
                                        .rounded_md()
                                        .cursor_pointer()
                                        .when(is_active, |el| el.bg(rgb(BG_SIDEBAR_ACTIVE)))
                                        .on_click(on_click)
                                        .child(table_icon())
                                        .child(
                                            div()
                                                .flex_1()
                                                .overflow_hidden()
                                                .text_sm()
                                                .when(is_active, |el| {
                                                    el.text_color(rgb(ACCENT))
                                                })
                                                .when(!is_active, |el| {
                                                    el.text_color(rgb(TEXT_PRIMARY))
                                                })
                                                .child(table),
                                        )
                                }),
                        ),
                ),
        )
}
