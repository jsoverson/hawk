use std::sync::Arc;

pub struct List<T> {
    items: Vec<Arc<T>>,
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

impl<T> List<T>
where
    T: Send + Sync,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, item: T) {
        self.items.push(Arc::new(item));
    }

    #[allow(unused)]
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.items.len();
    }

    #[allow(unused)]
    pub fn prev(&mut self) {
        if self.index == 0 {
            self.index = self.items.len() - 1;
        } else {
            self.index -= 1;
        }
    }

    pub fn selected(&self) -> Arc<T> {
        self.items.get(self.index).unwrap().clone()
    }
}
