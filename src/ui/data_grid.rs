use std::sync::Arc;
use gpui::{div, uniform_list, prelude::*, px, rgb, UniformListScrollHandle};
use gpui_component::scroll::Scrollbar;
use super::theme::*;

pub fn render_data_grid(
    columns: Vec<String>,
    column_types: Vec<String>,
    rows: Arc<Vec<Vec<Option<String>>>>,
    loading_next: bool,
    all_loaded: bool,
    scroll_handle: &UniformListScrollHandle,
) -> gpui::Div {
    let row_count = rows.len();

    // Arcs cloned once per render; the uniform_list closure holds them cheaply per frame.
    let cols_hdr = Arc::new(columns);
    let types_hdr = Arc::new(column_types);
    let cols_list = Arc::clone(&cols_hdr);
    let rows_list = Arc::clone(&rows);

    div()
        .flex_1()
        .flex()
        .flex_col()
        .overflow_hidden()
        // ── Content area: relative so the scrollbar can be pinned to its right edge
        .child(
            div()
                .flex_1()
                .relative()
                .overflow_hidden()
                // ── Horizontal scroll wrapper (header + body scroll together)
                .child(
                    div()
                        .id("grid-hscroll")
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .flex()
                        .flex_col()
                        .overflow_x_scroll()
                        // ── Fixed header
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .flex_shrink_0()
                                .h(px(48.0))
                                .bg(rgb(BG_HEADER))
                                .border_b_1()
                                .border_color(rgb(BORDER))
                                .children(cols_hdr.iter().enumerate().map(|(i, col)| {
                                    let type_label = types_hdr.get(i).cloned().unwrap_or_default();
                                    div()
                                        .w(px(CELL_W))
                                        .flex_shrink_0()
                                        .h_full()
                                        .flex()
                                        .flex_col()
                                        .justify_center()
                                        .px_3()
                                        .gap_0p5()
                                        .border_r_1()
                                        .border_color(rgb(BORDER))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(TEXT_SECONDARY))
                                                .child(col.to_uppercase()),
                                        )
                                        .when(!type_label.is_empty(), |el| {
                                            el.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(TEXT_MUTED))
                                                    .child(type_label),
                                            )
                                        })
                                })),
                        )
                        // ── Virtual body rows
                        .child(
                            uniform_list(
                                "grid-rows",
                                row_count,
                                move |visible_range, _window, _cx| {
                                    let rows = Arc::clone(&rows_list);
                                    let cols = Arc::clone(&cols_list);
                                    visible_range
                                        .map(|i| {
                                            let is_alt = i % 2 == 1;
                                            div()
                                                .flex()
                                                .flex_row()
                                                .flex_shrink_0()
                                                .h(px(ROW_H))
                                                .border_b_1()
                                                .border_color(rgb(BORDER))
                                                .when(is_alt, |el| el.bg(rgb(BG_HEADER)))
                                                .children((0..cols.len()).map(|c| {
                                                    let cell =
                                                        rows[i].get(c).and_then(|v| v.as_deref());
                                                    let is_null = cell.is_none();
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
                                                        .when(is_null, |el| {
                                                            el.text_color(rgb(TEXT_MUTED))
                                                                .child("NULL")
                                                        })
                                                        .when(!is_null, |el| {
                                                            el.text_color(rgb(TEXT_PRIMARY)).child(
                                                                cell.unwrap_or_default().to_string(),
                                                            )
                                                        })
                                                }))
                                        })
                                        .collect()
                                },
                            )
                            .track_scroll(scroll_handle.clone())
                            .flex_1(),
                        ),
                )
                // ── Vertical scrollbar pinned to the viewport right edge
                .child(
                    div()
                        .absolute()
                        .top(px(48.0))
                        .right_0()
                        .bottom_0()
                        .child(Scrollbar::vertical(scroll_handle)),
                ),
        )
        // ── Footer: row count + lazy-load status
        .child(
            div()
                .flex_shrink_0()
                .px_4()
                .py_2()
                .border_t_1()
                .border_color(rgb(BORDER))
                .text_xs()
                .text_color(rgb(TEXT_MUTED))
                .child(if loading_next {
                    format!("{row_count} rows — loading more…")
                } else if all_loaded {
                    format!(
                        "{} row{}",
                        row_count,
                        if row_count == 1 { "" } else { "s" }
                    )
                } else {
                    format!("{row_count} rows — scroll for more")
                }),
        )
}
