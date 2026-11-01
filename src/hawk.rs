// Monitoring system for new and old tasks in an actively running application

use log::info;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct Hawk<T, F>
where
    F: Fn(T) -> JoinHandle<anyhow::Result<()>>,
{
    settings: HawkSettings,
    tasks: Arc<RwLock<Vec<T>>>,
    create_task: F,
}

pub struct HawkSettings {
    pub check_interval: Duration,
}

struct HawkItem {
    id: HawkHash,
    handle: JoinHandle<anyhow::Result<()>>,
}

#[derive(Eq, PartialOrd, PartialEq, Ord, Debug)]
struct HawkHash(u64);

impl<
    T: Clone + Hash + Send + Sync + 'static,
    F: Fn(T) -> JoinHandle<anyhow::Result<()>> + Send + Sync + 'static,
> Hawk<T, F>
{
    pub fn new(settings: HawkSettings, tasks: Arc<RwLock<Vec<T>>>, create_task: F) -> Self {
        Self {
            settings,
            tasks,
            create_task,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let hawk = self;
            let mut active: Vec<HawkItem> = vec![];
            let mut interval = tokio::time::interval(hawk.settings.check_interval);
            loop {
                interval.tick().await;
                let read_lock = hawk.tasks.read().await;
                let tasks = read_lock.clone();
                drop(read_lock);

                let mut seen_hashes = vec![];
                for task in tasks {
                    // each task
                    let id = HawkHash::new(&task); // hash for its id
                    seen_hashes.push(id.0); // we've seen this id now
                    if !active.iter().any(|active| active.id == id) {
                        // if the id is new to active pool
                        info!("[+] Adding task [{}]", id.0);
                        let handle = (hawk.create_task)(task); // create the handle
                        let item = HawkItem::new(id, handle);
                        active.push(item); // add item to active pool
                    }
                }

                active
                    .extract_if(.., |item| {
                        seen_hashes.iter().all(|r_item| item.id.0 != *r_item)
                    }) // extract all ids that are no longer found in tasks and should be inactive
                    .for_each(|item| {
                        info!("[-] Removing task [{}]", item.id.0);
                        item.handle.abort(); // abort the handle and drop the task
                    });
            }
        })
    }
}

impl Default for HawkSettings {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_mins(1),
        }
    }
}

impl HawkItem {
    pub fn new(id: HawkHash, handle: JoinHandle<anyhow::Result<()>>) -> Self {
        Self { id, handle }
    }
}

impl HawkHash {
    pub fn new<T: Hash>(item: &T) -> Self {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[cfg(test)]
mod test {
    use crate::hawk::{Hawk, HawkSettings};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[derive(Debug, Hash, Clone, Eq, PartialEq)]
    struct HashableItem;

    #[tokio::test]
    async fn create_monitor() {
        let tasks = Arc::new(RwLock::new(vec![HashableItem]));

        let (tx, mut rx) = tokio::sync::mpsc::channel::<(HashableItem, Arc<str>)>(1);

        let extra_item_data: Arc<str> = "Banana".into();
        let hawk = Hawk::new(HawkSettings::default(), tasks, move |item| {
            let extra = extra_item_data.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let extra = extra;
                let item = item;
                assert!(tx.send((item, extra)).await.is_ok());
                Ok(())
            })
        });

        let handle = hawk.start();
        assert_eq!(rx.recv().await, Some((HashableItem, "Banana".into())));
        handle.abort();
    }
}
