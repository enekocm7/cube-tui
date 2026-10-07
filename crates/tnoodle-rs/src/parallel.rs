//! Data parallel helpers whose results never depend on the number of threads.
//!
//! With the `parallel` feature the work is spread over rayon's thread pool; without it every
//! helper runs sequentially. Either way the results are those of a plain in-order loop, so
//! scrambles stay bit-exact.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Tells an [`in_order`] job that its result is no longer needed, so it can return early
/// with a partial result (which is then discarded).
pub(crate) struct Stop<'a> {
    limit: &'a AtomicUsize,
    index: usize,
}

static NEVER: AtomicUsize = AtomicUsize::new(usize::MAX);

impl Stop<'_> {
    /// A token that never asks to stop.
    pub(crate) fn never() -> Stop<'static> {
        Stop {
            limit: &NEVER,
            index: 0,
        }
    }

    /// Whether a job before this one already made this job's result useless.
    #[inline]
    pub(crate) fn should_stop(&self) -> bool {
        self.limit.load(Ordering::Relaxed) < self.index
    }
}

/// Runs `a` and `b`, potentially in parallel.
pub(crate) fn join<A, B, RA, RB>(a: A, b: B) -> (RA, RB)
where
    A: FnOnce() -> RA + Send,
    B: FnOnce() -> RB + Send,
    RA: Send,
    RB: Send,
{
    #[cfg(feature = "parallel")]
    {
        rayon::join(a, b)
    }
    #[cfg(not(feature = "parallel"))]
    {
        (a(), b())
    }
}

/// Calls `f(i, &mut items[i])` for every item, potentially in parallel.
pub(crate) fn fill<T, F>(items: &mut [T], f: F)
where
    T: Send,
    F: Fn(usize, &mut T) + Sync,
{
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        items
            .par_iter_mut()
            .with_min_len(64)
            .enumerate()
            .for_each(|(i, item)| f(i, item));
    }
    #[cfg(not(feature = "parallel"))]
    for (i, item) in items.iter_mut().enumerate() {
        f(i, item);
    }
}

/// Sums `f(&mut scratch, i)` over `0..n`, potentially in parallel. Every thread gets its own
/// scratch value from `init`.
pub(crate) fn sum<W, I, F>(n: usize, init: I, f: F) -> usize
where
    I: Fn() -> W + Sync,
    F: Fn(&mut W, usize) -> usize + Sync,
{
    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        (0..n)
            .into_par_iter()
            .with_min_len(1024)
            .map_init(&init, |w, i| f(w, i))
            .sum()
    }
    #[cfg(not(feature = "parallel"))]
    {
        let mut w = init();
        (0..n).map(|i| f(&mut w, i)).sum()
    }
}

/// Runs `job` on the indices `0..n` as if by the loop
///
/// ```text
/// for i in 0..n {
///     let r = job(i);
///     results.push(r);
///     if done(&results[i]) { break; }
/// }
/// ```
///
/// and returns `results`, but evaluates the jobs in parallel.
///
/// `done` sees the results in index order (it can keep a running total). `enough` lets a
/// single result cut the remaining jobs short before the results before it are known: it
/// must only return `true` for a result on which `done` would return `true` (or have
/// returned `true` earlier). Jobs whose results end up unused may be stopped through their
/// [`Stop`] token. Every thread gets its own scratch value from `init`.
pub(crate) fn in_order<W, R, I, J, D, E>(n: usize, init: I, job: J, done: D, enough: E) -> Vec<R>
where
    R: Send,
    I: Fn() -> W + Sync,
    J: Fn(&mut W, usize, &Stop<'_>) -> R + Sync,
    D: FnMut(&R) -> bool + Send,
    E: Fn(&R) -> bool + Sync,
{
    #[cfg(feature = "parallel")]
    {
        let threads = rayon::current_num_threads().min(n);
        if threads > 1 {
            return in_order_parallel(n, threads, init, job, done, enough);
        }
    }
    let _ = &enough;
    let mut done = done;
    let mut w = init();
    let mut results = Vec::new();
    for i in 0..n {
        let r = job(&mut w, i, &Stop::never());
        let finished = done(&r);
        results.push(r);
        if finished {
            break;
        }
    }
    results
}

#[cfg(feature = "parallel")]
fn in_order_parallel<W, R, I, J, D, E>(
    n: usize,
    threads: usize,
    init: I,
    job: J,
    done: D,
    enough: E,
) -> Vec<R>
where
    R: Send,
    I: Fn() -> W + Sync,
    J: Fn(&mut W, usize, &Stop<'_>) -> R + Sync,
    D: FnMut(&R) -> bool + Send,
    E: Fn(&R) -> bool + Sync,
{
    use std::sync::{Mutex, PoisonError};

    struct Progress<R, D> {
        results: Vec<Option<R>>,
        /// The results before this index have been passed to `done`.
        scanned: usize,
        /// The number of results to return, once known.
        end: Option<usize>,
        done: D,
    }

    // Jobs are taken in index order. The limit only decreases, and only to the index of a
    // job whose result makes all later jobs useless, so a job at or below the final limit
    // is never stopped and the returned prefix is complete.
    let next = AtomicUsize::new(0);
    let limit = AtomicUsize::new(usize::MAX);
    let progress = Mutex::new(Progress {
        results: (0..n).map(|_| None).collect(),
        scanned: 0,
        end: None,
        done,
    });

    rayon::scope(|s| {
        for _ in 0..threads {
            s.spawn(|_| {
                let mut w = init();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n || i > limit.load(Ordering::Relaxed) {
                        break;
                    }
                    let r = job(
                        &mut w,
                        i,
                        &Stop {
                            limit: &limit,
                            index: i,
                        },
                    );
                    if i > limit.load(Ordering::Relaxed) {
                        // The job may have been stopped; its result is not needed anyway.
                        break;
                    }
                    if enough(&r) {
                        limit.fetch_min(i, Ordering::Relaxed);
                    }
                    let mut guard = progress.lock().unwrap_or_else(PoisonError::into_inner);
                    let p = &mut *guard;
                    p.results[i] = Some(r);
                    while p.end.is_none() && p.scanned < n {
                        let Some(r) = &p.results[p.scanned] else {
                            break;
                        };
                        let finished = (p.done)(r);
                        p.scanned += 1;
                        if finished {
                            p.end = Some(p.scanned);
                            limit.fetch_min(p.scanned - 1, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });

    let p = progress
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    let end = p.end.unwrap_or(n);
    p.results
        .into_iter()
        .take(end)
        .map(|r| r.expect("every job before the end has finished"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Stop, in_order, sum};

    #[test]
    fn in_order_matches_a_sequential_loop() {
        for n in [0, 1, 2, 7, 100] {
            for target in [1, 5, 40, 10_000] {
                let mut total = 0;
                let results = in_order(
                    n,
                    || (),
                    |(), i, _: &Stop<'_>| i % 7,
                    |&r| {
                        total += r;
                        total >= target
                    },
                    |&r| r >= target,
                );
                let mut expected = Vec::new();
                let mut total = 0;
                for i in 0..n {
                    expected.push(i % 7);
                    total += i % 7;
                    if total >= target {
                        break;
                    }
                }
                assert_eq!(results, expected, "n = {n}, target = {target}");
            }
        }
    }

    #[test]
    fn in_order_finds_the_first_success() {
        let results = in_order(
            1000,
            || (),
            |(), i, stop: &Stop<'_>| {
                if stop.should_stop() {
                    return None;
                }
                (i % 97 == 96).then_some(i)
            },
            Option::is_some,
            Option::is_some,
        );
        assert_eq!(results.len(), 97);
        assert_eq!(results.last(), Some(&Some(96)));
        assert!(results[..96].iter().all(Option::is_none));
    }

    #[test]
    fn sums() {
        assert_eq!(sum(10_000, || 0, |_, i| i), 49_995_000);
    }
}
