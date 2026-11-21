use std::mem;

use super::{Progress, SkillKey, SkillRows};

pub(super) struct SkillCache {
    rows: SkillRows,
    pending: SkillRows,
    flushing: bool,
}

impl SkillCache {
    pub(super) fn new(rows: SkillRows) -> Self {
        Self {
            rows,
            pending: SkillRows::new(),
            flushing: false,
        }
    }

    pub(super) fn progress(&self, key: SkillKey) -> Progress {
        self.rows.get(&key).copied().unwrap_or_default()
    }

    pub(super) fn update(
        &mut self,
        key: SkillKey,
        update: impl FnOnce(Progress) -> Progress,
    ) -> Progress {
        let previous = self.progress(key);
        let progress = update(previous);
        if progress != previous {
            self.rows.insert(key, progress);
            self.pending.insert(key, progress);
        }
        progress
    }

    pub(super) fn begin_flush(&mut self) -> Option<SkillRows> {
        if self.flushing || self.pending.is_empty() {
            return None;
        }
        self.flushing = true;
        Some(self.pending.clone())
    }

    pub(super) fn finish_flush(&mut self, batch: &SkillRows, committed: bool) {
        self.flushing = false;
        if committed {
            self.pending
                .retain(|key, value| batch.get(key) != Some(value));
        }
    }

    pub(super) fn take_pending(&mut self) -> SkillRows {
        mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use crate::skill_manager::SkillType;

    use super::*;

    #[test]
    fn retries_failed_batches_and_retains_newer_progress() {
        let mut cache = SkillCache::new(SkillRows::new());
        let key = (1, SkillType::Mining);
        let first = Progress {
            level: 2,
            experience: 5,
        };
        let newer = Progress {
            level: 2,
            experience: 15,
        };

        cache.update(key, |_| first);
        let batch = cache.begin_flush().unwrap();
        assert!(cache.begin_flush().is_none());
        cache.finish_flush(&batch, false);
        let retry = cache.begin_flush().unwrap();
        assert_eq!(retry, batch);

        cache.update(key, |_| newer);
        cache.finish_flush(&retry, true);
        assert_eq!(cache.take_pending().get(&key), Some(&newer));
    }
}
