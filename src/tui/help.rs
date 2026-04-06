use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;
use crate::tui::meta_key_hint;

pub fn render(f: &mut Frame, theme: &Theme) {
    let area = f.area();

    // Center a popup wide/tall enough for the full shortcut reference.
    let popup_width = 68u16.min(area.width.saturating_sub(4));
    let popup_height = 48u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup = Rect::new(x, y, popup_width, popup_height);

    // Clear the area behind the popup
    f.render_widget(Clear, popup);

    let key_style = Style::default().fg(theme.key_hint).bold();
    let heading_style = Style::default().fg(theme.active_tab).bold();
    let desc_style = Style::default().fg(theme.text_primary);
    let dim = Style::default().fg(theme.text_muted);
    let session_jump_hint = format!("    {:<12}", meta_key_hint("1-9"));
    let prev_next_hint = format!("    {:<12}", meta_key_hint("←/→"));
    let last_session_hint = format!("    {:<12}", meta_key_hint("Tab"));
    let split_hint = format!("    {:<12}", meta_key_hint("s"));
    let split_resize_hint = format!("    {:<12}", meta_key_hint("[/]"));
    let monitor_hint = format!("    {:<12}", meta_key_hint("m"));
    let portfwd_hint = format!("    {:<12}", meta_key_hint("p"));
    let files_hint = format!("    {:<12}", meta_key_hint("f"));
    let detach_hint = format!("    {:<12}", meta_key_hint("d"));
    let close_hint = format!("    {:<12}", meta_key_hint("w"));
    let theme_hint = format!("    {:<12}", meta_key_hint("t"));

    let lines = vec![
        Line::raw(""),
        Line::styled("  Global", heading_style),
        Line::from(vec![
            Span::styled("    ?           ", key_style),
            Span::styled("Toggle this help menu", desc_style),
        ]),
        Line::from(vec![
            Span::styled(session_jump_hint, key_style),
            Span::styled("Jump to session N", desc_style),
        ]),
        Line::from(vec![
            Span::styled(prev_next_hint, key_style),
            Span::styled("Previous / next session", desc_style),
        ]),
        Line::from(vec![
            Span::styled(last_session_hint, key_style),
            Span::styled("Switch to last session", desc_style),
        ]),
        Line::from(vec![
            Span::styled(split_hint, key_style),
            Span::styled("Toggle split-pane (terminal + monitor)", desc_style),
        ]),
        Line::from(vec![
            Span::styled(split_resize_hint, key_style),
            Span::styled("Adjust split-pane width", desc_style),
        ]),
        Line::from(vec![
            Span::styled(monitor_hint, key_style),
            Span::styled("Toggle host monitor (full-screen)", desc_style),
        ]),
        Line::from(vec![
            Span::styled(portfwd_hint, key_style),
            Span::styled("Toggle port forwarding manager", desc_style),
        ]),
        Line::from(vec![
            Span::styled(files_hint, key_style),
            Span::styled("File browser (upload/download)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+p      ", key_style),
            Span::styled("Command palette (fuzzy search)", desc_style),
        ]),
        Line::from(vec![
            Span::styled(detach_hint, key_style),
            Span::styled("Detach to dashboard", desc_style),
        ]),
        Line::from(vec![
            Span::styled(close_hint, key_style),
            Span::styled("Close active session", desc_style),
        ]),
        Line::from(vec![
            Span::styled(theme_hint, key_style),
            Span::styled("Cycle theme", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Dashboard", heading_style),
        Line::from(vec![
            Span::styled("    1-4         ", key_style),
            Span::styled("Switch tab (Sessions/Hosts/Fleet/Config)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    j/k ↑/↓     ", key_style),
            Span::styled("Navigate host list", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Enter       ", key_style),
            Span::styled("Connect to selected host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    a           ", key_style),
            Span::styled("Add host (user@host[:port])", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    r           ", key_style),
            Span::styled("Refresh hosts", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    /           ", key_style),
            Span::styled("Start host search/filter", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    e           ", key_style),
            Span::styled("Edit host (Hosts) / config (Config)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    t           ", key_style),
            Span::styled("Cycle theme", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    d           ", key_style),
            Span::styled("Delete selected host", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    q / Ctrl+c  ", key_style),
            Span::styled("Quit", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Search", heading_style),
        Line::from(vec![
            Span::styled("    type / ⌫    ", key_style),
            Span::styled("Filter hosts as you type", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Enter / Esc ", key_style),
            Span::styled("Connect first match / cancel search", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Command Palette", heading_style),
        Line::from(vec![
            Span::styled("    ↑/↓ Tab     ", key_style),
            Span::styled("Move selection (Shift+Tab moves up)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Enter       ", key_style),
            Span::styled("Run selected action", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Esc / Ctrl+p", key_style),
            Span::styled("Close palette", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Session", heading_style),
        Line::from(vec![
            Span::styled("    (all keys)  ", key_style),
            Span::styled("Forwarded to remote shell", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  File Browser", heading_style),
        Line::from(vec![
            Span::styled("    ↑/↓ Enter   ", key_style),
            Span::styled("Select and open directories", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Backspace   ", key_style),
            Span::styled("Go to parent directory", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Tab         ", key_style),
            Span::styled("Switch local/remote pane", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    u d m Del   ", key_style),
            Span::styled("Upload, download, mkdir, delete", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Port Forwarding", heading_style),
        Line::from(vec![
            Span::styled("    ↑/↓ a d     ", key_style),
            Span::styled("Select, add, and delete forwards", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    type Enter  ", key_style),
            Span::styled("In add mode, enter a forward spec", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    ⌫ / Esc     ", key_style),
            Span::styled("Edit or cancel add-mode input", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Monitor", heading_style),
        Line::from(vec![
            Span::styled("    s           ", key_style),
            Span::styled("Toggle sort (CPU / Memory)", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    ↑/↓         ", key_style),
            Span::styled("Scroll process list", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    Esc         ", key_style),
            Span::styled("Return to terminal", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    t           ", key_style),
            Span::styled("Cycle theme", desc_style),
        ]),
        Line::raw(""),
        Line::styled("  Config", heading_style),
        Line::from(vec![
            Span::styled("    e           ", key_style),
            Span::styled("Open ~/.essh/config.toml and reload", desc_style),
        ]),
        Line::from(vec![
            Span::styled("    notification_patterns", key_style),
            Span::styled("  Background alert regexes", desc_style),
        ]),
        Line::raw(""),
        Line::styled("                    Press ? or Esc to close", dim),
    ];

    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(theme.brand).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.brand));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, popup);
}
