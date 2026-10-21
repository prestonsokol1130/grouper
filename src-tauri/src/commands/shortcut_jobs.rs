//! What the shortcut worker ([`super::shortcut_worker`]) does for each
//! job. Runs on the worker thread, under its `ComGuard`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use super::shortcut_cleanup::{remove_deleted_group, sweep_at_startup};
use super::shortcut_worker::{Job, ShortcutUpdated, SHORTCUT_UPDATED_EVENT};
use super::state_cmds::emit_state_changed;
use crate::state::paths::cache_dir;
use crate::state::Store;
use crate::taskbar::group_icon_files::current_icon;
use crate::taskbar::shortcut::shortcut_dir;
use crate::taskbar::shortcut_sync::{repair_group_shortcut, update_group_shortcut, Change, Repair};
use crate::taskbar::shortcut_write::write_group_shortcut;
use crate::win32::shell_notify::notify_shortcut_changed;

/// What a job needs from the app.
struct Ctx {
    store: Arc<Store>,
    data: PathBuf,
    cache_dir: PathBuf,
    exe: PathBuf,
}

fn ctx(app: &AppHandle) -> Result<Ctx, String> {
    let store = app
        .try_state::<Arc<Store>>()
        .map(|store| Arc::clone(&store))
        .ok_or("the app state is not loaded")?;
    let data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("resolve app data dir: {e}"))?;
    let exe = std::env::current_exe().map_err(|e| format!("find Grouper.exe: {e}"))?;
    Ok(Ctx {
        store,
        cache_dir: cache_dir(&data),
        data,
        exe,
    })
}

/// Runs one job on the worker thread; failures are logged.
pub(super) fn run(app: &AppHandle, job: Job) {
    let outcome = match job {
        Job::Create { group_id, reply } => {
            // The caller may have given up waiting; nothing else to do.
            let _ = reply.send(create(app, &group_id));
            Ok(())
        }
        Job::Update { group_id, change } => update(app, &group_id, change),
        Job::Remove { group_id } => remove(app, &group_id),
        Job::Repair => repair_all(app),
    };
    if let Err(error) = outcome {
        log::warn!("shortcut sync failed: {error}");
    }
}

/// Writes the shortcut and records it on the group. When the group was
/// renamed or its icon changed while this ran, the shortcut is brought up
/// to date before returning.
pub(super) fn create(app: &AppHandle, group_id: &str) -> Result<PathBuf, String> {
    let ctx = ctx(app)?;
    let state = ctx.store.snapshot();
    let dir = shortcut_dir()?;
    let lnk = write_group_shortcut(&state, group_id, &ctx.cache_dir, &dir, &ctx.exe)?;
    log::info!("wrote shortcut for group {group_id}: {}", lnk.display());
    notify(&ctx, group_id, &lnk);
    record(app, &ctx.store, group_id, &lnk)?;
    let now = ctx.store.snapshot();
    let (Some(written), Some(latest)) = (state.group_by_id(group_id), now.group_by_id(group_id))
    else {
        return Ok(lnk);
    };
    let change = Change {
        renamed: latest.name != written.name,
        icon: latest.icon_version != written.icon_version,
    };
    if !change.renamed && !change.icon {
        return Ok(lnk);
    }
    let lnk = update_group_shortcut(&now, group_id, change, &ctx.cache_dir, &dir, &ctx.exe)?;
    finish(app, &ctx, group_id, &lnk)?;
    Ok(lnk)
}

fn update(app: &AppHandle, group_id: &str, change: Change) -> Result<(), String> {
    let ctx = ctx(app)?;
    let state = ctx.store.snapshot();
    // Deleted since, or never given a shortcut: nothing to keep in sync.
    if !state.group_by_id(group_id).is_some_and(|g| g.has_shortcut) {
        return Ok(());
    }
    let dir = shortcut_dir()?;
    let lnk = update_group_shortcut(&state, group_id, change, &ctx.cache_dir, &dir, &ctx.exe)?;
    log::info!("updated shortcut for group {group_id}: {}", lnk.display());
    finish(app, &ctx, group_id, &lnk)
}

fn remove(app: &AppHandle, group_id: &str) -> Result<(), String> {
    let ctx = ctx(app)?;
    if ctx.store.snapshot().group_by_id(group_id).is_some() {
        // The id is in use again; its files are not stale.
        return Ok(());
    }
    remove_deleted_group(group_id, &ctx.cache_dir);
    Ok(())
}

/// Cleans up after deleted groups ([`sweep_at_startup`]), then repairs
/// every tracked group's shortcut, one group at a time; one group's
/// failure is logged and the rest still run.
fn repair_all(app: &AppHandle) -> Result<(), String> {
    let ctx = ctx(app)?;
    sweep_at_startup(&ctx.store.snapshot(), &ctx.data);
    let tracked: Vec<String> = ctx
        .store
        .snapshot()
        .groups
        .iter()
        .filter(|group| group.has_shortcut)
        .map(|group| group.id.clone())
        .collect();
    if tracked.is_empty() {
        return Ok(());
    }
    let dir = shortcut_dir()?;
    for group_id in &tracked {
        if let Err(error) = repair_one(app, &ctx, &dir, group_id) {
            log::warn!("could not repair group {group_id}'s shortcut: {error}");
        }
    }
    Ok(())
}

/// Repairs group `group_id`'s shortcut from the latest state, skipping a
/// group that is gone or no longer has a shortcut. Older `.ico` versions
/// are kept: an old taskbar pin may still show one. They go only when the
/// group is deleted.
fn repair_one(app: &AppHandle, ctx: &Ctx, dir: &Path, group_id: &str) -> Result<(), String> {
    let state = ctx.store.snapshot();
    if !state.group_by_id(group_id).is_some_and(|g| g.has_shortcut) {
        return Ok(());
    }
    match repair_group_shortcut(&state, group_id, &ctx.cache_dir, dir, &ctx.exe)? {
        Repair::Intact(lnk) => record(app, &ctx.store, group_id, &lnk)?,
        Repair::Rebuilt(lnk) => {
            log::info!("repaired shortcut for group {group_id}: {}", lnk.display());
            finish(app, ctx, group_id, &lnk)?;
        }
    }
    Ok(())
}

/// After a rewrite: refreshes the Shell, records the path, and emits
/// [`SHORTCUT_UPDATED_EVENT`].
fn finish(app: &AppHandle, ctx: &Ctx, group_id: &str, lnk: &Path) -> Result<(), String> {
    notify(ctx, group_id, lnk);
    record(app, &ctx.store, group_id, lnk)?;
    let payload = ShortcutUpdated {
        group_id: group_id.to_owned(),
        shortcut_path: lnk.to_string_lossy().into_owned(),
    };
    if let Err(error) = app.emit(SHORTCUT_UPDATED_EVENT, payload) {
        log::warn!("could not emit {SHORTCUT_UPDATED_EVENT}: {error}");
    }
    Ok(())
}

/// Tells the Shell that `lnk` and the `.ico` it now shows changed.
fn notify(ctx: &Ctx, group_id: &str, lnk: &Path) {
    let icon = current_icon(&ctx.cache_dir, group_id);
    notify_shortcut_changed(lnk, icon.as_deref().unwrap_or(lnk));
}

/// Marks group `group_id` as having its shortcut at `lnk`. Nothing is
/// saved when that is already recorded. `Err` when the group is gone.
fn record(app: &AppHandle, store: &Store, group_id: &str, lnk: &Path) -> Result<(), String> {
    let path = lnk.to_string_lossy().into_owned();
    let current = store.snapshot();
    let group = current
        .group_by_id(group_id)
        .ok_or_else(|| format!("Group not found: {group_id}"))?;
    if group.has_shortcut && group.shortcut_path.as_deref() == Some(path.as_str()) {
        return Ok(());
    }
    store.update_and_publish(
        |state| {
            let group = state
                .group_by_id_mut(group_id)
                .ok_or_else(|| format!("Group not found: {group_id}"))?;
            group.has_shortcut = true;
            group.shortcut_path = Some(path);
            Ok(())
        },
        |state| emit_state_changed(app, state),
    )
}
