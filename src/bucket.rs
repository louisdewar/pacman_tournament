use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Bucket<T> {
    inner: HashMap<usize, T>,
    max_id: usize,
}

impl<T> Bucket<T> {
    pub fn new() -> Self {
        Bucket {
            inner: HashMap::new(),
            max_id: 0,
        }
    }

    pub fn add(&mut self, item: T) -> usize {
        if self.max_id == self.inner.len() {
            self.inner.insert(self.max_id, item);
            self.max_id += 1;
            self.max_id - 1
        } else {
            use std::collections::hash_map::Entry;
            for i in 0..self.max_id {
                if let Entry::Vacant(entry) = self.inner.entry(i) {
                    entry.insert(item);
                    return i;
                }
            }

            unreachable!("only possible if len > max_id which should never happen");
        }
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
