pub mod dashboard;
pub mod help;
pub mod host_monitor;
pub mod session_view;
pub mod widgets;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    widgets::TableState,
};

use crate::session::manager::SessionManager;
use crate::monitor::{HostMetrics, history::MetricHistory};
use crate::diagnostics::DiagnosticsSnapshot;

#[derive(Clone, Debug)]
pub struct HostDisplay {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub user: String,
    pub status: HostStatus,
    pub last_seen: String,
    pub tags: String,
}

#[derive(Clone, Debug)]
pub enum HostStatus {
    Online,
    Offline,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DashboardTab {
    Sessions,
    Hosts,
    Fleet,
    Config,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppView {
    Dashboard,
    Session,
    Monitor,
}

pub struct App {
    pub hosts: Vec<HostDisplay>,
    pub selected_host: usize,
    pub table_state: TableState,
    pub session_manager: SessionManager,
    pub view: AppView,
    pub dashboard_tab: DashboardTab,
    pub status_message: Option<String>,
    pub monitor_sort: host_monitor::ProcessSort,
    pub monitor_process_scroll: usize,
    pub show_help: bool,
    // Per-session diagnostics snapshots (indexed by session manager index)
    pub session_diagnostics: Vec<Option<DiagnosticsSnapshot>>,
    // Per-session host metrics (indexed by session manager index)
    pub session_metrics: Vec<Option<HostMetrics>>,
    pub session_cpu_history: Vec<MetricHistory>,
    pub session_mem_history: Vec<MetricHistory>,
    pub session_net_rx_history: Vec<MetricHistory>,
    pub session_net_tx_history: Vec<MetricHistory>,
}

impl App {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            hosts: Vec::new(),
            selected_host: 0,
            table_state: TableState::default(),
            session_manager: SessionManager::new(max_sessions),
            view: AppView::Dashboard,
            dashboard_tab: DashboardTab::Hosts,
            status_message: None,
            monitor_sort: host_monitor::ProcessSort::Cpu,
            monitor_process_scroll: 0,
            show_help: false,
            session_diagnostics: Vec::new(),
            session_metrics: Vec::new(),
            session_cpu_history: Vec::new(),
            session_mem_history: Vec::new(),
            session_net_rx_history: Vec::new(),
            session_net_tx_history: Vec::new(),
        }
    }

    pub fn set_hosts(&mut self, hosts: Vec<HostDisplay>) {
        self.hosts = hosts;
        if self.selected_host >= self.hosts.len() {
            self.selected_host = 0;
        }
        if !self.hosts.is_empty() {
            self.table_state.select(Some(self.selected_host));
        } else {
            self.table_state.select(None);
        }
    }

    pub fn selected_host(&self) -> Option<&HostDisplay> {
        self.hosts.get(self.selected_host)
    }

    pub fn next_host(&mut self) {
        if self.hosts.is_empty() {
            return;
        }
        self.selected_host = (self.selected_host + 1) % self.hosts.len();
        self.table_state.select(Some(self.selected_host));
    }

    pub fn prev_host(&mut self) {
        if self.hosts.is_empty() {
            return;
        }
        if self.selected_host == 0 {
            self.selected_host = self.hosts.len() - 1;
        } else {
            self.selected_host -= 1;
        }
        self.table_state.select(Some(self.selected_host));
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some(msg);
    }

    pub fn add_session_tracking(&mut self, history_samples: usize) {
        self.session_diagnostics.push(None);
        self.session_metrics.push(None);
        self.session_cpu_history.push(MetricHistory::new(history_samples));
        self.session_mem_history.push(MetricHistory::new(history_samples));
        self.session_net_rx_history.push(MetricHistory::new(history_samples));
        self.session_net_tx_history.push(MetricHistory::new(history_samples));
    }

    pub fn remove_session_tracking(&mut self, index: usize) {
        if index < self.session_diagnostics.len() {
            self.session_diagnostics.remove(index);
            self.session_metrics.remove(index);
            self.session_cpu_history.remove(index);
            self.session_mem_history.remove(index);
            self.session_net_rx_history.remove(index);
            self.session_net_tx_history.remove(index);
        }
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    match app.view {
        AppView::Dashboard => {
            dashboard::render(
                frame,
                frame.area(),
                &app.session_manager.sessions,
                &app.hosts,
                app.selected_host,
                &mut app.table_state,
                app.dashboard_tab,
                app.status_message.as_deref(),
            );
        }
        AppView::Session => {
            if let Some(active_idx) = app.session_manager.active_index {
                let area = frame.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // tab bar
                        Constraint::Min(4),    // terminal
                        Constraint::Length(2), // status bar
                        Constraint::Length(2), // footer
                    ])
                    .split(area);

                session_view::render_tab_bar(
                    frame,
                    chunks[0],
                    &app.session_manager.sessions,
                    active_idx,
                );

                // Resize virtual terminal to match the render area
                let term_area = chunks[1];
                if let Some(session) = app.session_manager.sessions.get_mut(active_idx) {
                    session.terminal.resize(term_area.height, term_area.width);
                }

                if let Some(session) = app.session_manager.sessions.get(active_idx) {
                    session_view::render_terminal(frame, chunks[1], session);
                    let diag = app.session_diagnostics.get(active_idx).and_then(|d| d.as_ref());
                    session_view::render_status_bar(frame, chunks[2], session, diag);
                }

                session_view::render_footer(frame, chunks[3]);
            }
        }
        AppView::Monitor => {
            if let Some(active_idx) = app.session_manager.active_index {
                let area = frame.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // tab bar
                        Constraint::Min(10),   // monitor
                    ])
                    .split(area);

                session_view::render_tab_bar(
                    frame,
                    chunks[0],
                    &app.session_manager.sessions,
                    active_idx,
                );

                let metrics = app.session_metrics.get(active_idx)
                    .and_then(|m| m.as_ref())
                    .cloned()
                    .unwrap_or_default();

                let cpu_hist = app.session_cpu_history.get(active_idx)
                    .cloned()
                    .unwrap_or_else(|| MetricHistory::new(60));
                let mem_hist = app.session_mem_history.get(active_idx)
                    .cloned()
                    .unwrap_or_else(|| MetricHistory::new(60));
                let rx_hist = app.session_net_rx_history.get(active_idx)
                    .cloned()
                    .unwrap_or_else(|| MetricHistory::new(60));
                let tx_hist = app.session_net_tx_history.get(active_idx)
                    .cloned()
                    .unwrap_or_else(|| MetricHistory::new(60));

                host_monitor::render(
                    frame,
                    chunks[1],
                    &metrics,
                    &cpu_hist,
                    &mem_hist,
                    &rx_hist,
                    &tx_hist,
                    &app.monitor_sort,
                    app.monitor_process_scroll,
                );
            }
        }
    }

    // Help overlay (rendered on top of any view)
    if app.show_help {
        help::render(frame);
    }
}
