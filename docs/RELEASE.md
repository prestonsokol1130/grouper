# Releasing Grouper

## What the installer is

`pnpm tauri build` makes one NSIS installer, `src-tauri/target/release/bundle/nsis/Grouper_<version>_x64-setup.exe`.

- **Per-user install.** It installs into `%LOCALAPPDATA%\Grouper` and never asks for admin rights (`bundle.windows.nsis.installMode: "currentUser"`).
- **Version.** `tauri.conf.json` reads it from `package.json` (`"version": "../package.json"`). Bump it there.
- **WebView2: `downloadBootstrapper`.** Windows 11 ships the WebView2 runtime, so the bootstrapper almost never runs. Embedding it (`embedBootstrapper`) would add about 1.8 MB to every download to help only a machine where someone removed WebView2, and that machine can still download it. The download runs silently.
- **Uninstall hook.** `src-tauri/windows/hooks.nsh` runs `grouper.exe --cleanup` before files are removed. Cleanup unpins and deletes the group shortcuts in `Start Menu\Programs\Grouper`, deletes that folder and removes the `Grouper` value from `HKCU\...\CurrentVersion\Run` (plus its Task Manager flag under `StartupApproved\Run`), but only when the value points at a `grouper.exe`. When the user ticks the uninstaller's own "Delete the application data" box it also passes `--delete-data`, which deletes `%APPDATA%\com.grouper.app`. Errors go to `grouper.log`; cleanup always exits 0 so it never blocks an uninstall. An update (`/UPDATE`) skips cleanup.

## Pinned groups

Windows does not unpin a taskbar item when its shortcut or program is removed; the pin stays with a blank icon. Cleanup therefore calls `IStartMenuPinnedList::RemoveFromList` on each group shortcut before deleting it, the same call the Tauri uninstaller makes for Grouper's own shortcut. If a pin survives anyway, the README tells the user to right-click it and choose **Unpin from taskbar**.

## Cutting a release

1. Bump `version` in `package.json` and merge that to `main`.
2. Tag the merge commit `v<version>` (for example `v0.2.0`) and push the tag. Only Preston does this.
3. `.github/workflows/release.yml` runs:
   - the full CI suite (`ci.yml`, called as a reusable workflow),
   - a check that the tag matches `package.json`,
   - `pnpm tauri build` on `windows-latest`,
   - `gh release create --draft --generate-notes` with the installer attached.
4. Open the draft on the Releases page, read the notes and press **Publish**.

Only the release job has `contents: write`, through the built-in `GITHUB_TOKEN`. The workflow never publishes on its own.

## Adding code signing later

The installer is unsigned, so SmartScreen warns on first run until it builds reputation. Signing removes most of that warning. **A code-signing certificate costs money** (roughly a few hundred US dollars a year for an OV or EV certificate, or a monthly fee for a cloud service such as Azure Trusted Signing). Nothing below is set up yet.

With a `.pfx` certificate:

1. Add two repository secrets (Settings → Secrets and variables → Actions):
   - `WINDOWS_CERTIFICATE`: the `.pfx` file, base64-encoded.
   - `WINDOWS_CERTIFICATE_PASSWORD`: the `.pfx` password.
2. In `release.yml`, before **Build the installer**, add a step that decodes `WINDOWS_CERTIFICATE` to a temporary `.pfx` and imports it into `Cert:\CurrentUser\My` with `Import-PfxCertificate`, using `WINDOWS_CERTIFICATE_PASSWORD`.
3. In `src-tauri/tauri.conf.json`, under `bundle.windows`, set `certificateThumbprint` to the certificate's thumbprint, `digestAlgorithm` to `"sha256"` and `timestampUrl` to your certificate vendor's timestamp server. Tauri then signs `grouper.exe` and the installer during `pnpm tauri build`.

With a cloud signing service, use `bundle.windows.signCommand` instead and store that service's credentials as secrets. Never commit the certificate or its password to the repository.
