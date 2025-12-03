use std::sync::Arc;

use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent,
    Context, KeyBinding, TitlebarOptions, UniformListScrollHandle, Window, WindowBounds,
    WindowOptions, point,
};
use gpui_component::{
    Root,
    input::{InputState},
};
use db::DbDriver;
use state::{AppState, ConnectionStatus, PagedTableData};
use ui::theme::CHUNK_SIZE;

use crate::{connections, db, state, ui};

actions!(db_client, [RunQuery]);

// ─── Root component ───────────────────────────────────────────────────────────

pub struct AppRoot {
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
    // Persisted connections
    conn_store: connections::ConnectionStore,
    // Set by "fill from saved connection" click; applied in render where window is available.
    pending_fill: Option<[String; 4]>, // [host, port, user, database]
}

impl AppRoot {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            conn_store: connections::ConnectionStore::load(),
            pending_fill: None,
        }
    }

    // ── Connection screen ─────────────────────────────────────────────────────

    fn render_connection_screen(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_connecting = self.state.connection_status == ConnectionStatus::Connecting;
        let conn_error = self.state.connection_error.clone();

        // Build per-saved-connection fill+delete listeners before borrowing self further.
        let saved_conn_rows: Vec<_> = self
            .conn_store
            .connections()
            .iter()
            .enumerate()
            .map(|(idx, conn)| {
                let host = conn.host.clone();
                let port = conn.port.clone();
                let user = conn.user.clone();
                let database = conn.database.clone();
                let name = conn.name.clone();

                let on_fill: ui::connection_screen::ClickCb =
                    Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.pending_fill = Some([
                            host.clone(),
                            port.clone(),
                            user.clone(),
                            database.clone(),
                        ]);
                        cx.notify();
                    }));

                let on_delete: ui::connection_screen::ClickCb =
                    Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.conn_store.remove(idx);
                        cx.notify();
                    }));

                (name, on_fill, on_delete)
            })
            .collect();

        let on_connect: ui::connection_screen::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
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

                cx.spawn(async move |this_weak, async_cx| {
                    let result = db::run_blocking(move || {
                        db::postgres::PostgresDriver::connect(
                            &host, &port, &user, &password, &database,
                        )
                        .map(|d| Arc::new(d) as Arc<dyn DbDriver>)
                        .map_err(|e| e.to_string())
                    })
                    .await;

                    match result {
                        Some(Ok(driver)) => {
                            this_weak
                                .update(async_cx, |model, cx| {
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
                                    model.conn_store.upsert(conn_to_save);
                                    cx.notify();
                                })
                                .ok();
                        }
                        Some(Err(err)) => {
                            this_weak
                                .update(async_cx, |model, cx| {
                                    model.state.connection_status = ConnectionStatus::Disconnected;
                                    model.state.connection_error = Some(err);
                                    cx.notify();
                                })
                                .ok();
                        }
                        None => {
                            this_weak
                                .update(async_cx, |model, cx| {
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
            }));

        ui::connection_screen::render(
            &self.host_input,
            &self.port_input,
            &self.user_input,
            &self.password_input,
            &self.db_input,
            is_connecting,
            conn_error,
            saved_conn_rows,
            on_connect,
        )
    }

    // ── Sidebar ───────────────────────────────────────────────────────────────

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let on_table_clicks: Vec<ui::sidebar::ClickCb> = self
            .state
            .tables
            .iter()
            .map(|table| {
                let name = table.clone();
                let cb: ui::sidebar::ClickCb = Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let browse_query = format!("SELECT * FROM \"{}\"", name);
                    let result = this.state.driver.as_ref().and_then(|d| {
                        d.execute_query_paged(&browse_query, 0, CHUNK_SIZE).ok()
                    });
                    if let Some(td) = result {
                        let all_loaded = td.rows.len() < CHUNK_SIZE;
                        this.state.results = Some(PagedTableData {
                            columns: td.columns,
                            column_types: td.column_types,
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
                }));
                cb
            })
            .collect();

        ui::sidebar::render(
            self.state.tables.clone(),
            self.state.active_table.clone(),
            on_table_clicks,
        )
    }

    // ── Main panel ────────────────────────────────────────────────────────────

    fn render_main_panel(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let is_running = self.state.query_in_progress;

        let on_run: ui::query_panel::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| this.do_run(cx)));

        let on_cancel: ui::query_panel::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.query_task = None; // dropping the Task cancels the future
                this.state.query_in_progress = false;
                cx.notify();
            }));

        // ── Lazy-load trigger: if user has scrolled ≥75% of loaded content,
        //    kick off the next chunk (fires once because loading_next guards re-entry).
        if let Some(r) = &self.state.results {
            if !r.all_loaded && !r.loading_next {
                let scroll_y: f32 =
                    self.scroll_handle.0.borrow().base_handle.offset().y.into();
                let max_y: f32 =
                    self.scroll_handle.0.borrow().base_handle.max_offset().height.into();
                // Also load when the list fits entirely in the viewport (max_y ≈ 0)
                let should_load = max_y < 1.0 || scroll_y / max_y >= 0.75;
                if should_load {
                    self.load_next_chunk(cx);
                }
            }
        }

        let query_error = self.state.query_error.clone();
        let results_snapshot = self.state.results.as_ref().map(|r| {
            (
                r.columns.clone(),
                r.column_types.clone(),
                Arc::clone(&r.rows),
                r.loading_next,
                r.all_loaded,
            )
        });

        ui::query_panel::render(
            &self.sql_input,
            is_running,
            query_error,
            results_snapshot,
            on_run,
            on_cancel,
            &self.scroll_handle,
        )
    }

    // ── Query execution ───────────────────────────────────────────────────────

    /// Execute the current SQL query (shared by button click and Cmd+Enter shortcut).
    fn do_run(&mut self, cx: &mut Context<Self>) {
        if self.state.query_in_progress {
            return;
        }
        let query = self.sql_input.read(cx).value().to_string();
        let trimmed = query.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        // Synthetic dataset for virtualisation stress-test
        if trimmed.eq_ignore_ascii_case("test 100k") {
            self.state.results = Some(synthetic_stress_result());
            self.state.query_error = None;
            self.state.active_table = None;
            cx.notify();
            return;
        }

        let driver = match self.state.driver.clone() {
            Some(d) => d,
            None => return,
        };

        self.state.query_in_progress = true;
        self.state.query_error = None;
        cx.notify();

        let trimmed_for_state = trimmed.clone();
        let task = cx.spawn(async move |this_weak, async_cx| {
            let result = db::run_blocking(move || {
                driver
                    .execute_query_paged(&trimmed, 0, CHUNK_SIZE)
                    .map_err(|e| e.to_string())
            })
            .await;

            match result {
                Some(Ok(data)) => {
                    this_weak
                        .update(async_cx, |model, cx| {
                            let all_loaded = data.rows.len() < CHUNK_SIZE;
                            model.state.results = Some(PagedTableData {
                                columns: data.columns,
                                column_types: data.column_types,
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
                Some(Err(err)) => {
                    this_weak
                        .update(async_cx, |model, cx| {
                            model.state.query_error = Some(err);
                            model.state.results = None;
                            model.state.query_in_progress = false;
                            cx.notify();
                        })
                        .ok();
                }
                None => {} // future was cancelled by Cancel button — state already reset
            }
        });
        self.query_task = Some(task);
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
            let result = db::run_blocking(move || {
                driver
                    .execute_query_paged(&query, offset, chunk_size)
                    .map_err(|e| e.to_string())
            })
            .await;

            match result {
                Some(Ok(new_data)) => {
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
}

// ─── Synthetic stress-test dataset ───────────────────────────────────────────

fn synthetic_stress_result() -> PagedTableData {
    let columns = vec![
        "id".to_string(),
        "name".to_string(),
        "value".to_string(),
        "status".to_string(),
        "created_at".to_string(),
    ];
    let column_types = vec![
        "int8".to_string(),
        "text".to_string(),
        "float8".to_string(),
        "text".to_string(),
        "date".to_string(),
    ];
    let rows: Vec<Vec<Option<String>>> = (0u64..100_000)
        .map(|i| {
            vec![
                Some(i.to_string()),
                if i % 50 == 0 { None } else { Some(format!("Item {i}")) },
                Some(format!("{:.4}", i as f64 * 1.337)),
                if i % 3 == 0 {
                    Some("active".to_string())
                } else {
                    Some("inactive".to_string())
                },
                Some(format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1)),
            ]
        })
        .collect();
    PagedTableData {
        columns,
        column_types,
        query: String::new(),
        rows: Arc::new(rows),
        chunk_size: CHUNK_SIZE,
        all_loaded: true,
        loading_next: false,
    }
}

// ─── Render impl ─────────────────────────────────────────────────────────────

impl Render for AppRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Apply pending form-fill from a saved-connection click (needs window).
        if let Some([host, port, user, db]) = self.pending_fill.take() {
            self.host_input.update(cx, |s, cx| s.set_value(host, window, cx));
            self.port_input.update(cx, |s, cx| s.set_value(port, window, cx));
            self.user_input.update(cx, |s, cx| s.set_value(user, window, cx));
            self.db_input.update(cx, |s, cx| s.set_value(db, window, cx));
        }

        let on_run_action = cx.listener(|this, _: &RunQuery, _, cx| this.do_run(cx));

        let root = div()
            .flex()
            .w_full()
            .h_full()
            .bg(rgb(ui::theme::BG_APP))
            .on_action(on_run_action);

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

pub fn run() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.bind_keys([KeyBinding::new("cmd-enter", RunQuery, None)]);

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
