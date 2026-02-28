use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Cell, Paragraph, Row, Table, TableState},
};

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

#[derive(Clone, Debug, PartialEq)]
pub enum AppMode {
    Dashboard,
    Connecting,
    Connected,
}

#[derive(Clone, Debug)]
pub struct DiagnosticsView {
    pub session_id: String,
    pub hostname: String,
    pub rtt_ms: Option<f64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub throughput_up: f64,
    pub throughput_down: f64,
    pub packet_loss_pct: f64,
    pub quality: String,
    pub uptime: String,
    pub cipher: String,
    pub auth_method: String,
    pub channels: u32,
    pub server_banner: String,
}

pub struct App {
    pub hosts: Vec<HostDisplay>,
    pub selected_host: usize,
    pub mode: AppMode,
    pub diagnostics: Option<DiagnosticsView>,
    pub status_message: Option<String>,
    pub table_state: TableState,
}

impl App {
    pub fn new() -> Self {
        Self {
            hosts: Vec::new(),
            selected_host: 0,
            mode: AppMode::Dashboard,
            diagnostics: None,
            status_message: None,
            table_state: TableState::default(),
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

    pub fn next(&mut self) {
        if self.hosts.is_empty() {
            return;
        }
        self.selected_host = (self.selected_host + 1) % self.hosts.len();
        self.table_state.select(Some(self.selected_host));
    }

    pub fn previous(&mut self) {
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

    pub fn set_diagnostics(&mut self, diag: DiagnosticsView) {
        self.diagnostics = Some(diag);
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics = None;
    }
}

pub fn render_dashboard(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Title bar
    let title = Paragraph::new("⚡ ESSH — Enterprise SSH Client")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    // Host table
    let header = Row::new(["Name", "Hostname", "Port", "User", "Status", "Last Seen", "Tags"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .hosts
        .iter()
        .map(|h| {
            let status_cell = match h.status {
                HostStatus::Online => Cell::from("● Online").style(Style::default().fg(Color::Green)),
                HostStatus::Offline => Cell::from("● Offline").style(Style::default().fg(Color::Red)),
                HostStatus::Unknown => Cell::from("● Unknown").style(Style::default().fg(Color::DarkGray)),
            };
            Row::new([
                Cell::from(h.name.clone()),
                Cell::from(h.hostname.clone()),
                Cell::from(h.port.to_string()),
                Cell::from(h.user.clone()),
                status_cell,
                Cell::from(h.last_seen.clone()),
                Cell::from(h.tags.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(7),
        Constraint::Percentage(10),
        Constraint::Percentage(12),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title("Hosts"))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

    // Status bar
    let mut status_lines = vec![Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Connect  "),
        Span::styled("[a]", Style::default().fg(Color::Cyan)),
        Span::raw(" Add Host  "),
        Span::styled("[d]", Style::default().fg(Color::Cyan)),
        Span::raw(" Delete  "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" Refresh  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit"),
    ])];

    if let Some(ref msg) = app.status_message {
        status_lines.push(Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        )));
    }

    let status_bar = Paragraph::new(status_lines)
        .block(Block::bordered().title("Status"));
    frame.render_widget(status_bar, chunks[2]);
}

pub fn render_status_bar(frame: &mut Frame, area: Rect, diag: &DiagnosticsView) {
    let rtt_text = match diag.rtt_ms {
        Some(rtt) => format!("{:.1}ms", rtt),
        None => "N/A".to_string(),
    };

    let quality_color = match diag.quality.as_str() {
        "Excellent" | "Good" => Color::Green,
        "Fair" => Color::Yellow,
        _ => Color::Red,
    };

    let line = Line::from(vec![
        Span::styled("RTT: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&rtt_text),
        Span::raw("  "),
        Span::styled("↑ ", Style::default().fg(Color::Green)),
        Span::raw(format!("{:.1} KB/s", diag.throughput_up / 1024.0)),
        Span::raw("  "),
        Span::styled("↓ ", Style::default().fg(Color::Cyan)),
        Span::raw(format!("{:.1} KB/s", diag.throughput_down / 1024.0)),
        Span::raw("  "),
        Span::styled("Loss: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{:.1}%", diag.packet_loss_pct)),
        Span::raw("  "),
        Span::styled("Quality: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&diag.quality, Style::default().fg(quality_color)),
        Span::raw("  "),
        Span::styled("Up: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&diag.uptime),
        Span::raw("  "),
        Span::styled("Cipher: ", Style::default().fg(Color::DarkGray)),
        Span::raw(&diag.cipher),
        Span::raw("  "),
        Span::styled("Ch: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{}", diag.channels)),
    ]);

    let block = Block::bordered().title(diag.hostname.as_str());
    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}
