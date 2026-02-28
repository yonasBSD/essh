# Enterprise SSH Client — Product Specification

## 1. Overview

A terminal-based SSH client built for enterprise environments, providing host/key caching, real-time connection diagnostics, session management, and centralized configuration. Designed for operations teams managing large fleets of servers.

---

## 2. Goals

- **Zero-friction connections**: Auto-discover and cache hosts and keys so engineers connect once and never re-configure.
- **Real-time diagnostics**: Surface latency, throughput, packet loss, cipher negotiation, and auth method details during and after sessions.
- **Enterprise-grade security**: Support hardware tokens, certificate authorities, key rotation policies, and audit logging.
- **Fleet management**: Manage hundreds of hosts with tagging, grouping, and bulk operations.
- **Extensibility**: Plugin architecture for custom auth providers, host discovery backends, and diagnostic exporters.

---

## 3. Architecture

```
┌──────────────────────────────────────────────────────┐
│                      CLI / TUI                       │
│  (Dashboard · Session View · Host Browser · Config)  │
├──────────────────────────────────────────────────────┤
│                    Core Engine                        │
│  ┌─────────┐ ┌───────────┐ ┌───────────────────┐    │
│  │ Session  │ │ Diagnostics│ │  Host/Key Cache   │    │
│  │ Manager  │ │  Engine    │ │  (SQLite)         │    │
│  └─────────┘ └───────────┘ └───────────────────┘    │
│  ┌─────────┐ ┌───────────┐ ┌───────────────────┐    │
│  │ Auth     │ │ Plugin    │ │  Audit Logger     │    │
│  │ Provider │ │ System    │ │                   │    │
│  └─────────┘ └───────────┘ └───────────────────┘    │
├──────────────────────────────────────────────────────┤
│              Transport (libssh2 / native SSH)        │
└──────────────────────────────────────────────────────┘
```

---

## 4. Core Features

### 4.1 Host & Key Cache

| Capability | Details |
|---|---|
| **Host discovery** | Manual add, SSH config import (`~/.ssh/config`), LDAP/AD lookup, cloud provider APIs (AWS EC2, GCP, Azure), DNS SRV records |
| **Host fingerprint cache** | SQLite-backed store at `~/.essh/cache.db`; stores hostname, IP, port, host key fingerprint (SHA-256), first-seen and last-seen timestamps |
| **Key management** | Cache and index user private keys; map keys → hosts/groups; support ED25519, RSA (≥3072-bit), ECDSA |
| **TOFU policy** | Configurable Trust-On-First-Use: `strict` (reject unknown), `prompt` (ask user), `auto` (accept and cache) |
| **Key rotation detection** | Alert on host key change with diff of old vs. new fingerprint, timestamp of last known good key, and option to accept/reject/investigate |
| **Certificate authority** | Support OpenSSH CA-signed host and user certificates; pin trusted CA public keys in config |
| **Cache expiry** | Configurable TTL per host/group; stale entries flagged in host browser |

### 4.2 Real-Time Diagnostics Dashboard

Displayed as a persistent status bar or toggleable overlay (TUI split-pane).

| Metric | Source | Update Frequency |
|---|---|---|
| **RTT / Latency** | SSH keepalive round-trip timing | 1 s |
| **Throughput** | Bytes sent/received per second | 1 s |
| **Packet loss** | Keepalive miss ratio | 5 s |
| **Cipher suite** | Negotiated algorithms (kex, cipher, MAC, compression) | On connect |
| **Auth method** | publickey / password / keyboard-interactive / certificate | On connect |
| **Session uptime** | Wall-clock duration | 1 s |
| **Channel count** | Active channels (shell, forwarded ports, SCP/SFTP) | On change |
| **Rekey status** | Data transferred since last rekey; threshold warning | 10 s |
| **Server banner** | Remote SSH server version string | On connect |
| **Connection quality** | Composite score (latency + loss + throughput) as color-coded indicator | 5 s |

**Diagnostic log**: All metrics are written to `~/.essh/sessions/<session-id>.jsonl` for post-session analysis.

### 4.3 Session Management

- **Named sessions**: Bookmark connection profiles (host + user + key + port + jump hosts + env vars).
- **Session resume**: Detect broken connections and auto-reconnect with exponential backoff.
- **Multiplexing**: Reuse a single TCP connection for multiple channels (ControlMaster-style).
- **Session recording**: Optional terminal session capture (asciicast format) for compliance and replay.
- **Concurrent sessions**: Tabbed or split-pane multi-session view in TUI mode.

### 4.4 Authentication

| Method | Details |
|---|---|
| **Public key** | ED25519, RSA, ECDSA; agent forwarding; `ssh-agent` integration |
| **Certificate** | OpenSSH user certificates with CA pinning |
| **Password** | Prompted; never stored on disk |
| **MFA / 2FA** | Keyboard-interactive for TOTP/FIDO2 challenge-response |
| **Hardware tokens** | PKCS#11 / FIDO2 (e.g., YubiKey) via `ssh-agent` or direct |
| **SSO / OIDC** | Plugin-based: exchange OIDC token for short-lived SSH certificate |

### 4.5 Audit & Compliance

- **Structured audit log**: JSON log at `~/.essh/audit.log` — every connection attempt, auth result, host key event, and session lifecycle event.
- **Syslog / SIEM export**: Forward audit events via syslog (RFC 5424) or webhook.
- **Session recording**: Opt-in full terminal I/O capture for regulated environments.
- **Policy engine**: Enforce org-wide rules via a config file (`/etc/essh/policy.toml`):
  - Minimum key size
  - Allowed ciphers / KEX algorithms
  - Required MFA for specific host groups
  - Max session duration
  - Disallowed commands (logged, not blocked — or blocked in strict mode)

### 4.6 Fleet / Host Management

- **Tagging**: Arbitrary key-value tags on hosts (e.g., `env:prod`, `team:platform`).
- **Groups**: Logical groups with inherited connection defaults.
- **Search & filter**: Full-text and tag-based search in the host browser.
- **Bulk operations**: Run a command across a group of hosts (fan-out with parallel execution, streamed output).
- **Health checks**: Periodic background connectivity probes; mark hosts as reachable/unreachable.

---

## 5. Configuration

### 5.1 File Layout

```
~/.essh/
├── config.toml            # User configuration
├── cache.db               # SQLite host/key cache
├── known_cas/             # Trusted CA public keys
├── audit.log              # Local audit log
├── sessions/              # Per-session diagnostic logs
│   └── <session-id>.jsonl
├── recordings/            # Terminal session recordings
│   └── <session-id>.cast
└── plugins/               # Installed plugins
```

### 5.2 Example `config.toml`

```toml
[general]
default_user = "matt"
default_key = "~/.ssh/id_ed25519"
tofu_policy = "prompt"          # strict | prompt | auto
cache_ttl = "30d"
log_level = "info"

[diagnostics]
enabled = true
display = "status_bar"         # status_bar | overlay | hidden
export_format = "jsonl"
keepalive_interval = 15

[session]
auto_reconnect = true
reconnect_max_retries = 5
multiplex = true
recording = false

[security]
min_key_bits = 3072
allowed_ciphers = ["chacha20-poly1305@openssh.com", "aes256-gcm@openssh.com"]
allowed_kex = ["curve25519-sha256", "curve25519-sha256@libssh.org"]
allowed_macs = ["hmac-sha2-256-etm@openssh.com", "hmac-sha2-512-etm@openssh.com"]
require_mfa_groups = ["prod-*"]

[audit]
enabled = true
syslog_target = "udp://siem.corp.example.com:514"

[[hosts]]
name = "bastion-us-east"
hostname = "bastion.us-east-1.corp.example.com"
port = 22
user = "ops"
key = "~/.ssh/id_ed25519_ops"
tags = { env = "prod", region = "us-east-1", role = "bastion" }
jump_host = ""

[[hosts]]
name = "db-primary"
hostname = "db01.internal.corp.example.com"
port = 22
user = "dba"
tags = { env = "prod", role = "database" }
jump_host = "bastion-us-east"

[[host_groups]]
name = "prod-databases"
match_tags = { env = "prod", role = "database" }
defaults = { user = "dba", key = "~/.ssh/id_ed25519_dba" }
```

---

## 6. CLI Interface

```
essh                                # Launch TUI dashboard
essh connect <host>                 # Connect to a cached host by name
essh connect <user>@<hostname>      # Ad-hoc connection (auto-cache)
essh hosts list [--tag key=val]     # List cached hosts
essh hosts add <hostname> [opts]    # Add host to cache
essh hosts import <ssh_config>      # Import from SSH config file
essh hosts discover --provider aws  # Auto-discover from cloud API
essh hosts health [--group <name>]  # Run connectivity health checks
essh keys list                      # List cached keys
essh keys add <path>                # Add key to cache
essh keys rotate <host>             # Trigger host key re-verification
essh session list                   # List saved session profiles
essh session replay <id>            # Replay a recorded session
essh diag <session-id>              # Show diagnostics for a past session
essh run <group> -- <command>       # Execute command across host group
essh config edit                    # Open config in $EDITOR
essh audit tail                     # Stream audit log
essh plugin install <name>          # Install a plugin
```

---

## 7. Technology Stack

| Component | Choice | Rationale |
|---|---|---|
| Language | **Rust** | Memory safety, performance, single binary distribution |
| SSH library | `russh` (pure Rust) | No C dependency; full protocol control for diagnostics |
| TUI framework | `ratatui` | Mature, flexible terminal UI |
| Database | `SQLite` via `rusqlite` | Embedded, zero-config, battle-tested |
| Serialization | `serde` + TOML/JSON | Ecosystem standard |
| Async runtime | `tokio` | Industry standard for async Rust |
| Plugin system | `libloading` (dynamic) or WASM (`wasmtime`) | Sandboxed extensibility |

---

## 8. Security Considerations

- Private keys are **never** written to the cache database; only public key fingerprints and metadata are stored.
- All cached host fingerprints are integrity-checked with HMAC using a local device key.
- Audit logs are append-only; tampering is detectable via chained hashes.
- Plugin sandboxing via WASM prevents untrusted plugins from accessing the filesystem or network beyond defined capabilities.
- Memory holding passwords or key material is zeroed after use (`zeroize` crate).

---

## 9. Milestones

| Phase | Scope | Target |
|---|---|---|
| **M1 — Core** | SSH connect, host/key cache (SQLite), TOFU, basic diagnostics status bar | 6 weeks |
| **M2 — Diagnostics** | Full real-time dashboard, session diagnostic logs, connection quality score | 4 weeks |
| **M3 — Fleet** | Tagging, groups, bulk `run`, health checks, SSH config import | 4 weeks |
| **M4 — Enterprise Auth** | Certificate auth, PKCS#11/FIDO2, SSO/OIDC plugin, MFA enforcement | 4 weeks |
| **M5 — Audit & Compliance** | Structured audit log, syslog export, session recording, policy engine | 3 weeks |
| **M6 — Cloud Discovery** | AWS/GCP/Azure host discovery plugins, DNS SRV | 3 weeks |
| **M7 — Polish** | Multiplexing, auto-reconnect, concurrent sessions, packaging (Homebrew, deb, rpm) | 3 weeks |

---

## 10. Open Questions

1. Should the cache database be encrypted at rest (adds complexity vs. threat model)?
2. Should bulk `run` output be streamed interleaved or host-by-host?
3. WASM vs. dynamic library plugins — sandboxing vs. ecosystem reach tradeoff?
4. Should we support Windows (PuTTY-style) or Linux/macOS only?
