use std::collections::HashMap;

/// HTTP headers.
#[derive(Debug, Default)]
pub struct Header {
    values: HashMap<String, String>,
}

impl Header {
    /// Creates an empty header collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the value of a header.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// Inserts a header.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.values.insert(name.into(), value.into())
    }

    /// Removes a header.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.values.remove(name)
    }

    /// Returns whether the headers contain the given name.
    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// Returns the number of headers.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether there are no headers.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over the headers.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }
}
