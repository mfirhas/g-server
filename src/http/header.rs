pub trait Header {
    fn get(&self, name: &str) -> Option<&[u8]>;

    fn get_all(&self, name: &str) -> impl Iterator<Item = &[u8]>;

    fn contains(&self, name: &str) -> bool;

    fn is_empty(&self) -> bool;

    fn len(&self) -> usize;
}
