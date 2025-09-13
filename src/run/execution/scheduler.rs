use std::{collections::{HashMap, HashSet}, fmt::Debug, hash::Hash, task::Poll};

use linked_hash_map::LinkedHashMap;
use linked_hash_set::LinkedHashSet;
use tokio::task::JoinSet;

// TODO add a number to describe how "heavy" a task is, so that we can better schedule them
#[derive(Debug)]
struct TaskTreeQueue<T: Hash + Eq> {
    /// Sorted list of tasks with their dependencies
    queue: LinkedHashMap<T, HashSet<T>>,

    /// For fast lookup of dependant tasks
    parents: HashMap<T, HashSet<T>>,
}

impl<T> TaskTreeQueue<T>
where
    T: Debug + Hash + Eq + Clone,
{
    pub fn new() -> Self {
        Self {
            queue: LinkedHashMap::new(),
            parents: HashMap::new(),
        }
    }

    pub fn add(&mut self, task: T, deps: impl IntoIterator<Item = T>) {
        self.add_set(task, deps.into_iter().collect())
    }

    pub fn add_set(&mut self, task: T, deps: HashSet<T>) {
        for dep in &deps {
            self.parents.entry(dep.clone()).or_default().insert(task.clone());
        }

        // TODO the stuff below is quite messy and inefficient, we are building the
        // deps set twice!
        debug_assert!(!self.queue.contains_key(&task));
        self.queue
            .entry(task.clone())
            .or_insert_with(|| HashSet::new()) // <- Here is the second build
            .extend(deps.into_iter());                                 // <- Here we drain the first one

        // TODO maybe debug assert of consistency
    }

    pub fn mark_fulfilled(&mut self, task: &T) {
        // TODO we should assert (or return a result)
        // that it's dependencies are fulfilled
        let deps = self.queue.remove(task);
        assert!(deps.is_none(), "Task {:?} was not taken", task);

        let Some(parents) = self.parents.remove(task) else {
            return;
        };

        for parent in parents {
            if let Some(deps) = self.queue.get_mut(&parent) {
                deps.remove(task);
            }
        }
    }

    pub fn take_next_ready_task(&mut self) -> Poll<Option<T>> {
        if self.queue.is_empty() {
            // no more tasks
            return Poll::Ready(None);
        }

        let next = self.queue
            .iter()
            .find(|(_, deps)| deps.is_empty())
            .map(|(task, _)| task.clone());

        let Some(next) = next else {
            // no task is ready yet
            return Poll::Pending;
        };

        let deps = self.queue.remove(&next);
        assert!(deps.unwrap().is_empty());
        Poll::Ready(Some(next))
    }
}

pub async fn execute_tasks_concurrently<Ref, F>(
    max_concurrency: usize,
    queue: impl IntoIterator<Item = Ref>,
    deps_graph: LinkedHashMap<Ref, LinkedHashSet<Ref>>,
    run_while: impl Fn() -> bool + Send + Sync + 'static, // TODO test
    run: impl Fn(Ref) -> F,
) -> anyhow::Result<()>
where
    Ref: Debug + Hash + Eq + Clone + Send + 'static,
    F: std::future::Future<Output = Result<(), anyhow::Error>> + Send + 'static,
{
    // TODO check max_concurrency > 0

    let mut running = JoinSet::<Result<Ref, (Ref, anyhow::Error)>>::new();

    // build the task queue
    let mut tq = TaskTreeQueue::new();
    for task in queue {
        let deps = deps_graph.get(&task).cloned().unwrap_or_default();
        tq.add(task, deps);
    }

    let mut interrupted = false;

    loop {
        // feed the running tasks
        while running.len() < max_concurrency {
            let next = if run_while() {
                tq.take_next_ready_task()
            } else {
                // stop feeding new tasks
                interrupted = true;
                Poll::Ready(None)
            };
            match next {
                Poll::Pending => break, // no more ready tasks
                Poll::Ready(Some(next)) => {
                    let f = run(next.clone());
                    running.spawn({
                        async move {
                            // TODO avoid clone with a match
                            f.await.map(|_| next.clone()).map_err(|e| (next, e))
                        }
                    });
                },
                Poll::Ready(None) => {
                    // no more task to run, wait for the running ones to finish
                    let all_failures = running
                        .join_all().await
                        .into_iter()
                        .filter_map(|r| r.err())
                        .collect::<Vec<_>>();
                    if all_failures.is_empty() {
                        if !interrupted {
                            return Ok(());
                        } else {
                            anyhow::bail!("Execution interrupted");
                        }
                    } else {
                        anyhow::bail!("One of the tasks failed: {:?}", all_failures);
                    }
                }
            }
        }

        // pool is full or no more ready tasks, wait for one to finish
        let Some(r) = running.join_next().await else {
            anyhow::bail!("No more running tasks, but queue is waiting");
        };

        // TODO handle join error
        let r = r.unwrap();

        match r {
            Ok(task) => tq.mark_fulfilled(&task),
            Err(e) => {
                running.abort_all();
                let mut all_failures = running
                    .join_all().await
                    .into_iter()
                    .filter_map(|r| r.err())
                    .collect::<Vec<_>>();
                all_failures.insert(0, e);
                anyhow::bail!("One of the tasks failed: {:?}", all_failures);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::sync::Barrier;

    use super::*;

    // TODO test empty queue

    /// Test the expected internal representation
    #[test]
    fn task_queue_construction() {
        let mut tq = TaskTreeQueue::new();
        tq.add(1, [2, 3]);
        tq.add(2, [4]);
        tq.add(3, []);
        tq.add(4, []);

        assert_eq!(tq.queue.len(), 4);
        assert_eq!(tq.parents.len(), 3);
        assert_eq!(tq.queue.get(&1).unwrap().len(), 2);
        assert_eq!(tq.queue.get(&2).unwrap().len(), 1);
        assert_eq!(tq.queue.get(&3).unwrap().len(), 0);
        assert_eq!(tq.queue.get(&4).unwrap().len(), 0);
        assert_eq!(tq.parents.get(&1), None);
        assert_eq!(tq.parents.get(&2).unwrap().len(), 1);
        assert_eq!(tq.parents.get(&3).unwrap().len(), 1);
        assert_eq!(tq.parents.get(&4).unwrap().len(), 1);
    }

    /// Test the behavior in a simple case
    #[test]
    fn task_queue_poll() {
        let mut tq = TaskTreeQueue::new();
        tq.add(1, [2, 3]);
        tq.add(2, [4]);
        tq.add(3, []);
        tq.add(4, []);

        assert_eq!(tq.take_next_ready_task(), Poll::Ready(Some(3)));
        assert_eq!(tq.take_next_ready_task(), Poll::Ready(Some(4)));
        assert_eq!(tq.take_next_ready_task(), Poll::Pending);

        tq.mark_fulfilled(&3);
        assert_eq!(tq.take_next_ready_task(), Poll::Pending);

        tq.mark_fulfilled(&4);
        assert_eq!(tq.take_next_ready_task(), Poll::Ready(Some(2)));
        assert_eq!(tq.take_next_ready_task(), Poll::Pending);

        tq.mark_fulfilled(&2);
        assert_eq!(tq.take_next_ready_task(), Poll::Ready(Some(1)));
        assert_eq!(tq.take_next_ready_task(), Poll::Ready(None));

        assert!(tq.queue.is_empty());
        assert!(tq.parents.is_empty());
    }

    #[test]
    #[should_panic]
    fn invalid_mark_fulfilled() {
        let mut tq = TaskTreeQueue::new();
        tq.add(1, [2, 3]);
        tq.add(2, [4]);
        tq.add(3, []);
        tq.add(4, []);

        tq.mark_fulfilled(&3); // <- This should panic
    }

    #[tokio::test]
    async fn null_run() {
        execute_tasks_concurrently(
            1,
            vec![],
            Default::default(),
            || true,
            |()| async move { Ok(()) },
        ).await.unwrap();
    }

    #[tokio::test]
    async fn trivial_run() {
        let results = Arc::new(Mutex::new(vec![]));

        execute_tasks_concurrently(
            1,
            vec![1, 2, 3],
            Default::default(),
            || true,
            |t| {
                let results = results.clone();
                async move {
                    results.lock().unwrap().push(t);
                    Ok(())
                }
            },
        ).await.unwrap();

        assert_eq!(*results.lock().unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn less_trivial_run() {
        let results = Arc::new(Mutex::new(vec![]));

        execute_tasks_concurrently(
            1,
            vec![1, 2, 3],
            [(1, [2].into_iter().collect())].into_iter().collect(), // 1 depends on 2
            || true,
            |t| {
                let results = results.clone();
                async move {
                    results.lock().unwrap().push(t);
                    Ok(())
                }
            },
        ).await.unwrap();

        assert_eq!(*results.lock().unwrap(), vec![2, 1, 3]);
    }

    #[tokio::test]
    async fn less_trivial_run_2() {
        let results: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![]));

        let barrier_1 = Arc::new(Barrier::new(3));
        let barrier_2 = Arc::new(Barrier::new(3));
        let barrier_3 = Arc::new(Barrier::new(3));

        let j = tokio::spawn({
            let results = results.clone();
            let barrier_1 = barrier_1.clone();
            let barrier_2 = barrier_2.clone();
            let barrier_3 = barrier_3.clone();
            async move {
                eprintln!("Watcher waiting at barrier 1");
                barrier_1.wait().await;
                eprintln!("Checking results after barrier 1");
                assert_eq!(*results.lock().unwrap(), vec![1]);
                eprintln!("Watcher waiting at barrier 2");
                barrier_2.wait().await;
                eprintln!("Watcher waiting at barrier 3");
                barrier_3.wait().await;
                eprintln!("Checking results after barrier 3");
                assert_eq!(*results.lock().unwrap(), vec![1, 2]);
                eprintln!("Watcher done");
            }
        });

        let results2 = results.clone();
        let j0 = execute_tasks_concurrently(
            1000,
            vec![1, 2, 3],
            [
                (3, [1].into_iter().collect()), // 3 depends on 1
                (3, [2].into_iter().collect()), // 3 depends on 2
            ].into_iter().collect(),
            || true,
            move |t| {
                let results = results2.clone();
                let barrier_1 = barrier_1.clone();
                let barrier_2 = barrier_2.clone();
                let barrier_3 = barrier_3.clone();
                async move {
                    if t == 1 {
                        results.lock().unwrap().push(t);
                    }
                    if t <= 2 {
                        eprintln!("Task {} waiting at barrier 1", t);
                        barrier_1.wait().await;
                        eprintln!("Task {} waiting at barrier 2", t);
                        barrier_2.wait().await;
                    }
                    eprintln!("Task {} running", t);
                    if t != 1 {
                        results.lock().unwrap().push(t);
                    }
                    if t <= 2 {
                        eprintln!("Task {} waiting at barrier 3", t);
                        barrier_3.wait().await;
                    }
                    eprintln!("Task {} done", t);
                    Ok(())
                }
            },
        );

        let j = tokio::spawn(async move {
            j0.await.unwrap();
            j.await.unwrap();
        });

        tokio::select! {
            _ = j => { Ok(()) },
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                Err("Deadlock")
            }
        }.unwrap();

        assert_eq!(*results.lock().unwrap(), vec![1, 2, 3]);
    }

    /// Same as [`less_trivial_run_2`] but with max_concurrency = 1 should deadlock
    #[tokio::test]
    #[should_panic]
    async fn less_trivial_run_3() {
        let results: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(vec![]));

        let barrier_1 = Arc::new(Barrier::new(3));
        let barrier_2 = Arc::new(Barrier::new(3));
        let barrier_3 = Arc::new(Barrier::new(3));

        let j = tokio::spawn({
            let results = results.clone();
            let barrier_1 = barrier_1.clone();
            let barrier_2 = barrier_2.clone();
            let barrier_3 = barrier_3.clone();
            async move {
                eprintln!("Watcher waiting at barrier 1");
                barrier_1.wait().await;
                eprintln!("Checking results after barrier 1");
                assert_eq!(*results.lock().unwrap(), vec![1]);
                eprintln!("Watcher waiting at barrier 2");
                barrier_2.wait().await;
                eprintln!("Watcher waiting at barrier 3");
                barrier_3.wait().await;
                eprintln!("Checking results after barrier 3");
                assert_eq!(*results.lock().unwrap(), vec![1, 2]);
                eprintln!("Watcher done");
            }
        });

        let results2 = results.clone();
        let j0 = execute_tasks_concurrently(
            1,
            vec![1, 2, 3],
            [
                (3, [1].into_iter().collect()), // 3 depends on 1
                (3, [2].into_iter().collect()), // 3 depends on 2
            ].into_iter().collect(),
            || true,
            move |t| {
                let results = results2.clone();
                let barrier_1 = barrier_1.clone();
                let barrier_2 = barrier_2.clone();
                let barrier_3 = barrier_3.clone();
                async move {
                    if t == 1 {
                        results.lock().unwrap().push(t);
                    }
                    if t <= 2 {
                        eprintln!("Task {} waiting at barrier 1", t);
                        barrier_1.wait().await;
                        eprintln!("Task {} waiting at barrier 2", t);
                        barrier_2.wait().await;
                    }
                    eprintln!("Task {} running", t);
                    if t != 1 {
                        results.lock().unwrap().push(t);
                    }
                    if t <= 2 {
                        eprintln!("Task {} waiting at barrier 3", t);
                        barrier_3.wait().await;
                    }
                    eprintln!("Task {} done", t);
                    Ok(())
                }
            },
        );

        let j = tokio::spawn(async move {
            j0.await.unwrap();
            j.await.unwrap();
        });

        tokio::select! {
            _ = j => { Ok(()) },
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                Err("Deadlock")
            }
        }.unwrap();

        assert_eq!(*results.lock().unwrap(), vec![1, 2, 3]);
    }
}