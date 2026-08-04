use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathSeg {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A snapshot-relative navigation address. Path is not part of Change and is
/// not stable across concurrent edits.
pub struct Path(Vec<PathSeg>);

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn key(key: impl Into<String>) -> Self {
        Self(vec![PathSeg::Key(key.into())])
    }

    pub fn index(index: usize) -> Self {
        Self(vec![PathSeg::Index(index)])
    }

    pub fn push_key(mut self, key: impl Into<String>) -> Self {
        self.0.push(PathSeg::Key(key.into()));
        self
    }

    pub fn push_index(mut self, index: usize) -> Self {
        self.0.push(PathSeg::Index(index));
        self
    }

    pub fn push(&mut self, segment: PathSeg) {
        self.0.push(segment);
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn segments(&self) -> &[PathSeg] {
        &self.0
    }

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

#[macro_export]
macro_rules! path {
    () => { $crate::Path::new() };
    ($($segment:expr),+ $(,)?) => {{
        let mut path = $crate::Path::new();
        $(path.push($crate::path::path_segment($segment));)+
        path
    }};
}

pub trait IntoPathSeg {
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

pub fn path_segment(value: impl IntoPathSeg) -> PathSeg {
    value.into_path_seg()
}
