# Grouper QA matrix

A checklist for testing Grouper by hand on real setups. Work through a
table, tick **Pass** or **Fail** in each row, and write what you saw in
**Notes** when something is off. Rows you cannot set up (for example a
second monitor you do not have) can be marked **Skip**.

Use a release build (`pnpm tauri build`), not `pnpm tauri dev`: dev builds
are much slower and their timings mean nothing.

## Before you start

- **Have a test group.** In the Manager, make a group with 6 apps and pin
  its shortcut to the taskbar. Most rows below click it.
- **Where the log is:** `%LOCALAPPDATA%\com.grouper.app\logs\grouper.log`.
  Timings are written there (search for `_us=` and `_ms=`).
- **Change one thing at a time.** After changing display scale, taskbar
  position or monitors, wait a few seconds before testing. If Windows asks
  you to sign out for a scale change, do it.
- **How to change things in Windows 11:**
  - Display scale: Settings > System > Display > Scale.
  - Taskbar auto-hide: Settings > Personalization > Taskbar > Taskbar
    behaviors > Automatically hide the taskbar.
  - Light or dark: Settings > Personalization > Colors > Choose your mode.
  - Reduced motion: Settings > Accessibility > Visual effects > Animation
    effects (off means reduced motion).
  - Taskbar on top, left or right: Windows 11 itself only puts the taskbar
    at the bottom. Moving it needs a tool such as ExplorerPatcher or
    StartAllBack. Only test those rows if you already use one.

## Budgets

These are the speed targets. Check them once per setup you care about,
with the release build.

| Budget                                 | Target                                    | How to check                                                                             |
| -------------------------------------- | ----------------------------------------- | ---------------------------------------------------------------------------------------- |
| Idle CPU                               | About 0%                                  | Task Manager > Details, Grouper and its `msedgewebview2.exe` children, after 5 quiet min |
| Idle memory                            | As low as practical (about 250 MB in all) | Same place, "Memory (active private working set)", Grouper plus its WebView2 processes   |
| Flyout: click to window visible        | Under 16 ms                               | `visible_us=` on the `flyout_show_us` lines (each show writes one)                       |
| Flyout: first paint                    | Under 50 ms                               | `flyout_paint_ms=` lines                                                                 |
| Launchpad with 500 apps: open to paint | Under 100 ms                              | `launchpad_show_us=` plus the `launchpad_paint_ms=` line after it                        |
| Search: keystroke to results on screen | Under 16 ms                               | Type in the Launchpad search; results should keep up with fast typing                    |

## Results from S6-04

Measured on Preston's PC (24 cores, 4K monitor at 300%) with release
builds, using a separate copy of the state (186 apps, or 500 apps for the
Launchpad and search rows). p50 is the typical value, p95 the slow end.

| Budget                                     | Before            | After                         | Met?                         |
| ------------------------------------------ | ----------------- | ----------------------------- | ---------------------------- |
| Idle CPU, Grouper + WebView2, 5 min        | 0.23% of one core | 0.11% of one core             | Yes (about 0%)               |
| Idle wake-ups of grouper.exe threads       | about 22 a second | about 22 (0 with corners off) | See "Why hot corners poll"   |
| Idle memory, Grouper + WebView2            | 250 MB            | 250 to 330 MB                 | No change (see below)        |
| Flyout click to visible, p50 / p95         | 22.3 / 51.5 ms    | 15.7 / 20.5 ms                | p50 yes, p95 no              |
| Flyout first paint, p50 / p95              | 1.5 / 3.5 ms      | 1.5 / 8.1 ms                  | Yes                          |
| Launchpad, 500 apps, show, p50 / p95       | 23.1 / 82.6 ms    | 19.9 / 77.7 ms                | Yes, with the paint below    |
| Launchpad, 500 apps, first paint p50 / p95 | 5.2 / 19.6 ms     | 4.3 / 18.8 ms                 | Yes (show + paint under 100) |
| Search, 500 apps, keystroke to frame       | 12.6 / 25.5 ms    | 3.9 / 10.7 ms                 | Yes                          |
| Search, keystroke to painted, p50 / p95    | 20.2 / 54.3 ms    | 5.3 / 18.6 ms                 | p50 yes, p95 just over       |
| Search for "e" (376 results), painted      | about 48 ms       | 12 to 23 ms                   | Mostly                       |

What changed:

- **Flyout:** placing it is now one Windows call (none when it is already
  in place) instead of three. What is left, about 15 ms, is Windows
  showing and activating the window itself (Tauri's `show`).
- **Launchpad:** the same one-call placement, and opening it no longer
  copies every app in memory.
- **Search:** the first screenful of results is drawn with the keystroke
  and the rest right after, so a broad search no longer holds up typing.
- **Memory:** each window now loads only its own code. The total barely
  moved because most of it is WebView2's own graphics process (100 to
  200 MB, and it changes from run to run); Grouper itself is about 46 MB.
- **Idle:** the icon cache watch no longer wakes every 3 seconds; it
  sleeps until something in the app data folder changes. The hot-corner
  watcher sleeps completely while both corners are off.

### Why hot corners still poll

With a corner on, a background thread checks where the mouse is 20 times
a second. Measured cost: about 0.04% of one CPU core (156 ms of CPU in 7
minutes). The alternative, a low-level mouse hook, would wake Grouper on
every mouse movement in Windows (125 to 1000 times a second while the
mouse moves), put Grouper in the path of every mouse move (if Grouper is
slow, the pointer lags, and Windows silently removes a hook that is too
slow), and still needs a timer, because a mouse resting in a corner sends
no events. So polling stays; turning both corners off makes it free.

## The matrix

Every table has the same columns: **ID**, **Setup**, **Steps**,
**Expected**, **Pass / Fail**, **Notes**.

### A. Monitors

| ID  | Setup                                      | Steps                                                                                             | Expected                                                                                                    | Pass / Fail       | Notes |
| --- | ------------------------------------------ | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ----------------- | ----- |
| A1  | One monitor                                | Click the group's taskbar button. Click it again. Open the Launchpad with the shortcut.           | Flyout opens just above the button, then closes on the second click. Launchpad fills the screen.            | [ ] Pass [ ] Fail |       |
| A2  | Two monitors, both at 100%                 | Pin the group on the taskbar of each monitor (or show taskbar on all displays). Click it on each. | Flyout opens above the button you clicked, on that monitor, never on the other one.                         | [ ] Pass [ ] Fail |       |
| A3  | Two monitors, main at 100%, second at 150% | Click the group on the 100% monitor, then on the 150% one, then back. Do it 5 times.              | Same size on screen each time (bigger pixels on 150%), sharp text, right above the button, never stretched. | [ ] Pass [ ] Fail |       |
| A4  | Two monitors, main at 150%, second at 100% | Same as A3.                                                                                       | Same as A3.                                                                                                 | [ ] Pass [ ] Fail |       |
| A5  | Two monitors, mixed DPI                    | Put the mouse on each monitor in turn and open the Launchpad with the shortcut.                   | It covers exactly the monitor the mouse is on, edge to edge, no gap and no overlap onto the other one.      | [ ] Pass [ ] Fail |       |
| A6  | Two monitors, mixed DPI, hot corners on    | Rest the mouse in the top-left corner of each monitor, then the top-right.                        | The Launchpad opens on that monitor after about a quarter second, from that corner.                         | [ ] Pass [ ] Fail |       |
| A7  | Two monitors                               | Open the flyout, then unplug or turn off the monitor it is on.                                    | The flyout closes. Clicking the group again opens it on a monitor that is still there.                      | [ ] Pass [ ] Fail |       |

### B. Display scale (one monitor)

Repeat each row at 100%, 125%, 150% and 200%. Write the scale in Notes.

| ID  | Setup                    | Steps                                                                   | Expected                                                                                            | Pass / Fail       | Notes |
| --- | ------------------------ | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | ----------------- | ----- |
| B1  | Scale 100, 125, 150, 200 | Click the group's taskbar button.                                       | Flyout opens above the button with a 12px gap, sharp icons and text, nothing cut off.               | [ ] Pass [ ] Fail |       |
| B2  | Same                     | Make a group with 12 or more apps and open its flyout.                  | Three columns; the flyout grows upward and scrolls if it would not fit, never covers the taskbar.   | [ ] Pass [ ] Fail |       |
| B3  | Same                     | Open the Launchpad.                                                     | Fills the screen; the grid has the same number of columns and rows as the setting; icons are sharp. | [ ] Pass [ ] Fail |       |
| B4  | Same                     | Open the Manager.                                                       | Opens at a sensible size, text sharp, nothing overlapping.                                          | [ ] Pass [ ] Fail |       |
| B5  | Same                     | Open the flyout 10 times, then read the last 10 `flyout_show_us` lines. | `visible_us` under 16000 on most lines. Write the highest in Notes.                                 | [ ] Pass [ ] Fail |       |

### C. Taskbar position

Windows 11 only puts the taskbar at the bottom. Test top, left and right
only if you use ExplorerPatcher, StartAllBack or a similar tool.

| ID  | Setup                       | Steps                                  | Expected                                                                            | Pass / Fail       | Notes |
| --- | --------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------- | ----------------- | ----- |
| C1  | Taskbar at the bottom       | Click the group's taskbar button.      | Flyout opens above the button and grows out of it.                                  | [ ] Pass [ ] Fail |       |
| C2  | Taskbar at the top (tool)   | Same.                                  | Flyout opens below the button and grows downward from it.                           | [ ] Pass [ ] Fail |       |
| C3  | Taskbar on the left (tool)  | Same.                                  | Flyout opens to the right of the button, lined up with it.                          | [ ] Pass [ ] Fail |       |
| C4  | Taskbar on the right (tool) | Same.                                  | Flyout opens to the left of the button, lined up with it.                           | [ ] Pass [ ] Fail |       |
| C5  | Any position                | Open the Launchpad, then press Escape. | Launchpad covers the whole screen including over the taskbar, and closes on Escape. | [ ] Pass [ ] Fail |       |

### D. Taskbar auto-hide

| ID  | Setup             | Steps                                                                   | Expected                                                                                     | Pass / Fail       | Notes |
| --- | ----------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------- | ----- |
| D1  | Auto-hide off     | Click the group's taskbar button.                                       | Flyout opens above the taskbar with a 12px gap.                                              | [ ] Pass [ ] Fail |       |
| D2  | Auto-hide on      | Move the mouse to the screen edge so the taskbar slides in, then click. | Flyout opens clear of the taskbar (not covered by it) and stays open when the taskbar hides. | [ ] Pass [ ] Fail |       |
| D3  | Auto-hide on      | Open the Launchpad with the shortcut.                                   | Covers the whole screen; the taskbar does not slide over it.                                 | [ ] Pass [ ] Fail |       |
| D4  | Switch while open | Open the flyout, then turn auto-hide on or off in Settings.             | Nothing breaks; the next click places the flyout correctly for the new setting.              | [ ] Pass [ ] Fail |       |

### E. Light and dark

| ID  | Setup                | Steps                                                         | Expected                                                     | Pass / Fail       | Notes |
| --- | -------------------- | ------------------------------------------------------------- | ------------------------------------------------------------ | ----------------- | ----- |
| E1  | Light mode           | Open the flyout, the Launchpad and the Manager.               | All three are light, text readable, focus rings visible.     | [ ] Pass [ ] Fail |       |
| E2  | Dark mode            | Same.                                                         | All three are dark, text readable, focus rings visible.      | [ ] Pass [ ] Fail |       |
| E3  | Switch while running | With Grouper running, switch light to dark, then open each.   | Each window follows the new mode without restarting Grouper. | [ ] Pass [ ] Fail |       |
| E4  | Change accent colour | Pick another accent colour in Windows, then open the Manager. | Buttons and highlights use the new accent.                   | [ ] Pass [ ] Fail |       |

### F. Reduced motion

| ID  | Setup                 | Steps                                                            | Expected                                                               | Pass / Fail       | Notes |
| --- | --------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------- | ----------------- | ----- |
| F1  | Animation effects on  | Open and close the flyout and the Launchpad; turn a page.        | They grow from their button or corner and pages slide.                 | [ ] Pass [ ] Fail |       |
| F2  | Animation effects off | Same.                                                            | Everything fades in about a tenth of a second instead; nothing slides. | [ ] Pass [ ] Fail |       |
| F3  | Switch while running  | Turn animation effects off with Grouper running, then repeat F2. | The fade is used at once, without restarting Grouper.                  | [ ] Pass [ ] Fail |       |

### G. Combined spot checks

A few mixes that catch problems the single rows miss.

| ID  | Setup                                    | Steps                                | Expected                                              | Pass / Fail       | Notes |
| --- | ---------------------------------------- | ------------------------------------ | ----------------------------------------------------- | ----------------- | ----- |
| G1  | 150%, dark, reduced motion on            | Open the flyout, then the Launchpad. | Correct place, dark, fades only.                      | [ ] Pass [ ] Fail |       |
| G2  | Two monitors 100% and 150%, auto-hide on | Open the flyout on each monitor.     | Correct monitor, clear of the taskbar, sharp.         | [ ] Pass [ ] Fail |       |
| G3  | 200%, taskbar on top (tool), light       | Open the flyout and the Launchpad.   | Flyout below the button, Launchpad covers the screen. | [ ] Pass [ ] Fail |       |

### H. Speed and idle

| ID  | Setup                         | Steps                                                                                       | Expected                                                                        | Pass / Fail       | Notes |
| --- | ----------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------- | ----- |
| H1  | Grouper running, nothing open | Leave the PC alone for 5 minutes, then look at Task Manager > Details.                      | Grouper and its `msedgewebview2.exe` processes show 0% CPU nearly all the time. | [ ] Pass [ ] Fail |       |
| H2  | Same                          | Add up "Memory (active private working set)" for Grouper and its WebView2 processes.        | About 250 MB or less. Write the total in Notes.                                 | [ ] Pass [ ] Fail |       |
| H3  | Hot corners off               | Turn both corners off in Settings, wait a minute, check H1 again.                           | Same as H1 (the corner watcher sleeps completely).                              | [ ] Pass [ ] Fail |       |
| H4  | Any                           | Open the flyout 10 times, then read the last 10 `flyout_show_us` lines.                     | `visible_us` under 16000 on most lines; `flyout_paint_ms` under 50.             | [ ] Pass [ ] Fail |       |
| H5  | Many apps                     | Open the Launchpad 5 times and read the `launchpad_show_us` and `launchpad_paint_ms` lines. | `launchpad_show_us` plus `launchpad_paint_ms` under 100 ms.                     | [ ] Pass [ ] Fail |       |
| H6  | Many apps                     | Type `e` in the Launchpad search, then keep typing a name quickly.                          | Results appear with each key, no visible lag.                                   | [ ] Pass [ ] Fail |       |

### I. Background watchers

| ID  | Setup                    | Steps                                                                                                         | Expected                                                                                               | Pass / Fail       | Notes |
| --- | ------------------------ | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ----------------- | ----- |
| I1  | Grouper running          | Delete the `cache` folder in `%APPDATA%\com.grouper.app`, wait 5 seconds, open the Launchpad.                 | The folder is back, the icons show again within a few seconds, and the log says the cache was rebuilt. | [ ] Pass [ ] Fail |       |
| I2  | Hot corners on           | Rest the mouse in the top-left corner.                                                                        | The Launchpad opens after about a quarter second.                                                      | [ ] Pass [ ] Fail |       |
| I3  | Hot corners off, then on | Turn both corners off, rest the mouse in a corner (nothing happens), turn top-left back on, rest there again. | Nothing while off; opens again at once after turning it back on, without restarting Grouper.           | [ ] Pass [ ] Fail |       |
