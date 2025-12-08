use std::sync::Arc;

use gpui::{
    actions, div, prelude::*, px, rgb, size, App, Application, Bounds, ClickEvent,
    Context, KeyBinding, MouseButton, MouseDownEvent, TitlebarOptions,
    UniformListScrollHandle, Window, WindowBounds, WindowOptions, point,
};
use gpui_component::{
    Root,
    input::{InputState},
};
use db::DbDriver;
use state::{AppState, ConnectionStatus, PagedTableData};
use ui::theme::{BG_CARD, BORDER, CHUNK_SIZE, TEXT_ERROR, TEXT_PRIMARY};

use crate::{connections, db, state, ui};

actions!(db_client, [RunQuery]);

// ─── Root component ───────────────────────────────────────────────────────────

pub struct AppRoot {
    state: AppState,
    // Connection form input states
    name_input: gpui::Entity<InputState>,
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
    pending_fill: Option<[String; 5]>, // [name, host, port, user, database]
    // Connection index + cursor position for the right-click context menu.
    context_menu_conn: Option<(usize, gpui::Point<gpui::Pixels>)>,
    // Whether the "test connection succeeded" modal is visible.
    show_test_success: bool,
}

impl AppRoot {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("My connection"));
        let host_input = cx.new(|cx| InputState::new(window, cx).default_value("localhost"));
        let port_input = cx.new(|cx| InputState::new(window, cx).default_value("5432"));
        let user_input = cx.new(|cx| InputState::new(window, cx).placeholder("postgres"));
        let password_input = cx.new(|cx| InputState::new(window, cx).placeholder(""));
        let db_input = cx.new(|cx| InputState::new(window, cx).placeholder("mydb"));
        let sql_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("SELECT * FROM table LIMIT 100")
        });

        Self {
            state: AppState::new(),
            name_input,
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
            context_menu_conn: None,
            show_test_success: false,
        }
    }

    // ── Home sidebar (disconnected state) ────────────────────────────────────

    fn render_home_sidebar(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let on_new_connection: ui::home_sidebar::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.state.show_connection_form = true;
                this.state.selected_conn_idx = None;
                this.state.connection_error = None;
                cx.notify();
            }));

        let saved_conns: Vec<_> = self
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
                let name_for_fill = name.clone();

                let on_click: ui::home_sidebar::ClickCb =
                    Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.pending_fill = Some([
                            name_for_fill.clone(),
                            host.clone(),
                            port.clone(),
                            user.clone(),
                            database.clone(),
                        ]);
                        this.state.show_connection_form = true;
                        this.state.selected_conn_idx = Some(idx);
                        this.state.connection_error = None;
                        this.context_menu_conn = None;
                        cx.notify();
                    }));

                let on_right_click: ui::home_sidebar::RightClickCb =
                    Box::new(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        this.context_menu_conn = Some((idx, event.position));
                        cx.notify();
                    }));

                (name, on_click, on_right_click)
            })
            .collect();

        let selected_idx = self.state.selected_conn_idx;
        ui::home_sidebar::render(saved_conns, selected_idx, on_new_connection)
    }

    // ── Connection form (right panel when show_connection_form = true) ────────

    fn render_connection_form(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let is_connecting = self.state.connection_status == ConnectionStatus::Connecting;
        let conn_error = self.state.connection_error.clone();

        // Save: if a connection is selected, update it in-place; otherwise upsert by name.
        let on_save: ui::connection_screen::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                let name = this.name_input.read(cx).value().to_string();
                let host = this.host_input.read(cx).value().to_string();
                let port = this.port_input.read(cx).value().to_string();
                let user = this.user_input.read(cx).value().to_string();
                let database = this.db_input.read(cx).value().to_string();
                let name = if name.trim().is_empty() {
                    format!("{user}@{host}/{database}")
                } else {
                    name
                };
                let conn = connections::SavedConnection { name, host, port, user, database };
                match this.state.selected_conn_idx {
                    Some(idx) => this.conn_store.update_at(idx, conn),
                    None => this.conn_store.upsert(conn),
                }
                cx.notify();
            }));

        // Test: same as Connect but shows result without navigating.
        let on_test: ui::connection_screen::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.state.connection_status == ConnectionStatus::Connecting {
                    return;
                }
                let host = this.host_input.read(cx).value().to_string();
                let port = this.port_input.read(cx).value().to_string();
                let user = this.user_input.read(cx).value().to_string();
                let password = this.password_input.read(cx).value().to_string();
                let database = this.db_input.read(cx).value().to_string();

                this.state.connection_status = ConnectionStatus::Connecting;
                this.state.connection_error = None;
                cx.notify();

                cx.spawn(async move |this_weak, async_cx| {
                    let result = db::run_blocking(move || {
                        db::postgres::PostgresDriver::connect(
                            &host, &port, &user, &password, &database,
                        )
                        .map_err(|e| e.to_string())
                    })
                    .await;

                    this_weak
                        .update(async_cx, |model, cx| {
                            model.state.connection_status = ConnectionStatus::Disconnected;
                            match result {
                                Some(Ok(_)) => {
                                    model.state.connection_error = None;
                                    model.show_test_success = true;
                                }
                                Some(Err(e)) => model.state.connection_error = Some(e),
                                None => {
                                    model.state.connection_error =
                                        Some("Connection thread failed".to_string());
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                })
                .detach();
            }));

        // Connect: connect and navigate to main view.
        let on_connect: ui::connection_screen::ClickCb =
            Box::new(cx.listener(|this, _: &ClickEvent, _, cx| {
                if this.state.connection_status == ConnectionStatus::Connecting {
                    return;
                }
                let name = this.name_input.read(cx).value().to_string();
                let host = this.host_input.read(cx).value().to_string();
                let port = this.port_input.read(cx).value().to_string();
                let user = this.user_input.read(cx).value().to_string();
                let password = this.password_input.read(cx).value().to_string();
                let database = this.db_input.read(cx).value().to_string();

                let conn_name = if name.trim().is_empty() {
                    format!("{user}@{host}/{database}")
                } else {
                    name
                };
                let conn_to_save = connections::SavedConnection {
                    name: conn_name,
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
                                    model.state.show_connection_form = false;
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
            &self.name_input,
            &self.host_input,
            &self.port_input,
            &self.user_input,
            &self.password_input,
            &self.db_input,
            is_connecting,
            conn_error,
            on_test,
            on_save,
            on_connect,
        )
    }

    // ── Empty right panel ─────────────────────────────────────────────────────

    fn render_empty_panel() -> gpui::Div {
        div()
            .flex_1()
            .h_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(rgb(ui::theme::BG_APP))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    // Abstract watermark shape
                    .child(
                        div()
                            .w(px(64.0))
                            .h(px(64.0))
                            .rounded(px(16.0))
                            .bg(rgb(0x161b22))
                            .border_1()
                            .border_color(rgb(0x21262d))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_color(rgb(0x2d333b))
                                    .child("db"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x2d333b))
                            .child("Select or create a connection"),
                    ),
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
        if let Some([name, host, port, user, db]) = self.pending_fill.take() {
            self.name_input.update(cx, |s, cx| s.set_value(name, window, cx));
            self.host_input.update(cx, |s, cx| s.set_value(host, window, cx));
            self.port_input.update(cx, |s, cx| s.set_value(port, window, cx));
            self.user_input.update(cx, |s, cx| s.set_value(user, window, cx));
            self.db_input.update(cx, |s, cx| s.set_value(db, window, cx));
        }

        // Snapshot copy-able state before any borrows.
        let context_menu = self.context_menu_conn;
        let show_connection_form = self.state.show_connection_form;
        let show_test_success = self.show_test_success;

        let on_run_action = cx.listener(|this, _: &RunQuery, _, cx| this.do_run(cx));

        let root = div()
            .id("app-root")
            .flex()
            .w_full()
            .h_full()
            .bg(rgb(ui::theme::BG_APP))
            .on_action(on_run_action);

        let base = match self.state.connection_status {
            ConnectionStatus::Disconnected | ConnectionStatus::Connecting => {
                let home_sidebar = self.render_home_sidebar(cx);
                let right_panel: gpui::Div = if show_connection_form {
                    self.render_connection_form(cx)
                } else {
                    Self::render_empty_panel()
                };
                root.child(home_sidebar).child(right_panel)
            }
            ConnectionStatus::Connected => {
                let panel = self.render_main_panel(cx);
                let sidebar = self.render_sidebar(cx);
                root.child(sidebar).child(panel)
            }
        };

        // ── Context menu overlay (only in disconnected state)
        //
        // Timing contract:
        //   • Menu items use on_mouse_down → fire immediately on press, before any other phase.
        //   • Backdrop uses on_click (press + release) → only fires when the user completes a
        //     click entirely on the backdrop, never while a menu item is being pressed.
        let base = if let Some((idx, pos)) = context_menu {
            // Backdrop dismiss: on_click so it never fires while a menu item on_mouse_down is live.
            let on_dismiss = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.context_menu_conn = None;
                cx.notify();
            });
            // Actions: on_mouse_down so they fire before the backdrop can interfere.
            let on_duplicate = cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                if let Some(conn) = this.conn_store.connections().get(idx).cloned() {
                    let mut dup = conn;
                    dup.name = format!("{} (copy)", dup.name);
                    this.conn_store.upsert(dup);
                }
                this.context_menu_conn = None;
                cx.notify();
            });
            let on_delete = cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                this.conn_store.remove(idx);
                if this.state.selected_conn_idx == Some(idx) {
                    this.state.selected_conn_idx = None;
                    this.state.show_connection_form = false;
                }
                this.context_menu_conn = None;
                cx.notify();
            });

            base
                // Full-screen backdrop: dismiss on completed click outside the menu.
                .child(
                    div()
                        .id("ctx-backdrop")
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h_full()
                        .on_click(on_dismiss),
                )
                // Floating menu positioned at the cursor.
                .child(
                    div()
                        .id("ctx-menu")
                        .absolute()
                        .left(pos.x)
                        .top(pos.y)
                        .w(px(160.0))
                        .bg(rgb(BG_CARD))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .rounded_md()
                        .py_1()
                        .child(
                            div()
                                .id("ctx-duplicate")
                                .px_3()
                                .py(px(7.0))
                                .cursor_pointer()
                                .text_sm()
                                .text_color(rgb(TEXT_PRIMARY))
                                .on_mouse_down(MouseButton::Left, on_duplicate)
                                .child("Duplicate"),
                        )
                        .child(
                            div()
                                .id("ctx-delete")
                                .px_3()
                                .py(px(7.0))
                                .cursor_pointer()
                                .text_sm()
                                .text_color(rgb(TEXT_ERROR))
                                .on_mouse_down(MouseButton::Left, on_delete)
                                .child("Delete"),
                        ),
                )
        } else {
            base
        };

        // ── Test-success modal
        if show_test_success {
            let on_ok = cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_test_success = false;
                cx.notify();
            });
            with_test_success_modal(base, on_ok)
        } else {
            base
        }
    }
}

fn with_test_success_modal(
    base: gpui::Stateful<gpui::Div>,
    on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    use gpui::{px, rgb};
    use crate::ui::theme::*;

    base.child(
        // Dark full-screen backdrop
        div()
            .id("test-modal-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .bg(rgb(0x080b10))
            .flex()
            .justify_center()
            .items_center()
            .child(
                // Modal card
                div()
                    .w(px(300.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .p_8()
                    .bg(rgb(BG_CARD))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_lg()
                    // ── Green checkmark circle
                    .child(
                        div()
                            .w(px(52.0))
                            .h(px(52.0))
                            .rounded(px(26.0))
                            .bg(rgb(0x1a4731))
                            .border_1()
                            .border_color(rgb(0x2ea043))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xl()
                            .text_color(rgb(0x3fb950))
                            .child("✓"),
                    )
                    // ── Message
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT_PRIMARY))
                                    .child("Connection successful"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_SECONDARY))
                                    .child("Your database is reachable"),
                            ),
                    )
                    // ── OK button
                    .child(
                        div()
                            .id("test-modal-ok")
                            .mt_2()
                            .px_8()
                            .h(px(34.0))
                            .flex()
                            .items_center()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(rgb(ACCENT))
                            .text_sm()
                            .text_color(rgb(0xffffff))
                            .on_click(on_ok)
                            .child("OK"),
                    ),
            ),
    )
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn run() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.bind_keys([KeyBinding::new("cmd-enter", RunQuery, None)]);

        let bounds = Bounds::centered(None, size(px(760.0), px(520.0)), cx);
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
