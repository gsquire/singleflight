use std::fmt::Debug;

use crossbeam_utils::sync::WaitGroup;
use hashbrown::HashMap;
use parking_lot::Mutex;

// Call is an in-flight or completed call to work.
#[derive(Clone, Debug)]
struct Call<T>
where
    T: Clone + Debug,
{
    wg: WaitGroup,
    res: Option<T>,
}

impl<T> Call<T>
where
    T: Clone + Debug,
{
    fn new() -> Call<T> {
        Call {
            wg: WaitGroup::new(),
            res: None,
        }
    }
}

/// Group represents a class of work and creates a space in which units of work
/// can be executed with duplicate suppression.
#[derive(Default)]
pub struct Group<T>
where
    T: Clone + Debug,
{
    m: Mutex<HashMap<String, Box<Call<T>>>>,
}

impl<T> Group<T>
where
    T: Clone + Debug,
{
    /// Create a new Group to do work with.
    pub fn new() -> Group<T> {
        Group {
            m: Mutex::new(HashMap::new()),
        }
    }

    /// Execute and return the value for a given function, making sure that only one
    /// operation is in-flight at a given moment. If a duplicate call comes in, that caller will
    /// wait until the original call completes and return the same value.
    pub fn work<F>(&self, key: &str, func: F) -> T
    where
        F: Fn() -> T,
    {
        let mut m = self.m.lock();

        if let Some(c) = m.get(key) {
            let c = c.clone();
            drop(m);
            c.wg.wait();
            return c.res.unwrap();
        }

        let c = Call::new();
        let wg = c.wg.clone();
        let mut job = m.entry(key.to_owned()).or_insert(Box::new(c));
        job.res = Some(func());
        drop(m);
        drop(wg);

        let mut m = self.m.lock();
        let c = m.remove(key).unwrap();
        drop(m);

        c.res.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::Group;

    const RES: usize = 7;

    #[test]
    fn test_simple() {
        let g = Group::new();
        let res = g.work("key", || RES);
        assert_eq!(res, RES);
    }

    #[test]
    fn test_multiple_threads() {
        use std::time::Duration;

        use crossbeam_utils::thread;

        fn expensive_fn() -> usize {
            std::thread::sleep(Duration::new(0, 500));
            RES
        }

        let g = Group::new();
        thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|_| {
                    let res = g.work("key", expensive_fn);
                    assert_eq!(res, RES);
                });
            }
        })
        .unwrap();
    }
}
