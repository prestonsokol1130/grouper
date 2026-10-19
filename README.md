# Grouper

[![CI](https://github.com/prestonsokol1130/grouper/actions/workflows/ci.yml/badge.svg)](https://github.com/prestonsokol1130/grouper/actions/workflows/ci.yml)

Grouper is a Windows 11 tray utility (Tauri v2 + React) that adds iOS-style app-group folders to the taskbar and a macOS-style fullscreen Launchpad. Agent and contributor rules live in [AGENTS.md](./AGENTS.md).

## Development

```sh
pnpm install
pnpm tauri dev
```

The manager, Launchpad, and flyout windows start hidden. Stop the dev process from the terminal.
