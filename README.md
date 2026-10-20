# Grouper

[![CI](https://github.com/prestonsokol1130/grouper/actions/workflows/ci.yml/badge.svg)](https://github.com/prestonsokol1130/grouper/actions/workflows/ci.yml)

Grouper puts app-group folders on your Windows 11 taskbar, like folders on an iPhone home screen. Click a group on the taskbar and its apps pop up above it. It also has a fullscreen Launchpad with every app, a search box and folders.

## Install

1. Download `Grouper_x.y.z_x64-setup.exe` from the [Releases page](https://github.com/prestonsokol1130/grouper/releases).
2. Open it. It installs for your account only, so it does not ask for an administrator password.
3. **If Windows shows "Windows protected your PC":** Grouper is not code-signed yet, so Microsoft SmartScreen does not recognise it. Click **More info**, then **Run anyway**.
4. Grouper starts in the system tray (the area by the clock). The first time, a welcome window opens.

Grouper needs Microsoft Edge WebView2, which Windows 11 already has. If it is missing, the installer downloads it.

## Make a group and pin it

1. Click the Grouper icon in the tray to open the Manager.
2. Click **New group**, give it a name and add apps.
3. Click **Show in Explorer**. A folder opens with the group's shortcut in it.
4. Right-click the shortcut. If you don't see **Pin to taskbar**, choose **Show more options** first.
5. Choose **Pin to taskbar**.

Windows does not let apps pin themselves, so this last step is up to you.

## Open the Launchpad

- Press **Alt + `** (the key above Tab). You can change this in Settings.
- Or move the mouse into the top-left or top-right corner of the screen. You can turn each corner off in Settings.
- Or right-click the tray icon and choose **Open Launchpad**.

## Uninstall

Open **Settings → Apps → Installed apps**, find **Grouper**, click **⋯** and choose **Uninstall**.

- Uninstalling removes Grouper's group shortcuts, unpins them from the taskbar and stops Grouper starting with Windows.
- Your groups and settings are kept, in case you install Grouper again. To remove them too, tick **Delete the application data** in the uninstaller.
- If a group icon is still on the taskbar afterwards, right-click it and choose **Unpin from taskbar**.

When you install a newer version over an old one, choose **Do not uninstall** if the installer asks. Uninstalling first unpins your groups.

## Development

```sh
pnpm install
pnpm tauri dev
```

The manager, Launchpad, and flyout windows start hidden. Stop the dev process from the terminal. Agent and contributor rules live in [AGENTS.md](./AGENTS.md). Releases and code signing are described in [docs/RELEASE.md](./docs/RELEASE.md).
