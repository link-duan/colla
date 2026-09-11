use super::{codec, ElementId, Error, ErrorCode, IdAllocator, Result};
use cocodec::{Decode, Encode};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cocodec(transparent)]
/// Atomic same-document weak reference; ownership and encoding never follow its target.
pub struct Ref {
    /// Stable target identity; Ref targets may be absent.
    pub target: ElementId,
}

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
    /// Collaborative Unicode scalar text or a scalar text operation.
    Text {
        /// UTF-8 text content addressed in Unicode scalars.
        text: String,
        /// Atomic formatting attributes or their explicit patch.
        attrs: Attrs,
    },
    #[cocodec(tag = 1)]
    /// One atomic owning Value in RichText.
    Embed {
        /// Immutable owning content carried by this operation or embed.
        value: Value,
        /// Atomic formatting attributes or their explicit patch.
        attrs: Attrs,
    },
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
/// The closed content model of an immutable identity-bearing Value.
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
    /// Collaborative Unicode scalar text or a scalar text operation.
    Text(String),
    #[cocodec(tag = 6)]
    /// Formatted scalar text with atomic embeds.
    RichText(Vec<RichSpan>),
    #[cocodec(tag = 7)]
    /// A one-hop weak reference to an element identity.
    Ref(Ref),
    #[cocodec(tag = 8)]
    /// An ordered collection of owning child Values.
    List(Vec<Value>),
    #[cocodec(tag = 9)]
    /// A canonical sorted mapping from unique keys to owning Values.
    Map(BTreeMap<String, Value>),
}
impl Eq for Body {}

#[derive(Debug)]
struct Node {
    id: ElementId,
    body: Body,
    index: OnceLock<Index>,
}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.body == other.body
    }
}
impl Eq for Node {}
impl Encode for Node {
    fn encode<W: cocodec::Write>(&self, w: &mut W) -> std::result::Result<(), cocodec::Error> {
        self.id.encode(w)?;
        self.body.encode(w)
    }
}
// Derived lazily per immutable subtree. Parent links avoid duplicating complete
// paths for descendants when an ancestor moves. Neither index is serialized.
#[derive(Debug, Default)]
struct Index {
    parents: BTreeMap<ElementId, (ElementId, Segment)>,
    references: BTreeMap<ElementId, Vec<ElementId>>,
}
impl Index {
    fn build(root: &Value) -> Self {
        let mut index = Self::default();
        root.visit(&mut |value| match value.body() {
            Body::Map(map) => {
                for (key, child) in map {
                    index
                        .parents
                        .insert(child.id(), (value.id(), Segment::Key(key.clone())));
                }
            }
            Body::List(list) => {
                for (position, child) in list.iter().enumerate() {
                    index
                        .parents
                        .insert(child.id(), (value.id(), Segment::Index(position)));
                }
            }
            _ => {}
        });
        root.visit_all(&mut |value| {
            if let Body::Ref(reference) = value.body() {
                index
                    .references
                    .entry(reference.target)
                    .or_default()
                    .push(value.id());
            }
        });
        index
    }
}

/// Immutable, identity-preserving subtree. Clones share storage.
#[derive(Debug, Clone, PartialEq, Eq, Encode)]
#[cocodec(transparent)]
pub struct Value(Arc<Node>);

// Count ownership depth once per Value, rather than once per implementation
// wrapper. Containers are bounded before allocation and map ordering is strict.
impl Decode for Value {
    fn decode<R: cocodec::Read>(
        d: &mut cocodec::Decoder<R>,
    ) -> std::result::Result<Self, cocodec::Error> {
        d.nested(|d| {
            let id = ElementId::decode(d)?;
            let offset = d.offset();
            let invalid = |reason| cocodec::Error::NonCanonical { offset, reason };
            let body = match d.varint()? {
                0 => Body::Null,
                1 => Body::Bool(bool::decode(d)?),
                2 => Body::Int(i64::decode(d)?),
                3 => Body::Float(f64::decode(d)?),
                4 => Body::String(String::decode(d)?),
                5 => Body::Text(String::decode(d)?),
                6 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("rich span limit exceeded"));
                    }
                    let mut spans = Vec::new();
                    for _ in 0..length {
                        spans.push(match d.varint()? {
                            0 => RichSpan::Text {
                                text: String::decode(d)?,
                                attrs: Attrs::decode(d)?,
                            },
                            1 => RichSpan::Embed {
                                value: <Self as Decode>::decode(d)?,
                                attrs: Attrs::decode(d)?,
                            },
                            _ => return Err(invalid("invalid rich span tag")),
                        });
                    }
                    Body::RichText(spans)
                }
                7 => Body::Ref(Ref::decode(d)?),
                8 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("list limit exceeded"));
                    }
                    let mut values = Vec::new();
                    for _ in 0..length {
                        values.push(<Self as Decode>::decode(d)?);
                    }
                    Body::List(values)
                }
                9 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("map limit exceeded"));
                    }
                    let mut values = BTreeMap::new();
                    for _ in 0..length {
                        let key = String::decode(d)?;
                        if values
                            .last_key_value()
                            .is_some_and(|(previous, _)| previous >= &key)
                        {
                            return Err(invalid("map keys must be unique and sorted"));
                        }
                        values.insert(key, <Self as Decode>::decode(d)?);
                    }
                    Body::Map(values)
                }
                _ => return Err(invalid("invalid Value tag")),
            };
            Ok(Self::trusted(id, body))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
/// One Map-key or List-index step in a snapshot-relative Path.
pub enum Segment {
    #[cocodec(tag = 0)]
    /// A Map key.
    Key(String),
    #[cocodec(tag = 1)]
    /// A List index.
    Index(usize),
}
/// A temporary sequence of Map keys and List indexes in one snapshot.
pub type Path = Vec<Segment>;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// An explicit stable identity or snapshot-relative Path.
pub enum Location {
    #[cocodec(tag = 0)]
    /// A stable element identity.
    Id(ElementId),
    #[cocodec(tag = 1)]
    /// A snapshot-relative navigation path.
    Path(Path),
}
impl From<ElementId> for Location {
    fn from(id: ElementId) -> Self {
        Self::Id(id)
    }
}
impl From<Path> for Location {
    fn from(path: Path) -> Self {
        Self::Path(path)
    }
}

impl Value {
    /// Constructs and validates a canonical object.
    pub fn new(body: Body) -> Result<Self> {
        Self::with_id(ElementId::fresh(), body)
    }
    /// Constructs content using an explicit identity allocator for reproducible tests.
    pub fn with_allocator(body: Body, allocator: &mut IdAllocator) -> Result<Self> {
        Self::with_id(allocator.allocate()?, body)
    }
    pub(crate) fn with_id(id: ElementId, body: Body) -> Result<Self> {
        let body = match body {
            Body::Float(n) if n.is_finite() => Body::Float(if n == 0.0 { 0.0 } else { n }),
            Body::Float(_) => {
                return Err(Error::new(ErrorCode::InvalidValue, "float must be finite"))
            }
            Body::RichText(mut spans) => {
                for span in &mut spans {
                    let attrs = match span {
                        RichSpan::Text { attrs, .. } | RichSpan::Embed { attrs, .. } => attrs,
                    };
                    for attr in attrs.values_mut() {
                        if let Attr::Float(value) = attr {
                            if *value == 0.0 {
                                *value = 0.0;
                            }
                        }
                    }
                }
                Body::RichText(normalize_spans(spans)?)
            }
            other => other,
        };
        let value = Self(Arc::new(Node {
            id,
            body,
            index: OnceLock::new(),
        }));
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn trusted(id: ElementId, body: Body) -> Self {
        Self(Arc::new(Node {
            id,
            body,
            index: OnceLock::new(),
        }))
    }
    /// Creates a new Null element.
    pub fn null() -> Self {
        Self::trusted(ElementId::fresh(), Body::Null)
    }
    /// Creates a new Bool element.
    pub fn bool(value: bool) -> Self {
        Self::trusted(ElementId::fresh(), Body::Bool(value))
    }
    /// Creates a new signed 64-bit Int element.
    pub fn int(value: i64) -> Self {
        Self::trusted(ElementId::fresh(), Body::Int(value))
    }
    /// Creates a finite Float, canonicalizing negative zero.
    pub fn float(value: f64) -> Result<Self> {
        Self::new(Body::Float(value))
    }
    /// Creates a new atomic String element, rejecting content over 16 MiB.
    pub fn string(value: impl Into<String>) -> Result<Self> {
        Self::new(Body::String(value.into()))
    }
    /// Creates a new collaboratively editable Text element, rejecting content over 16 MiB.
    pub fn text(value: impl Into<String>) -> Result<Self> {
        Self::new(Body::Text(value.into()))
    }
    /// Creates a new weak atomic Ref without requiring its target to exist.
    pub fn reference(target: ElementId) -> Self {
        Self::trusted(ElementId::fresh(), Body::Ref(Ref { target }))
    }
    /// Creates a List from owning children, rejecting duplicate identities.
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
    /// Returns this element's stable owning identity.
    pub fn id(&self) -> ElementId {
        self.0.id
    }
    /// Borrows the immutable typed content of this element.
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
            Body::Ref(_) => "ref",
            Body::List(_) => "list",
            Body::Map(_) => "map",
        }
    }
    /// Looks up a Path or stable ID; missing or incompatible locations return an error.
    pub fn get(&self, location: impl Into<Location>) -> Result<Value> {
        match location.into() {
            Location::Id(id) => self.find(id).cloned().ok_or_else(|| missing(id)),
            Location::Path(path) => {
                let mut value = self;
                for segment in path {
                    value = match (value.body(), segment) {
                        (Body::Map(map), Segment::Key(key)) => map
                            .get(&key)
                            .ok_or_else(|| Error::new(ErrorCode::MissingKey, key))?,
                        (Body::List(list), Segment::Index(index)) => {
                            list.get(index).ok_or_else(|| {
                                Error::new(ErrorCode::OutOfBounds, "list index out of bounds")
                            })?
                        }
                        _ => {
                            return Err(Error::new(
                                ErrorCode::TypeMismatch,
                                "path segment does not match container",
                            ))
                        }
                    }
                }
                Ok(value.clone())
            }
        }
    }
    /// Returns whether the Path or stable ID resolves in this content.
    pub fn has(&self, location: impl Into<Location>) -> bool {
        self.get(location).is_ok()
    }
    /// Returns the stable owning ID at an existing Path.
    pub fn id_at(&self, path: Path) -> Result<ElementId> {
        Ok(self.get(path)?.id())
    }
    fn index(&self) -> &Index {
        self.0.index.get_or_init(|| Index::build(self))
    }
    /// Borrows the element identified by ID within this owning subtree.
    pub fn find(&self, id: ElementId) -> Option<&Value> {
        let path = self.path_of(id)?;
        let mut value = self;
        for segment in path {
            value = match (value.body(), segment) {
                (Body::Map(map), Segment::Key(key)) => map.get(&key)?,
                (Body::List(list), Segment::Index(index)) => list.get(index)?,
                _ => return None,
            };
        }
        Some(value)
    }
    /// Resolves one weak Ref hop within this snapshot; dangling targets return None.
    pub fn resolve(&self, reference: Ref) -> Option<Value> {
        self.find(reference.target).cloned()
    }
    /// Returns referring element IDs, including Ref values within atomic embeds.
    pub fn references_to(&self, id: ElementId) -> Vec<ElementId> {
        self.index()
            .references
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }
    /// Derives the current Path from parent links, or returns None for an absent ID.
    pub fn path_of(&self, mut id: ElementId) -> Option<Path> {
        let mut path = Vec::new();
        while id != self.id() {
            let (parent, segment) = self.index().parents.get(&id)?;
            path.push(segment.clone());
            id = *parent;
        }
        path.reverse();
        Some(path)
    }
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(1, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(1, bytes)?;
        value.validate()?;
        Ok(value)
    }
    /// Compares content ignoring owning IDs; Ref target IDs are compared literally.
    pub fn content_equals(&self, other: &Self) -> bool {
        match (self.body(), other.body()) {
            (Body::Map(a), Body::Map(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.get(k).is_some_and(|other| v.content_equals(other)))
            }
            (Body::List(a), Body::List(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.content_equals(b))
            }
            (Body::RichText(a), Body::RichText(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b).all(|(a, b)| match (a, b) {
                        (
                            RichSpan::Text { text: a, attrs: aa },
                            RichSpan::Text { text: b, attrs: ba },
                        ) => a == b && aa == ba,
                        (
                            RichSpan::Embed {
                                value: a,
                                attrs: aa,
                            },
                            RichSpan::Embed {
                                value: b,
                                attrs: ba,
                            },
                        ) => a.content_equals(b) && aa == ba,
                        _ => false,
                    })
            }
            (a, b) => a == b,
        }
    }
    /// Copies all owning identities and remaps Ref targets internal to this subtree.
    pub fn copied(&self) -> Result<Self> {
        self.copy_into(None)
    }
    pub(crate) fn copy_into(&self, root: Option<ElementId>) -> Result<Self> {
        let mut ids = BTreeMap::new();
        self.visit_all(&mut |v| {
            ids.insert(v.id(), ElementId::fresh());
        });
        if let Some(id) = root {
            ids.insert(self.id(), id);
        }
        Ok(remap(self, &ids))
    }
    pub(crate) fn visit(&self, f: &mut impl FnMut(&Value)) {
        f(self);
        match self.body() {
            Body::Map(map) => {
                for v in map.values() {
                    v.visit(f);
                }
            }
            Body::List(list) => {
                for v in list {
                    v.visit(f);
                }
            }
            _ => {}
        }
    }
    pub(crate) fn visit_all(&self, f: &mut impl FnMut(&Value)) {
        f(self);
        match self.body() {
            Body::Map(map) => {
                for v in map.values() {
                    v.visit_all(f);
                }
            }
            Body::List(list) => {
                for v in list {
                    v.visit_all(f);
                }
            }
            Body::RichText(spans) => {
                for span in spans {
                    if let RichSpan::Embed { value, .. } = span {
                        value.visit_all(f);
                    }
                }
            }
            _ => {}
        }
    }
    pub(crate) fn ids(&self) -> BTreeSet<ElementId> {
        let mut result = BTreeSet::new();
        self.visit(&mut |v| {
            result.insert(v.id());
        });
        result
    }
    /// Checks canonical content, identity uniqueness and resource limits.
    pub fn validate(&self) -> Result<()> {
        fn walk(value: &Value, seen: &mut BTreeSet<ElementId>, depth: usize) -> Result<()> {
            if depth > 100 || seen.len() >= 1_000_000 {
                return Err(Error::new(
                    ErrorCode::LimitExceeded,
                    "value depth or node limit exceeded",
                ));
            }
            if !seen.insert(value.id()) {
                return Err(
                    Error::new(ErrorCode::InvalidValue, "duplicate owning element ID")
                        .detail("elementId", value.id().to_string()),
                );
            }
            match value.body() {
                Body::Float(n) if !n.is_finite() || (n.to_bits() == (-0.0f64).to_bits()) => {
                    return Err(Error::new(ErrorCode::InvalidValue, "noncanonical float"))
                }
                Body::String(s) | Body::Text(s) if s.len() > 16 * 1024 * 1024 => {
                    return Err(Error::new(
                        ErrorCode::LimitExceeded,
                        "string exceeds input limit",
                    ))
                }
                Body::Map(map) => {
                    for (key, child) in map {
                        check_string(key)?;
                        walk(child, seen, depth + 1)?;
                    }
                }
                Body::List(list) => {
                    for child in list {
                        walk(child, seen, depth + 1)?;
                    }
                }
                Body::RichText(spans) => {
                    if &normalize_spans(spans.clone())? != spans {
                        return Err(Error::new(
                            ErrorCode::InvalidValue,
                            "noncanonical rich text spans",
                        ));
                    }
                    for span in spans {
                        if let RichSpan::Embed { value, .. } = span {
                            walk(value, seen, depth + 1)?;
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        }
        walk(self, &mut BTreeSet::new(), 1)
    }
    pub(crate) fn replace_at(&self, id: ElementId, replacement: &Value) -> Result<Value> {
        let path = self.path_of(id).ok_or_else(|| missing(id))?;
        fn replace(value: &Value, path: &[Segment], replacement: &Value) -> Value {
            let Some((segment, rest)) = path.split_first() else {
                return replacement.clone();
            };
            let body = match (value.body(), segment) {
                (Body::Map(map), Segment::Key(key)) => {
                    let mut next = map.clone();
                    next.insert(key.clone(), replace(&map[key], rest, replacement));
                    Body::Map(next)
                }
                (Body::List(list), Segment::Index(index)) => {
                    let mut next = list.clone();
                    next[*index] = replace(&list[*index], rest, replacement);
                    Body::List(next)
                }
                _ => unreachable!("derived index matches immutable tree"),
            };
            Value::trusted(value.id(), body)
        }
        Ok(replace(self, &path, replacement))
    }
}

fn remap(value: &Value, ids: &BTreeMap<ElementId, ElementId>) -> Value {
    let body = match value.body() {
        Body::Map(map) => Body::Map(
            map.iter()
                .map(|(k, v)| (k.clone(), remap(v, ids)))
                .collect(),
        ),
        Body::List(list) => Body::List(list.iter().map(|v| remap(v, ids)).collect()),
        Body::Ref(r) => Body::Ref(Ref {
            target: ids.get(&r.target).copied().unwrap_or(r.target),
        }),
        Body::RichText(spans) => Body::RichText(
            spans
                .iter()
                .map(|s| match s {
                    RichSpan::Embed { value, attrs } => RichSpan::Embed {
                        value: remap(value, ids),
                        attrs: attrs.clone(),
                    },
                    other => other.clone(),
                })
                .collect(),
        ),
        other => other.clone(),
    };
    Value::trusted(ids[&value.id()], body)
}
pub(crate) fn missing(id: ElementId) -> Error {
    Error::new(ErrorCode::MissingKey, "element not found").detail("elementId", id.to_string())
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

fn check_string(value: &str) -> Result<()> {
    if value.len() > 16 * 1024 * 1024 {
        return Err(Error::new(
            ErrorCode::LimitExceeded,
            "string exceeds input limit",
        ));
    }
    Ok(())
}
