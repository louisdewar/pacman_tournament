use std::collections::hash_map::{Entry, VacantEntry};
use std::collections::HashMap;

// TODO: based on current implementation the best internal datastructure for
// bucket would be a vector of options of T

#[derive(Clone, Debug)]
pub struct Bucket<T> {
    inner: HashMap<usize, T>,
    max_id: usize,
}

impl<T> Default for Bucket<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Bucket<T> {
    pub fn new() -> Self {
        Bucket {
            inner: HashMap::new(),
            max_id: 0,
        }
    }

    pub fn insert(&mut self, id: usize, item: T) -> Option<T> {
        if id >= self.max_id {
            self.max_id = id + 1;
        }

        self.inner.insert(id, item)
    }

    pub fn minimum_available_id(&self) -> usize {
        if self.max_id == self.inner.len() {
            self.max_id
        } else {
            let max_id = self.max_id;
            for i in 0..max_id {
                if !self.inner.contains_key(&i) {
                    return i;
                }
            }
            unreachable!(
                "only possible if len ({}) > max_id ({}) which should never happen",
                self.inner.len(),
                max_id
            );
        }
    }

    pub fn add(&mut self, item: T) -> usize {
        let i = self.minimum_available_id();
        let item = self.insert(i, item);
        debug_assert!(
            item.is_none(),
            "id {} already existed but add method tried to insert",
            i
        );
        i
    }

    pub fn remove(&mut self, id: usize) -> Option<T> {
        self.inner.remove(&id)
    }

    pub fn get(&self, id: usize) -> Option<&T> {
        self.inner.get(&id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.inner.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&usize, &T)> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&usize, &mut T)> {
        self.inner.iter_mut()
    }

    pub fn keys(&self) -> impl Iterator<Item = &usize> {
        self.inner.keys()
    }

    /// All ids are less than this value
    pub fn max_id(&self) -> usize {
        self.max_id
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
