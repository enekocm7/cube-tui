//! Per-instance, per-thread values: the counterpart of a Java `ThreadLocal` field.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, PoisonError};
use std::thread::{self, ThreadId};

/// One value per thread, owned by the containing object.
///
/// TNoodle keeps its solvers in `ThreadLocal` fields, so every puzzle instance has its own
/// solver on every thread. Some solvers carry scratch state from one solve to the next that
/// influences the result, so this is reproduced exactly. The value is taken out of the map
/// while it is in use, so no lock is held during a (long) solve.
pub(crate) struct PerThread<T> {
    values: Mutex<HashMap<ThreadId, T>>,
}

impl<T> Default for PerThread<T> {
    fn default() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> fmt::Debug for PerThread<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerThread").finish_non_exhaustive()
    }
}

impl<T> PerThread<T> {
    /// Runs `f` with this thread's value, creating it with `init` on first use.
    pub(crate) fn with<R>(&self, init: impl FnOnce() -> T, f: impl FnOnce(&mut T) -> R) -> R {
        let id = thread::current().id();
        let taken = self
            .values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
        let mut value = taken.unwrap_or_else(init);
        let result = f(&mut value);
        self.values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, value);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::PerThread;

    #[test]
    fn values_are_per_thread_and_persist() {
        let p = PerThread::default();
        assert_eq!(p.with(|| 1, |v| *v), 1);
        p.with(|| 0, |v| *v += 10);
        assert_eq!(p.with(|| 0, |v| *v), 11);
        std::thread::scope(|s| {
            s.spawn(|| assert_eq!(p.with(|| 5, |v| *v), 5));
        });
        assert_eq!(p.with(|| 0, |v| *v), 11);
        assert!(format!("{p:?}").contains("PerThread"));
    }
}
