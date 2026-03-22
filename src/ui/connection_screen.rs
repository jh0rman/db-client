use gpui::{div, prelude::*, px, rgb, App, ClickEvent, Window};
use gpui_component::input::{Input, InputState};
use crate::connections::DbType;
use super::theme::*;

pub type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

const LABEL_W: f32 = 80.0;

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
    db_type: &DbType,
    name_input: &gpui::Entity<InputState>,
    // PostgreSQL fields
    host_input: &gpui::Entity<InputState>,
    port_input: &gpui::Entity<InputState>,
    user_input: &gpui::Entity<InputState>,
    password_input: &gpui::Entity<InputState>,
    db_input: &gpui::Entity<InputState>,
    // SQLite field
    path_input: &gpui::Entity<InputState>,
    is_connecting: bool,
    conn_error: Option<String>,
    on_postgres: ClickCb,
    on_sqlite: ClickCb,
    on_test: ClickCb,
    on_save: ClickCb,
    on_connect: ClickCb,
) -> gpui::Div {
    let is_postgres = *db_type == DbType::Postgres;

    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .justify_center()
        .bg(rgb(BG_APP))
        .px(px(40.0))
        // ── DB type toggle
        .child(
            div()
                .flex()
                .flex_row()
                .mb(px(24.0))
                .ml(px(LABEL_W + 12.0))
                .gap_1()
                .child(
                    div()
                        .id("tab-postgres")
                        .h(px(30.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .when(is_postgres, |el| {
                            el.bg(rgb(BG_CARD))
                                .border_1()
                                .border_color(rgb(BORDER))
                                .text_color(rgb(TEXT_PRIMARY))
                        })
                        .when(!is_postgres, |el| el.text_color(rgb(TEXT_MUTED)))
                        .on_click(on_postgres)
                        .child("PostgreSQL"),
                )
                .child(
                    div()
                        .id("tab-sqlite")
                        .h(px(30.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .cursor_pointer()
                        .text_sm()
                        .when(!is_postgres, |el| {
                            el.bg(rgb(BG_CARD))
                                .border_1()
                                .border_color(rgb(BORDER))
                                .text_color(rgb(TEXT_PRIMARY))
                        })
                        .when(is_postgres, |el| el.text_color(rgb(TEXT_MUTED)))
                        .on_click(on_sqlite)
                        .child("SQLite"),
                ),
        )
        // ── Fields
        .child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                // Name
                .child(field_row("Name", name_input))
                // PostgreSQL-specific fields
                .when(is_postgres, |el| {
                    el
                        // Host + Port
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
                        .child(field_row("User", user_input))
                        .child(field_row("Password", password_input))
                        .child(field_row("Database", db_input))
                })
                // SQLite-specific field
                .when(!is_postgres, |el| {
                    el.child(field_row("File", path_input))
                })
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
                // Test (left) — only for postgres (sqlite connect is instant)
                .when(is_postgres, |el| {
                    el.child(
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
                })
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
                        .when(is_connecting, |el| {
                            el.bg(rgb(ACCENT_BG)).text_color(rgb(TEXT_MUTED))
                        })
                        .on_click(on_connect)
                        .child(if is_connecting { "Connecting…" } else { "Connect" }),
                ),
        )
}
