//! Which shortcuts the Launchpad may use. The Settings recorder applies the
//! same rules (`src/windows/manager/settings/hotkeyCombo.ts`), so a refused
//! shortcut here is only a backstop.
//!
//! A shortcut needs Ctrl, Alt or Win; Shift alone is enough only with a
//! function key, since Shift plus a letter is just typing. Shortcuts
//! Windows keeps for itself (Win+L, Alt+Tab and the like) are refused.

use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

/// Shortcuts Windows uses itself: Grouper could never receive them, or
/// would take them from the system.
const RESERVED: [(Modifiers, Code); 25] = {
    const WIN: Modifiers = Modifiers::SUPER;
    const ALT: Modifiers = Modifiers::ALT;
    const CTRL: Modifiers = Modifiers::CONTROL;
    [
        (WIN, Code::KeyL),
        (WIN, Code::KeyD),
        (WIN, Code::KeyE),
        (WIN, Code::KeyR),
        (WIN, Code::KeyI),
        (WIN, Code::KeyA),
        (WIN, Code::KeyS),
        (WIN, Code::KeyX),
        (WIN, Code::KeyM),
        (WIN, Code::KeyV),
        (WIN, Code::KeyP),
        (WIN, Code::KeyK),
        (WIN, Code::KeyH),
        (WIN, Code::KeyN),
        (WIN, Code::KeyW),
        (WIN, Code::Tab),
        (WIN, Code::Space),
        (WIN, Code::Period),
        (ALT, Code::Tab),
        (ALT, Code::F4),
        (ALT, Code::Escape),
        (ALT, Code::Space),
        (CTRL, Code::Escape),
        (CTRL.union(Modifiers::SHIFT), Code::Escape),
        (CTRL.union(ALT), Code::Delete),
    ]
};

/// F1 to F24.
const FUNCTION_KEYS: [Code; 24] = {
    use Code::*;
    [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ]
};

/// `Ok` when `shortcut` may be the Launchpad shortcut; otherwise why not,
/// for the log.
pub fn check(shortcut: &Shortcut) -> Result<(), String> {
    let strong = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
    let shift_f_key =
        shortcut.mods.contains(Modifiers::SHIFT) && FUNCTION_KEYS.contains(&shortcut.key);
    if !shortcut.mods.intersects(strong) && !shift_f_key {
        return Err(format!("{shortcut} needs Ctrl, Alt or Win"));
    }
    if RESERVED.contains(&(shortcut.mods, shortcut.key)) {
        return Err(format!("{shortcut} is reserved by Windows"));
    }
    Ok(())
}

/// `hotkey` the way DESIGN.md writes it: `Alt+\`` becomes `Alt + \``,
/// and `Super` is called `Win`.
pub fn display(hotkey: &str) -> String {
    hotkey
        .split('+')
        .map(|part| match part.trim() {
            "Super" => "Win",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

#[cfg(test)]
mod tests {
    use super::{check, display};
    use crate::hotkey::parse;

    fn allowed(hotkey: &str) -> bool {
        check(&parse(hotkey).unwrap()).is_ok()
    }

    #[test]
    fn common_shortcuts_are_allowed() {
        for hotkey in [
            "Alt+`",
            "Shift+Alt+F12",
            "Ctrl+Alt+Space",
            "Shift+F12",
            "Super+F12",
        ] {
            assert!(allowed(hotkey), "{hotkey}");
        }
    }

    #[test]
    fn a_shortcut_needs_a_real_modifier() {
        for hotkey in ["A", "F5", "Shift+A", "Shift+`"] {
            assert!(!allowed(hotkey), "{hotkey}");
        }
    }

    #[test]
    fn windows_shortcuts_are_refused() {
        for hotkey in ["Super+L", "Super+D", "Alt+Tab", "Alt+F4", "Ctrl+Alt+Delete"] {
            assert!(!allowed(hotkey), "{hotkey}");
        }
        // Only the exact combination is Windows'.
        assert!(allowed("Ctrl+Super+L"));
    }

    #[test]
    fn shortcuts_display_like_design_md() {
        assert_eq!(display("Alt+`"), "Alt + `");
        assert_eq!(display("Shift+Super+F12"), "Shift + Win + F12");
    }
}
