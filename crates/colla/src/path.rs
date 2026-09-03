//! Snapshot-relative navigation paths.

use std::fmt;

/// One segment in a Snapshot-relative [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathSeg {
    /// Navigates to a Map entry by key.
    Key(String),
    /// Navigates to a List element by index.
    Index(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A snapshot-relative navigation address. Path is not part of Change and is
/// not stable across concurrent edits.
pub struct Path(Vec<PathSeg>);

impl Path {
    /// Creates the root Path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a one-segment Map-key Path.
    pub fn key(key: impl Into<String>) -> Self {
        Self(vec![PathSeg::Key(key.into())])
    }

    /// Creates a one-segment List-index Path.
    pub fn index(index: usize) -> Self {
        Self(vec![PathSeg::Index(index)])
    }

    /// Appends a Map key and returns the extended Path.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.0.push(PathSeg::Key(key.into()));
        self
    }

    /// Appends a List index and returns the extended Path.
    pub fn with_index(mut self, index: usize) -> Self {
        self.0.push(PathSeg::Index(index));
        self
    }

    /// Appends a Map key and returns the extended Path.
    ///
    /// Prefer [`Path::with_key`] when building a Path fluently.
    pub fn push_key(self, key: impl Into<String>) -> Self {
        self.with_key(key)
    }

    /// Appends a List index and returns the extended Path.
    ///
    /// Prefer [`Path::with_index`] when building a Path fluently.
    pub fn push_index(self, index: usize) -> Self {
        self.with_index(index)
    }

    /// Appends a Map key in place.
    pub fn push_key_mut(&mut self, key: impl Into<String>) {
        self.0.push(PathSeg::Key(key.into()));
    }

    /// Appends a List index in place.
    pub fn push_index_mut(&mut self, index: usize) {
        self.0.push(PathSeg::Index(index));
    }

    /// Appends one segment in place.
    pub fn push(&mut self, segment: PathSeg) {
        self.0.push(segment);
    }

    /// Removes the final segment, if any.
    pub fn pop(&mut self) {
        self.0.pop();
    }

    /// Returns all path segments from root to target.
    pub fn segments(&self) -> &[PathSeg] {
        &self.0
    }

    /// Returns whether this is the root Path.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "$");
        }
        write!(f, "$")?;
        for segment in &self.0 {
            match segment {
                PathSeg::Key(key) => write!(f, ".{key}")?,
                PathSeg::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// Creates a [`Path`] from string Map keys and `usize` List indexes.
///
/// # Examples
///
/// ```
/// use colla::{path, PathSeg};
///
/// let path = path!["items", 2usize];
/// assert_eq!(path.segments(), &[
///     PathSeg::Key("items".into()),
///     PathSeg::Index(2),
/// ]);
/// ```
#[macro_export]
macro_rules! path {
    () => { $crate::Path::new() };
    ($($segment:expr),+ $(,)?) => {{
        let mut path = $crate::Path::new();
        $(path.push($crate::path::path_segment($segment));)+
        path
    }};
}

/// Converts a supported path macro argument into a [`PathSeg`].
pub trait IntoPathSeg {
    /// Converts this value into one Path segment.
    fn into_path_seg(self) -> PathSeg;
}

impl IntoPathSeg for &str {
    fn into_path_seg(self) -> PathSeg {
        PathSeg::Key(self.to_owned())
    }
}

impl IntoPathSeg for String {
    fn into_path_seg(self) -> PathSeg {
        PathSeg::Key(self)
    }
}

impl IntoPathSeg for usize {
    fn into_path_seg(self) -> PathSeg {
        PathSeg::Index(self)
    }
}

/// Converts one supported path value into a [`PathSeg`].
///
/// This function supports the exported [`crate::path!`] macro and is rarely
/// needed directly.
pub fn path_segment(value: impl IntoPathSeg) -> PathSeg {
    value.into_path_seg()
}

impl FromIterator<PathSeg> for Path {
    fn from_iter<T: IntoIterator<Item = PathSeg>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl std::ops::Deref for Path {
    type Target = [PathSeg];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
