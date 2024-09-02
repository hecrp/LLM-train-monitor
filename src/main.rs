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
    terminal::{Clear, ClearType},
    cursor::MoveTo,
};
use std::fs::File;
use std::io::stdout;
use std::io::{BufRead, BufReader, Write};
use regex::Regex;

// Under development.
// Right now, the program is general-purpose, not LLM-specific. Will work on it.
// TODO: Try crossterm and view update instead of println!(). 
// TODO: Add support for specific frameworks (Hugging Face Transformers?).

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

    // Display current system information
    fn display<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writeln!(writer, "LLM Training Monitor")?;
        writeln!(writer, "-------------------")?;
        
        // CPU metrics
        let cpu_usage = self.system.global_cpu_info().cpu_usage();
        writeln!(writer, "CPU Usage: {:.2}%", cpu_usage)?;

        // GPU metrics. Now it can be run without GPU!
        if let Err(_) = self.display_gpu_info(writer) {
            writeln!(writer, "GPU information not available")?;
        }

        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        // print memory in MB!!
        writeln!(writer, "Memory Usage: {} / {} MB", 
            used_memory / 1024 / 1024, 
            total_memory / 1024 / 1024)?;

        if let Some(process) = self.system.processes_by_exact_name(&self.process_name).next() {
            writeln!(writer, "Process CPU Usage: {:.2}%", process.cpu_usage())?;
            writeln!(writer, "Process Memory Usage: {} MB", process.memory() / 1024 / 1024)?;
        } else {
            writeln!(writer, "Process '{}' not found", self.process_name)?;
        }

        self.display_log_metrics(writer)?;
        Ok(())
    }

    // Display GPU information. Now works with multiple GPUs. Dope...
    fn display_gpu_info<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let nvml = self.nvml.as_ref().ok_or(std::io::Error::new(std::io::ErrorKind::Other, "NVML not initialized"))?;
        let device_count = nvml.device_count().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        for i in 0..device_count {
            let device = nvml.device_by_index(i).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            writeln!(writer, "GPU {}:", i)?;
            if let Ok(gpu_utilization) = device.utilization_rates() {
                writeln!(writer, "  Usage: {}%", gpu_utilization.gpu)?;
            }
            if let Ok(gpu_memory) = device.memory_info() {
                writeln!(writer, "  Memory: {} / {} MB (Used/Total)", 
                    gpu_memory.used / 1024 / 1024, 
                    gpu_memory.total / 1024 / 1024)?;
            }
            if let Ok(temp) = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
                writeln!(writer, "  Temperature: {}°C", temp)?;
            }
        }
        Ok(())
    }

    // Display log metrics
    fn display_log_metrics<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        if let (Some(log_path), Some(regex)) = (&self.log_file_path, &self.metric_regex) {
            if let Ok(file) = File::open(log_path) {
                let reader = BufReader::new(file);
                let lines: Vec<_> = reader.lines().collect::<Result<_, _>>()?;
                for line in lines.iter().rev().take(10) {
                    if let Some(captures) = regex.captures(line) {
                        if let Some(metric) = captures.get(1) {
                            writeln!(writer, "Extracted metric: {}", metric.as_str())?;
                        }
                    }
                }
            } else {
                writeln!(writer, "Failed to open log file")?;
            }
        }
        Ok(())
    }

    // Main loop to continuously update and display information
    fn run(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout();
        loop {
            self.update();
            execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
            self.display(&mut stdout)?;
            stdout.flush()?;
            std::thread::sleep(self.update_interval);
        }
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
            .help("Path to the log file to monitor")
            .required(false)
            .index(3))
        .arg(Arg::with_name("metric_regex")
            .help("Regex to extract metrics from log file")
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