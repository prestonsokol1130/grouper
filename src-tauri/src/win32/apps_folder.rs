//! Reads the Shell's Applications folder (`shell:AppsFolder`): every app
//! Start lists under "All apps", Store (MSIX) apps included.

/// One item of the Applications folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppsFolderItem {
    /// The name Start shows.
    pub name: String,
    /// The item's parsing name inside the folder (what `Get-StartApps` calls
    /// the AppID): an AppUserModelID such as `Claude_pzs8sxrjxfjjc!Claude`,
    /// or a path for a plain desktop shortcut.
    pub parsing_name: String,
    /// `System.AppUserModel.ID`, when the item has one.
    pub aumid: Option<String>,
    /// `System.Link.TargetParsingPath`: what a desktop shortcut starts (a
    /// file, a `steam:` address, a Shell folder). `None` for Store apps.
    pub target_path: Option<String>,
}

/// Lists every item of the Applications folder.
///
/// The caller's thread must hold a live
/// [`ComGuard`](super::com::ComGuard). An item whose name cannot be read is
/// skipped and logged. `Err` when the folder itself cannot be opened or
/// listed; always `Err("unsupported platform")` off Windows.
pub fn list_apps_folder() -> Result<Vec<AppsFolderItem>, String> {
    imp::list_apps_folder()
}

/// True when `shell:AppsFolder\{parsing_name}` still resolves to an item,
/// that is the app is still installed. Initialises COM on the calling
/// thread for the call. Always false off Windows.
pub fn apps_folder_item_exists(parsing_name: &str) -> bool {
    imp::item_exists(parsing_name)
}

/// The `System.Tile.Background` of `shell:AppsFolder\{parsing_name}`
/// (`0xAABBGGRR`), `None` when it cannot be read. The caller's thread must
/// hold a live [`ComGuard`](super::com::ComGuard). Always `None` off Windows.
pub fn tile_background(parsing_name: &str) -> Option<u32> {
    imp::tile_background(parsing_name)
}

#[cfg(windows)]
mod imp {
    use windows::core::{Interface, GUID, PCWSTR, PWSTR};
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::Storage::EnhancedStorage::{
        PKEY_AppUserModel_ID, PKEY_Link_TargetParsingPath,
    };
    use windows::Win32::System::Com::{CoTaskMemFree, IBindCtx};
    use windows::Win32::UI::Shell::{
        BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, IShellItem2,
        SHCreateItemFromParsingName, SHGetKnownFolderItem, KF_FLAG_DEFAULT, SIGDN,
        SIGDN_NORMALDISPLAY, SIGDN_PARENTRELATIVEPARSING,
    };

    use super::AppsFolderItem;
    use crate::state::apps_folder_target;
    use crate::win32::com::ComGuard;
    use crate::win32::wide::to_wide;

    /// `System.Tile.Background` (propkey.h), which the windows crate lacks.
    const PKEY_TILE_BACKGROUND: PROPERTYKEY = PROPERTYKEY {
        fmtid: GUID::from_u128(0x86d40b4d_9069_443c_819a_2a54090dccec),
        pid: 4,
    };

    pub(super) fn list_apps_folder() -> Result<Vec<AppsFolderItem>, String> {
        // SAFETY: FOLDERID_AppsFolder is a live constant; no token (current
        // user). COM is initialised on this thread by the caller's guard.
        let folder: IShellItem =
            unsafe { SHGetKnownFolderItem(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, None) }
                .map_err(|e| format!("open the Applications folder: {e}"))?;
        // SAFETY: folder is a live item on this COM thread; no bind context.
        let items: IEnumShellItems =
            unsafe { folder.BindToHandler(None::<&IBindCtx>, &BHID_EnumItems) }
                .map_err(|e| format!("list the Applications folder: {e}"))?;
        let mut found = Vec::new();
        loop {
            let mut next = [None];
            // SAFETY: next has room for exactly the one item asked for; no
            // fetched-count pointer is needed for a single item. The end
            // of the list is S_FALSE (Ok) with nothing fetched.
            unsafe { items.Next(&mut next, None) }
                .map_err(|e| format!("list the Applications folder: {e}"))?;
            let Some(item) = next[0].take() else { break };
            match read_item(&item) {
                Ok(entry) => found.push(entry),
                Err(error) => log::warn!("skipping an Applications folder item: {error}"),
            }
        }
        Ok(found)
    }

    /// Reads one item's names and properties.
    fn read_item(item: &IShellItem) -> Result<AppsFolderItem, String> {
        let parsing_name = display_name(item, SIGDN_PARENTRELATIVEPARSING)?;
        let name = display_name(item, SIGDN_NORMALDISPLAY)?;
        let mut found = AppsFolderItem {
            name,
            parsing_name,
            ..AppsFolderItem::default()
        };
        if let Ok(item2) = item.cast::<IShellItem2>() {
            found.aumid = string_property(&item2, &PKEY_AppUserModel_ID);
            found.target_path = string_property(&item2, &PKEY_Link_TargetParsingPath);
        }
        Ok(found)
    }

    /// One `GetDisplayName` call, the string copied and freed.
    fn display_name(item: &IShellItem, form: SIGDN) -> Result<String, String> {
        // SAFETY: item is live on this COM thread; on success the returned
        // string is the caller's to free with CoTaskMemFree.
        let raw = unsafe { item.GetDisplayName(form) }.map_err(|e| format!("name: {e}"))?;
        take_string(raw).ok_or_else(|| "name is not UTF-16".to_owned())
    }

    /// A string property, `None` when it is absent, empty or not a string.
    fn string_property(item: &IShellItem2, key: &PROPERTYKEY) -> Option<String> {
        // SAFETY: item is live on this COM thread and key is a live
        // constant; on success the string is the caller's to free.
        let raw = unsafe { item.GetString(key) }.ok()?;
        take_string(raw).filter(|text| !text.is_empty())
    }

    /// Copies a CoTaskMem string out and frees it.
    fn take_string(raw: PWSTR) -> Option<String> {
        if raw.is_null() {
            return None;
        }
        // SAFETY: raw is a non-NULL, NUL-terminated string from the Shell.
        let text = unsafe { raw.to_string() }.ok();
        // SAFETY: raw was allocated with CoTaskMemAlloc by the Shell, is
        // freed exactly once here, and is not used afterwards.
        unsafe { CoTaskMemFree(Some(raw.0.cast_const().cast())) };
        text
    }

    pub(super) fn item_exists(parsing_name: &str) -> bool {
        let Ok(_com) = ComGuard::new() else {
            return false;
        };
        open_item::<IShellItem>(parsing_name).is_ok()
    }

    pub(super) fn tile_background(parsing_name: &str) -> Option<u32> {
        let item = open_item::<IShellItem2>(parsing_name).ok()?;
        // SAFETY: item is live on this COM thread; the key is a constant.
        unsafe { item.GetUInt32(&PKEY_TILE_BACKGROUND) }.ok()
    }

    /// `shell:AppsFolder\{parsing_name}` as a Shell item. COM must be
    /// initialised on this thread.
    fn open_item<T: Interface>(parsing_name: &str) -> windows::core::Result<T> {
        let wide = to_wide(&apps_folder_target(parsing_name));
        // SAFETY: wide is NUL-terminated and outlives the call; COM is
        // initialised on this thread (otherwise this returns Err).
        unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::AppsFolderItem;

    pub(super) fn list_apps_folder() -> Result<Vec<AppsFolderItem>, String> {
        Err("unsupported platform".to_owned())
    }

    pub(super) fn item_exists(_parsing_name: &str) -> bool {
        false
    }

    pub(super) fn tile_background(_parsing_name: &str) -> Option<u32> {
        None
    }
}
