use std::sync::Arc;

use crate::attrs::Attrs;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RichInsert {
    Text(Arc<str>),
    Embed(Value),
}

impl RichInsert {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(Arc::from(value.into()))
    }
    pub fn embed(value: Value) -> Self {
        Self::Embed(value)
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Text(value) => value.chars().count(),
            Self::Embed(_) => 1,
        }
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Text(value) if value.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RichSpan {
    pub content: RichInsert,
    pub attrs: Attrs,
}

impl RichSpan {
    pub fn text(value: impl Into<String>, attrs: Attrs) -> Self {
        Self {
            content: RichInsert::text(value),
            attrs,
        }
    }
    pub fn embed(value: Value, attrs: Attrs) -> Self {
        Self {
            content: RichInsert::embed(value),
            attrs,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RichText(Arc<Vec<RichSpan>>);

impl RichText {
    pub fn new(spans: Vec<RichSpan>) -> Self {
        Self(Arc::new(normalize_spans(spans)))
    }
    pub(crate) fn from_canonical(spans: Vec<RichSpan>) -> Self {
        Self(Arc::new(spans))
    }
    pub fn spans(&self) -> &[RichSpan] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.iter().map(|span| span.content.len()).sum()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn to_plain_string(&self) -> String {
        self.0
            .iter()
            .filter_map(|span| match &span.content {
                RichInsert::Text(text) => Some(text.as_ref()),
                RichInsert::Embed(_) => None,
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RichAtom {
    pub content: RichInsert,
    pub attrs: Attrs,
}

pub(crate) fn flatten(spans: &[RichSpan]) -> Vec<RichAtom> {
    let mut out = Vec::new();
    for span in spans {
        match &span.content {
            RichInsert::Text(text) => {
                for ch in text.chars() {
                    out.push(RichAtom {
                        content: RichInsert::text(ch.to_string()),
                        attrs: span.attrs.clone(),
                    });
                }
            }
            RichInsert::Embed(value) => out.push(RichAtom {
                content: RichInsert::Embed(value.clone()),
                attrs: span.attrs.clone(),
            }),
        }
    }
    out
}

pub(crate) fn collapse(atoms: Vec<RichAtom>) -> RichText {
    let spans = atoms
        .into_iter()
        .map(|atom| RichSpan {
            content: atom.content,
            attrs: atom.attrs,
        })
        .collect();
    RichText::new(spans)
}

fn normalize_spans(spans: Vec<RichSpan>) -> Vec<RichSpan> {
    let mut out: Vec<RichSpan> = Vec::new();
    for span in spans {
        if span.content.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.attrs == span.attrs {
                if let (RichInsert::Text(left), RichInsert::Text(right)) =
                    (&mut last.content, &span.content)
                {
                    let mut merged = String::from(left.as_ref());
                    merged.push_str(right);
                    *left = Arc::from(merged);
                    continue;
                }
            }
        }
        out.push(span);
    }
    out
}
