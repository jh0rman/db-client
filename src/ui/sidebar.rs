use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window};
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub fn render(
    tables: Vec<String>,
    active_table: Option<String>,
    on_table_click: Vec<ClickCb>,
) -> impl gpui::IntoElement {
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
                        .zip(on_table_click)
                        .enumerate()
                        .map(|(i, (table, on_click))| {
                            let is_active = active_table.as_deref() == Some(&table);
                            div()
                                .id(("table-item", i))
                                .px_3()
                                .py_2()
                                .mx_2()
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .when(is_active, |el| {
                                    el.bg(rgb(BG_SIDEBAR_ACTIVE)).text_color(rgb(ACCENT))
                                })
                                .when(!is_active, |el| el.text_color(rgb(TEXT_PRIMARY)))
                                .on_click(on_click)
                                .child(table)
                        }),
                ),
        )
}
