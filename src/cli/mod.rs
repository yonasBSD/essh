use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "essh", version, about = "Enterprise SSH Client")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Connect to a host
    Connect {
        /// Host name or user@hostname
        target: String,

        /// Port number
        #[arg(short, long, default_value_t = 22)]
        port: u16,

        /// Identity file (private key)
        #[arg(short = 'i', long)]
        identity: Option<PathBuf>,

        /// Use password authentication
        #[arg(long)]
        password: bool,
    },

    /// Manage cached hosts
    Hosts {
        #[command(subcommand)]
        action: HostsAction,
    },

    /// Manage SSH keys
    Keys {
        #[command(subcommand)]
        action: KeysAction,
    },

    /// Manage session profiles
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Show diagnostics for a past session
    Diag {
        /// Session ID
        session_id: String,
    },

    /// Execute a command across a host group
    Run {
        /// Group name
        group: String,

        /// Command to execute (everything after --)
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Open configuration in $EDITOR
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Stream or view audit log
    Audit {
        #[command(subcommand)]
        action: AuditAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum HostsAction {
    /// List all cached hosts
    List {
        /// Filter by tag (key=value)
        #[arg(long)]
        tag: Option<String>,
    },

    /// Add a host to the cache
    Add {
        /// Hostname or IP
        hostname: String,

        /// Port
        #[arg(short, long, default_value_t = 22)]
        port: u16,

        /// Display name
        #[arg(short, long)]
        name: Option<String>,

        /// User
        #[arg(short, long)]
        user: Option<String>,

        /// Tags (key=value, can repeat)
        #[arg(long)]
        tag: Vec<String>,
    },

    /// Remove a host from the cache
    Remove {
        /// Hostname
        hostname: String,

        /// Port
        #[arg(short, long, default_value_t = 22)]
        port: u16,
    },

    /// Import hosts from SSH config file
    Import {
        /// Path to SSH config (default: ~/.ssh/config)
        path: Option<PathBuf>,
    },

    /// Run connectivity health checks
    Health {
        /// Group name
        #[arg(long)]
        group: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum KeysAction {
    /// List cached keys
    List,

    /// Add a key to the cache
    Add {
        /// Path to the private key file
        path: PathBuf,

        /// Friendly name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Remove a key from the cache
    Remove {
        /// Key name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SessionAction {
    /// List saved session profiles
    List,

    /// Replay a recorded session
    Replay {
        /// Session ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Open config in $EDITOR
    Edit,

    /// Show current config
    Show,

    /// Initialize default config
    Init,
}

#[derive(Subcommand, Debug)]
pub enum AuditAction {
    /// Show recent audit log entries
    Tail {
        /// Number of entries to show
        #[arg(short, long, default_value_t = 20)]
        lines: usize,
    },
}
