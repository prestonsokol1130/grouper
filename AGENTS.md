# AGENTS.md

Single source of agent rules for Grouper. Follow this file for every task.

## Project summary

Grouper is a Windows 11 tray utility that brings iOS-style app-group folders to the taskbar. Each folder is a 2×2 composite `.ico` paired with a `.lnk` that has a unique `AppUserModelID` of the form `com.grouper.group.<group_id>`. Activating the shortcut launches `Grouper.exe --flyout <group_id>`, which opens a flyout over the taskbar for that group.

It also ships a macOS-style fullscreen Launchpad: searchable app grid, pagination, and drag-and-drop folders. Groups and Launchpad share the same persistence model so rearranging apps in one surface stays consistent with the other.

## Stack

- **Shell:** Tauri v2 (Rust backend)
- **Frontend:** React 19, TypeScript (strict), Vite, Tailwind CSS
- **Package manager:** pnpm
- **Rust:** stable toolchain

## Rust / Win32 rules

- No naked `.unwrap()`, `.expect()`, `panic!`, or `unreachable!` in runtime code. Tests may use them under `#[cfg(test)]`.
- Fallible OS and file work returns `Result<T, String>` or falls back gracefully — a missing file must never crash the daemon.
- Every thread that uses Shell/COM initializes COM with `CoInitializeEx` via a `ComGuard` (RAII) and leaves it cleaned up on drop.
- Icon extraction uses `IShellItemImageFactory` with `SIIGBF_BIGGERSIZEOK` at 256×256 and always frees the `HBITMAP` with `DeleteObject`.
- `main.rs` keeps `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
- Win32 code lives behind `#[cfg(windows)]`; non-Windows builds get stubs that compile.
- When the crate exists, the crate root carries `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`, with `#[cfg(test)]` allowances for tests. See `clippy.toml`.

## File size rule

Every `.rs`, `.ts`, and `.tsx` file must stay under **250 lines** (blank lines and comments count). Enforce with `node scripts/check-loc.mjs`.

## Persistence paths

All under the app data directory (`{app_data_dir}`):

| Path                                         | Purpose                                          |
| -------------------------------------------- | ------------------------------------------------ |
| `{app_data_dir}/app_state.json`              | App state; write atomically (temp file + rename) |
| `{app_data_dir}/cache/icons/{app_id}.png`    | Cached app icons                                 |
| `{app_data_dir}/cache/groups/{group_id}.ico` | Cached group composite icons                     |

## Workflow

- **One task = one branch = one PR.** Branch name: `task/<ID>-<slug>` (e.g. `task/S0-01-agents-md`).
- Before opening any PR the agent **stops**, pushes the branch, and posts a review summary covering: changes, edge cases, unverified items, and Windows smoke-test steps.
- The PR opens **only after Preston approves** that summary.
- CI on `windows-latest` must be green before merge.
- **Only Preston merges.** Agents never merge.

## UI / design

`DESIGN.md` is the visual source of truth once it exists. Design decisions come from a one-question-at-a-time interview with Preston. Agents never invent the aesthetic.

## Definition of done

When the toolchain exists, a change is done only after:

1. `node scripts/check-loc.mjs` exits 0
2. `cargo fmt --check` (in `src-tauri/`)
3. `cargo clippy -- -D warnings` (in `src-tauri/`)
4. `cargo test` (in `src-tauri/`)
5. `pnpm format:check`
6. `pnpm lint` / `pnpm typecheck` / frontend tests as configured
7. Windows smoke steps for the touched surface (flyout, Launchpad, tray) are documented in the review summary

Until those commands exist, at minimum run `node scripts/check-loc.mjs` and document anything left unverified.
