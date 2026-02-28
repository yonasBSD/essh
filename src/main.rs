mod audit;
mod cache;
mod cli;
mod config;
mod diagnostics;
mod ssh;
mod tui;

use std::io::{self, Read as _, Write as _};
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

use audit::{AuditEventType, AuditLogger};
use cache::{CacheDb, HostKeyStatus};
use cli::{AuditAction, Cli, Commands, ConfigAction, HostsAction, KeysAction, SessionAction};
use config::{AppConfig, TofuPolicy};
use diagnostics::DiagnosticsEngine;
use ssh::{AuthMethod, ConnectConfig, SshClient, SshSession};
use tui::{App, HostDisplay, HostStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    AppConfig::ensure_dirs()?;
    let app_config = AppConfig::load()?;

    match cli.command {
        None => run_tui(app_config).await,
        Some(cmd) => run_command(cmd, app_config).await,
    }
}

// ---------------------------------------------------------------------------
// CLI command dispatch
// ---------------------------------------------------------------------------

async fn run_command(cmd: Commands, config: AppConfig) -> anyhow::Result<()> {
    let audit = AuditLogger::default_logger();

    match cmd {
        Commands::Connect {
            target,
            port,
            identity,
            password,
        } => {
            let (user, host) = parse_target(&target, &config);
            let auth = if password {
                let pw = prompt_password(&format!("{}@{}'s password: ", user, host))?;
                AuthMethod::Password(pw)
            } else if let Some(key_path) = identity {
                AuthMethod::KeyFile(key_path)
            } else if let Some(ref default_key) = config.general.default_key {
                let expanded = shellexpand::tilde(default_key).to_string();
                AuthMethod::KeyFile(expanded.into())
            } else {
                let pw = prompt_password(&format!("{}@{}'s password: ", user, host))?;
                AuthMethod::Password(pw)
            };

            let connect_config = ConnectConfig {
                hostname: host.clone(),
                port,
                username: user.clone(),
                auth,
            };

            connect_and_shell(connect_config, &config, &audit).await?;
        }

        Commands::Hosts { action } => match action {
            HostsAction::List { tag } => {
                let db = CacheDb::open_default()?;
                let hosts = if let Some(tag_str) = tag {
                    let (k, v) = tag_str
                        .split_once('=')
                        .ok_or_else(|| anyhow::anyhow!("Tag must be key=value"))?;
                    db.find_hosts_by_tag(k, v)?
                } else {
                    db.list_hosts()?
                };

                if hosts.is_empty() {
                    println!("No cached hosts.");
                } else {
                    println!(
                        "{:<20} {:<30} {:<6} {:<16} {:<24} {}",
                        "Fingerprint", "Hostname", "Port", "Key Type", "Last Seen", "Tags"
                    );
                    println!("{}", "-".repeat(110));
                    for h in &hosts {
                        let tags: Vec<String> =
                            h.tags.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                        println!(
                            "{:<20} {:<30} {:<6} {:<16} {:<24} {}",
                            &h.fingerprint[..20.min(h.fingerprint.len())],
                            h.hostname,
                            h.port,
                            h.key_type,
                            h.last_seen,
                            tags.join(", ")
                        );
                    }
                }
            }
            HostsAction::Add {
                hostname,
                port,
                name: _,
                user: _,
                tag,
            } => {
                let db = CacheDb::open_default()?;
                let tags: std::collections::HashMap<String, String> = tag
                    .iter()
                    .filter_map(|t| t.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
                    .collect();
                db.trust_host(&hostname, None, port, "unknown", "unknown")?;
                if !tags.is_empty() {
                    db.set_host_tags(&hostname, port, &tags)?;
                }
                println!("Host {} added to cache.", hostname);
            }
            HostsAction::Remove { hostname, port } => {
                let db = CacheDb::open_default()?;
                if db.remove_host(&hostname, port)? {
                    println!("Host {} removed.", hostname);
                } else {
                    println!("Host {} not found in cache.", hostname);
                }
            }
            HostsAction::Import { path } => {
                let ssh_config_path = path.unwrap_or_else(|| {
                    dirs::home_dir()
                        .expect("home dir")
                        .join(".ssh")
                        .join("config")
                });
                import_ssh_config(&ssh_config_path)?;
            }
            HostsAction::Health { group } => {
                health_check(&config, group.as_deref()).await?;
            }
        },

        Commands::Keys { action } => match action {
            KeysAction::List => {
                let db = CacheDb::open_default()?;
                let keys = db.list_keys()?;
                if keys.is_empty() {
                    println!("No cached keys.");
                } else {
                    println!(
                        "{:<20} {:<40} {:<12} {:<24}",
                        "Name", "Path", "Type", "Added"
                    );
                    println!("{}", "-".repeat(96));
                    for k in &keys {
                        println!(
                            "{:<20} {:<40} {:<12} {:<24}",
                            k.name, k.path, k.key_type, k.added_at
                        );
                    }
                }
            }
            KeysAction::Add { path, name } => {
                let db = CacheDb::open_default()?;
                let key_name = name.unwrap_or_else(|| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
                let key = russh_keys::load_secret_key(&path, None)?;
                let key_type = format!("{:?}", key);
                let key_type_short = key_type.split('(').next().unwrap_or("unknown").to_string();
                db.add_key(
                    &key_name,
                    &path.to_string_lossy(),
                    &key_type_short,
                    &key_type_short,
                )?;
                println!("Key '{}' added.", key_name);
            }
            KeysAction::Remove { name } => {
                let db = CacheDb::open_default()?;
                if db.remove_key(&name)? {
                    println!("Key '{}' removed.", name);
                } else {
                    println!("Key '{}' not found.", name);
                }
            }
        },

        Commands::Session { action } => match action {
            SessionAction::List => {
                let session_dir = AppConfig::data_dir().join("sessions");
                if session_dir.exists() {
                    for entry in std::fs::read_dir(&session_dir)? {
                        let entry = entry?;
                        println!("{}", entry.file_name().to_string_lossy());
                    }
                } else {
                    println!("No sessions recorded yet.");
                }
            }
            SessionAction::Replay { id } => {
                let path = AppConfig::data_dir()
                    .join("sessions")
                    .join(format!("{}.jsonl", id));
                if path.exists() {
                    let content = std::fs::read_to_string(&path)?;
                    for line in content.lines() {
                        let snap: diagnostics::DiagnosticsSnapshot = serde_json::from_str(line)?;
                        println!(
                            "[{}] RTT={:?}ms  ↑{:.0}B/s  ↓{:.0}B/s  Loss={:.1}%  Quality={:?}",
                            snap.timestamp,
                            snap.rtt_ms,
                            snap.throughput_up_bps,
                            snap.throughput_down_bps,
                            snap.packet_loss_pct,
                            snap.quality
                        );
                    }
                } else {
                    println!("Session {} not found.", id);
                }
            }
        },

        Commands::Diag { session_id } => {
            let path = AppConfig::data_dir()
                .join("sessions")
                .join(format!("{}.jsonl", session_id));
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let lines: Vec<&str> = content.lines().collect();
                if let Some(last) = lines.last() {
                    let snap: diagnostics::DiagnosticsSnapshot = serde_json::from_str(last)?;
                    println!("Session: {}", snap.session_id);
                    println!("Last snapshot: {}", snap.timestamp);
                    println!("RTT: {:?} ms", snap.rtt_ms);
                    println!("Bytes sent: {}", snap.bytes_sent);
                    println!("Bytes received: {}", snap.bytes_received);
                    println!("Throughput ↑: {:.1} B/s", snap.throughput_up_bps);
                    println!("Throughput ↓: {:.1} B/s", snap.throughput_down_bps);
                    println!("Packet loss: {:.1}%", snap.packet_loss_pct);
                    println!("Quality: {:?}", snap.quality);
                    println!("Uptime: {}s", snap.uptime_secs);
                }
            } else {
                println!("No diagnostics found for session {}.", session_id);
            }
        }

        Commands::Run { group, command } => {
            run_on_group(&config, &group, &command).await?;
        }

        Commands::Config { action } => match action {
            ConfigAction::Edit => {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                let path = AppConfig::data_dir().join("config.toml");
                if !path.exists() {
                    config.save()?;
                }
                std::process::Command::new(editor)
                    .arg(&path)
                    .status()?;
            }
            ConfigAction::Show => {
                let toml_str = toml::to_string_pretty(&config)?;
                println!("{}", toml_str);
            }
            ConfigAction::Init => {
                let default = AppConfig::default();
                default.save()?;
                println!(
                    "Default config written to {}",
                    AppConfig::data_dir().join("config.toml").display()
                );
            }
        },

        Commands::Audit { action } => match action {
            AuditAction::Tail { lines } => {
                let audit = AuditLogger::default_logger();
                match audit.tail(lines) {
                    Ok(events) => {
                        for e in &events {
                            println!(
                                "[{}] {:?} host={} session={}",
                                e.timestamp,
                                e.event_type,
                                e.hostname.as_deref().unwrap_or("-"),
                                e.session_id.as_deref().unwrap_or("-"),
                            );
                        }
                    }
                    Err(e) => println!("Could not read audit log: {}", e),
                }
            }
        },
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SSH connect + interactive shell
// ---------------------------------------------------------------------------

async fn connect_and_shell(
    connect_config: ConnectConfig,
    app_config: &AppConfig,
    audit: &AuditLogger,
) -> anyhow::Result<()> {
    let session_id = uuid::Uuid::new_v4().to_string();

    audit.log_connection_attempt(
        &session_id,
        &connect_config.hostname,
        connect_config.port,
        &connect_config.username,
    );

    println!(
        "Connecting to {}@{}:{}...",
        connect_config.username, connect_config.hostname, connect_config.port
    );

    let (mut session, fingerprint, banner) = SshClient::connect(&connect_config).await?;

    // TOFU host key check
    let db = CacheDb::open_default()?;
    let status = db.check_host_key(&connect_config.hostname, connect_config.port, &fingerprint)?;

    match status {
        HostKeyStatus::Trusted => {
            audit.log_host_key_event(
                &session_id,
                &connect_config.hostname,
                connect_config.port,
                AuditEventType::HostKeyVerified,
                &fingerprint,
            );
        }
        HostKeyStatus::Unknown => match app_config.general.tofu_policy {
            TofuPolicy::Auto => {
                db.trust_host(
                    &connect_config.hostname,
                    None,
                    connect_config.port,
                    &fingerprint,
                    "ssh",
                )?;
                audit.log_host_key_event(
                    &session_id,
                    &connect_config.hostname,
                    connect_config.port,
                    AuditEventType::HostKeyNewTrust,
                    &fingerprint,
                );
                println!("Host key cached (auto-trust).");
            }
            TofuPolicy::Prompt => {
                println!("Unknown host key: {}", fingerprint);
                print!("Trust this host? [y/N] ");
                io::stdout().flush()?;
                let mut answer = String::new();
                io::stdin().read_line(&mut answer)?;
                if answer.trim().eq_ignore_ascii_case("y") {
                    db.trust_host(
                        &connect_config.hostname,
                        None,
                        connect_config.port,
                        &fingerprint,
                        "ssh",
                    )?;
                    audit.log_host_key_event(
                        &session_id,
                        &connect_config.hostname,
                        connect_config.port,
                        AuditEventType::HostKeyNewTrust,
                        &fingerprint,
                    );
                } else {
                    audit.log_host_key_event(
                        &session_id,
                        &connect_config.hostname,
                        connect_config.port,
                        AuditEventType::HostKeyRejected,
                        &fingerprint,
                    );
                    anyhow::bail!("Host key rejected by user.");
                }
            }
            TofuPolicy::Strict => {
                audit.log_host_key_event(
                    &session_id,
                    &connect_config.hostname,
                    connect_config.port,
                    AuditEventType::HostKeyRejected,
                    &fingerprint,
                );
                anyhow::bail!(
                    "Unknown host key for {}. Add it to the cache first (strict TOFU policy).",
                    connect_config.hostname
                );
            }
        },
        HostKeyStatus::Changed {
            old_fingerprint,
            old_last_seen,
        } => {
            audit.log_host_key_event(
                &session_id,
                &connect_config.hostname,
                connect_config.port,
                AuditEventType::HostKeyChanged,
                &fingerprint,
            );
            eprintln!("⚠️  WARNING: HOST KEY HAS CHANGED!");
            eprintln!("  Old fingerprint: {}", old_fingerprint);
            eprintln!("  New fingerprint: {}", fingerprint);
            eprintln!("  Last seen:       {}", old_last_seen);
            eprintln!("This could indicate a man-in-the-middle attack.");
            print!("Accept new key? [y/N] ");
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if answer.trim().eq_ignore_ascii_case("y") {
                db.trust_host(
                    &connect_config.hostname,
                    None,
                    connect_config.port,
                    &fingerprint,
                    "ssh",
                )?;
            } else {
                anyhow::bail!("Connection aborted — host key change rejected.");
            }
        }
    }

    db.update_last_seen(&connect_config.hostname, connect_config.port)?;

    audit.log_auth_result(
        &session_id,
        &connect_config.hostname,
        connect_config.port,
        &connect_config.username,
        &format!("{:?}", connect_config.auth),
        true,
    );

    if let Some(ref b) = banner {
        println!("Server banner: {}", b.trim());
    }

    // Set up diagnostics
    let log_dir = AppConfig::data_dir().join("sessions");
    let diag = DiagnosticsEngine::new(
        &session_id,
        &connect_config.hostname,
        connect_config.port,
        Some(log_dir.as_path()),
    );
    diag.set_connection_info(
        banner.clone(),
        None,
        None,
        None,
        None,
        Some(format!("{:?}", connect_config.auth)),
    )
    .await;

    // Get terminal size
    let (cols, rows) = terminal::size()?;

    let channel = session
        .open_shell("xterm-256color", cols as u32, rows as u32)
        .await?;

    audit.log_session_event(
        &session_id,
        &connect_config.hostname,
        connect_config.port,
        AuditEventType::SessionStart,
    );

    println!("Connected. Session ID: {}", session_id);

    // Run interactive shell
    run_interactive_shell(channel, diag).await?;

    audit.log_session_event(
        &session_id,
        &connect_config.hostname,
        connect_config.port,
        AuditEventType::SessionEnd,
    );

    session.close().await.ok();

    println!("Session ended.");
    Ok(())
}

async fn run_interactive_shell(
    mut channel: russh::Channel<russh::client::Msg>,
    diag: DiagnosticsEngine,
) -> anyhow::Result<()> {
    // Put terminal in raw mode (may fail if stdin is not a TTY)
    let is_tty = std::io::IsTerminal::is_terminal(&io::stdin());
    if is_tty {
        terminal::enable_raw_mode()?;
    }

    let _raw_guard = scopeguard::guard((), move |_| {
        if is_tty {
            terminal::disable_raw_mode().ok();
        }
    });

    // Diagnostics logging task
    let diag_metrics = diag.metrics();
    let log_dir = AppConfig::data_dir().join("sessions");
    let session_id_for_log = diag_metrics.read().await.session_id.clone();
    let log_engine = DiagnosticsEngine::new(
        &session_id_for_log,
        &diag_metrics.read().await.hostname,
        diag_metrics.read().await.port,
        Some(log_dir.as_path()),
    );

    // Spawn diagnostics writer
    let diag_metrics_clone = diag.metrics();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let _ = log_engine.write_log_entry().await;
            let _m = diag_metrics_clone.read().await;
        }
    });

    // Stdin reader task
    let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        loop {
            match handle.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if stdin_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    loop {
        tokio::select! {
            Some(msg) = channel.wait() => {
                match msg {
                    russh::ChannelMsg::Data { data } => {
                        let bytes = data.to_vec();
                        diag.record_bytes_received(bytes.len() as u64).await;
                        io::stdout().write_all(&bytes)?;
                        io::stdout().flush()?;
                    }
                    russh::ChannelMsg::ExtendedData { data, ext } => {
                        if ext == 1 {
                            let bytes = data.to_vec();
                            io::stderr().write_all(&bytes)?;
                            io::stderr().flush()?;
                        }
                    }
                    russh::ChannelMsg::Eof | russh::ChannelMsg::Close => {
                        break;
                    }
                    russh::ChannelMsg::ExitStatus { exit_status } => {
                        if exit_status != 0 {
                            eprintln!("\r\nRemote process exited with status {}", exit_status);
                        }
                        break;
                    }
                    _ => {}
                }
            }
            Some(data) = stdin_rx.recv() => {
                diag.record_bytes_sent(data.len() as u64).await;
                channel.data(&data[..]).await?;
            }
            else => break,
        }
    }

    // Final diagnostics flush
    diag.write_log_entry().await.ok();

    Ok(())
}

// ---------------------------------------------------------------------------
// TUI dashboard
// ---------------------------------------------------------------------------

async fn run_tui(config: AppConfig) -> anyhow::Result<()> {
    let mut app = App::new();

    // Load hosts from cache + config
    load_hosts_into_app(&mut app, &config)?;

    io::stdout().execute(EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = tui_loop(&mut terminal, &mut app, &config).await;

    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn tui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    config: &AppConfig,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            tui::render_dashboard(frame, app);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(())
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Enter => {
                        if let Some(host) = app.selected_host().cloned() {
                            // Exit TUI, connect in CLI mode
                            terminal::disable_raw_mode()?;
                            io::stdout().execute(LeaveAlternateScreen)?;

                            let user = if host.user.is_empty() {
                                config
                                    .general
                                    .default_user
                                    .clone()
                                    .unwrap_or_else(whoami)
                            } else {
                                host.user.clone()
                            };

                            let auth = if let Some(ref key) = config.general.default_key {
                                let expanded = shellexpand::tilde(key).to_string();
                                AuthMethod::KeyFile(expanded.into())
                            } else {
                                let pw = prompt_password(&format!(
                                    "{}@{}'s password: ",
                                    user, host.hostname
                                ))?;
                                AuthMethod::Password(pw)
                            };

                            let connect_config = ConnectConfig {
                                hostname: host.hostname.clone(),
                                port: host.port,
                                username: user,
                                auth,
                            };

                            let audit = AuditLogger::default_logger();
                            connect_and_shell(connect_config, config, &audit).await?;

                            // Re-enter TUI
                            io::stdout().execute(EnterAlternateScreen)?;
                            terminal::enable_raw_mode()?;
                            load_hosts_into_app(app, config)?;
                        }
                    }
                    KeyCode::Char('r') => {
                        load_hosts_into_app(app, config)?;
                        app.set_status("Hosts refreshed.".to_string());
                    }
                    KeyCode::Char('d') => {
                        if let Some(host) = app.selected_host().cloned() {
                            let db = CacheDb::open_default()?;
                            db.remove_host(&host.hostname, host.port)?;
                            load_hosts_into_app(app, config)?;
                            app.set_status(format!("Removed {}.", host.hostname));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_target(target: &str, config: &AppConfig) -> (String, String) {
    if let Some((user, host)) = target.split_once('@') {
        (user.to_string(), host.to_string())
    } else {
        // Check if target matches a named host in config
        if let Some(entry) = config.hosts.iter().find(|h| h.name == target) {
            let user = entry
                .user
                .clone()
                .or_else(|| config.general.default_user.clone())
                .unwrap_or_else(whoami);
            return (user, entry.hostname.clone());
        }
        let user = config
            .general
            .default_user
            .clone()
            .unwrap_or_else(whoami);
        (user, target.to_string())
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

fn prompt_password(prompt: &str) -> anyhow::Result<String> {
    eprint!("{}", prompt);
    io::stderr().flush()?;
    terminal::enable_raw_mode()?;
    let mut pw = String::new();
    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    terminal::disable_raw_mode()?;
                    eprintln!();
                    return Ok(pw);
                }
                KeyCode::Backspace => {
                    pw.pop();
                }
                KeyCode::Char(c) => pw.push(c),
                _ => {}
            }
        }
    }
}

fn load_hosts_into_app(app: &mut App, config: &AppConfig) -> anyhow::Result<()> {
    let mut displays = Vec::new();

    // Load from cache DB
    if let Ok(db) = CacheDb::open_default() {
        if let Ok(hosts) = db.list_hosts() {
            for h in hosts {
                let tags: Vec<String> = h.tags.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                displays.push(HostDisplay {
                    name: h.hostname.clone(),
                    hostname: h.hostname,
                    port: h.port,
                    user: String::new(),
                    status: HostStatus::Unknown,
                    last_seen: h.last_seen,
                    tags: tags.join(", "),
                });
            }
        }
    }

    // Merge config-defined hosts
    for entry in &config.hosts {
        if !displays.iter().any(|d| d.hostname == entry.hostname && d.port == entry.port) {
            let tags: Vec<String> = entry
                .tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            displays.push(HostDisplay {
                name: entry.name.clone(),
                hostname: entry.hostname.clone(),
                port: entry.port,
                user: entry.user.clone().unwrap_or_default(),
                status: HostStatus::Unknown,
                last_seen: String::new(),
                tags: tags.join(", "),
            });
        }
    }

    app.set_hosts(displays);
    Ok(())
}

fn import_ssh_config(path: &std::path::Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(path)?;
    let db = CacheDb::open_default()?;
    let mut count = 0;

    let mut current_host: Option<String> = None;
    let mut current_hostname: Option<String> = None;
    let mut current_port: u16 = 22;
    let mut current_user: Option<String> = None;

    let flush = |host: &Option<String>,
                 hostname: &Option<String>,
                 port: u16,
                 _user: &Option<String>,
                 db: &CacheDb,
                 count: &mut u32| {
        if let Some(ref hn) = hostname {
            db.trust_host(hn, None, port, "imported", "unknown").ok();
            *count += 1;
            println!(
                "  Imported: {} -> {}:{}",
                host.as_deref().unwrap_or(hn),
                hn,
                port
            );
        }
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0].to_lowercase();
        let value = parts[1].trim();

        match key.as_str() {
            "host" => {
                flush(
                    &current_host,
                    &current_hostname,
                    current_port,
                    &current_user,
                    &db,
                    &mut count,
                );
                current_host = Some(value.to_string());
                current_hostname = None;
                current_port = 22;
                current_user = None;
            }
            "hostname" => current_hostname = Some(value.to_string()),
            "port" => current_port = value.parse().unwrap_or(22),
            "user" => current_user = Some(value.to_string()),
            _ => {}
        }
    }
    flush(
        &current_host,
        &current_hostname,
        current_port,
        &current_user,
        &db,
        &mut count,
    );

    println!("Imported {} hosts from {}", count, path.display());
    Ok(())
}

async fn health_check(config: &AppConfig, group: Option<&str>) -> anyhow::Result<()> {
    let db = CacheDb::open_default()?;
    let hosts = if let Some(group_name) = group {
        if let Some(grp) = config.host_groups.iter().find(|g| g.name == group_name) {
            let mut matched = Vec::new();
            for (tag_key, tag_val) in &grp.match_tags {
                matched.extend(db.find_hosts_by_tag(tag_key, tag_val)?);
            }
            matched
        } else {
            anyhow::bail!("Host group '{}' not found in config.", group_name);
        }
    } else {
        db.list_hosts()?
    };

    if hosts.is_empty() {
        println!("No hosts to check.");
        return Ok(());
    }

    println!("Checking {} hosts...", hosts.len());

    for host in &hosts {
        let addr = format!("{}:{}", host.hostname, host.port);
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(&addr),
        )
        .await;
        match result {
            Ok(Ok(_)) => println!("  ✓ {}:{} — reachable", host.hostname, host.port),
            Ok(Err(e)) => println!("  ✗ {}:{} — {}", host.hostname, host.port, e),
            Err(_) => println!("  ✗ {}:{} — timeout", host.hostname, host.port),
        }
    }
    Ok(())
}

async fn run_on_group(
    config: &AppConfig,
    group_name: &str,
    command: &[String],
) -> anyhow::Result<()> {
    let db = CacheDb::open_default()?;
    let grp = config
        .host_groups
        .iter()
        .find(|g| g.name == group_name)
        .ok_or_else(|| anyhow::anyhow!("Host group '{}' not found.", group_name))?;

    let mut hosts = Vec::new();
    for (tag_key, tag_val) in &grp.match_tags {
        hosts.extend(db.find_hosts_by_tag(tag_key, tag_val)?);
    }

    if hosts.is_empty() {
        println!("No hosts matched group '{}'.", group_name);
        return Ok(());
    }

    let cmd_str = command.join(" ");
    println!(
        "Running '{}' on {} hosts in group '{}'...",
        cmd_str,
        hosts.len(),
        group_name
    );

    let user = grp
        .defaults
        .user
        .clone()
        .or_else(|| config.general.default_user.clone())
        .unwrap_or_else(whoami);

    let auth = if let Some(ref key) = grp.defaults.key.as_ref().or(config.general.default_key.as_ref()) {
        let expanded = shellexpand::tilde(key).to_string();
        AuthMethod::KeyFile(expanded.into())
    } else {
        anyhow::bail!("No key configured for group. Set defaults.key or general.default_key.");
    };

    let mut handles = Vec::new();
    for host in hosts {
        let user = user.clone();
        let auth = auth.clone();
        let cmd_str = cmd_str.clone();
        handles.push(tokio::spawn(async move {
            let connect_config = ConnectConfig {
                hostname: host.hostname.clone(),
                port: host.port,
                username: user,
                auth,
            };
            let result: Result<(SshSession, String, Option<String>), _> =
                SshClient::connect(&connect_config).await;
            match result {
                Ok((session, _, _)) => {
                    let channel_result = session.handle.channel_open_session().await;
                    match channel_result {
                        Ok(mut channel) => {
                            channel.exec(true, cmd_str.as_bytes()).await.ok();
                            let mut output = Vec::new();
                            while let Some(msg) = channel.wait().await {
                                match msg {
                                    russh::ChannelMsg::Data { data } => {
                                        output.extend_from_slice(&data);
                                    }
                                    russh::ChannelMsg::Eof | russh::ChannelMsg::Close => break,
                                    _ => {}
                                }
                            }
                            println!("--- {} ---", host.hostname);
                            io::stdout().write_all(&output).ok();
                            println!();
                            session.close().await.ok();
                        }
                        Err(e) => {
                            eprintln!("--- {} --- ERROR: {}", host.hostname, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("--- {} --- CONNECT ERROR: {}", host.hostname, e);
                }
            }
        }));
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}
