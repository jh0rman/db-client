mod connections;
mod db;
mod state;

use std::sync::Arc;

use gpui::{
    div, uniform_list, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent, Context,
    TitlebarOptions, UniformListScrollHandle, Window, WindowBounds, WindowOptions, point,
};
use gpui_component::{
    Root,
    input::{Input, InputState},
};
use db::DbDriver;
use state::{AppState, ConnectionStatus, PagedTableData};

const CELL_W: f32 = 160.0;
const ROW_H: f32 = 36.0;
/// Rows fetched per lazy-load request.
const CHUNK_SIZE: usize = 1_000;

// ─── Color palette ───────────────────────────────────────────────────────────
const BG_APP: u32 = 0x0f1117;
const BG_SIDEBAR: u32 = 0x161b22;
const BG_PANEL: u32 = 0x0d1117;
const BG_ACTIVE: u32 = 0x1c2333;
const BG_HEADER: u32 = 0x161b22;
const BG_CARD: u32 = 0x161b22;
const BORDER: u32 = 0x21262d;
const TEXT_MUTED: u32 = 0x484f58;
const TEXT_SECONDARY: u32 = 0x7d8590;
const TEXT_PRIMARY: u32 = 0xcdd9e5;
const ACCENT: u32 = 0x388bfd;
const ACCENT_BG: u32 = 0x1f3458;

// ─── Root component ──────────────────────────────────────────────────────────

struct AppRoot {
    state: AppState,
    // Connection form input states
    host_input: gpui::Entity<InputState>,
    port_input: gpui::Entity<InputState>,
    user_input: gpui::Entity<InputState>,
    password_input: gpui::Entity<InputState>,
    db_input: gpui::Entity<InputState>,
    // SQL editor
    sql_input: gpui::Entity<InputState>,
    // Holds the running query task; dropping it cancels the future
    query_task: Option<gpui::Task<()>>,
    // Tracks virtual list scroll position for lazy-load trigger
    scroll_handle: UniformListScrollHandle,
    // Persisted connections loaded from ~/.db-client/connections.json
    saved_connections: Vec<connections::SavedConnection>,
    // Set by "fill from saved connection" click; applied in render where window is available.
    pending_fill: Option<[String; 4]>, // [host, port, user, database]
}

impl AppRoot {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let host_input = cx.new(|cx| InputState::new(window, cx).default_value("localhost"));
        let port_input = cx.new(|cx| InputState::new(window, cx).default_value("5432"));
        let user_input = cx.new(|cx| InputState::new(window, cx).placeholder("postgres"));
        let password_input = cx.new(|cx| InputState::new(window, cx).placeholder("password"));
        let db_input = cx.new(|cx| InputState::new(window, cx).placeholder("mydb"));
        let sql_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("SELECT * FROM table LIMIT 100")
        });

        Self {
            state: AppState::new(),
            host_input,
            port_input,
            user_input,
            password_input,
            db_input,
            sql_input,
            query_task: None,
            scroll_handle: UniformListScrollHandle::new(),
            saved_connections: connections::load(),
            pending_fill: None,
        }
    }

    // ── Connection screen ─────────────────────────────────────────────────────

    fn render_connection_screen(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connecting = self.state.connection_status == ConnectionStatus::Connecting;
        let conn_error = self.state.connection_error.clone();

        // Build per-saved-connection fill+delete listeners before borrowing self further.
        let saved_conn_rows: Vec<_> = self
            .saved_connections
            .iter()
            .enumerate()
            .map(|(idx, conn)| {
                let host = conn.host.clone();
                let port = conn.port.clone();
                let user = conn.user.clone();
                let database = conn.database.clone();
                let name = conn.name.clone();

                let on_fill = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.pending_fill = Some([
                        host.clone(),
                        port.clone(),
                        user.clone(),
                        database.clone(),
                    ]);
                    cx.notify();
                });

                let on_delete = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    connections::remove(&mut this.saved_connections, idx);
                    cx.notify();
                });

                (name, on_fill, on_delete)
            })
            .collect();

        let on_connect = cx.listener(|this, _: &ClickEvent, _, cx| {
            if this.state.connection_status == ConnectionStatus::Connecting {
                return;
            }
            // Read form values before entering the async closure.
            let host = this.host_input.read(cx).value().to_string();
            let port = this.port_input.read(cx).value().to_string();
            let user = this.user_input.read(cx).value().to_string();
            let password = this.password_input.read(cx).value().to_string();
            let database = this.db_input.read(cx).value().to_string();

            // Build a saved-connection entry for auto-save on success.
            let conn_to_save = connections::SavedConnection {
                name: format!("{user}@{host}/{database}"),
                host: host.clone(),
                port: port.clone(),
                user: user.clone(),
                database: database.clone(),
            };

            this.state.connection_status = ConnectionStatus::Connecting;
            this.state.connection_error = None;
            cx.notify();

            cx.spawn(async move |this, async_cx| {
                // Run the blocking Postgres connect on a background thread.
                let (tx, rx) = futures::channel::oneshot::channel::<Result<Arc<dyn DbDriver>, String>>();
                std::thread::spawn(move || {
                    let result = db::postgres::PostgresDriver::connect(
                        &host, &port, &user, &password, &database,
                    )
                    .map(|d| Arc::new(d) as Arc<dyn DbDriver>)
                    .map_err(|e| e.to_string());
                    let _ = tx.send(result);
                });

                match rx.await {
                    Ok(Ok(driver)) => {
                        this.update(async_cx, |model, cx| {
                            match driver.get_tables() {
                                Ok(tables) => model.state.tables = tables,
                                Err(e) => {
                                    eprintln!("[db-client] get_tables error: {e}");
                                    model.state.query_error =
                                        Some(format!("get_tables: {e}"));
                                }
                            }
                            model.state.driver = Some(driver);
                            model.state.connection_status = ConnectionStatus::Connected;
                            // Auto-save the connection (upsert by name).
                            connections::upsert(&mut model.saved_connections, conn_to_save);
                            cx.notify();
                        })
                        .ok();
                    }
                    Ok(Err(err)) => {
                        this.update(async_cx, |model, cx| {
                            model.state.connection_status = ConnectionStatus::Disconnected;
                            model.state.connection_error = Some(err);
                            cx.notify();
                        })
                        .ok();
                    }
                    Err(_) => {
                        this.update(async_cx, |model, cx| {
                            model.state.connection_status = ConnectionStatus::Disconnected;
                            model.state.connection_error =
                                Some("Connection thread failed".to_string());
                            cx.notify();
                        })
                        .ok();
                    }
                }
            })
            .detach();
        });

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
                                    .child(Input::new(&self.host_input)),
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
                                    .child(Input::new(&self.port_input)),
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
                            .child(Input::new(&self.user_input)),
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
                            .child(Input::new(&self.password_input)),
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
                            .child(Input::new(&self.db_input)),
                    )
                    // ── Connection error (shown only on failure)
                    .when_some(conn_error, |el, err| {
                        el.child(
                            div()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(rgb(0x2d1515))
                                .border_1()
                                .border_color(rgb(0x6e1b1b))
                                .text_xs()
                                .text_color(rgb(0xff6b6b))
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

    // ── Main view (sidebar + panel) ───────────────────────────────────────────

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let table_listeners: Vec<_> = self
            .state
            .tables
            .iter()
            .map(|table| {
                let name = table.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let browse_query = format!("SELECT * FROM \"{}\"", name);
                    let result = this.state.driver.as_ref().and_then(|d| {
                        d.execute_query_paged(&browse_query, 0, CHUNK_SIZE).ok()
                    });
                    if let Some(td) = result {
                        let all_loaded = td.rows.len() < CHUNK_SIZE;
                        this.state.results = Some(PagedTableData {
                            columns: td.columns,
                            query: browse_query,
                            rows: td.rows,
                            chunk_size: CHUNK_SIZE,
                            all_loaded,
                            loading_next: false,
                        });
                        this.state.query_error = None;
                    }
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
                                        el.bg(rgb(BG_ACTIVE)).text_color(rgb(ACCENT))
                                    })
                                    .when(!is_active, |el| el.text_color(rgb(TEXT_PRIMARY)))
                                    .on_click(on_click)
                                    .child(table)
                            }),
                    ),
            )
    }

    fn render_main_panel(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let is_running = self.state.query_in_progress;

        let on_run = cx.listener(|this, _: &ClickEvent, _, cx| {
            if this.state.query_in_progress {
                return;
            }
            let query = this.sql_input.read(cx).value().to_string();
            let trimmed = query.trim().to_string();
            if trimmed.is_empty() {
                return;
            }
            // Synthetic dataset for virtualisation stress-test
            if trimmed.eq_ignore_ascii_case("test 100k") {
                let columns = vec![
                    "id".to_string(), "name".to_string(), "value".to_string(),
                    "status".to_string(), "created_at".to_string(),
                ];
                let rows: Vec<Vec<String>> = (0u64..100_000)
                    .map(|i| vec![
                        i.to_string(),
                        format!("Item {i}"),
                        format!("{:.4}", i as f64 * 1.337),
                        if i % 3 == 0 { "active" } else { "inactive" }.to_string(),
                        format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1),
                    ])
                    .collect();
                this.state.results = Some(PagedTableData {
                    columns,
                    query: String::new(),
                    rows: Arc::new(rows),
                    chunk_size: CHUNK_SIZE,
                    all_loaded: true,
                    loading_next: false,
                });
                this.state.query_error = None;
                this.state.active_table = None;
                cx.notify();
                return;
            }

            let driver = match this.state.driver.clone() {
                Some(d) => d,
                None => return,
            };

            this.state.query_in_progress = true;
            this.state.query_error = None;
            cx.notify();

            let task = cx.spawn(async move |this_weak, async_cx| {
                let (tx, rx) = futures::channel::oneshot::channel::<Result<crate::state::TableData, String>>();
                let trimmed_for_state = trimmed.clone();
                std::thread::spawn(move || {
                    let result = driver
                        .execute_query_paged(&trimmed, 0, CHUNK_SIZE)
                        .map_err(|e| e.to_string());
                    let _ = tx.send(result);
                });

                match rx.await {
                    Ok(Ok(data)) => {
                        this_weak
                            .update(async_cx, |model, cx| {
                                let all_loaded = data.rows.len() < CHUNK_SIZE;
                                model.state.results = Some(PagedTableData {
                                    columns: data.columns,
                                    query: trimmed_for_state,
                                    rows: data.rows,
                                    chunk_size: CHUNK_SIZE,
                                    all_loaded,
                                    loading_next: false,
                                });
                                model.state.query_error = None;
                                model.state.active_table = None;
                                model.state.query_in_progress = false;
                                cx.notify();
                            })
                            .ok();
                    }
                    Ok(Err(err)) => {
                        this_weak
                            .update(async_cx, |model, cx| {
                                model.state.query_error = Some(err);
                                model.state.results = None;
                                model.state.query_in_progress = false;
                                cx.notify();
                            })
                            .ok();
                    }
                    Err(_) => {} // future was cancelled by Cancel button — state already reset
                }
            });
            this.query_task = Some(task);
        });

        let on_cancel = cx.listener(|this, _: &ClickEvent, _, cx| {
            this.query_task = None; // dropping the Task cancels the future
            this.state.query_in_progress = false;
            cx.notify();
        });

        // ── Lazy-load trigger: if user has scrolled ≥75% of loaded content,
        //    kick off the next chunk (fires once because loading_next guards re-entry).
        if let Some(r) = &self.state.results {
            if !r.all_loaded && !r.loading_next {
                let scroll_y: f32 = self.scroll_handle.0.borrow().base_handle.offset().y.into();
                let max_y: f32 = self.scroll_handle.0.borrow().base_handle.max_offset().height.into();
                // Also load when the list fits entirely in the viewport (max_y ≈ 0)
                let should_load = max_y < 1.0 || scroll_y / max_y >= 0.75;
                if should_load {
                    self.load_next_chunk(cx);
                }
            }
        }

        let query_error = self.state.query_error.clone();
        let results_snapshot = self.state.results.as_ref().map(|r| {
            (r.columns.clone(), Arc::clone(&r.rows), r.loading_next, r.all_loaded)
        });

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
                    .child(div().flex_1().child(Input::new(&self.sql_input)))
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
                                .border_color(rgb(0x6e1b1b))
                                .text_sm()
                                .text_color(rgb(0xff6b6b))
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
                        .bg(rgb(0x2d1515))
                        .border_1()
                        .border_color(rgb(0x6e1b1b))
                        .text_xs()
                        .text_color(rgb(0xff6b6b))
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
            } else if let Some((columns, rows, loading_next, all_loaded)) = results_snapshot {
                Self::render_data_grid(columns, rows, loading_next, all_loaded, &self.scroll_handle)
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

    /// Loads the next page of rows in the background and appends them to state.results.
    fn load_next_chunk(&mut self, cx: &mut Context<Self>) {
        let r = match &mut self.state.results {
            Some(r) => r,
            None => return,
        };
        r.loading_next = true;

        let driver = match self.state.driver.clone() {
            Some(d) => d,
            None => return,
        };
        let query = r.query.clone();
        let offset = r.rows.len();
        let chunk_size = r.chunk_size;

        cx.spawn(async move |this_weak, async_cx| {
            let (tx, rx) =
                futures::channel::oneshot::channel::<Result<crate::state::TableData, String>>();
            std::thread::spawn(move || {
                let result = driver
                    .execute_query_paged(&query, offset, chunk_size)
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });

            match rx.await {
                Ok(Ok(new_data)) => {
                    this_weak
                        .update(async_cx, |model, cx| {
                            if let Some(r) = &mut model.state.results {
                                let all_loaded = new_data.rows.len() < chunk_size;
                                // Extend existing rows with the new chunk (O(n) clone once)
                                let mut combined = (*r.rows).clone();
                                combined.extend((*new_data.rows).iter().cloned());
                                r.rows = Arc::new(combined);
                                r.all_loaded = all_loaded;
                                r.loading_next = false;
                            }
                            cx.notify();
                        })
                        .ok();
                }
                _ => {
                    // On error or cancellation, just stop trying so we don't loop
                    this_weak
                        .update(async_cx, |model, cx| {
                            if let Some(r) = &mut model.state.results {
                                r.loading_next = false;
                                r.all_loaded = true;
                            }
                            cx.notify();
                        })
                        .ok();
                }
            }
        })
        .detach();
    }

    fn render_data_grid(
        columns: Vec<String>,
        rows: Arc<Vec<Vec<String>>>,
        loading_next: bool,
        all_loaded: bool,
        scroll_handle: &UniformListScrollHandle,
    ) -> gpui::Div {
        let row_count = rows.len();

        // Arcs cloned once per render; the uniform_list closure holds them cheaply per frame.
        let cols_hdr = Arc::new(columns);
        let cols_list = Arc::clone(&cols_hdr);
        let rows_list = Arc::clone(&rows);

        div()
            .flex_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            // ── Fixed header (not virtualized)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_shrink_0()
                    .h(px(ROW_H))
                    .bg(rgb(BG_HEADER))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .children(cols_hdr.iter().map(|col| {
                        div()
                            .w(px(CELL_W))
                            .flex_shrink_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .px_3()
                            .border_r_1()
                            .border_color(rgb(BORDER))
                            .text_xs()
                            .text_color(rgb(TEXT_SECONDARY))
                            .child(col.to_uppercase())
                    })),
            )
            // ── Virtual body: uniform_list only instantiates visible rows each frame
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
                                        let cell: String =
                                            rows[i].get(c).cloned().unwrap_or_default();
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
                                            .text_color(rgb(TEXT_PRIMARY))
                                            .child(cell)
                                    }))
                            })
                            .collect()
                    },
                )
                .track_scroll(scroll_handle.clone())
                .flex_1(),
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
}

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Apply pending form-fill from a saved-connection click (needs window).
        if let Some([host, port, user, db]) = self.pending_fill.take() {
            self.host_input.update(cx, |s, cx| s.set_value(host, window, cx));
            self.port_input.update(cx, |s, cx| s.set_value(port, window, cx));
            self.user_input.update(cx, |s, cx| s.set_value(user, window, cx));
            self.db_input.update(cx, |s, cx| s.set_value(db, window, cx));
        }

        let root = div().flex().w_full().h_full().bg(rgb(BG_APP));

        match self.state.connection_status {
            ConnectionStatus::Disconnected | ConnectionStatus::Connecting => {
                root.child(self.render_connection_screen(cx))
            }
            ConnectionStatus::Connected => {
                let panel = self.render_main_panel(cx);
                let sidebar = self.render_sidebar(cx);
                root.child(sidebar).child(panel)
            }
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(12.0), px(12.0))),
                }),
                ..Default::default()
            },
            |window, cx| {
                let app_root = cx.new(|cx| AppRoot::new(window, cx));
                cx.new(|cx| Root::new(app_root, window, cx))
            },
        )
        .unwrap();
    });
}
