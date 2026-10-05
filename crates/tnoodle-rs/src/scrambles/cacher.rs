//! Background scramble generation (`ScrambleCacher`).

use std::collections::VecDeque;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;

use thiserror::Error;

use super::registry::Scrambler;
use crate::java::Sha1Prng;

const DEFAULT_CACHE_SIZE: usize = 100;

/// Called whenever the number of cached scrambles changes.
pub type CacheListener = Arc<dyn Fn(&ScrambleCacher) + Send + Sync>;

/// The scramble generator thread failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("the scramble generator failed: {0}")]
pub struct CacherError(String);

#[derive(Debug)]
struct State {
    scrambles: VecDeque<String>,
    running: bool,
    error: Option<CacherError>,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    cache_size: usize,
    listeners: Vec<CacheListener>,
}

/// Generates scrambles on a background thread and hands them out on demand, blocking when
/// none are ready. The generator stops when the cache is full and resumes as scrambles are
/// taken; call [`stop`](Self::stop) to end it.
///
/// Handles are cheap to clone and all refer to the same cache.
#[derive(Clone)]
pub struct ScrambleCacher {
    shared: Arc<Shared>,
}

impl fmt::Debug for ScrambleCacher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrambleCacher")
            .field("available", &self.available_count())
            .field("cache_size", &self.shared.cache_size)
            .field("running", &self.is_running())
            .finish_non_exhaustive()
    }
}

impl ScrambleCacher {
    /// Starts caching up to 100 scrambles of `puzzle`.
    pub fn new(puzzle: &'static Scrambler) -> Self {
        Self::with_options(puzzle, DEFAULT_CACHE_SIZE, false, Vec::new())
    }

    /// Starts caching up to `cache_size` scrambles of `puzzle`. With `draw_scramble`, every
    /// scramble is also drawn (to exercise the drawing code concurrently). `listeners` are
    /// called from the generator thread and from [`new_scramble`](Self::new_scramble)
    /// whenever the number of available scrambles changes.
    ///
    /// # Panics
    ///
    /// Panics if `cache_size` is zero or the thread cannot be spawned.
    pub fn with_options(
        puzzle: &'static Scrambler,
        cache_size: usize,
        draw_scramble: bool,
        listeners: Vec<CacheListener>,
    ) -> Self {
        assert!(cache_size > 0, "the cache must hold at least one scramble");
        let cacher = Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    scrambles: VecDeque::with_capacity(cache_size),
                    running: true,
                    error: None,
                }),
                changed: Condvar::new(),
                cache_size,
                listeners,
            }),
        };
        let worker = cacher.clone();
        thread::Builder::new()
            .name(format!("tnoodle-{}-cacher", puzzle.short_name()))
            .spawn(move || worker.run(puzzle, draw_scramble))
            .expect("failed to spawn the scramble cacher thread");
        cacher
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn run(&self, puzzle: &'static Scrambler, draw_scramble: bool) {
        let mut r = Sha1Prng::from_entropy();
        loop {
            let generated = panic::catch_unwind(AssertUnwindSafe(|| {
                let scramble = puzzle.generate_wca_scramble(&mut r);
                if draw_scramble {
                    // Drawing a scramble we just generated cannot fail.
                    let _ = puzzle.draw_scramble(Some(&scramble), None);
                }
                scramble
            }));
            let scramble = match generated {
                Ok(scramble) => scramble,
                Err(payload) => {
                    let message = payload
                        .downcast_ref::<&str>()
                        .map(ToString::to_string)
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "panic".to_owned());
                    // Let everyone waiting for a scramble know that we have crashed.
                    self.lock().error = Some(CacherError(message));
                    self.shared.changed.notify_all();
                    return;
                }
            };

            {
                let mut state = self.lock();
                while state.running && state.scrambles.len() == self.shared.cache_size {
                    state = self
                        .shared
                        .changed
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                if !state.running {
                    return;
                }
                state.scrambles.push_back(scramble);
                self.shared.changed.notify_all();
            }
            self.fire_cache_updated();
        }
    }

    /// Notifies the listeners; must not be called while holding the lock.
    fn fire_cache_updated(&self) {
        for listener in &self.shared.listeners {
            listener(self);
        }
    }

    /// Stops the generator thread.
    pub fn stop(&self) {
        self.lock().running = false;
        self.shared.changed.notify_all();
    }

    /// Whether the generator thread is running.
    pub fn is_running(&self) -> bool {
        self.lock().running
    }

    /// The number of scrambles ready to be taken.
    pub fn available_count(&self) -> usize {
        self.lock().scrambles.len()
    }

    /// The maximum number of cached scrambles.
    pub fn cache_size(&self) -> usize {
        self.shared.cache_size
    }

    /// Takes a scramble from the cache, blocking until one is available.
    pub fn new_scramble(&self) -> Result<String, CacherError> {
        let scramble = {
            let mut state = self.lock();
            loop {
                if let Some(error) = &state.error {
                    return Err(error.clone());
                }
                if let Some(scramble) = state.scrambles.pop_front() {
                    break scramble;
                }
                state = self
                    .shared
                    .changed
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        self.shared.changed.notify_all();
        self.fire_cache_updated();
        Ok(scramble)
    }

    /// Takes `count` scrambles, blocking as needed.
    pub fn new_scrambles(&self, count: usize) -> Result<Vec<String>, CacherError> {
        (0..count).map(|_| self.new_scramble()).collect()
    }
}
