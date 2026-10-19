# Grouper threat model

Codex Security Review knowledge base. Path for settings: `docs/threat-model.md`.

## Overview

Grouper is a **local Windows 11 tray utility** (Tauri v2 + React 19). It does not expose a network service. Intended usage:

- Taskbar app-group folders: each group is a 2×2 composite `.ico` plus a `.lnk` with `AppUserModelID` `com.grouper.group.<group_id>` that launches `Grouper.exe --flyout <group_id>`.
- Fullscreen Launchpad: searchable app grid, pagination, drag-and-drop folders.
- Persistence and caches live under the app data directory only (`app_state.json`, icon PNGs, group ICOs). See `AGENTS.md`.

**Primary assets:** integrity of group/Launchpad layout; correct app identity when launching; confidentiality of ordinary local paths and icons (not secrets by design); availability of the tray daemon (a crash is a DoS of the product).

**Out of product scope:** multi-tenant SaaS, remote APIs, browsers as the primary UI, cryptography product features, and serving untrusted remote content.

## Threat Model, Trust Boundaries, and Assumptions

### Actors

| Actor                           | Trust          | Notes                                                                   |
| ------------------------------- | -------------- | ----------------------------------------------------------------------- |
| Local interactive user          | Mostly trusted | Owns the machine; can edit app data, shortcuts, and run other processes |
| Other local processes / malware | Untrusted      | May race or rewrite files Grouper reads                                 |
| Grouper frontend (WebView)      | Semi-trusted   | UI only; must not gain OS powers beyond allowlisted Tauri commands      |
| Grouper Rust backend            | Trusted core   | Owns Shell/COM, shortcuts, icons, process launch, persistence           |
| Windows Shell / COM             | Trusted OS     | Icon extraction (`IShellItemImageFactory`), shortcuts, taskbar          |

### Trust boundaries

1. **WebView ↔ Rust (Tauri IPC)** — Highest priority. Every command is an allowlisted entry point; inputs from the UI are attacker-controlled if XSS or a compromised renderer is assumed. Prefer typed payloads, path allowlists, and no shelling out on raw strings.
2. **CLI / activation** — `Grouper.exe --flyout <group_id>` and similar args come from shortcuts or other local launchers. Treat `group_id` as untrusted string input (validate format; never use as a raw filesystem path segment without sanitization).
3. **App data filesystem** — `{app_data_dir}/app_state.json` and caches may be rewritten by other local actors. Parse defensively; atomic writes; never follow unexpected symlinks outside the app data tree when writing.
4. **Taskbar `.lnk` / AUMID surface** — Shortcuts on the taskbar are part of the product contract. Spoofed or hand-edited `.lnk` files can point at unexpected targets; Grouper should only create/update its own AUMID namespace and should not trust arbitrary shortcut targets when reading back.
5. **Win32 / COM** — COM must be initialized via `ComGuard` on every Shell-using thread; bitmaps from icon extraction must always be freed with `DeleteObject`. Failures must return `Result` / graceful fallback — missing files must not panic the daemon.

### Assumptions

- Single-user desktop; no remote clients talk to Grouper over the network.
- No secrets store is required for v1 (no cloud tokens). If secrets appear later, this model must be updated.
- Non-Windows builds are stubs; production security review targets Windows.
- Design and agent process rules live in `AGENTS.md` / future `DESIGN.md`; they are not substitutes for IPC and path checks in code.

## Attack Surface, Mitigations, and Attacker Stories

### Attack surfaces (priority order)

1. **Tauri commands / events** — launch app, mutate groups, read/write state, open paths, extract icons.
2. **CLI flags** — `--flyout` and any future flags.
3. **Persistence** — JSON schema confusion, path traversal via `app_id` / `group_id`, TOCTOU on cache files.
4. **Shortcut + icon pipeline** — writing `.lnk`/`.ico`, reading Shell items, oversized or malformed icon data.
5. **Process spawn** — launching third-party apps the user selected; must not invent executable paths from free-form UI strings without resolution through known app records.

### Relevant vulnerability classes

- Path traversal / arbitrary file write under or beyond app data
- Command injection / unintended process execution via IPC or CLI
- XSS in the WebView escalating to privileged Tauri commands
- Insecure deserialization of `app_state.json`
- Resource leaks / panic paths in COM and GDI (availability)
- Shortcut or AUMID confusion (wrong app launched, phishing-style local UI)

### Less relevant / out of scope (calibrate down)

- Classic web SSRF, CSRF against remote sites, tenant isolation bugs
- Network authn bypass (no remote auth in product)
- Supply-chain issues in dependencies still matter at build time but are not product runtime entry points

### Attacker stories (in scope)

- Malware or a shared-machine peer rewrites `app_state.json` or a cache path so Grouper launches a different binary than the UI shows.
- A compromised or XSS’d WebView calls a Tauri command with `../../../` style ids or absolute paths.
- A crafted `--flyout` argument or group id causes a panic, hang, or write outside app data.
- Icon extraction on a hostile Shell item exhausts handles or crashes the process if `HBITMAP` is not freed or COM is initialized incorrectly.

### Attacker stories (out of scope / unrealistic)

- Unauthenticated internet attacker exploiting Grouper without already having code execution or a local foothold on the machine.
- Cross-tenant data theft (no tenants).

### Expected mitigations (as the codebase grows)

- Deny unwrap/expect/panic in runtime Rust (`clippy` denials in crate root per `AGENTS.md`)
- Atomic writes for `app_state.json`
- `#[cfg(windows)]` Win32 with safe stubs elsewhere
- Strict allowlisting of Tauri commands and validated ids (`group_id`, `app_id`)
- No raw `cmd.exe` / PowerShell with concatenated user strings

## Severity Calibration (Critical, High, Medium, Low)

**Critical** — Local code execution as the user _beyond_ launching apps they already chose, or silent replacement of launch targets so the UI shows App A but runs App B without further interaction. Example: IPC accepts an arbitrary executable path and spawns it; or group state points at attacker-controlled PE while the displayed name/icon stays trusted.

**High** — Arbitrary file write/delete outside `{app_data_dir}`; path traversal via ids; WebView XSS that can invoke privileged commands; persistent shortcut that launches attacker PE under Grouper’s AUMID branding.

**Medium** — Crash/hang DoS of the tray daemon via malformed state, COM misuse, or unbounded icon work; integrity loss confined to caches that is obvious after restart; information disclosure of local paths the user could already see via Explorer.

**Low** — Cosmetic state corruption recoverable by deleting app data; missing-file errors that are handled and logged; failures that only affect a single icon cache entry.

When scoring findings, prefer **High+** for IPC/path/launch correctness; prefer **Medium/Low** for availability-only COM/GDI mistakes that cannot redirect execution.
