#!/usr/bin/env python3
"""Generate a demo GIF showcasing key ESSH features with simulated TUI frames."""

from PIL import Image, ImageDraw, ImageFont
import os, struct

# ── Dimensions & Colors ──────────────────────────────────────────────────────
W, H = 960, 560
CHAR_W, CHAR_H = 9, 18
PAD_X, PAD_Y = 12, 8

# Terminal colors (Netwatch-inspired palette)
BG       = (18, 18, 24)
FG       = (204, 204, 204)
CYAN     = (0, 220, 255)
YELLOW   = (255, 220, 0)
GREEN    = (0, 220, 100)
RED      = (220, 60, 60)
MAGENTA  = (200, 100, 255)
DGRAY    = (68, 68, 80)
LGRAY    = (140, 140, 155)
WHITE    = (240, 240, 240)
BLUE     = (80, 140, 255)
DIM      = (90, 90, 110)
ORANGE   = (255, 165, 0)
BAR_BG   = (35, 35, 48)
SEL_BG   = (35, 50, 70)

def get_font(size=14):
    """Try to get a monospace font, fall back to default."""
    mono_paths = [
        "/System/Library/Fonts/SFMono-Regular.otf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.dfont",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]
    for p in mono_paths:
        if os.path.exists(p):
            try:
                return ImageFont.truetype(p, size)
            except Exception:
                continue
    return ImageFont.load_default()

FONT = get_font(14)
FONT_SM = get_font(12)
FONT_BOLD = get_font(15)

def new_frame():
    img = Image.new("RGB", (W, H), BG)
    return img, ImageDraw.Draw(img)

def txt(draw, x, y, text, color=FG, font=None):
    draw.text((x, y), text, fill=color, font=font or FONT)

def row_y(row):
    return PAD_Y + row * CHAR_H

def fill_row(draw, row, color=BAR_BG, x0=0, x1=W):
    y = row_y(row)
    draw.rectangle([x0, y, x1, y + CHAR_H], fill=color)

def hline(draw, row, color=DGRAY):
    y = row_y(row) + CHAR_H // 2
    draw.line([(PAD_X, y), (W - PAD_X, y)], fill=color, width=1)

def bar_gauge(draw, x, y, width, pct, fg_color=CYAN, bg_color=BAR_BG):
    draw.rectangle([x, y, x + width, y + 12], fill=bg_color)
    fill_w = int(width * pct / 100)
    if fill_w > 0:
        draw.rectangle([x, y, x + fill_w, y + 12], fill=fg_color)

def sparkline(draw, x, y, values, color=CYAN, w=3, h=14):
    blocks = "▁▂▃▄▅▆▇█"
    for i, v in enumerate(values):
        idx = min(int(v / 100 * 8), 7)
        txt(draw, x + i * (w + 3), y, blocks[idx], color, FONT_SM)


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 1: Dashboard — Hosts Tab
# ═══════════════════════════════════════════════════════════════════════════════
def frame_dashboard():
    img, d = new_frame()
    # Title bar
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 220, row_y(0), "│", DGRAY)
    txt(d, 240, row_y(0), " [1] Sessions ", LGRAY)
    txt(d, 380, row_y(0), " [2] Hosts ", YELLOW)
    txt(d, 490, row_y(0), " [3] Fleet ", LGRAY)
    txt(d, 590, row_y(0), " [4] Config ", LGRAY)
    txt(d, W - 120, row_y(0), "?:Help", DIM)
    txt(d, W - 60, row_y(0), "14:32", LGRAY)

    hline(d, 1)

    # Hosts header
    txt(d, PAD_X, row_y(2), " Hosts (6)", CYAN, FONT_BOLD)
    txt(d, PAD_X + 4, row_y(3.5), "  Name              Hostname                 Port   User       Status", DGRAY)
    hline(d, 4.5)

    hosts = [
        ("» bastion-east", "bastion.us-east.corp.io", "22", "ops", "● Online", GREEN, True),
        ("  web-prod-1",   "10.0.1.10",              "22", "deploy", "● Online", GREEN, False),
        ("  web-prod-2",   "10.0.1.11",              "22", "deploy", "● Online", GREEN, False),
        ("  db-primary",   "db01.internal.corp",     "22", "dba",    "● Online", GREEN, False),
        ("  staging-1",    "10.0.3.5",               "2222","matt",  "● Online", YELLOW, False),
        ("  dev-box",      "192.168.1.42",           "22", "matt",   "○ Offline", RED, False),
    ]
    for i, (name, host, port, user, status, scolor, selected) in enumerate(hosts):
        r = 5 + i
        if selected:
            fill_row(d, r, SEL_BG)
        ncolor = YELLOW if selected else FG
        txt(d, PAD_X + 4, row_y(r), f"{name:<20s}{host:<25s}{port:<7s}{user:<11s}", ncolor)
        txt(d, PAD_X + 4 + CHAR_W * 63, row_y(r), status, scolor)

    # Tags shown for selected
    txt(d, PAD_X + 30, row_y(12), "Tags:", DGRAY)
    txt(d, PAD_X + 80, row_y(12), "env:prod", MAGENTA)
    txt(d, PAD_X + 165, row_y(12), "region:us-east-1", MAGENTA)
    txt(d, PAD_X + 330, row_y(12), "role:bastion", MAGENTA)

    hline(d, 13.5)

    # Fleet summary
    txt(d, PAD_X, row_y(14.5), " Fleet Health", CYAN, FONT_BOLD)
    txt(d, PAD_X + 20, row_y(16), "Online:", GREEN)
    txt(d, PAD_X + 90, row_y(16), "5", WHITE)
    txt(d, PAD_X + 130, row_y(16), "│", DGRAY)
    txt(d, PAD_X + 150, row_y(16), "Offline:", RED)
    txt(d, PAD_X + 225, row_y(16), "1", WHITE)
    txt(d, PAD_X + 260, row_y(16), "│", DGRAY)
    txt(d, PAD_X + 280, row_y(16), "Availability:", LGRAY)
    bar_gauge(d, PAD_X + 400, row_y(16) + 2, 160, 83, GREEN)
    txt(d, PAD_X + 570, row_y(16), "83%", GREEN)

    hline(d, 18)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " Enter", YELLOW)
    txt(d, PAD_X + 55, row_y(28), ":Connect", LGRAY)
    txt(d, PAD_X + 135, row_y(28), "a", YELLOW)
    txt(d, PAD_X + 147, row_y(28), ":Add", LGRAY)
    txt(d, PAD_X + 190, row_y(28), "/", YELLOW)
    txt(d, PAD_X + 200, row_y(28), ":Search", LGRAY)
    txt(d, PAD_X + 275, row_y(28), "r", YELLOW)
    txt(d, PAD_X + 285, row_y(28), ":Refresh", LGRAY)
    txt(d, PAD_X + 365, row_y(28), "Ctrl+p", YELLOW)
    txt(d, PAD_X + 420, row_y(28), ":Palette", LGRAY)
    txt(d, PAD_X + 505, row_y(28), "q", YELLOW)
    txt(d, PAD_X + 515, row_y(28), ":Quit", LGRAY)

    # Outer border
    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)

    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 2: Session View with Diagnostics
# ═══════════════════════════════════════════════════════════════════════════════
def frame_session():
    img, d = new_frame()
    # Tab bar
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 100, row_y(0), "──", DGRAY)
    txt(d, 130, row_y(0), " [1] bastion-east ", YELLOW)
    txt(d, 310, row_y(0), " [2] db-primary ", LGRAY)
    txt(d, 465, row_y(0), " [3] web-prod-1 ", LGRAY)
    txt(d, W - 60, row_y(0), "14:33", LGRAY)

    hline(d, 1)

    # Terminal content
    lines = [
        ("ops@bastion:~$ ", CYAN, "uptime", GREEN),
        (" 14:33:42 up 14 days,  6:32,  3 users,  load average: 0.82, 0.64, 0.55", FG, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "df -h /", GREEN),
        ("Filesystem      Size  Used Avail Use% Mounted on", DGRAY, "", FG),
        ("/dev/sda1        20G   12G  7.6G  62% /", FG, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "free -h", GREEN),
        ("              total        used        free      shared  buff/cache   available", DGRAY, "", FG),
        ("Mem:          8.0Gi       3.2Gi       1.8Gi       256Mi       3.0Gi       4.5Gi", FG, "", FG),
        ("Swap:         2.0Gi          0B       2.0Gi", FG, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "docker ps --format 'table {{.Names}}\\t{{.Status}}'", GREEN),
        ("NAMES               STATUS", DGRAY, "", FG),
        ("nginx-proxy         Up 14 days", FG, "", FG),
        ("app-server          Up 14 days", FG, "", FG),
        ("redis-cache         Up 12 days", FG, "", FG),
        ("prometheus          Up 14 days", FG, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "█", WHITE),
    ]
    for i, (prompt, pc, cmd, cc) in enumerate(lines):
        r = 2 + i
        if r > 23:
            break
        if prompt:
            txt(d, PAD_X + 4, row_y(r), prompt, pc)
            txt(d, PAD_X + 4 + len(prompt) * 8, row_y(r), cmd, cc)
        else:
            txt(d, PAD_X + 4, row_y(r), cmd, cc)

    hline(d, 23)

    # Diagnostics status bar
    fill_row(d, 24, (25, 30, 40))
    txt(d, PAD_X + 4, row_y(24), "RTT:", DGRAY)
    txt(d, PAD_X + 40, row_y(24), "12.3ms", GREEN)
    txt(d, PAD_X + 110, row_y(24), "↑", CYAN)
    txt(d, PAD_X + 125, row_y(24), "1.2KB/s", LGRAY)
    txt(d, PAD_X + 210, row_y(24), "↓", CYAN)
    txt(d, PAD_X + 225, row_y(24), "48.5KB/s", LGRAY)
    txt(d, PAD_X + 320, row_y(24), "Loss:", DGRAY)
    txt(d, PAD_X + 365, row_y(24), "0.0%", GREEN)
    txt(d, PAD_X + 430, row_y(24), "●", GREEN)
    txt(d, PAD_X + 445, row_y(24), "Excellent", GREEN)
    txt(d, PAD_X + 550, row_y(24), "Up:", DGRAY)
    txt(d, PAD_X + 580, row_y(24), "2h 14m", LGRAY)
    txt(d, PAD_X + 660, row_y(24), "Fwd:", DGRAY)
    txt(d, PAD_X + 700, row_y(24), "L:8080→80", MAGENTA)

    hline(d, 25.5)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " Alt+←→", YELLOW)
    txt(d, PAD_X + 70, row_y(28), ":Switch", LGRAY)
    txt(d, PAD_X + 140, row_y(28), "Alt+s", YELLOW)
    txt(d, PAD_X + 195, row_y(28), ":Split", LGRAY)
    txt(d, PAD_X + 250, row_y(28), "Alt+m", YELLOW)
    txt(d, PAD_X + 305, row_y(28), ":Monitor", LGRAY)
    txt(d, PAD_X + 380, row_y(28), "Alt+f", YELLOW)
    txt(d, PAD_X + 435, row_y(28), ":Files", LGRAY)
    txt(d, PAD_X + 490, row_y(28), "Alt+p", YELLOW)
    txt(d, PAD_X + 545, row_y(28), ":Fwd", LGRAY)
    txt(d, PAD_X + 590, row_y(28), "Ctrl+p", YELLOW)
    txt(d, PAD_X + 650, row_y(28), ":Palette", LGRAY)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 3: Host Monitor (Remote htop)
# ═══════════════════════════════════════════════════════════════════════════════
def frame_monitor():
    img, d = new_frame()
    # Title bar
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 100, row_y(0), "──", DGRAY)
    txt(d, 130, row_y(0), " [1] bastion-east ", YELLOW)
    txt(d, 310, row_y(0), " [2] db-primary ", LGRAY)
    txt(d, W - 130, row_y(0), "Host Monitor", CYAN)
    txt(d, W - 60, row_y(0), "14:34", LGRAY)

    hline(d, 1)

    # CPU
    txt(d, PAD_X + 4, row_y(2), "CPU", CYAN, FONT_BOLD)
    txt(d, PAD_X + 50, row_y(2), "23.4%", WHITE)
    cpu_vals = [15,22,30,28,20,45,30,18,12,20,30,38,50,58,48,28,18,14,20,30,28,15,10,12,22,35,55,48,30,22,28,30,18,14,22,30,28,15]
    sparkline(d, PAD_X + 120, row_y(2), cpu_vals, CYAN)
    bar_gauge(d, PAD_X + 50, row_y(3.3) + 2, 400, 23, CYAN)
    txt(d, PAD_X + 460, row_y(3.3), "23%", CYAN)

    txt(d, PAD_X + 50, row_y(4.6), "Core 0:", DGRAY)
    bar_gauge(d, PAD_X + 120, row_y(4.6) + 2, 100, 28, GREEN)
    txt(d, PAD_X + 225, row_y(4.6), "28%", GREEN)
    txt(d, PAD_X + 290, row_y(4.6), "Core 1:", DGRAY)
    bar_gauge(d, PAD_X + 360, row_y(4.6) + 2, 100, 19, GREEN)
    txt(d, PAD_X + 465, row_y(4.6), "19%", GREEN)

    txt(d, PAD_X + 50, row_y(5.6), "Core 2:", DGRAY)
    bar_gauge(d, PAD_X + 120, row_y(5.6) + 2, 100, 31, GREEN)
    txt(d, PAD_X + 225, row_y(5.6), "31%", GREEN)
    txt(d, PAD_X + 290, row_y(5.6), "Core 3:", DGRAY)
    bar_gauge(d, PAD_X + 360, row_y(5.6) + 2, 100, 15, GREEN)
    txt(d, PAD_X + 465, row_y(5.6), "15%", GREEN)

    hline(d, 7)

    # Memory
    txt(d, PAD_X + 4, row_y(7.5), "MEM", CYAN, FONT_BOLD)
    txt(d, PAD_X + 50, row_y(7.5), "3.2G / 8.0G (40%)", WHITE)
    txt(d, PAD_X + 280, row_y(7.5), "Swap: 0B / 2.0G", DGRAY)
    mem_vals = [35,35,36,37,38,38,39,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40,40]
    sparkline(d, PAD_X + 50, row_y(8.5), mem_vals, MAGENTA)
    bar_gauge(d, PAD_X + 50, row_y(9.5) + 2, 400, 40, MAGENTA)
    txt(d, PAD_X + 460, row_y(9.5), "40%", MAGENTA)

    hline(d, 10.8)

    # Load & Uptime
    txt(d, PAD_X + 4, row_y(11.3), "LOAD", CYAN, FONT_BOLD)
    txt(d, PAD_X + 60, row_y(11.3), "0.82", WHITE)
    txt(d, PAD_X + 110, row_y(11.3), "0.64", LGRAY)
    txt(d, PAD_X + 160, row_y(11.3), "0.55", LGRAY)
    txt(d, PAD_X + 260, row_y(11.3), "UPTIME", CYAN, FONT_BOLD)
    txt(d, PAD_X + 340, row_y(11.3), "14d 6h 32m", WHITE)

    hline(d, 12.5)

    # Disk
    txt(d, PAD_X + 4, row_y(13), "DISK", CYAN, FONT_BOLD)
    txt(d, PAD_X + 60, row_y(13), "/", WHITE)
    txt(d, PAD_X + 170, row_y(13), "12.4G / 20.0G", LGRAY)
    bar_gauge(d, PAD_X + 330, row_y(13) + 2, 150, 62, YELLOW)
    txt(d, PAD_X + 490, row_y(13), "62%", YELLOW)
    txt(d, PAD_X + 60, row_y(14), "/data", WHITE)
    txt(d, PAD_X + 170, row_y(14), "84.2G / 200G", LGRAY)
    bar_gauge(d, PAD_X + 330, row_y(14) + 2, 150, 42, GREEN)
    txt(d, PAD_X + 490, row_y(14), "42%", GREEN)

    hline(d, 15.3)

    # Network
    txt(d, PAD_X + 4, row_y(16), "NET", CYAN, FONT_BOLD)
    txt(d, PAD_X + 60, row_y(16), "RX", CYAN)
    rx_vals = [10,20,30,20,10,20,35,50,30,20,10]
    sparkline(d, PAD_X + 90, row_y(16), rx_vals, GREEN)
    txt(d, PAD_X + 190, row_y(16), "48.5KB/s", GREEN)
    txt(d, PAD_X + 300, row_y(16), "TX", CYAN)
    tx_vals = [5,5,5,10,5,5,5,12,5,5,5]
    sparkline(d, PAD_X + 330, row_y(16), tx_vals, BLUE)
    txt(d, PAD_X + 430, row_y(16), "1.2KB/s", BLUE)

    hline(d, 17.3)

    # Process table
    txt(d, PAD_X + 4, row_y(18), "Top Processes", CYAN, FONT_BOLD)
    txt(d, PAD_X + 160, row_y(18), "(by CPU)", DGRAY)
    txt(d, PAD_X + 10, row_y(19), "  PID     Name                    CPU%     MEM%     RSS", DGRAY)
    procs = [
        ("1842", "node", "8.2", "3.1", "256M"),
        ("2104", "nginx", "4.1", "0.8", "64M"),
        ("3921", "postgres", "3.7", "5.2", "420M"),
        ("1203", "containerd", "2.1", "1.4", "112M"),
        ("5502", "prometheus", "1.8", "2.3", "188M"),
    ]
    for i, (pid, name, cpu, mem, rss) in enumerate(procs):
        r = 20 + i
        cpu_c = YELLOW if float(cpu) > 5 else FG
        txt(d, PAD_X + 10, row_y(r), f"  {pid:<8s}{name:<24s}{cpu:>5s}    {mem:>5s}    {rss:>5s}", FG)
        # Highlight high CPU
        if float(cpu) > 5:
            txt(d, PAD_X + 10 + 8 * 32, row_y(r), f"{cpu:>5s}", YELLOW)

    hline(d, 25.5)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " Esc", YELLOW)
    txt(d, PAD_X + 40, row_y(28), ":Terminal", LGRAY)
    txt(d, PAD_X + 130, row_y(28), "s", YELLOW)
    txt(d, PAD_X + 142, row_y(28), ":Sort(→mem)", LGRAY)
    txt(d, PAD_X + 260, row_y(28), "p", YELLOW)
    txt(d, PAD_X + 272, row_y(28), ":Pause", LGRAY)
    txt(d, PAD_X + 340, row_y(28), "r", YELLOW)
    txt(d, PAD_X + 352, row_y(28), ":Refresh", LGRAY)
    txt(d, PAD_X + 430, row_y(28), "↑↓", YELLOW)
    txt(d, PAD_X + 455, row_y(28), ":Scroll", LGRAY)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 4: Split-Pane View (Terminal + Monitor)
# ═══════════════════════════════════════════════════════════════════════════════
def frame_split():
    img, d = new_frame()
    # Tab bar
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 100, row_y(0), "──", DGRAY)
    txt(d, 130, row_y(0), " [1] bastion-east ", YELLOW)
    txt(d, 310, row_y(0), " [2] db-primary ", LGRAY)
    txt(d, W - 120, row_y(0), "Split Pane", CYAN)
    txt(d, W - 60, row_y(0), "14:35", LGRAY)

    hline(d, 1)

    MID = W // 2 + 30

    # Left pane — Terminal
    d.rectangle([4, row_y(1.5), MID - 4, row_y(26)], outline=CYAN, width=1)
    txt(d, 10, row_y(1.7), " Terminal", CYAN, FONT_BOLD)

    term_lines = [
        ("ops@bastion:~$ ", CYAN, "top -bn1 | head -5", GREEN),
        ("top - 14:35:12 up 14 days, 6:32, 3 users", FG, "", FG),
        ("Tasks: 142 total, 1 running, 141 sleeping", FG, "", FG),
        ("%Cpu:  23.4 us,  2.1 sy,  0.0 ni, 74.5 id", FG, "", FG),
        ("MiB Mem:  8192.0 total, 1843.2 free, 3276.8 used", FG, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "curl -s localhost:80", GREEN),
        ("HTTP/1.1 200 OK", GREEN, "", FG),
        ("Content-Type: text/html", DGRAY, "", FG),
        ("", FG, "", FG),
        ("ops@bastion:~$ ", CYAN, "█", WHITE),
    ]
    for i, (prompt, pc, cmd, cc) in enumerate(term_lines):
        r = 3 + i
        if prompt:
            txt(d, 12, row_y(r), prompt, pc, FONT_SM)
            txt(d, 12 + len(prompt) * 7, row_y(r), cmd, cc, FONT_SM)
        else:
            txt(d, 12, row_y(r), cmd, cc, FONT_SM)

    # Right pane — Monitor
    d.rectangle([MID + 2, row_y(1.5), W - 4, row_y(26)], outline=YELLOW, width=1)
    txt(d, MID + 10, row_y(1.7), " Monitor", YELLOW, FONT_BOLD)

    RX = MID + 12
    txt(d, RX, row_y(3), "CPU", CYAN, FONT_SM)
    txt(d, RX + 35, row_y(3), "23.4%", WHITE, FONT_SM)
    cpu_v = [15,22,30,28,20,45,30,18,12,20,30,38,50,58,48,28,18,14,20,30]
    sparkline(d, RX + 90, row_y(3), cpu_v, CYAN)
    bar_gauge(d, RX + 35, row_y(4) + 2, 300, 23, CYAN)

    txt(d, RX, row_y(6), "MEM", CYAN, FONT_SM)
    txt(d, RX + 35, row_y(6), "3.2G/8G", WHITE, FONT_SM)
    bar_gauge(d, RX + 35, row_y(7) + 2, 300, 40, MAGENTA)

    txt(d, RX, row_y(9), "LOAD", CYAN, FONT_SM)
    txt(d, RX + 50, row_y(9), "0.82  0.64  0.55", WHITE, FONT_SM)

    txt(d, RX, row_y(10.5), "NET", CYAN, FONT_SM)
    txt(d, RX + 38, row_y(10.5), "↓48.5K", GREEN, FONT_SM)
    txt(d, RX + 110, row_y(10.5), "↑1.2K", BLUE, FONT_SM)

    txt(d, RX, row_y(12.5), "DISK /", CYAN, FONT_SM)
    bar_gauge(d, RX + 60, row_y(12.5) + 2, 200, 62, YELLOW)
    txt(d, RX + 270, row_y(12.5), "62%", YELLOW, FONT_SM)

    # Mini process list
    txt(d, RX, row_y(14.5), "Procs", CYAN, FONT_SM)
    txt(d, RX + 50, row_y(14.5), "(CPU)", DGRAY, FONT_SM)
    mini_procs = [("node", "8.2%"), ("nginx", "4.1%"), ("postgres", "3.7%"), ("containerd", "2.1%")]
    for i, (name, cpu) in enumerate(mini_procs):
        txt(d, RX + 8, row_y(15.5 + i), f"{name:<14s}{cpu}", FG, FONT_SM)

    hline(d, 26.3)

    # Diagnostics bar
    fill_row(d, 26.8, (25, 30, 40))
    txt(d, PAD_X + 4, row_y(26.8), "RTT:", DGRAY, FONT_SM)
    txt(d, PAD_X + 35, row_y(26.8), "12.3ms", GREEN, FONT_SM)
    txt(d, PAD_X + 100, row_y(26.8), "↑1.2K ↓48.5K", LGRAY, FONT_SM)
    txt(d, PAD_X + 240, row_y(26.8), "●Excellent", GREEN, FONT_SM)
    txt(d, PAD_X + 370, row_y(26.8), "Up:2h14m", LGRAY, FONT_SM)
    txt(d, PAD_X + 500, row_y(26.8), "via bastion-east", DIM, FONT_SM)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " Alt+s", YELLOW)
    txt(d, PAD_X + 55, row_y(28), ":Unsplit", LGRAY)
    txt(d, PAD_X + 140, row_y(28), "Alt+[/]", YELLOW)
    txt(d, PAD_X + 210, row_y(28), ":Resize", LGRAY)
    txt(d, PAD_X + 280, row_y(28), "Alt+f", YELLOW)
    txt(d, PAD_X + 335, row_y(28), ":Files", LGRAY)
    txt(d, PAD_X + 390, row_y(28), "Alt+p", YELLOW)
    txt(d, PAD_X + 445, row_y(28), ":Fwd", LGRAY)
    txt(d, PAD_X + 500, row_y(28), "Ctrl+p", YELLOW)
    txt(d, PAD_X + 560, row_y(28), ":Palette", LGRAY)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 5: Command Palette (Ctrl+P)
# ═══════════════════════════════════════════════════════════════════════════════
def frame_palette():
    img, d = new_frame()
    # Background: faded dashboard
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 240, row_y(0), " [1] Sessions ", LGRAY)
    txt(d, 380, row_y(0), " [2] Hosts ", YELLOW)
    hline(d, 1)
    # Dim background content
    for r in range(2, 28):
        txt(d, PAD_X + 20, row_y(r), "░" * 70, (30, 30, 40), FONT_SM)

    # Palette overlay
    PX, PY = 180, 80
    PW, PH = 600, 380
    # Shadow
    d.rectangle([PX + 4, PY + 4, PX + PW + 4, PY + PH + 4], fill=(10, 10, 15))
    # Box
    d.rectangle([PX, PY, PX + PW, PY + PH], fill=(22, 22, 32), outline=CYAN, width=2)

    # Title
    txt(d, PX + 20, PY + 10, "Command Palette", CYAN, FONT_BOLD)
    txt(d, PX + PW - 80, PY + 10, "Ctrl+P", DGRAY)

    # Search input
    d.rectangle([PX + 15, PY + 38, PX + PW - 15, PY + 60], fill=(30, 35, 48), outline=DGRAY)
    txt(d, PX + 22, PY + 40, "> bast", WHITE)
    txt(d, PX + 75, PY + 40, "█", CYAN)

    # Divider
    d.line([(PX + 15, PY + 68), (PX + PW - 15, PY + 68)], fill=DGRAY)

    # Results
    results = [
        ("» ", YELLOW, "[Host]", MAGENTA, "  bastion-east", WHITE, "  bastion.us-east.corp.io", DGRAY, True),
        ("  ", FG, "[Session]", BLUE, "  bastion-east", FG, "  ops@bastion — Active 2h14m", DGRAY, False),
        ("  ", FG, "[Host]", MAGENTA, "  bastion-west", FG, "  bastion.us-west.corp.io", DGRAY, False),
    ]
    for i, (arrow, ac, cat, catc, name, nc, desc, dc, selected) in enumerate(results):
        ry = PY + 78 + i * 30
        if selected:
            d.rectangle([PX + 15, ry - 2, PX + PW - 15, ry + 22], fill=SEL_BG)
        txt(d, PX + 22, ry, arrow, ac)
        txt(d, PX + 38, ry, cat, catc, FONT_SM)
        txt(d, PX + 120, ry, name, nc)
        txt(d, PX + 260, ry, desc, dc, FONT_SM)

    # Hints at bottom
    txt(d, PX + 20, PY + PH - 32, "↑↓", YELLOW, FONT_SM)
    txt(d, PX + 42, PY + PH - 32, ":Navigate", DGRAY, FONT_SM)
    txt(d, PX + 130, PY + PH - 32, "Enter", YELLOW, FONT_SM)
    txt(d, PX + 175, PY + PH - 32, ":Execute", DGRAY, FONT_SM)
    txt(d, PX + 260, PY + PH - 32, "Esc", YELLOW, FONT_SM)
    txt(d, PX + 290, PY + PH - 32, ":Close", DGRAY, FONT_SM)
    txt(d, PX + 360, PY + PH - 32, "3 results", DIM, FONT_SM)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 6: File Browser
# ═══════════════════════════════════════════════════════════════════════════════
def frame_filebrowser():
    img, d = new_frame()
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 100, row_y(0), "──", DGRAY)
    txt(d, 130, row_y(0), " [1] bastion-east ", YELLOW)
    txt(d, W - 120, row_y(0), "File Browser", CYAN)
    txt(d, W - 60, row_y(0), "14:36", LGRAY)

    hline(d, 1)

    MID = W // 2

    # Left pane — Local
    d.rectangle([4, row_y(1.5), MID - 4, row_y(24)], outline=YELLOW, width=1)
    txt(d, 10, row_y(1.7), " Local: ~/deploy/", YELLOW, FONT_BOLD)

    local_files = [
        ("📁", "configs/", CYAN, False),
        ("📁", "scripts/", CYAN, False),
        ("📄", "deploy.sh", FG, True),
        ("📄", "nginx.conf", FG, False),
        ("📄", "docker-compose.yml", FG, False),
        ("📄", "README.md", FG, False),
    ]
    for i, (icon, name, color, selected) in enumerate(local_files):
        r = 3 + i
        if selected:
            fill_row(d, r, SEL_BG, 6, MID - 6)
            txt(d, 14, row_y(r), f"» {icon} {name}", YELLOW)
        else:
            txt(d, 14, row_y(r), f"  {icon} {name}", color)

    # Right pane — Remote
    d.rectangle([MID + 2, row_y(1.5), W - 4, row_y(24)], outline=CYAN, width=1)
    txt(d, MID + 10, row_y(1.7), " Remote: /opt/app/", CYAN, FONT_BOLD)

    remote_files = [
        ("📁", "logs/", CYAN, False),
        ("📁", "config/", CYAN, False),
        ("📁", "data/", CYAN, False),
        ("📄", "app.jar", FG, False),
        ("📄", "start.sh", FG, False),
        ("📄", "application.yml", FG, False),
    ]
    for i, (icon, name, color, selected) in enumerate(remote_files):
        r = 3 + i
        txt(d, MID + 14, row_y(r), f"  {icon} {name}", color)

    # Transfer progress
    d.rectangle([4, row_y(24.5), W - 4, row_y(26.5)], outline=DGRAY, width=1)
    txt(d, 14, row_y(24.8), "Uploading:", LGRAY)
    txt(d, 110, row_y(24.8), "deploy.sh", YELLOW)
    txt(d, 220, row_y(24.8), "→", CYAN)
    txt(d, 240, row_y(24.8), "/opt/app/deploy.sh", CYAN)
    bar_gauge(d, 14, row_y(25.8) + 2, 700, 67, GREEN)
    txt(d, 730, row_y(25.8), "67%", GREEN)
    txt(d, 780, row_y(25.8), "4.2KB/s", LGRAY, FONT_SM)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " Tab", YELLOW)
    txt(d, PAD_X + 40, row_y(28), ":Switch", LGRAY)
    txt(d, PAD_X + 110, row_y(28), "u", YELLOW)
    txt(d, PAD_X + 122, row_y(28), ":Upload", LGRAY)
    txt(d, PAD_X + 190, row_y(28), "d", YELLOW)
    txt(d, PAD_X + 202, row_y(28), ":Download", LGRAY)
    txt(d, PAD_X + 290, row_y(28), "m", YELLOW)
    txt(d, PAD_X + 302, row_y(28), ":Mkdir", LGRAY)
    txt(d, PAD_X + 370, row_y(28), "Del", YELLOW)
    txt(d, PAD_X + 400, row_y(28), ":Remove", LGRAY)
    txt(d, PAD_X + 480, row_y(28), "Esc", YELLOW)
    txt(d, PAD_X + 515, row_y(28), ":Close", LGRAY)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Frame 7: Port Forwarding Manager
# ═══════════════════════════════════════════════════════════════════════════════
def frame_portfwd():
    img, d = new_frame()
    fill_row(d, 0, BAR_BG)
    txt(d, PAD_X, row_y(0), " ESSH", CYAN, FONT_BOLD)
    txt(d, 100, row_y(0), "──", DGRAY)
    txt(d, 130, row_y(0), " [1] bastion-east ", YELLOW)
    txt(d, W - 140, row_y(0), "Port Forwarding", CYAN)
    txt(d, W - 60, row_y(0), "14:37", LGRAY)

    hline(d, 1)

    txt(d, PAD_X + 4, row_y(2), " Active Port Forwards", CYAN, FONT_BOLD)
    txt(d, PAD_X + 10, row_y(3.5), "  Direction   Bind Address        Target                Status", DGRAY)
    hline(d, 4.5)

    forwards = [
        ("» Local (-L)", "localhost:8080", "web-internal:80", "● Active", GREEN, True),
        ("  Local (-L)", "localhost:5432", "db-internal:5432", "● Active", GREEN, False),
        ("  Local (-L)", "localhost:6379", "redis.local:6379", "● Active", GREEN, False),
        ("  Local (-L)", "localhost:9090", "prometheus:9090", "○ Stopped", RED, False),
    ]
    for i, (dir, bind, target, status, sc, selected) in enumerate(forwards):
        r = 5 + i
        if selected:
            fill_row(d, r, SEL_BG)
        nc = YELLOW if selected else FG
        txt(d, PAD_X + 10, row_y(r), f"{dir:<14s}{bind:<20s}{target:<22s}", nc)
        txt(d, PAD_X + 10 + 56 * 8, row_y(r), status, sc)

    hline(d, 10)

    # Add forward input
    txt(d, PAD_X + 4, row_y(11), " Add Forward", CYAN, FONT_BOLD)
    txt(d, PAD_X + 10, row_y(12.5), "Format:", DGRAY)
    txt(d, PAD_X + 80, row_y(12.5), "L:bind_port:target_host:target_port", LGRAY)

    hline(d, 14)

    # Stats
    txt(d, PAD_X + 4, row_y(15), " Forward Statistics", CYAN, FONT_BOLD)
    txt(d, PAD_X + 20, row_y(16.5), "localhost:8080 → web-internal:80", WHITE)
    txt(d, PAD_X + 40, row_y(17.5), "Connections:", DGRAY)
    txt(d, PAD_X + 160, row_y(17.5), "142", WHITE)
    txt(d, PAD_X + 240, row_y(17.5), "Bytes TX:", DGRAY)
    txt(d, PAD_X + 330, row_y(17.5), "2.4MB", CYAN)
    txt(d, PAD_X + 420, row_y(17.5), "Bytes RX:", DGRAY)
    txt(d, PAD_X + 510, row_y(17.5), "18.7MB", CYAN)

    # Footer
    fill_row(d, 28, BAR_BG)
    txt(d, PAD_X, row_y(28), " a", YELLOW)
    txt(d, PAD_X + 15, row_y(28), ":Add", LGRAY)
    txt(d, PAD_X + 70, row_y(28), "d", YELLOW)
    txt(d, PAD_X + 82, row_y(28), ":Delete", LGRAY)
    txt(d, PAD_X + 155, row_y(28), "Enter", YELLOW)
    txt(d, PAD_X + 205, row_y(28), ":Toggle", LGRAY)
    txt(d, PAD_X + 290, row_y(28), "Esc", YELLOW)
    txt(d, PAD_X + 322, row_y(28), ":Close", LGRAY)

    d.rectangle([2, 2, W-3, H-3], outline=DGRAY, width=1)
    return img


# ═══════════════════════════════════════════════════════════════════════════════
#  Assemble GIF
# ═══════════════════════════════════════════════════════════════════════════════
def main():
    print("Generating frames...")
    frames = [
        ("Dashboard — Hosts Tab", frame_dashboard),
        ("Session Terminal + Diagnostics", frame_session),
        ("Host Monitor (Remote htop)", frame_monitor),
        ("Split-Pane View", frame_split),
        ("Command Palette (Ctrl+P)", frame_palette),
        ("File Browser (Alt+F)", frame_filebrowser),
        ("Port Forwarding (Alt+P)", frame_portfwd),
    ]

    images = []
    for name, fn in frames:
        print(f"  → {name}")
        img = fn()
        # Add label at bottom-right corner
        draw = ImageDraw.Draw(img)
        txt(draw, W - 250, H - 20, f"▸ {name}", DIM, FONT_SM)
        images.append(img)

    out_path = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "demo.gif")

    # 3.5 seconds per frame
    durations = [3500] * len(images)

    images[0].save(
        out_path,
        save_all=True,
        append_images=images[1:],
        duration=durations,
        loop=0,
        optimize=False,
    )

    size_kb = os.path.getsize(out_path) / 1024
    print(f"\n✅ Saved: {out_path} ({size_kb:.0f} KB, {len(images)} frames, {sum(durations)/1000:.1f}s total)")


if __name__ == "__main__":
    main()
