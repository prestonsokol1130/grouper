# Grouper design system

The visual source of truth for Grouper, a Windows 11 tray app that puts iOS-style app-group folders on the taskbar and adds a Mac-style fullscreen Launchpad. Every decision here comes from Preston's design interview (2026-10-19/20, full answers in `docs/design-interview.md`). Where a value is marked _proposed_, Preston chose the direction and this file fills in the exact number; change it here, in one place.

Agents never invent aesthetic choices outside this file. If something isn't covered, ask Preston.

---

## 1. Direction

- **Overall look:** native Windows 11 (Fluent). Grouper should feel built into Windows, with three Mac touches Preston asked for: the Launchpad itself, the genie animation, and lift-and-grow hover.
- **Theme:** follows the Windows light/dark setting live, like built-in apps.
- **Customization is a feature.** Many choices below are user settings with a default. Every setting lives in the Manager's Settings page (see section 9).

## 2. Surfaces

| Surface            | What it is                                                               | Material (default) |
| ------------------ | ------------------------------------------------------------------------ | ------------------ |
| Taskbar group icon | 2x2 composite `.ico` of a group's first four apps, pinned to the taskbar | n/a (image)        |
| Flyout             | Small panel that pops up above the taskbar when a group icon is clicked  | Acrylic            |
| Launchpad          | Fullscreen overlay: search, paged app grid, drag-and-drop folders        | Acrylic            |
| Manager            | Normal window to create and edit groups, plus Settings                   | Mica               |
| Tray icon and menu | Grouper's own icon in the notification area                              | system menu        |

## 3. Color

Accent defaults to the **Windows accent color** (read from the system and updated live). **Setting:** "Accent color" with "Use Windows accent" (default) or a custom color picker.

Fallback accent when the system color can't be read: light `#0067C0`, dark `#4CC2FF` (the Windows default blue).

Neutral tokens follow Fluent 2. Names are the CSS custom properties the frontend uses.

| Token                   | Light                    | Dark                      | Use                                      |
| ----------------------- | ------------------------ | ------------------------- | ---------------------------------------- |
| `--text-primary`        | `rgba(0,0,0,0.896)`      | `#FFFFFF`                 | main text                                |
| `--text-secondary`      | `rgba(0,0,0,0.606)`      | `rgba(255,255,255,0.786)` | captions, app labels on hover-off states |
| `--text-disabled`       | `rgba(0,0,0,0.361)`      | `rgba(255,255,255,0.363)` | disabled                                 |
| `--fill-subtle-hover`   | `rgba(0,0,0,0.037)`      | `rgba(255,255,255,0.061)` | list row hover                           |
| `--fill-subtle-pressed` | `rgba(0,0,0,0.024)`      | `rgba(255,255,255,0.042)` | list row pressed                         |
| `--card-bg`             | `rgba(255,255,255,0.70)` | `rgba(255,255,255,0.051)` | cards on Mica                            |
| `--stroke-card`         | `rgba(0,0,0,0.058)`      | `rgba(0,0,0,0.10)`        | card and panel borders                   |
| `--stroke-surface`      | `rgba(117,117,117,0.40)` | `rgba(117,117,117,0.40)`  | flyout and Launchpad outer edge          |
| `--solid-bg`            | `#F3F3F3`                | `#202020`                 | Solid material and fallbacks             |
| `--layer-bg`            | `#FBFBFB`                | `#2C2C2C`                 | Manager content area                     |
| `--danger`              | `#C42B1C`                | `#FF99A4`                 | errors                                   |
| `--success`             | `#0F7B0F`                | `#6CCB5F`                 | confirmations                            |

Accent shades (`--accent`, `--accent-hover`, `--accent-pressed`, `--text-on-accent`) are derived from the accent at runtime: hover = 90% opacity, pressed = 80% opacity, text on accent is black or white, whichever has the higher contrast.

## 4. Materials

**Setting:** "Window background" with Mixed (default), Mica, Acrylic or Solid. Mixed is the per-surface default in the table below. Mica, Acrylic or Solid override every surface at once, and choosing Mixed again goes back to the per-surface default.

| Surface   | Default | Notes                                                                                          |
| --------- | ------- | ---------------------------------------------------------------------------------------------- |
| Manager   | Mica    | Tinted by the wallpaper, never shows windows behind                                            |
| Flyout    | Acrylic | Frosted glass; blur radius 30px, tint opacity 0.8 light / 0.7 dark (_proposed_)                |
| Launchpad | Acrylic | Same recipe, tint opacity 0.6 light / 0.55 dark so the desktop shows through more (_proposed_) |

Fallbacks: if the system has transparency effects off, or battery saver is on, everything uses Solid (`--solid-bg`). Windows 10 (no Mica) uses Acrylic for the Manager.

## 5. Shape

Fluent radii: **4px** for controls (buttons, inputs, list rows), **8px** for windows, flyout and cards.

**Folder tile shape (also the taskbar group icon)** is a setting:

- **Windows square (default):** rounded square, corner radius = 18% of tile size (_proposed_; ~46px at 256px).
- **iPhone squircle:** superellipse, exponent n = 5.

Folder tile recipe (used by the Launchpad folder tile and the 2x2 taskbar `.ico`):

- Canvas 256x256. Padding 14% of size. Gap between mini icons 6%.
- Background: light `rgba(255,255,255,0.72)`, dark `rgba(255,255,255,0.14)` in the UI. The taskbar `.ico` can't follow the theme, so it uses a neutral `rgba(235,235,240,0.92)` with a 1px `rgba(0,0,0,0.12)` inner border so it reads on both light and dark taskbars (_proposed_).
- Mini icons: up to four, corner radius 22% of their own size. Layout for 1, 2 and 3 apps as specified in ticket S2-01.

## 6. Typography

**Segoe UI Variable** (the Windows 11 system font; nothing bundled). Styles at 14px and below use the Text optical size: `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`. Styles above 14px (Subtitle, Title, Title Large) use the Display optical size: `"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif`.

Fluent type ramp:

| Style       | Size / line height | Weight   | Use                                |
| ----------- | ------------------ | -------- | ---------------------------------- |
| Caption     | 12 / 16            | Regular  | app labels, hints                  |
| Body        | 14 / 20            | Regular  | default                            |
| Body Strong | 14 / 20            | Semibold | list titles, buttons               |
| Subtitle    | 20 / 28            | Semibold | flyout group name, section headers |
| Title       | 28 / 36            | Semibold | Manager page titles                |
| Title Large | 40 / 52            | Semibold | Launchpad empty state              |

## 7. Icons and labels

- **App icons:** medium by default. **Setting:** "Icon size" Small / Medium / Large.

| Size             | Launchpad icon | Flyout icon |
| ---------------- | -------------- | ----------- |
| Small            | 56px           | 40px        |
| Medium (default) | 72px           | 48px        |
| Large            | 96px           | 64px        |

- **Labels:** app name under every icon, Caption style, one line, cut off with "…" when too long, full name in a tooltip.

## 8. Layout

### Launchpad

- **Grid:** 7 across x 5 down (35 apps per page) by default. **Setting:** "Launchpad grid" (columns and rows). Bigger screens get more spacing, not more apps.
- **Search:** pill centered at the top, width 320px (_proposed_), height 36px, radius 18px. Typing anywhere focuses it.
- **Pages:** dots centered at the bottom; the current page dot uses the accent.
- **Folders:** dragging one app onto another makes a folder (iOS-style); folder tiles use section 5.

### Flyout

- 3 apps across with names, grows downward in rows; no pointer arrow.
- Sits 12px off the taskbar edge: above a bottom taskbar, below a top one, to the right of a left one and to the left of a right one. It is centered on the group's taskbar icon along the taskbar and kept inside the work area.
- Padding 12px, gap 8px, group name in Subtitle at the top.

### Manager

- Sidebar on the left (groups list, 280px wide, _proposed_), the selected group's apps on the right as a grid of tiles with "Add apps" and "Remove" actions.
- Settings is its own entry at the bottom of the sidebar, like Windows Settings.

## 9. Settings (from the interview)

| Setting            | Options                                 | Default                       |
| ------------------ | --------------------------------------- | ----------------------------- |
| Accent color       | Windows accent, custom color            | Windows accent                |
| Window background  | Mixed (section 4), Mica, Acrylic, Solid | Mixed                         |
| Folder shape       | Windows square, iPhone squircle         | Windows square                |
| Icon size          | Small, Medium, Large                    | Medium                        |
| Launchpad grid     | columns x rows                          | 7 x 5                         |
| Animation style    | Genie, Scale, Windows smooth            | Genie                         |
| Launchpad shortcut | any key combo                           | Alt + `                       |
| Hot corners        | top-left, top-right (each on/off)       | _not decided_, see section 12 |

Light/dark is not a Grouper setting; it follows Windows.

## 10. Motion

**Setting:** "Animation style" with Genie (default), Scale or Windows smooth.

| Style           | Open                                                                                    | Close                                          | Duration                       |
| --------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------- | ------------------------------ |
| Genie (default) | Window pours out of its origin point and un-pinches into shape (Mac minimize, reversed) | Squeezes and gets sucked into the origin point | 500ms (_proposed_)             |
| Scale           | Grows from the origin point while fading in                                             | Shrinks back into it                           | 300ms                          |
| Windows smooth  | Fades in and slides up 12px                                                             | Fades out and slides down                      | 200ms, `cubic-bezier(0,0,0,1)` |

Origin points: the flyout's origin is its group's taskbar icon; the Launchpad's origin is the screen centre bottom when opened by hotkey, or the hot corner when opened by a corner; the Manager's origin is the tray icon.

**Reduced motion:** when Windows "Animation effects" is off, every style becomes a 120ms fade.

Other motion (_proposed_): page swipe in the Launchpad 250ms ease-out; folder open uses the same style as the flyout; drag lifts the dragged icon with the hover lift and shows a 1.15 scale on the drop target.

## 11. Interaction states

- **Hover:** lift and grow. The icon rises 3px and scales to 1.08 with a soft shadow (`0 8px 16px rgba(0,0,0,0.25)`), 150ms ease-out.
- **Pressed:** scales to 0.95, 80ms.
- **Keyboard focus:** Windows focus ring, 2px outer `--text-primary` plus 1px inner contrasting ring, 3px offset, always visible on keyboard focus (not on mouse click).
- **Selected (Manager sidebar):** `--fill-subtle-hover` background plus a 3px accent pill on the left edge, like Windows Settings.
- Lists and menus use the Fluent subtle fill on hover instead of lift.

## 12. Launchpad shortcut and hot corners

- Default shortcut **Alt + `** (the key left of 1). Alt+Space is not used because Windows and PowerToys already take it. Changeable in settings.
- **Hot corners:** moving the mouse into the top-left or top-right corner of the screen opens the Launchpad. A setting turns each corner on or off. _Open question for Preston:_ should corners be on or off by default, and how long should the mouse rest in the corner before it triggers (proposed 250ms)?

## 13. Voice and messages

Friendly and short. Say what happened and what to do next, in plain words, no jargon or error codes in the main text (codes go to the log).

- Empty group: "No apps in this group yet. Drag apps here or click Add apps."
- App missing: "Couldn't open Spotify. It may have been moved or uninstalled. Click to find it."
- Shortcut taken: "Alt + ` is already used by another app. Pick a different shortcut in Settings."

Toasts appear at the bottom-right above the taskbar, Fluent InfoBar style, and close by themselves after 5 seconds unless they need an action.

---

## 14. App icon

**Chosen: the Diamond Tile** (Preston approved it on 2026-10-20). Three blue tiles in a 2x2 grid, with the fourth tile orange and turned 45° into a diamond in its own slot. The files are in `src-tauri/icons/`.

Grouper's own icon uses fixed brand colours and the diamond, while a user's group icon shows the group's real app icons, so the two stay easy to tell apart.

- `icon.ico` has 16, 20, 24, 32, 40, 48, 64 and 256px frames; 16, 20 and 24 are hand-tuned on the pixel grid.
- Colours: blue `#5ACBFF`→`#0E6DD2`, deep blue `#3A9CF2`→`#0A55B3`, sky `#A8E2FF`→`#3FA7F0`, orange `#FFC766`→`#F7682A`.
- Changing the icon needs Preston's approval, like every other choice in this file.
