use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    /// Column-major 2d, blocks of a single columns (increasing y) are stored sequentially.
    inner: Vec<T>,
}

impl<T> Grid<T> {
    pub fn new(inner: Vec<T>, width: usize, height: usize) -> Grid<T> {
        Grid {
            inner,
            width,
            height,
        }
    }

    pub fn fill_with_default(width: usize, height: usize) -> Grid<T>
    where
        T: Default + Clone,
    {
        Self::fill_with_clone(T::default(), width, height)
    }

    pub fn fill_with_clone(item: T, width: usize, height: usize) -> Grid<T>
    where
        T: Clone,
    {
        Grid {
            inner: vec![item; width * height],
            width,
            height,
        }
    }

    pub fn swap(&mut self, (o_x, o_y): (usize, usize), (d_x, d_y): (usize, usize)) {
        let origin = o_x * self.height + o_y;
        let destination = d_x * self.height + d_y;
        self.inner.swap(origin, destination);
    }

    pub fn len(&self) -> usize {
        debug_assert_eq!(self.width * self.height, self.inner.len());
        self.inner.len()
    }
}

impl<T> Index<usize> for Grid<T> {
    type Output = [T];

    fn index(&self, x: usize) -> &Self::Output {
        let start = x * self.height;
        &self.inner[start..start + self.height]
    }
}

impl<T> IndexMut<usize> for Grid<T> {
    fn index_mut(&mut self, x: usize) -> &mut Self::Output {
        let start = x * self.height;
        &mut self.inner[start..start + self.height]
    }
}

impl<T> AsRef<[T]> for Grid<T> {
    fn as_ref(&self) -> &[T] {
        &self.inner
    }
}
