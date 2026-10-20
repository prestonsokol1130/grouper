//! Which material each window is drawn on (DESIGN.md section 4), and
//! applying it at startup.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::flyout::FLYOUT_LABEL;
use crate::win32::material::{apply_backdrop, backdrops_allowed, clear_backdrop, Backdrop};

/// The material a window ended up with.
// With `solid` the page paints `--solid-bg` itself; with the others it
// stays transparent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Material {
    Mica,
    Acrylic,
    Solid,
}

pub use crate::state::settings::BackgroundSetting;

/// The windows that get a material, by label.
pub const MATERIAL_WINDOWS: [&str; 3] = ["manager", "launchpad", FLYOUT_LABEL];

/// The material window `label` asks for under `setting`. Under `Mixed`:
/// Mica for the Manager, Acrylic for the flyout and Launchpad, Solid for
/// anything else.
pub fn wanted(setting: BackgroundSetting, label: &str) -> Material {
    match setting {
        BackgroundSetting::Mixed => match label {
            "manager" => Material::Mica,
            "launchpad" | FLYOUT_LABEL => Material::Acrylic,
            _ => Material::Solid,
        },
        BackgroundSetting::Mica => Material::Mica,
        BackgroundSetting::Acrylic => Material::Acrylic,
        BackgroundSetting::Solid => Material::Solid,
    }
}

/// What to try, in order, for `wanted`. Windows 10 has no Mica, so Mica
/// falls back to Acrylic, as DESIGN.md section 4 specifies for the
/// Manager; everything ends at Solid, which always works.
///
/// Known trade-off: on Windows 10 v1903+ (and Windows 11 build 22000)
/// `apply_acrylic` uses an undocumented API that makes a window lag while
/// it is resized or dragged (window-vibrancy's documented warning). That
/// hits only the resizable Manager on Windows 10, since Windows 11 gives it
/// Mica and the flyout and Launchpad are never resized or dragged. Falling
/// back to Solid there instead is a DESIGN.md change for Preston to make;
/// it would be a one-line change to the Mica chain below.
pub fn fallbacks(wanted: Material) -> &'static [Material] {
    match wanted {
        Material::Mica => &[Material::Mica, Material::Acrylic, Material::Solid],
        Material::Acrylic => &[Material::Acrylic, Material::Solid],
        Material::Solid => &[Material::Solid],
    }
}

/// The first material in `chain` for which `try_apply` succeeds, or Solid.
/// With `allowed` false (transparency effects off, battery saver on) the
/// answer is Solid without trying anything.
pub fn pick(
    chain: &[Material],
    allowed: bool,
    mut try_apply: impl FnMut(Backdrop) -> Result<(), String>,
) -> Material {
    if !allowed {
        return Material::Solid;
    }
    for material in chain {
        let backdrop = match material {
            Material::Mica => Backdrop::Mica,
            Material::Acrylic => Backdrop::Acrylic,
            Material::Solid => return Material::Solid,
        };
        match try_apply(backdrop) {
            Ok(()) => return *material,
            Err(error) => log::info!("backdrop unavailable, falling back: {error}"),
        }
    }
    Material::Solid
}

/// The material each window got, for `get_window_material`.
#[derive(Debug, Default)]
pub struct MaterialState(Mutex<HashMap<String, Material>>);

impl MaterialState {
    /// The material of window `label`; Solid for a window never set up.
    pub fn get(&self, label: &str) -> Material {
        let map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        map.get(label).copied().unwrap_or(Material::Solid)
    }

    fn set(&self, label: &str, material: Material) {
        let mut map = self.0.lock().unwrap_or_else(|p| p.into_inner());
        map.insert(label.to_owned(), material);
    }
}

/// Emitted to every window after [`reapply`] changed the materials; each
/// page then asks for its own again (`get_window_material`).
pub const MATERIAL_CHANGED_EVENT: &str = "material:changed";

/// Manages the (still empty) [`MaterialState`]. Call at startup before
/// any window is created, so `get_window_material` always finds it.
pub fn manage<R: Runtime>(app: &AppHandle<R>) {
    app.manage(MaterialState::default());
}

/// Applies each window's material under `setting`, records the result and
/// emits [`MATERIAL_CHANGED_EVENT`], so a page that asked before this ran
/// asks again. Call once at startup, on the main thread, after
/// [`manage`] and after the windows exist.
pub fn init<R: Runtime>(app: &AppHandle<R>, setting: BackgroundSetting) {
    apply_all(app, setting, false);
    if let Err(error) = app.emit(MATERIAL_CHANGED_EVENT, ()) {
        log::warn!("could not emit {MATERIAL_CHANGED_EVENT}: {error}");
    }
}

/// Applies `setting` to every window again after the "Window background"
/// setting changed, then emits [`MATERIAL_CHANGED_EVENT`]. Main thread.
pub fn reapply<R: Runtime>(app: &AppHandle<R>, setting: BackgroundSetting) {
    apply_all(app, setting, true);
    if let Err(error) = app.emit(MATERIAL_CHANGED_EVENT, ()) {
        log::warn!("could not emit {MATERIAL_CHANGED_EVENT}: {error}");
    }
}

fn apply_all<R: Runtime>(app: &AppHandle<R>, setting: BackgroundSetting, clear: bool) {
    let Some(state) = app.try_state::<MaterialState>() else {
        return;
    };
    let allowed = backdrops_allowed();
    for label in MATERIAL_WINDOWS {
        let Some(window) = app.get_webview_window(label) else {
            log::warn!("window {label:?} not found; no material applied");
            continue;
        };
        if clear {
            clear_backdrop(&window);
        }
        let wanted = wanted(setting, label);
        let got = pick(fallbacks(wanted), allowed, |b| apply_backdrop(&window, b));
        log::info!("window {label:?} material: {got:?} (wanted {wanted:?}, allowed {allowed})");
        state.set(label, got);
    }
}

#[cfg(test)]
mod tests {
    use super::{fallbacks, pick, wanted, BackgroundSetting as S, Material as M};
    use crate::win32::material::Backdrop;

    #[test]
    fn mixed_follows_design_md() {
        assert_eq!(wanted(S::Mixed, "manager"), M::Mica);
        assert_eq!(wanted(S::Mixed, "flyout"), M::Acrylic);
        assert_eq!(wanted(S::Mixed, "launchpad"), M::Acrylic);
        assert_eq!(wanted(S::Mixed, "other"), M::Solid);
    }

    #[test]
    fn overrides_apply_to_every_window() {
        for label in ["manager", "flyout", "launchpad"] {
            assert_eq!(wanted(S::Mica, label), M::Mica);
            assert_eq!(wanted(S::Acrylic, label), M::Acrylic);
            assert_eq!(wanted(S::Solid, label), M::Solid);
        }
    }

    #[test]
    fn not_allowed_is_solid_without_trying() {
        let mut tried = 0;
        let got = pick(fallbacks(M::Mica), false, |_| {
            tried += 1;
            Ok(())
        });
        assert_eq!((got, tried), (M::Solid, 0));
    }

    #[test]
    fn mica_falls_back_to_acrylic_then_solid() {
        let got = pick(fallbacks(M::Mica), true, |b| match b {
            Backdrop::Mica => Err("no mica".into()),
            Backdrop::Acrylic => Ok(()),
        });
        assert_eq!(got, M::Acrylic);
        let got = pick(fallbacks(M::Mica), true, |_| Err("none".into()));
        assert_eq!(got, M::Solid);
    }

    #[test]
    fn first_success_wins() {
        assert_eq!(pick(fallbacks(M::Acrylic), true, |_| Ok(())), M::Acrylic);
        assert_eq!(pick(fallbacks(M::Solid), true, |_| Ok(())), M::Solid);
    }
}
