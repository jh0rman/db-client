use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window};
use gpui_component::input::{Input, InputState};
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub fn render(
    host_input: &gpui::Entity<InputState>,
    port_input: &gpui::Entity<InputState>,
    user_input: &gpui::Entity<InputState>,
    password_input: &gpui::Entity<InputState>,
    db_input: &gpui::Entity<InputState>,
    is_connecting: bool,
    conn_error: Option<String>,
    saved_conn_rows: Vec<(String, ClickCb, ClickCb)>,
    on_connect: ClickCb,
) -> impl gpui::IntoElement {
    div()
        .flex_1()
        .flex()
        .justify_center()
        .items_center()
        .bg(rgb(BG_APP))
        .child(
            div()
                .w(px(420.0))
                .flex()
                .flex_col()
                .gap_6()
                .p_8()
                .rounded_lg()
                .bg(rgb(BG_CARD))
                .border_1()
                .border_color(rgb(BORDER))
                // ── Title
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xl()
                                .text_color(rgb(TEXT_PRIMARY))
                                .child("DB Client"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("Connect to a database"),
                        ),
                )
                // ── Saved connections (shown only when at least one exists)
                .when(!saved_conn_rows.is_empty(), |card| {
                    card.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child("SAVED CONNECTIONS"),
                            )
                            .children(
                                saved_conn_rows
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, (name, on_fill, on_delete))| {
                                        div()
                                            .id(("saved-conn", i))
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py_2()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(rgb(BORDER))
                                            .cursor_pointer()
                                            .on_click(on_fill)
                                            // Connection name (flex-1 so delete button stays right)
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .text_sm()
                                                    .text_color(rgb(TEXT_PRIMARY))
                                                    .child(name),
                                            )
                                            // Delete ×
                                            .child(
                                                div()
                                                    .id(("del-conn", i))
                                                    .px_1()
                                                    .text_xs()
                                                    .text_color(rgb(TEXT_MUTED))
                                                    .on_click(on_delete)
                                                    .child("×"),
                                            )
                                    }),
                            ),
                    )
                })
                // ── Host + Port row
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(TEXT_SECONDARY))
                                        .child("HOST"),
                                )
                                .child(Input::new(host_input)),
                        )
                        .child(
                            div()
                                .w(px(90.0))
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(TEXT_SECONDARY))
                                        .child("PORT"),
                                )
                                .child(Input::new(port_input)),
                        ),
                )
                // ── User
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("USER"),
                        )
                        .child(Input::new(user_input)),
                )
                // ── Password
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("PASSWORD"),
                        )
                        .child(Input::new(password_input)),
                )
                // ── Database
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_SECONDARY))
                                .child("DATABASE"),
                        )
                        .child(Input::new(db_input)),
                )
                // ── Connection error (shown only on failure)
                .when_some(conn_error, |el, err| {
                    el.child(
                        div()
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
                })
                // ── Connect button
                .child(
                    div()
                        .id("btn-connect")
                        .w_full()
                        .h(px(38.0))
                        .flex()
                        .justify_center()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(rgb(ACCENT_BG))
                        .border_1()
                        .border_color(rgb(ACCENT))
                        .text_sm()
                        .text_color(rgb(ACCENT))
                        .when(is_connecting, |el| {
                            el.text_color(rgb(TEXT_MUTED))
                                .border_color(rgb(BORDER))
                                .bg(rgb(BG_CARD))
                        })
                        .on_click(on_connect)
                        .child(if is_connecting { "Connecting…" } else { "Connect" }),
                ),
        )
}
