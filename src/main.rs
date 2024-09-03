//! My personal LLM Training Monitor
//!
//! A Rust-based monitoring CLI tool for Large Language Model (LLM) training processes.
//!
//! # Author
//! Héctor Rodríguez (hecrp)<hectorp94@hotmail.com>
//!
//! # Version
//! 0.1.0
//!
//! # License
//! MIT License

use std::time::Duration;
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};
use clap::{App, Arg};
use crossterm::{
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    cursor,
};
use std::io::stdout;
use regex::Regex;
use tui::{
    backend::CrosstermBackend,
    layout::{Layout, Constraint, Direction},
    widgets::{Block, Borders, Paragraph, List, ListItem},
    style::{Style, Color},
    Terminal,
};
use std::env;

// Struct to hold the monitor's state
struct LLMTrainMonitor {
    system: System,
    nvml: Option<nvml_wrapper::Nvml>,
    process_name: String,
    update_interval: Duration,
    log_file_path: Option<String>,
    metric_regex: Option<Regex>,
}

impl LLMTrainMonitor {
    // Initialize a new LLMTrainMonitor
    fn new(process_name: String, update_interval: Duration, log_file_path: Option<String>, metric_regex: Option<String>) -> Self {
        Self {
            system: System::new_all(),
            nvml: nvml_wrapper::Nvml::init().ok(),
            process_name,
            update_interval,
            log_file_path,
            metric_regex: metric_regex.map(|r| Regex::new(&r).expect("Invalid regex pattern")),
        }
    }

    // Update system and GPU information
    fn update(&mut self) {
        self.system.refresh_all();
    }

    // Get GPU information
    fn get_gpu_info(&self) -> Option<(f32, u64, u64, u32)> {
        self.nvml.as_ref().and_then(|nvml| {
            nvml.device_by_index(0).ok().and_then(|device| {
                let utilization = device.utilization_rates().ok()?;
                let memory = device.memory_info().ok()?;
                let temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu).ok()?;
                Some((utilization.gpu as f32, memory.used, memory.total, temp))
            })
        })
    }

    // Get process information
    fn get_process_info(&self) -> Option<(f32, u64)> {
        self.system.processes_by_exact_name(&self.process_name).next().map(|process| {
            (process.cpu_usage(), process.memory())
        })
    }
    // Main loop to continuously update and display information
    fn run(&mut self) -> std::io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let username = env::var("USER").unwrap_or_else(|_| "Unknown".to_string());
        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "Unknown".to_string());
        let os_info = format!("{} {}", self.system.name().unwrap_or_default(), self.system.os_version().unwrap_or_default());
        
        let gpu_info = self.get_gpu_info();
        let gpu_summary = match &gpu_info {
            Some((_, _, gpu_memory_total, _)) => format!("GPU: {} MB", gpu_memory_total / 1024 / 1024),
            None => "No GPU detected".to_string(),
        };

        let ascii_frame1 = format!(r#"
  ʕ •ᴥ•ʔ                LLM Training Monitor v0.1.0
  ʕ •ᴥ•ʔ                 User: {}@{} | OS: {}
  ʕ •ᴥ•ʔ                 {}
        "#, username, hostname, os_info, gpu_summary);

        let ascii_frame2 = format!(r#"
  ʕ -ᴥ-ʔ                LLM Training Monitor v0.1.0
  ʕ -ᴥ-ʔ                 User: {}@{} | OS: {}
  ʕ -ᴥ-ʔ                 {}
        "#, username, hostname, os_info, gpu_summary);

        let mut frame_toggle = false;

        loop {
            self.update();
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(5),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ].as_ref())
                    .split(f.size());

                let title = Paragraph::new(if frame_toggle { ascii_frame1.clone() } else { ascii_frame2.clone() })
                    .style(Style::default().fg(Color::Green))
                    .block(Block::default().borders(Borders::NONE));
                f.render_widget(title, chunks[0]);

                frame_toggle = !frame_toggle;

                let cpu_usage = self.system.global_cpu_info().cpu_usage();
                let cpu_info = Paragraph::new(format!("CPU Usage: {:.2}%", cpu_usage))
                    .block(Block::default().title("CPU Info").borders(Borders::ALL));
                f.render_widget(cpu_info, chunks[1]);

                let memory_info = Paragraph::new(format!(
                    "Memory Usage: {} / {} MB",
                    self.system.used_memory() / 1024 / 1024,
                    self.system.total_memory() / 1024 / 1024
                ))
                .block(Block::default().title("System Memory").borders(Borders::ALL));
                f.render_widget(memory_info, chunks[2]);

                if let Some((process_cpu_usage, process_memory)) = self.get_process_info() {
                    let process_info = List::new(vec![
                        ListItem::new(format!("CPU Usage: {:.2}%", process_cpu_usage)),
                        ListItem::new(format!("Memory Usage: {} MB", process_memory / 1024 / 1024)),
                    ])
                    .block(Block::default().title(format!("Process: {}", self.process_name)).borders(Borders::ALL));
                    f.render_widget(process_info, chunks[3]);
                } else {
                    let no_process_info = Paragraph::new(format!("Process '{}' not found", self.process_name))
                        .block(Block::default().title("Process Info").borders(Borders::ALL));
                    f.render_widget(no_process_info, chunks[3]);
                }

                if let Some((gpu_usage, gpu_memory_used, gpu_memory_total, gpu_temp)) = gpu_info {
                    let gpu_info = List::new(vec![
                        ListItem::new(format!("GPU Usage: {:.2}%", gpu_usage)),
                        ListItem::new(format!("Memory: {} / {} MB", gpu_memory_used / 1024 / 1024, gpu_memory_total / 1024 / 1024)),
                        ListItem::new(format!("Temperature: {}°C", gpu_temp)),
                    ])
                    .block(Block::default().title("GPU Info").borders(Borders::ALL));
                    f.render_widget(gpu_info, chunks[3]);
                }
            })?;

            if crossterm::event::poll(self.update_interval)? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;

        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    // CLI interface. parse command line arguments:
    let matches = App::new("LLM Training Monitor")
        .version("0.1.0")
        .author("Héctor Rodríguez (hecrp)")
        .about("Monitors system resources for LLM training processes")
        .arg(Arg::with_name("process_name")
            .help("Name of the process to monitor")
            .required(true)
            .index(1))
        .arg(Arg::with_name("update_interval")
            .help("Update interval in seconds")
            .required(true)
            .index(2))
        .arg(Arg::with_name("log_file_path")
            .help("Path to the log file to monitor (under development)")
            .required(false)
            .index(3))
        .arg(Arg::with_name("metric_regex")
            .help("Regex to extract metrics from log file (under development)")
            .required(false)
            .index(4))
        .get_matches();

    let process_name = matches.value_of("process_name").unwrap().to_string();
    let update_interval = Duration::from_secs(matches.value_of("update_interval").unwrap().parse().unwrap());
    let log_file_path = matches.value_of("log_file_path").map(String::from);
    let metric_regex = matches.value_of("metric_regex").map(String::from);

    // Create and run the monitor
    let mut monitor = LLMTrainMonitor::new(process_name, update_interval, log_file_path, metric_regex);
    monitor.run()
}