use gpui::{div, prelude::*, px, rgb, App, ClickEvent, MouseButton, MouseDownEvent, Window};
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
pub type RightClickCb = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

pub fn render(
    saved_conns: Vec<(String, ClickCb, RightClickCb)>,
    selected_idx: Option<usize>,
    on_new_connection: ClickCb,
) -> gpui::Div {
    div()
        .w(px(240.0))
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(BG_SIDEBAR))
        .border_r_1()
        .border_color(rgb(BORDER))
        // ── Traffic-light spacer (transparent titlebar overlaps here)
        .child(div().h(px(36.0)))
        // ── New Connection button
        .child(
            div()
                .px_3()
                .pb_3()
                .border_b_1()
                .border_color(rgb(BORDER))
                .child(
                    div()
                        .id("btn-new-conn")
                        .w_full()
                        .h(px(32.0))
                        .flex()
                        .justify_center()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(rgb(TEXT_PRIMARY))
                        .text_sm()
                        .text_color(rgb(BG_APP))
                        .on_click(on_new_connection)
                        .child("New Connection"),
                ),
        )
        // ── Saved connections
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .overflow_hidden()
                .py_1()
                .children(
                    saved_conns
                        .into_iter()
                        .enumerate()
                        .map(|(i, (name, on_click, on_right_click))| {
                            let is_selected = selected_idx == Some(i);
                            div()
                                .id(("home-conn", i))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .py(px(7.0))
                                .mx_1()
                                .rounded_md()
                                .cursor_pointer()
                                .text_sm()
                                .when(is_selected, |el| {
                                    el.bg(rgb(ACCENT)).text_color(rgb(0xffffff))
                                })
                                .when(!is_selected, |el| el.text_color(rgb(TEXT_PRIMARY)))
                                .on_click(on_click)
                                .on_mouse_down(MouseButton::Right, on_right_click)
                                .child(
                                    div()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .bg(rgb(0x336791))
                                        .text_color(rgb(0xffffff))
                                        .text_xs()
                                        .child("pg"),
                                )
                                .child(div().flex_1().overflow_hidden().child(name))
                        }),
                ),
        )
}
