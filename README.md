# LLM Training Monitor 🖥️🔍

Personal project featuring a Rust-based monitoring CLI tool for Large Language Model (LLM) training processes. This tool provides real-time information about system resources used during LLM training.

The idea for this project stems from the study and development of NLP models, where continuous monitoring of training processes can be interesting. I decided to build my own tiny monitoring tool to get some hands-on experience with Rust system programming.

Currently under development. 🚧

## Features

- Monitor CPU usage 📊
- Monitor GPU usage (using NVIDIA Management Library) 🎮
- Track memory consumption 💾
- Display process-specific information 📈

## Installation

1. Ensure you have Rust installed on your system.
2. Clone this repository:
   ```bash
   git clone https://github.com/hecrp/llmtrain_monitor.git
   ```
3. Build the project:
   ```bash
   cd llmtrain_monitor
   cargo build --release
   ```

## Usage

Run the monitor with the following command:
```
./target/release/llmtrain_monitor <process_name> <update_interval_seconds>
```

- `<process_name>`: The name of the LLM training process to monitor
- `<update_interval_seconds>`: The interval (in seconds) between updates

## Output

The monitor displays the following information:

- CPU Usage (overall system)
- GPU Usage (if available)
- GPU Memory Usage (if available)
- System Memory Usage
- Process-specific CPU Usage
- Process-specific Memory Usage

If the specified process is not found, the monitor will indicate this in the output.

## Dependencies

- sysinfo: For system and process information
- nvml-wrapper: For NVIDIA GPU monitoring
- clap: For building the CLI interface

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
