# Grouper design interview (S3-01)

Preston's answers, one question at a time. S3-02 turns these into DESIGN.md. Recorded by the "Grouper build plan" thread.

## Q1. Overall look

- Chosen: **Windows 11 native** (Fluent: Mica glass, Segoe font, rounded Fluent tiles that match the taskbar)
- Preston's words: tapped "Windows 11 native" on the card, 2026-10-19 22:35 UTC

## Q2. Light or dark mode

- Chosen: **Follow Windows** (switches with the Windows light/dark setting, like built-in apps)
- Preston's words: tapped "Follow Windows" on the card, 2026-10-20 00:27 UTC

## Q3. Accent color

- Chosen: **Windows accent by default, plus a setting to pick a custom accent color**
- Preston's words: "A But also an option to customize it." (2026-10-20 00:29 UTC)
- Note: the custom-accent setting is a new user-facing option; it belongs in the Manager's settings when those are built.

## Q4. Window material

- Chosen: **all three (Mica, Acrylic, Solid) offered as a user setting**. Default (Claude's pick, Preston can change it): Mica for the Manager, Acrylic for the flyout and Launchpad.
- Preston's words: "all 3 please" (2026-10-20 00:34 UTC), after seeing the Mica / Acrylic / Solid mockup

## Q5. Folder tile shape (also the 2x2 taskbar group icon)

- Chosen: **both Windows 11 rounded square (A) and iPhone squircle (B), as a user setting**. Default (Claude's pick): Windows 11 rounded square. Circle not wanted.
- Preston's words: "a & b" (2026-10-20 00:35 UTC)

## Q6. App icon size and labels

- Chosen: **Medium icons with the app name underneath, truncated with ... when too long**. Icon size is also a user setting (stated on the card).
- Preston's words: tapped "Medium, names" on the card, 2026-10-20 03:27 UTC

## Q7. Font

- Chosen: **Segoe UI Variable** (the Windows 11 system font; nothing bundled)
- Preston's words: tapped "Segoe UI Variable" on the card, 2026-10-20 03:28 UTC

## Q8 to Q16 (answered together on the "Grouper design picks" page, pasted 2026-10-20 03:40 UTC)

Preston's pasted answers, verbatim:

```
Q8 Launchpad grid: 7 x 5 (35 per page, about 6 pages for you) | setting: yes
Q9 Taskbar flyout: 3 across with names, no pointer
Q10 Animations: Windows smooth (fade and slide up, 200 ms) | note: i was hoping to have it6 like the mac window animation when you open and minimize a window on macshow me what that would look like
Q11 Launchpad search bar: Pill centered at the top
Q12 Manager window layout: Sidebar list of groups with the selected group's apps on the right
Q13 Hover and click feel: Lift and grow
Q14 Message tone: Friendly and short
Q15 Keyboard shortcut for the Launchpad: Alt + ` (the key left of 1) | setting: yes | note: i want to have hot activation corners for this too like upper right and left corners mouse over triggers it
Q16 Anything else: nothing
```

Summary:

- Q8 Launchpad grid: 7 x 5 per page by default, and grid size is a user setting.
- Q9 Flyout: 3 across with app names, no pointer arrow.
- Q10 Motion: **Mac genie by default** (flyout pours out of / back into the group's taskbar icon; windows genie in and out), **with a setting to switch to Mac scale or Windows smooth** (fade and slide up, 200 ms). Windows' reduce-animations setting tones it down. Preston tapped "Genie + setting" after the demo, 2026-10-20 03:42 UTC. (Superseded his page pick of Windows smooth.)
- Q11 Search: pill centered at the top of the Launchpad.
- Q12 Manager: sidebar list of groups, selected group's apps on the right.
- Q13 Hover/click: lift and grow (icon rises with a shadow, presses down on click). Keyboard focus keeps a Windows-style ring.
- Q14 Tone: friendly and short.
- Q15 Launchpad shortcut: Alt + ` by default, changeable in settings. **New feature request:** hot corners (moving the mouse into the top-left or top-right screen corner opens the Launchpad).
- Q16: nothing else.
