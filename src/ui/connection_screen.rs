use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window};
use gpui_component::input::{Input, InputState};
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

const LABEL_W: f32 = 80.0;

/// A single label + input row (label right-aligned, input flex-1).
fn field_row(label: &'static str, input: &gpui::Entity<InputState>) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(LABEL_W))
                .flex()
                .justify_end()
                .text_sm()
                .text_color(rgb(TEXT_SECONDARY))
                .child(label),
        )
        .child(div().flex_1().child(Input::new(input)))
}

pub fn render(
    name_input: &gpui::Entity<InputState>,
    host_input: &gpui::Entity<InputState>,
    port_input: &gpui::Entity<InputState>,
    user_input: &gpui::Entity<InputState>,
    password_input: &gpui::Entity<InputState>,
    db_input: &gpui::Entity<InputState>,
    is_connecting: bool,
    conn_error: Option<String>,
    on_test: ClickCb,
    on_save: ClickCb,
    on_connect: ClickCb,
) -> gpui::Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .justify_center()
        .bg(rgb(BG_APP))
        .px(px(40.0))
        // ── Fields
        .child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                // Name
                .child(field_row("Name", name_input))
                // Host + Port (share label column width)
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .w(px(LABEL_W))
                                .flex()
                                .justify_end()
                                .text_sm()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("Host"),
                        )
                        .child(div().flex_1().child(Input::new(host_input)))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("Port"),
                        )
                        .child(div().w(px(70.0)).child(Input::new(port_input))),
                )
                // User
                .child(field_row("User", user_input))
                // Password
                .child(field_row("Password", password_input))
                // Database
                .child(field_row("Database", db_input))
                // Error banner
                .when_some(conn_error, |el, err| {
                    el.child(
                        div()
                            .ml(px(LABEL_W + 12.0))
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(BG_ERROR))
                            .border_1()
                            .border_color(rgb(BORDER_ERROR))
                            .text_xs()
                            .text_color(rgb(TEXT_ERROR))
                            .child(err),
                    )
                }),
        )
        // ── Bottom action bar
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .mt(px(32.0))
                // Test (left)
                .child(
                    div()
                        .id("btn-test")
                        .h(px(34.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(BG_CARD))
                        .text_sm()
                        .text_color(rgb(TEXT_PRIMARY))
                        .on_click(on_test)
                        .child("Test"),
                )
                // Spacer
                .child(div().flex_1())
                // Save
                .child(
                    div()
                        .id("btn-save")
                        .h(px(34.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(BG_CARD))
                        .text_sm()
                        .text_color(rgb(TEXT_PRIMARY))
                        .on_click(on_save)
                        .child("Save"),
                )
                .child(div().w(px(8.0)))
                // Connect
                .child(
                    div()
                        .id("btn-connect")
                        .h(px(34.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(rgb(ACCENT))
                        .text_sm()
                        .text_color(rgb(0xffffff))
                        .when(is_connecting, |el| el.bg(rgb(ACCENT_BG)).text_color(rgb(TEXT_MUTED)))
                        .on_click(on_connect)
                        .child(if is_connecting { "Connecting…" } else { "Connect" }),
                ),
        )
}
