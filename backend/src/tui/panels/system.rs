//! System panel - displays CPU, memory, and uptime statistics.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
use sysinfo::System;

use crate::tui::input::Action;
use crate::tui::panels::Panel;
use crate::tui::theme::{PanelKind, Theme};

const HISTORY_LIMIT: usize = 120;

/// Snapshot used by the system panel.
#[derive(Clone, Default)]
struct SystemSnapshot {
    cpu_percent: f64,
    memory_used: u64,
    memory_total: u64,
    uptime_secs: u64,
    total_requests: u64,
    active_requests: u64,
    cpu_history: Vec<u64>,
    mem_history: Vec<u64>,
}

fn format_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// System panel showing CPU, memory, uptime.
pub struct SystemPanel {
    sys: System,
    snapshot: SystemSnapshot,
    port: u16,
}

impl SystemPanel {
    /// Create a new system panel.
    pub fn new(port: u16) -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_usage();

        let mut panel = Self {
            sys,
            snapshot: SystemSnapshot::default(),
            port,
        };
        panel.refresh();
        panel
    }

    fn refresh(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        let cpu_percent = self.sys.global_cpu_usage() as f64;
        let memory_total = self.sys.total_memory();
        let memory_used = self.sys.used_memory();
        let mem_percent = if memory_total > 0 {
            ((memory_used as f64 / memory_total as f64) * 100.0).round() as u64
        } else {
            0
        };

        self.snapshot.cpu_percent = cpu_percent;
        self.snapshot.memory_total = memory_total;
        self.snapshot.memory_used = memory_used;
        self.snapshot.uptime_secs = System::uptime();

        self.snapshot.cpu_history.push(cpu_percent.round() as u64);
        if self.snapshot.cpu_history.len() > HISTORY_LIMIT {
            self.snapshot.cpu_history.remove(0);
        }

        self.snapshot.mem_history.push(mem_percent);
        if self.snapshot.mem_history.len() > HISTORY_LIMIT {
            self.snapshot.mem_history.remove(0);
        }
    }
}

impl Panel for SystemPanel {
    fn title(&self) -> &str {
        "sys"
    }
    fn kind(&self) -> PanelKind {
        PanelKind::System
    }

    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Theme::border(PanelKind::System)
        } else {
            Theme::border_dim()
        };

        let ip_addr = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

        let title = Line::from(vec![
            Span::styled("┐", border_style),
            Span::styled(PanelKind::System.superscript(), Theme::panel_number()),
            Span::styled("sys", Theme::panel_title(PanelKind::System)),
            Span::styled("┌─┐", border_style),
            Span::styled(&ip_addr, Theme::dim()),
            Span::styled("┌", border_style),
        ]);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        let show_endpoints = inner.height >= 9;
        let chunks = if show_endpoints {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner)
        } else {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner)
        };

        let label_width = 14;
        let sparkline_width = inner.width.saturating_sub(label_width + 1) as usize;
        let cpu_data: Vec<u64> = if self.snapshot.cpu_history.len() > sparkline_width {
            self.snapshot.cpu_history[self.snapshot.cpu_history.len() - sparkline_width..].to_vec()
        } else {
            self.snapshot.cpu_history.clone()
        };

        let cpu_row = chunks[0];
        let cpu_chunks = Layout::horizontal([Constraint::Length(label_width), Constraint::Min(10)])
            .split(cpu_row);

        let cpu_text = Paragraph::new(Line::from(vec![
            Span::styled("CPU ", Theme::dim()),
            Span::styled(format!("{:5.1}%", self.snapshot.cpu_percent), Theme::text()),
        ]));
        frame.render_widget(cpu_text, cpu_chunks[0]);
        frame.render_widget(
            Sparkline::default()
                .data(&cpu_data)
                .max(100)
                .style(Theme::sparkline_cpu()),
            cpu_chunks[1],
        );

        let mem_used_gb = self.snapshot.memory_used as f64 / 1_073_741_824.0;
        let mem_total_gb = self.snapshot.memory_total as f64 / 1_073_741_824.0;
        let mem_data: Vec<u64> = if self.snapshot.mem_history.len() > sparkline_width {
            self.snapshot.mem_history[self.snapshot.mem_history.len() - sparkline_width..].to_vec()
        } else {
            self.snapshot.mem_history.clone()
        };

        let mem_row = chunks[1];
        let mem_chunks = Layout::horizontal([Constraint::Length(label_width), Constraint::Min(10)])
            .split(mem_row);

        let mem_text = Paragraph::new(Line::from(vec![
            Span::styled("MEM ", Theme::dim()),
            Span::styled(
                format!("{:.1}G/{:.1}G", mem_used_gb, mem_total_gb),
                Theme::text(),
            ),
        ]));
        frame.render_widget(mem_text, mem_chunks[0]);
        frame.render_widget(
            Sparkline::default()
                .data(&mem_data)
                .max(100)
                .style(Theme::sparkline_mem()),
            mem_chunks[1],
        );

        let uptime = format_uptime(self.snapshot.uptime_secs);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("UP  ", Theme::dim()),
                Span::styled(uptime, Theme::text()),
            ])),
            chunks[2],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("REQ ", Theme::dim()),
                Span::styled(
                    format!("{} total", format_number(self.snapshot.total_requests)),
                    Theme::text(),
                ),
                Span::styled("  ", Theme::dim()),
                Span::styled(
                    format!("{} active", self.snapshot.active_requests),
                    if self.snapshot.active_requests > 0 {
                        Theme::log_level("INFO")
                    } else {
                        Theme::dim()
                    },
                ),
            ])),
            chunks[3],
        );

        if show_endpoints {
            let base_url = format_base_url(&ip_addr, self.port);
            let frontend_url = format!("{}/", base_url);
            let graphql_url = format!("{}/graphql", base_url);
            let graphql_ws_url = format!("{}/graphql/ws", base_url);
            let api_url = format!("{}/api", base_url);
            let health_url = format!("{}/api/health", base_url);

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("WEB ", Theme::dim()),
                    Span::styled(frontend_url, Theme::text()),
                ])),
                chunks[4],
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("GQL ", Theme::dim()),
                    Span::styled(graphql_url, Theme::text()),
                ])),
                chunks[5],
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("API ", Theme::dim()),
                    Span::styled(api_url, Theme::text()),
                ])),
                chunks[6],
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("WS  ", Theme::dim()),
                    Span::styled(graphql_ws_url, Theme::text()),
                ])),
                chunks[7],
            );
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("HLT ", Theme::dim()),
                    Span::styled(health_url, Theme::text()),
                ])),
                chunks[8],
            );
        }
    }

    fn handle_action(&mut self, action: &Action) {
        if let Action::Refresh = action {
            self.refresh();
        }
    }

    fn update(&mut self) {
        self.refresh();
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return Some(addr.ip().to_string());
            }
        }
    }

    if let Ok(hostname) = std::process::Command::new("hostname").arg("-I").output() {
        if hostname.status.success() {
            let output = String::from_utf8_lossy(&hostname.stdout);
            if let Some(ip) = output.split_whitespace().next() {
                return Some(ip.to_string());
            }
        }
    }

    None
}

fn format_base_url(ip: &str, port: u16) -> String {
    if ip.contains(':') {
        format!("http://[{}]:{}", ip, port)
    } else {
        format!("http://{}:{}", ip, port)
    }
}
