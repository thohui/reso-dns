use std::{
    hash::Hash,
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::anyhow;
use dashmap::DashMap;
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

/// A structure to manage inflight operations identified by keys.
pub struct Inflight<K, V> {
    map: Arc<DashMap<K, Arc<Entry<V>>>>,
}

impl<K, V> Inflight<K, V>
where
    K: Eq + Hash + Clone + std::fmt::Debug + 'static,
    V: Send + Sync + 'static,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            map: Arc::new(DashMap::new()),
        }
    }

    /// Run `make(token)` once per key; others await the same shared result.
    /// Cancels when the last waiter drops; removes the entry on completion or last-drop.
    pub async fn get_or_run<F, Fut>(&self, key: K, make: F) -> anyhow::Result<Arc<V>>
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<V>> + Send + 'static,
    {
        use dashmap::mapref::entry::Entry as DMEntry;

        // create or get the Entry for this key
        let entry = match self.map.entry(key.clone()) {
            DMEntry::Occupied(e) => Arc::clone(e.get()),
            DMEntry::Vacant(v) => {
                let new_entry = Arc::new(Entry::<V>::new());
                v.insert(Arc::clone(&new_entry));
                new_entry
            }
        };

        let mut guard = WaiterGuard::new(Arc::clone(&entry), key.clone(), Arc::clone(&self.map));

        let token = entry.token.child_token();

        // create the shared future that will run the operation
        let shared_future = {
            let work = make(token.clone()).map(|r| Arc::new(r.map(Arc::new)));
            async move {
                tokio::select! {
                    _ = token.cancelled() => Arc::new(Err(anyhow!("inflight cancelled"))),
                    res = work => res,
                }
            }
            .boxed()
            .shared()
        };

        #[allow(clippy::async_yields_async)]
        let shared = entry.fut.get_or_init(async move || shared_future).await.clone();

        let arc_res = shared.await;

        guard.finish_cleanup_if_last();

        match arc_res.deref() {
            Ok(v) => Ok(Arc::clone(v)),
            Err(e) => Err(anyhow!("{e:#}")), // we lose original backtrace but its fine
        }
    }
}

/// The type of the shared future stored in an Entry.
type EntryFut<V> = Shared<BoxFuture<'static, Arc<Result<Arc<V>, anyhow::Error>>>>;

/// An entry in the inflight map.
struct Entry<V> {
    // Shared future producing Arc<Result<Arc<V>, anyhow::Error>> (cloneable)
    fut: OnceCell<EntryFut<V>>,
    token: CancellationToken,
    waiters: AtomicUsize,
}

impl<V> Entry<V> {
    fn new() -> Self {
        Self {
            fut: OnceCell::new(),
            token: CancellationToken::new(),
            waiters: AtomicUsize::new(0),
        }
    }
}

/// A guard that tracks the number of waiters on an Entry, and cleans up when the last one is dropped.
struct WaiterGuard<K: Eq + Hash + std::fmt::Debug, V> {
    entry: Arc<Entry<V>>,
    key: K,
    map: Arc<DashMap<K, Arc<Entry<V>>>>,
    finished: bool,
}

impl<K: Eq + Hash + std::fmt::Debug, V> WaiterGuard<K, V> {
    fn new(entry: Arc<Entry<V>>, key: K, map: Arc<DashMap<K, Arc<Entry<V>>>>) -> Self {
        entry.waiters.fetch_add(1, Ordering::Relaxed);
        Self {
            entry,
            key,
            map,
            finished: false,
        }
    }

    /// Mark this waiter as finished and clean up if it was the last one.
    fn finish_cleanup_if_last(&mut self) {
        self.finished = true;
        self.cleanup();
    }

    /// Decrement the waiter count and clean up if it was the last one.
    fn cleanup(&mut self) {
        if self.entry.waiters.fetch_sub(1, Ordering::Relaxed) == 1 {
            // last waiter
            self.entry.token.cancel();
            let _ = self.map.remove(&self.key);
        }
    }
}

impl<K: Eq + Hash + std::fmt::Debug, V> Drop for WaiterGuard<K, V> {
    fn drop(&mut self) {
        if !self.finished {
            self.cleanup();
        }
    }
}
