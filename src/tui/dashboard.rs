use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};
use crate::session::{SessionState, Session};
use crate::tui::widgets;

/// Render the dashboard view (no active session focused)
pub fn render(
    f: &mut Frame,
    area: Rect,
    sessions: &[Session],
    hosts: &[super::HostDisplay],
    selected_host: usize,
    table_state: &mut TableState,
    active_tab: super::DashboardTab,
    status_message: Option<&str>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header + tab bar
            Constraint::Min(8),    // main content
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(f, chunks[0], active_tab);

    match active_tab {
        super::DashboardTab::Sessions => render_sessions_tab(f, chunks[1], sessions),
        super::DashboardTab::Hosts => render_hosts_tab(f, chunks[1], hosts, selected_host, table_state),
        super::DashboardTab::Fleet => render_fleet_tab(f, chunks[1], hosts, sessions),
        super::DashboardTab::Config => render_config_tab(f, chunks[1]),
    }

    render_footer(f, chunks[2], active_tab, status_message);
}

fn render_header(f: &mut Frame, area: Rect, active_tab: super::DashboardTab) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    
    let tabs = [
        ("[1] Sessions", super::DashboardTab::Sessions),
        ("[2] Hosts", super::DashboardTab::Hosts),
        ("[3] Fleet", super::DashboardTab::Fleet),
        ("[4] Config", super::DashboardTab::Config),
    ];

    let mut spans: Vec<Span> = vec![
        Span::styled(" ESSH ", Style::default().fg(Color::Cyan).bold()),
        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
    ];

    for (label, tab) in &tabs {
        if *tab == active_tab {
            spans.push(Span::styled(*label, Style::default().fg(Color::Yellow).bold()));
        } else {
            spans.push(Span::raw(*label));
        }
        spans.push(Span::raw("  "));
    }

    spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled("?", Style::default().fg(Color::Cyan)));
    spans.push(Span::styled(":Help", Style::default().fg(Color::DarkGray)));
    spans.push(Span::raw("  │ "));
    spans.push(Span::styled(now, Style::default().fg(Color::DarkGray)));

    let header = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(header, area);
}

fn render_sessions_tab(f: &mut Frame, area: Rect, sessions: &[Session]) {
    if sessions.is_empty() {
        let msg = Paragraph::new(vec![
            Line::raw(""),
            Line::styled("  No active sessions.", Style::default().fg(Color::DarkGray)),
            Line::raw(""),
            Line::styled("  Press [2] to browse hosts, or use 'essh connect <host>' to start a session.", Style::default().fg(Color::DarkGray)),
        ])
        .block(Block::bordered().title("Active Sessions").border_style(Style::default().fg(Color::DarkGray)));
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from(" # ").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Label").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Host").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("User").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Uptime").style(Style::default().fg(Color::Cyan).bold()),
    ]).height(1);

    let rows: Vec<Row> = sessions.iter().enumerate().map(|(i, s)| {
        let state_style = match &s.state {
            SessionState::Active => Style::default().fg(Color::Green),
            SessionState::Suspended => Style::default().fg(Color::DarkGray),
            SessionState::Reconnecting { .. } => Style::default().fg(Color::Red),
            SessionState::Connecting => Style::default().fg(Color::Yellow),
            SessionState::Disconnected { .. } => Style::default().fg(Color::Red),
        };
        let status_text = match &s.state {
            SessionState::Active => "● Active",
            SessionState::Suspended => "● Suspended",
            SessionState::Reconnecting { .. } => "● Recon.",
            SessionState::Connecting => "● Connecting",
            SessionState::Disconnected { .. } => "● Disconnected",
        };

        Row::new(vec![
            Cell::from(format!(" {} ", i + 1)),
            Cell::from(s.label.clone()),
            Cell::from(s.hostname.clone()),
            Cell::from(s.username.clone()),
            Cell::from(status_text).style(state_style),
            Cell::from(widgets::format_duration_short(s.uptime_secs() as i64)),
        ])
    }).collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Percentage(20),
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title("Active Sessions").border_style(Style::default().fg(Color::DarkGray)))
        .row_highlight_style(Style::default().bg(Color::DarkGray).bold())
        .highlight_symbol(">> ");
    f.render_widget(table, area);
}

fn render_hosts_tab(f: &mut Frame, area: Rect, hosts: &[super::HostDisplay], selected: usize, table_state: &mut TableState) {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Hostname").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Port").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("User").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Last Seen").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Tags").style(Style::default().fg(Color::Cyan).bold()),
    ]).height(1);

    let rows: Vec<Row> = hosts.iter().map(|h| {
        let status_cell = match h.status {
            super::HostStatus::Online => Cell::from("● Online").style(Style::default().fg(Color::Green)),
            super::HostStatus::Offline => Cell::from("● Offline").style(Style::default().fg(Color::Red)),
            super::HostStatus::Unknown => Cell::from("○ Unknown").style(Style::default().fg(Color::DarkGray)),
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
    }).collect();

    let widths = [
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(7),
        Constraint::Percentage(10),
        Constraint::Percentage(12),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
    ];

    let title = format!("Hosts ({})", hosts.len());
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::bordered().title(title).border_style(Style::default().fg(Color::DarkGray)))
        .row_highlight_style(Style::default().bg(Color::DarkGray).bold())
        .highlight_symbol(">> ");
    f.render_stateful_widget(table, area, table_state);
}

fn render_fleet_tab(f: &mut Frame, area: Rect, hosts: &[super::HostDisplay], sessions: &[Session]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // fleet health summary
            Constraint::Min(4),    // recent connections (placeholder)
        ])
        .split(area);

    // Fleet health summary
    let online = hosts.iter().filter(|h| matches!(h.status, super::HostStatus::Online)).count();
    let offline = hosts.iter().filter(|h| matches!(h.status, super::HostStatus::Offline)).count();
    let unknown = hosts.iter().filter(|h| matches!(h.status, super::HostStatus::Unknown)).count();
    let total = hosts.len();
    let pct = if total > 0 { (online as f64 / total as f64) * 100.0 } else { 0.0 };

    let bar = widgets::bar_gauge(pct, 40);

    let summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  Online: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", online), Style::default().fg(Color::Green)),
            Span::raw("  │  "),
            Span::styled("Offline: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", offline), Style::default().fg(Color::Red)),
            Span::raw("  │  "),
            Span::styled("Unknown: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", unknown), Style::default().fg(Color::DarkGray)),
            Span::raw("  │  "),
            Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}", total)),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(bar, Style::default().fg(Color::Green)),
            Span::raw(format!(" {:.0}%", pct)),
        ]),
    ])
    .block(Block::bordered().title("Fleet Health").border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(summary, chunks[0]);

    // Active sessions summary
    let active = sessions.iter().filter(|s| matches!(s.state, SessionState::Active)).count();
    let session_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  Active sessions: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", active), Style::default().fg(Color::Green)),
            Span::raw(format!("  │  Total sessions: {}", sessions.len())),
        ]),
    ])
    .block(Block::bordered().title("Sessions").border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(session_info, chunks[1]);
}

fn render_config_tab(f: &mut Frame, area: Rect) {
    let content = Paragraph::new(vec![
        Line::raw(""),
        Line::styled("  Configuration", Style::default().fg(Color::Cyan).bold()),
        Line::raw(""),
        Line::styled("  Config file: ~/.essh/config.toml", Style::default().fg(Color::DarkGray)),
        Line::styled("  Cache DB:    ~/.essh/cache.db", Style::default().fg(Color::DarkGray)),
        Line::styled("  Audit log:   ~/.essh/audit.log", Style::default().fg(Color::DarkGray)),
        Line::raw(""),
        Line::styled("  Use 'essh config edit' or press 'e' to open config in $EDITOR.", Style::default().fg(Color::DarkGray)),
    ])
    .block(Block::bordered().title("Config").border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(content, area);
}

fn render_footer(f: &mut Frame, area: Rect, tab: super::DashboardTab, status: Option<&str>) {
    let mut lines = vec![Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(":Connect  "),
        Span::styled("Alt+1-9", Style::default().fg(Color::Cyan)),
        Span::raw(":Session  "),
        Span::styled("a", Style::default().fg(Color::Cyan)),
        Span::raw(":Add  "),
        Span::styled("/", Style::default().fg(Color::Cyan)),
        Span::raw(":Search  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(":Refresh  "),
        Span::styled("d", Style::default().fg(Color::Cyan)),
        Span::raw(":Delete  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(":Quit"),
    ])];

    if let Some(msg) = status {
        lines.push(Line::from(Span::styled(msg.to_string(), Style::default().fg(Color::Yellow))));
    }

    let footer = Paragraph::new(lines)
        .block(Block::bordered().border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, area);
}
