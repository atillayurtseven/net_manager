# Net Manager

A lightweight macOS menu bar monitor for network traffic and system resources, written in Rust.

Each module gets its own menu bar item: a short readout on the left, and the full detail when you click it. No Dock icon, no window hanging around — it runs as an `LSUIElement` agent.

## Modules

| Module | Menu bar | Details |
| --- | --- | --- |
| `network` | live up/down rate | per-interface traffic, today's and this month's totals, today's peak, traffic chart |
| `cpu` | total load | user/system split, per-core usage, GPU usage, load average, top process |
| `memory` | used percentage | used/free/total, swap, top process |
| `disk` | used percentage | free space per volume, read/write throughput |
| `battery` | charge level | state, cycles, health, time remaining, connected devices |
| `sensors` | temperature | readable components (limited on Apple silicon — fan and CPU temp need private APIs) |
| `clock` | local time | configurable world clocks, moon phase |

By default only `network`, `cpu` and `memory` are enabled so the menu bar doesn't get crowded. Everything else is toggled from Settings.

## Features

- **Traffic history** — minute-level samples stored in SQLite, viewable as an hour/day/week/month chart.
- **Threshold alerts** — native macOS notifications when CPU, memory or disk usage crosses a percentage you set (10-minute cooldown per alert type).
- **Unit control** — bytes (KB/s) or bits (Mbps), with a configurable minimum and maximum unit so the readout doesn't jump around.
- **Interface selection** — monitor all interfaces or pick a single one.
- **Localization** — English (default) and Turkish, with an embedded catalog; missing keys fall back to English.
- **Auto-start** — optional launch at login.
- **Single instance** — a local port lock keeps a second copy from starting.

## Requirements

- macOS 11 or later
- Rust toolchain (stable, edition 2021)

## Build & install

```bash
./scripts/make_app.sh            # builds target/NetManager.app
./scripts/make_app.sh --install  # also copies it to /Applications and launches it
```

The script compiles in release mode, generates the icon set with the bundled `gen_icon` helper, assembles the `.app` bundle and ad-hoc signs it.

To run without bundling:

```bash
cargo run --release
```

## Configuration

Settings live in `~/Library/Application Support/net_manager/`:

- `config.json` — interface, enabled modules, units, language, alert thresholds, world clocks
- `stats.db` — traffic history

Everything in `config.json` is editable from the Settings window, so you normally don't need to touch the file.

## Adding a language

1. Create `i18n/<code>.json` (copy `i18n/en.json` as the key reference).
2. Add a row to `CATALOGS` in [src/i18n.rs](src/i18n.rs).

English is the source language — new keys go into `i18n/en.json` first.

## Project layout

```
src/
  main.rs          event loop, tray items, module scheduling
  modules/         cpu, memory, disk, battery, sensors, clock
  stats.rs         SQLite traffic history
  chart.rs         traffic chart (SVG in a wry webview)
  settings.rs      settings window
  alerts.rs        threshold notifications
  config.rs        persisted configuration
  i18n.rs          embedded translation catalog
  units.rs         rate formatting
  ifaces.rs        network interface enumeration
  bin/             gen_icon, dump_chart helpers
```

Built with [tao](https://crates.io/crates/tao), [tray-icon](https://crates.io/crates/tray-icon), [wry](https://crates.io/crates/wry), [sysinfo](https://crates.io/crates/sysinfo) and [rusqlite](https://crates.io/crates/rusqlite).
