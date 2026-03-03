# cube-tui

A terminal UI timer and session manager for speedcubing. 

Out of the box, it's a lightweight TUI timer that lets you time solves, scramble, and check basic stats directly from your terminal. All your session data is saved locally.

## Requirements

- [Rust & Cargo](https://rustup.rs/)
- [Bun](https://bun.sh/) (only required if you are installing the `dashboard` feature)

## Installation & Features

You can install the app globally using Cargo. By default, it installs the core terminal app without any heavy dependencies:

```sh
cargo install cube-tui
```

If you want extra functionality, you can enable specific features during installation depending on your setup.

### Dashboard

Spins up a local web server (localhost) with a companion app. This web UI can be used to look at your solve history and advanced statistics.

To install with the dashboard included:

```sh
cargo install cube-tui --features dashboard
```

*Note: The build script will automatically use Bun to install dependencies and build the web frontend, embedding it directly into the executable.*

### Bluetooth

Allows the app to connect directly to Bluetooth Low Energy (BLE) timers. Right now, only the GAN timer is supported.

To install with Bluetooth support:

```sh
cargo install cube-tui --features bluetooth
```

### All Features

If you want both the dashboard and Bluetooth timer support:

```sh
cargo install cube-tui --features "dashboard bluetooth"
```
