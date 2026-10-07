mod decode;

use super::{codec, Error, ErrorCode, Result};
use cocodec::{Decode, Encode};
use std::{collections::BTreeMap, fmt, sync::Arc};

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
/// An atomic RichText formatting value.
pub enum Attr {
    #[cocodec(tag = 0)]
    /// An atomic boolean.
    Bool(bool),
    #[cocodec(tag = 1)]
    /// A signed 64-bit integer.
    Int(i64),
    #[cocodec(tag = 2)]
    /// A finite canonical floating-point value.
    Float(f64),
    #[cocodec(tag = 3)]
    /// An atomic UTF-8 string without character-level editing.
    String(String),
}
impl Eq for Attr {}
/// Canonical sorted formatting attributes.
pub type Attrs = BTreeMap<String, Attr>;
/// Explicit formatting changes; None removes an attribute.
pub type AttrPatch = BTreeMap<String, Option<Attr>>;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// A UTF-8 text span or one atomic owning Embed with attributes.
pub enum RichSpan {
    #[cocodec(tag = 0)]
    /// A collaborative text span with formatting attributes.
    Text {
        /// UTF-8 text content addressed in Unicode scalars.
        text: String,
        /// Formatting attributes applied to the entire text span.
        attrs: Attrs,
    },
    #[cocodec(tag = 1)]
    /// One atomic owning Value in RichText.
    Embed {
        /// Owning content treated as one sequence unit.
        value: Value,
        /// Formatting attributes applied to the embed.
        attrs: Attrs,
    },
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
/// The closed content model of an immutable Value.
pub enum Body {
    #[cocodec(tag = 0)]
    /// The null atomic value.
    Null,
    #[cocodec(tag = 1)]
    /// An atomic boolean.
    Bool(bool),
    #[cocodec(tag = 2)]
    /// A signed 64-bit integer.
    Int(i64),
    #[cocodec(tag = 3)]
    /// A finite canonical floating-point value.
    Float(f64),
    #[cocodec(tag = 4)]
    /// An atomic UTF-8 string without character-level editing.
    String(String),
    #[cocodec(tag = 5)]
    /// Collaborative text addressed by Unicode scalar positions.
    Text(String),
    #[cocodec(tag = 6)]
    /// Formatted scalar text with atomic embeds.
    RichText(Vec<RichSpan>),
    #[cocodec(tag = 7)]
    /// An ordered collection of owning child Values.
    List(Vec<Value>),
    #[cocodec(tag = 8)]
    /// A canonical sorted mapping from unique keys to owning Values.
    Map(BTreeMap<String, Value>),
}
impl Eq for Body {}

// Subtree size and depth are cached so edits can enforce document limits
// without walking unchanged content.
struct Node {
    body: Body,
    size: u32,
    depth: u32,
}
/// Immutable owning subtree. Clones share storage; equality is structural.
#[derive(Clone, Encode)]
#[cocodec(transparent)]
pub struct Value(Arc<Node>);
impl Encode for Node {
    fn encode<W: cocodec::Write>(&self, w: &mut W) -> std::result::Result<(), cocodec::Error> {
        self.body.encode(w)
    }
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.body == other.0.body
    }
}
impl Eq for Value {}
impl Default for Value {
    fn default() -> Self {
        Self::null()
    }
}
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0.body, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
/// One Map-key or List-index step in a Path.
pub enum Segment {
    #[cocodec(tag = 0)]
    /// A Map key.
    Key(String),
    #[cocodec(tag = 1)]
    /// A List index.
    Index(usize),
}
/// A sequence of Map keys and List indexes interpreted against one content state.
pub type Path = Vec<Segment>;
/// Renders a Path for error details, such as `["items", 0]`; not a parse format.
pub(crate) fn render_path(path: &[Segment]) -> String {
    let segments: Vec<String> = path
        .iter()
        .map(|segment| match segment {
            Segment::Key(key) => format!("{key:?}"),
            Segment::Index(index) => index.to_string(),
        })
        .collect();
    format!("[{}]", segments.join(", "))
}

const MAX_DEPTH: u32 = 100;
const MAX_NODES: u32 = 1_000_000;

impl Value {
    /// Constructs, canonicalizes and validates content.
    pub fn new(mut body: Body) -> Result<Self> {
        canonicalize(&mut body)?;
        let value = Self::trusted(body);
        value.check_limits()?;
        Ok(value)
    }
    pub(crate) fn trusted(body: Body) -> Self {
        let mut size = 1u32;
        let mut depth = 0u32;
        let mut child = |value: &Value| {
            size = size.saturating_add(value.0.size);
            depth = depth.max(value.0.depth);
        };
        match &body {
            Body::List(list) => list.iter().for_each(&mut child),
            Body::Map(map) => map.values().for_each(&mut child),
            Body::RichText(spans) => {
                for span in spans {
                    if let RichSpan::Embed { value, .. } = span {
                        child(value);
                    }
                }
            }
            _ => {}
        }
        Self(Arc::new(Node {
            body,
            size,
            depth: depth.saturating_add(1),
        }))
    }
    /// Creates a Null value.
    pub fn null() -> Self {
        Self::trusted(Body::Null)
    }
    /// Creates a Bool value.
    pub fn bool(value: bool) -> Self {
        Self::trusted(Body::Bool(value))
    }
    /// Creates a signed 64-bit Int value.
    pub fn int(value: i64) -> Self {
        Self::trusted(Body::Int(value))
    }
    /// Creates a finite Float, canonicalizing negative zero.
    pub fn float(value: f64) -> Result<Self> {
        Self::new(Body::Float(value))
    }
    /// Creates an atomic String, rejecting content over 16 MiB.
    pub fn string(value: impl Into<String>) -> Result<Self> {
        Self::new(Body::String(value.into()))
    }
    /// Creates a collaboratively editable Text, rejecting content over 16 MiB.
    pub fn text(value: impl Into<String>) -> Result<Self> {
        Self::new(Body::Text(value.into()))
    }
    /// Creates a List from owning children.
    pub fn list(values: Vec<Value>) -> Result<Self> {
        Self::new(Body::List(values))
    }
    /// Creates a Map from unique keys and owning children.
    pub fn map(values: impl IntoIterator<Item = (String, Value)>) -> Result<Self> {
        let mut map = BTreeMap::new();
        for (key, value) in values {
            if map.insert(key, value).is_some() {
                return Err(Error::new(ErrorCode::InvalidValue, "duplicate map key"));
            }
        }
        Self::new(Body::Map(map))
    }
    /// Creates normalized RichText, coalescing equal-attribute adjacent text spans.
    pub fn rich_text(spans: Vec<RichSpan>) -> Result<Self> {
        Self::new(Body::RichText(spans))
    }
    /// Borrows the immutable typed content of this value.
    pub fn body(&self) -> &Body {
        &self.0.body
    }
    /// Returns the stable lowercase kind name.
    pub fn kind(&self) -> &'static str {
        match self.body() {
            Body::Null => "null",
            Body::Bool(_) => "bool",
            Body::Int(_) => "int",
            Body::Float(_) => "float",
            Body::String(_) => "string",
            Body::Text(_) => "text",
            Body::RichText(_) => "richtext",
            Body::List(_) => "list",
            Body::Map(_) => "map",
        }
    }
    /// Borrows the value at a Path; missing or incompatible steps return an error.
    pub fn get(&self, path: &[Segment]) -> Result<&Value> {
        let mut value = self;
        for segment in path {
            value = value.child(segment)?;
        }
        Ok(value)
    }
    /// Returns whether the Path resolves in this content.
    pub fn has(&self, path: &[Segment]) -> bool {
        self.get(path).is_ok()
    }
    fn child(&self, segment: &Segment) -> Result<&Value> {
        match (self.body(), segment) {
            (Body::Map(map), Segment::Key(key)) => map.get(key).ok_or_else(|| {
                Error::new(ErrorCode::MissingKey, "Map key is missing").detail("key", key)
            }),
            (Body::List(list), Segment::Index(index)) => list.get(*index).ok_or_else(|| {
                Error::new(ErrorCode::OutOfBounds, "list index out of bounds")
                    .detail("index", index.to_string())
            }),
            _ => Err(Error::new(
                ErrorCode::TypeMismatch,
                "path segment does not match container",
            )),
        }
    }
    /// Rebuilds the containers along a Path around a replacement for its target.
    pub(crate) fn update(
        &self,
        path: &[Segment],
        f: impl FnOnce(&Value) -> Result<Value>,
    ) -> Result<Value> {
        let Some((segment, rest)) = path.split_first() else {
            return f(self);
        };
        let child = self.child(segment)?.update(rest, f)?;
        Ok(Self::trusted(match (self.body(), segment) {
            (Body::Map(map), Segment::Key(key)) => {
                let mut map = map.clone();
                map.insert(key.clone(), child);
                Body::Map(map)
            }
            (Body::List(list), Segment::Index(index)) => {
                let mut list = list.clone();
                list[*index] = child;
                Body::List(list)
            }
            _ => unreachable!("child() matched the container"),
        }))
    }
    pub(crate) fn check_limits(&self) -> Result<()> {
        if self.0.depth > MAX_DEPTH || self.0.size > MAX_NODES {
            return Err(Error::new(
                ErrorCode::LimitExceeded,
                "value depth or node limit exceeded",
            ));
        }
        Ok(())
    }
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::Value, self)
    }
    /// Strictly decodes and validates a typed binary envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        codec::decode(codec::Kind::Value, bytes)
    }
}

/// Canonicalizes one node in place and reports whether it changed. Children
/// are already valid Values, so only this node's own content is checked.
pub(crate) fn canonicalize(body: &mut Body) -> Result<bool> {
    match body {
        Body::Float(n) if !n.is_finite() => {
            Err(Error::new(ErrorCode::InvalidValue, "float must be finite"))
        }
        Body::Float(n) if *n == 0.0 && n.is_sign_negative() => {
            *n = 0.0;
            Ok(true)
        }
        Body::String(s) | Body::Text(s) => check_string(s).map(|_| false),
        Body::Map(map) => {
            map.keys().try_for_each(|key| check_string(key))?;
            Ok(false)
        }
        Body::RichText(spans) => {
            let mut changed = false;
            for span in spans.iter_mut() {
                let attrs = match span {
                    RichSpan::Text { attrs, .. } | RichSpan::Embed { attrs, .. } => attrs,
                };
                for attr in attrs.values_mut() {
                    if let Attr::Float(value) = attr {
                        if *value == 0.0 && value.is_sign_negative() {
                            *value = 0.0;
                            changed = true;
                        }
                    }
                }
            }
            let normalized = normalize_spans(spans.clone())?;
            changed |= normalized != *spans;
            *spans = normalized;
            Ok(changed)
        }
        _ => Ok(false),
    }
}

pub(crate) fn normalize_spans(spans: Vec<RichSpan>) -> Result<Vec<RichSpan>> {
    let mut out: Vec<RichSpan> = Vec::new();
    if spans.len() > 1_000_000 {
        return Err(Error::new(
            ErrorCode::LimitExceeded,
            "rich span limit exceeded",
        ));
    }
    for span in spans {
        let attrs = match &span {
            RichSpan::Text { attrs, .. } | RichSpan::Embed { attrs, .. } => attrs,
        };
        for (key, value) in attrs {
            check_string(key)?;
            if let Attr::String(text) = value {
                check_string(text)?;
            }
        }
        if attrs.values().any(
            |a| matches!(a, Attr::Float(v) if !v.is_finite() || v.to_bits() == (-0.0f64).to_bits()),
        ) {
            return Err(Error::new(
                ErrorCode::InvalidValue,
                "invalid formatting float",
            ));
        }
        match span {
            RichSpan::Text { text, attrs } => {
                check_string(&text)?;
                if text.is_empty() {
                    continue;
                }
                if let Some(RichSpan::Text {
                    text: previous,
                    attrs: previous_attrs,
                }) = out.last_mut()
                {
                    if previous_attrs == &attrs {
                        previous.push_str(&text);
                        check_string(previous)?;
                        continue;
                    }
                }
                out.push(RichSpan::Text { text, attrs });
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

pub(crate) fn check_string(value: &str) -> Result<()> {
    if value.len() > 16 * 1024 * 1024 {
        return Err(Error::new(
            ErrorCode::LimitExceeded,
            "string exceeds input limit",
        ));
    }
    Ok(())
}
