use std::collections::HashMap;

/// A collection of HTTP headers.
///
/// Header names are case-insensitive.
#[derive(Debug, Default)]
pub struct Header {
    values: HashMap<String, Vec<String>>,
}

impl Header {
    /// Creates an empty collection of headers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the first value associated with a header.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.get_all(name)
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    /// Returns all values associated with a header.
    pub fn get_all(&self, name: &str) -> Option<&[String]> {
        self.values
            .get(&name.to_ascii_lowercase())
            .map(Vec::as_slice)
    }

    /// Inserts a header, replacing all existing values.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values
            .insert(name.into().to_ascii_lowercase(), vec![value.into()]);
    }

    /// Appends a value to an existing header.
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values
            .entry(name.into().to_ascii_lowercase())
            .or_default()
            .push(value.into());
    }

    /// Removes a header and returns all of its values.
    pub fn remove(&mut self, name: &str) -> Option<Vec<String>> {
        self.values.remove(&name.to_ascii_lowercase())
    }

    /// Returns whether a header exists.
    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(&name.to_ascii_lowercase())
    }

    /// Returns the number of distinct header names.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns whether there are no headers.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns an iterator over header names and their values.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.values
            .iter()
            .map(|(name, values)| (name.as_str(), values.as_slice()))
    }
}
