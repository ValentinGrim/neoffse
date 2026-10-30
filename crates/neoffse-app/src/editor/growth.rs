//! The level-up notice rule of the editor page.

use super::model::EditorViewModel;
use super::notice::growth_notice;
use super::overview::levels_gained;
use crate::EditNotice;

impl EditorViewModel {
    /// Runs `action`, then adds the level-up notice when `action` raised levels.
    ///
    /// An edit that went through in default mode may have raised levels, and the
    /// game interface then grew the stats with random draws
    /// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`). The overview before
    /// and after tells how many levels grew: `Some(growth_notice(n))` when `n`
    /// is above zero. An error notice from `action` wins and comes back as it
    /// is. Both overviews come from the session at the call, not from the last
    /// refresh of a window.
    ///
    /// Expert mode skips the game's growth rules, so `action`'s result comes
    /// back with no overview read. A save in progress does the same: its
    /// worker holds the session lock, and `overview` would wait for it on the
    /// UI thread. UI thread only.
    #[must_use]
    pub fn with_growth(&self, action: impl FnOnce() -> Option<EditNotice>) -> Option<EditNotice> {
        if self.expert() || self.is_saving() {
            return action();
        }
        let before = self.overview();
        let notice = action();
        if notice.is_some() {
            return notice;
        }
        let gained = levels_gained(&before, &self.overview());
        (gained > 0).then(|| growth_notice(gained))
    }
}
