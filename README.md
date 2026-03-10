<div align="center">

<img src="assets/icon.svg" alt="gtop logo" width="120">

# gtop

**A beautiful GUI system monitor built with [GPUI](https://gpui.rs)**

[![Rust](https://img.shields.io/badge/rust-2024_edition-orange?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)]()

<!-- Add a screenshot here: ![screenshot](./assets/screenshot.png) -->

</div>

---

gtop is a native, GPU-accelerated system monitor that provides real-time insight into your system's performance. Built with [GPUI Component](https://github.com/longbridge/gpui-component) — the UI framework behind [Zed Editor](https://zed.dev).

## Features

- **Real-time CPU & Memory charts** — smooth animated area charts with 120-point history
- **Per-core CPU usage** — color-coded grid showing individual core loads
- **Process manager** — sortable, searchable process table with right-click to kill
- **Network monitoring** — live upload/download rates
- **Disk & Battery** — storage usage and battery status at a glance
- **CPU temperature** — thermal monitoring (when sensors are available)
- **Native look & feel** — follows system dark/light theme, macOS-native title bar
- **Smooth animations** — interpolated metrics and tab transitions

## Prerequisites

- **Rust** (edition 2024, nightly recommended)
- Platform dependencies for GPUI:
  - **macOS**: Xcode Command Line Tools
  - **Linux**: see [GPUI Linux dependencies](https://github.com/zed-industries/zed/blob/main/docs/src/development/linux.md)
  - **Windows**: Visual Studio Build Tools with C++ workload

## Getting Started

```bash
# Clone the repository
git clone https://github.com/zhy0216/gtop.git
cd gtop

# Build and run (release mode recommended for performance)
cargo run --release
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+Q` (macOS) / `Alt+F4` (Windows/Linux) | Quit |
| Right-click on process | Kill process |

## Project Structure

```
gtop/
├── src/
│   └── main.rs          # Application entry point & all UI
├── crates/
│   ├── ui/              # gpui-component UI library
│   ├── assets/          # Icons and static assets
│   └── macros/          # Procedural macros
├── Cargo.toml           # Workspace configuration
└── README.md
```

## Screenshots

> TODO: Add screenshots of the System tab and Processes tab

## Built With

- [GPUI](https://gpui.rs) — GPU-accelerated UI framework from Zed
- [gpui-component](https://github.com/longbridge/gpui-component) — Component library for GPUI
- [sysinfo](https://crates.io/crates/sysinfo) — System information gathering
- [battery](https://crates.io/crates/battery) — Cross-platform battery monitoring

## Contributing

Contributions are welcome! Please feel free to open an issue or submit a pull request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
