//! Group order. The order of `AppState.groups` is the order the Manager
//! (and anything else that lists groups) shows them in.

use std::collections::HashSet;

use super::model::AppState;

impl AppState {
    /// Puts the groups in the order of `ids`, which must name every group
    /// exactly once. Anything else is an `Err` and changes nothing.
    pub fn reorder_groups(&mut self, ids: &[String]) -> Result<(), String> {
        if ids.len() != self.groups.len() {
            return Err(format!(
                "Expected {} group ids, got {}",
                self.groups.len(),
                ids.len()
            ));
        }
        let mut seen = HashSet::new();
        let mut order = Vec::with_capacity(ids.len());
        for id in ids {
            let index = self
                .groups
                .iter()
                .position(|group| &group.id == id)
                .ok_or_else(|| format!("Group not found: {id}"))?;
            if !seen.insert(index) {
                return Err(format!("Group listed twice: {id}"));
            }
            order.push(index);
        }
        let mut slots: Vec<_> = std::mem::take(&mut self.groups)
            .into_iter()
            .map(Some)
            .collect();
        self.groups = order
            .into_iter()
            .filter_map(|index| slots.get_mut(index).and_then(Option::take))
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::state::model::{AppState, Group};

    fn state(ids: &[&str]) -> AppState {
        AppState {
            groups: ids
                .iter()
                .map(|id| Group {
                    id: (*id).into(),
                    name: format!("Group {id}"),
                    app_ids: vec![format!("app-{id}")],
                    ..Group::default()
                })
                .collect(),
            ..AppState::default()
        }
    }

    fn ids(state: &AppState) -> Vec<&str> {
        state.groups.iter().map(|g| g.id.as_str()).collect()
    }

    fn owned(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_string()).collect()
    }

    #[test]
    fn reorders_and_keeps_every_group_intact() {
        let mut s = state(&["a", "b", "c"]);
        let before = s.clone();
        s.reorder_groups(&owned(&["c", "a", "b"])).unwrap();
        assert_eq!(ids(&s), ["c", "a", "b"]);
        for group in &before.groups {
            assert_eq!(s.group_by_id(&group.id), Some(group));
        }
    }

    #[test]
    fn same_order_is_a_no_op() {
        let mut s = state(&["a", "b"]);
        s.reorder_groups(&owned(&["a", "b"])).unwrap();
        assert_eq!(ids(&s), ["a", "b"]);
    }

    #[test]
    fn empty_list_reorders_no_groups() {
        let mut s = state(&[]);
        assert!(s.reorder_groups(&[]).is_ok());
    }

    #[test]
    fn rejects_anything_but_a_permutation() {
        let cases: [&[&str]; 5] = [
            &["a", "b"],           // one missing
            &["a", "b", "c", "c"], // too many
            &["a", "a", "b"],      // duplicate
            &["a", "b", "x"],      // unknown
            &[],                   // none
        ];
        for case in cases {
            let mut s = state(&["a", "b", "c"]);
            let before = s.clone();
            assert!(s.reorder_groups(&owned(case)).is_err(), "{case:?}");
            assert_eq!(s, before, "{case:?} changed the state");
        }
    }
}
