use crate::attrs::{AttrPatch, Attrs};
use crate::change::{Change, ListOp, RichTextOp, TextOp};
use crate::richtext::RichInsert;
use crate::Value;

#[derive(Clone, Copy)]
pub(super) enum TextOpRef<'a> {
    Retain(usize),
    Insert { text: &'a str, len: usize },
    Delete(usize),
}

pub(super) struct TextOpReader<'a> {
    ops: &'a [TextOp],
    index: usize,
    offset: usize,
    byte_offset: usize,
    remaining: usize,
}

impl<'a> TextOpReader<'a> {
    pub(super) fn new(ops: &'a [TextOp]) -> Self {
        let mut reader = Self {
            ops,
            index: 0,
            offset: 0,
            byte_offset: 0,
            remaining: 0,
        };
        reader.load();
        reader
    }

    pub(super) fn peek(&self) -> Option<TextOpRef<'a>> {
        match self.ops.get(self.index)? {
            TextOp::Retain(_) => Some(TextOpRef::Retain(self.remaining)),
            TextOp::Insert(text) => Some(TextOpRef::Insert {
                text: &text[self.byte_offset..],
                len: self.remaining,
            }),
            TextOp::Delete(_) => Some(TextOpRef::Delete(self.remaining)),
        }
    }

    pub(super) fn consume(&mut self, len: usize) {
        assert!(len > 0 && len <= self.remaining);
        if len == self.remaining {
            self.index += 1;
            self.offset = 0;
            self.byte_offset = 0;
            self.remaining = 0;
            self.load();
            return;
        }
        if let Some(TextOp::Insert(text)) = self.ops.get(self.index) {
            let rest = &text[self.byte_offset..];
            self.byte_offset += char_prefix_bytes(rest, len);
        } else {
            self.offset += len;
        }
        self.remaining -= len;
    }

    fn load(&mut self) {
        while let Some(op) = self.ops.get(self.index) {
            self.remaining = match op {
                TextOp::Retain(len) | TextOp::Delete(len) => len.saturating_sub(self.offset),
                TextOp::Insert(text) => text[self.byte_offset..].chars().count(),
            };
            if self.remaining > 0 {
                break;
            }
            self.index += 1;
            self.offset = 0;
            self.byte_offset = 0;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum ListOpRef<'a> {
    Retain(usize),
    Insert(&'a [Value]),
    Delete(usize),
    Modify(&'a Change),
}

pub(super) struct ListOpReader<'a> {
    ops: &'a [ListOp],
    index: usize,
    offset: usize,
    remaining: usize,
}

impl<'a> ListOpReader<'a> {
    pub(super) fn new(ops: &'a [ListOp]) -> Self {
        let mut reader = Self {
            ops,
            index: 0,
            offset: 0,
            remaining: 0,
        };
        reader.load();
        reader
    }

    pub(super) fn peek(&self) -> Option<ListOpRef<'a>> {
        match self.ops.get(self.index)? {
            ListOp::Retain(_) => Some(ListOpRef::Retain(self.remaining)),
            ListOp::Insert(values) => Some(ListOpRef::Insert(&values[self.offset..])),
            ListOp::Delete(_) => Some(ListOpRef::Delete(self.remaining)),
            ListOp::Modify(change) => Some(ListOpRef::Modify(change)),
        }
    }

    pub(super) fn consume(&mut self, len: usize) {
        assert!(len > 0 && len <= self.remaining);
        self.offset += len;
        self.remaining -= len;
        if self.remaining == 0 {
            self.index += 1;
            self.offset = 0;
            self.load();
        }
    }

    fn load(&mut self) {
        while let Some(op) = self.ops.get(self.index) {
            self.remaining = match op {
                ListOp::Retain(len) | ListOp::Delete(len) => len.saturating_sub(self.offset),
                ListOp::Insert(values) => values.len().saturating_sub(self.offset),
                ListOp::Modify(_) => 1usize.saturating_sub(self.offset),
            };
            if self.remaining > 0 {
                break;
            }
            self.index += 1;
            self.offset = 0;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum RichInsertRef<'a> {
    Text { text: &'a str, len: usize },
    Embed(&'a Value),
}

impl RichInsertRef<'_> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Text { len, .. } => len,
            Self::Embed(_) => 1,
        }
    }

    pub(super) fn prefix(self, len: usize) -> RichInsert {
        match self {
            Self::Text { text, .. } => RichInsert::text(&text[..char_prefix_bytes(text, len)]),
            Self::Embed(value) => {
                assert_eq!(len, 1);
                RichInsert::embed(value.clone())
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum RichTextOpRef<'a> {
    Retain {
        len: usize,
        attrs: &'a AttrPatch,
    },
    Insert {
        content: RichInsertRef<'a>,
        attrs: &'a Attrs,
    },
    Delete(usize),
}

pub(super) struct RichTextOpReader<'a> {
    ops: &'a [RichTextOp],
    index: usize,
    offset: usize,
    byte_offset: usize,
    remaining: usize,
}

impl<'a> RichTextOpReader<'a> {
    pub(super) fn new(ops: &'a [RichTextOp]) -> Self {
        let mut reader = Self {
            ops,
            index: 0,
            offset: 0,
            byte_offset: 0,
            remaining: 0,
        };
        reader.load();
        reader
    }

    pub(super) fn peek(&self) -> Option<RichTextOpRef<'a>> {
        match self.ops.get(self.index)? {
            RichTextOp::Retain { attrs, .. } => Some(RichTextOpRef::Retain {
                len: self.remaining,
                attrs,
            }),
            RichTextOp::Insert {
                content: RichInsert::Text(text),
                attrs,
            } => Some(RichTextOpRef::Insert {
                content: RichInsertRef::Text {
                    text: &text[self.byte_offset..],
                    len: self.remaining,
                },
                attrs,
            }),
            RichTextOp::Insert {
                content: RichInsert::Embed(value),
                attrs,
            } => Some(RichTextOpRef::Insert {
                content: RichInsertRef::Embed(value),
                attrs,
            }),
            RichTextOp::Delete(_) => Some(RichTextOpRef::Delete(self.remaining)),
        }
    }

    pub(super) fn consume(&mut self, len: usize) {
        assert!(len > 0 && len <= self.remaining);
        if len == self.remaining {
            self.index += 1;
            self.offset = 0;
            self.byte_offset = 0;
            self.remaining = 0;
            self.load();
            return;
        }
        match self.ops.get(self.index) {
            Some(RichTextOp::Insert {
                content: RichInsert::Text(text),
                ..
            }) => {
                let rest = &text[self.byte_offset..];
                self.byte_offset += char_prefix_bytes(rest, len);
            }
            _ => self.offset += len,
        }
        self.remaining -= len;
    }

    fn load(&mut self) {
        while let Some(op) = self.ops.get(self.index) {
            self.remaining = match op {
                RichTextOp::Retain { len, .. } | RichTextOp::Delete(len) => {
                    len.saturating_sub(self.offset)
                }
                RichTextOp::Insert {
                    content: RichInsert::Text(text),
                    ..
                } => text[self.byte_offset..].chars().count(),
                RichTextOp::Insert {
                    content: RichInsert::Embed(_),
                    ..
                } => 1usize.saturating_sub(self.offset),
            };
            if self.remaining > 0 {
                break;
            }
            self.index += 1;
            self.offset = 0;
            self.byte_offset = 0;
        }
    }
}

pub(super) fn text_prefix(text: &str, len: usize) -> &str {
    &text[..char_prefix_bytes(text, len)]
}

fn char_prefix_bytes(text: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    text.char_indices()
        .nth(len)
        .map_or(text.len(), |(index, _)| index)
}
