use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModMap<T: Clone> {
    inner: HashMap<i16, T>,
}

impl<T: Clone> ModMap<T> {
    pub fn new() -> Self { Self { inner: HashMap::new() } }
    pub fn get(&self, k: i16) -> Option<&T> { self.inner.get(&k) }
    pub fn set(&mut self, k: i16, v: T) { self.inner.insert(k, v); }
    pub fn remove(&mut self, k: i16) { self.inner.remove(&k); }
    pub fn clear(&mut self) { self.inner.clear(); }
    pub fn contains(&self, k: i16) -> bool { self.inner.contains_key(&k) }
    pub fn keys(&self) -> impl Iterator<Item = &i16> { self.inner.keys() }
    pub fn values(&self) -> impl Iterator<Item = &T> { self.inner.values() }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&i16, &mut T)> { self.inner.iter_mut() }
    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    pub fn entry(&mut self, k: i16) -> std::collections::hash_map::Entry<'_, i16, T> { self.inner.entry(k) }
}

impl<T: Clone> Default for ModMap<T> {
    fn default() -> Self { Self::new() }
}
