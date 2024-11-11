use std::ops::{Deref, DerefMut};

pub(crate) struct List<T> {
    items: Vec<T>,
    index: usize,
}

impl<T> Default for List<T>
where
    T: Send + Sync,
{
    fn default() -> Self {
        Self {
            items: Vec::new(),
            index: 0,
        }
    }
}

#[allow(unused)]
impl<T> List<T>
where
    T: Clone + Send + Sync,
{
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub(crate) fn add(&mut self, item: T) {
        self.items.push(item);
    }

    pub(crate) fn next(&mut self) {
        self.index = (self.index + 1) % self.items.len();
    }

    pub(crate) fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.items.len() - 1;
        } else {
            self.index -= 1;
        }
    }

    pub(crate) fn focused(&self) -> T {
        self.items.get(self.index).unwrap().clone()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut()
    }
}

impl<T> Deref for List<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T> DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}
