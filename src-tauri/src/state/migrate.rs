//! Schema migrations for `app_state.json`.

use super::model::{AppState, CURRENT_SCHEMA};

/// Brings a loaded state up to [`CURRENT_SCHEMA`].
///
/// v1 is the first schema, so this is a no-op today. Later versions add a
/// `if state.schema_version < N { ... }` step per bump, in order.
pub fn migrate(mut state: AppState) -> AppState {
    if state.schema_version > CURRENT_SCHEMA {
        log::warn!(
            "app_state.json is schema v{} but this build knows v{CURRENT_SCHEMA}; loading as-is",
            state.schema_version
        );
        return state;
    }
    // Files written before versioning (or with 0) are treated as v1.
    state.schema_version = CURRENT_SCHEMA;
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_or_missing_version_becomes_current() {
        let state = AppState {
            schema_version: 0,
            ..AppState::default()
        };
        assert_eq!(migrate(state).schema_version, CURRENT_SCHEMA);
    }

    #[test]
    fn newer_version_is_left_alone() {
        let state = AppState {
            schema_version: CURRENT_SCHEMA + 1,
            ..AppState::default()
        };
        assert_eq!(migrate(state).schema_version, CURRENT_SCHEMA + 1);
    }
}
