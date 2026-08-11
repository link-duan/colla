use crate::attrs::{AttrPatch, Attrs};
use crate::change::{Change, ListOp, RichTextOp, TextOp};
use crate::richtext::{char_prefix_bytes, RichContent};
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
pub(super) enum RichContentRef {
    Text { len: usize },
    Embed,
}

impl RichContentRef {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Text { len } => len,
            Self::Embed => 1,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum RichTextOpRef<'a> {
    Retain { len: usize, attrs: &'a AttrPatch },
    Insert { content: RichContentRef },
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
                content: RichContent::Text(_),
                attrs: _,
            } => Some(RichTextOpRef::Insert {
                content: RichContentRef::Text {
                    len: self.remaining,
                },
            }),
            RichTextOp::Insert {
                content: RichContent::Embed(_),
                attrs: _,
            } => Some(RichTextOpRef::Insert {
                content: RichContentRef::Embed,
            }),
            RichTextOp::Delete(_) => Some(RichTextOpRef::Delete(self.remaining)),
        }
    }

    pub(super) fn consume(&mut self, len: usize) {
        assert!(len > 0 && len <= self.remaining);
        if len == self.remaining {
            self.finish_current();
            return;
        }
        match self.ops.get(self.index) {
            Some(RichTextOp::Insert {
                content: RichContent::Text(text),
                ..
            }) => {
                let next_byte_offset = text.byte_offset_after(self.byte_offset, len);
                self.advance_partial_insert(len, next_byte_offset);
            }
            _ => {
                self.offset += len;
                self.remaining -= len;
            }
        }
    }

    pub(super) fn take_insert(&mut self, len: usize) -> Option<(RichContent, Attrs)> {
        assert!(len > 0 && len <= self.remaining);
        let op = self.ops.get(self.index)?;
        match op {
            RichTextOp::Insert {
                content: RichContent::Text(text),
                attrs,
            } => {
                let slice = text.slice_prefix_from(self.byte_offset, len);
                let next_byte_offset = slice.next_byte_offset();
                let result = (RichContent::Text(slice.into_chunk()), attrs.clone());
                if len == self.remaining {
                    self.finish_current();
                } else {
                    self.advance_partial_insert(len, next_byte_offset);
                }
                Some(result)
            }
            RichTextOp::Insert {
                content: RichContent::Embed(value),
                attrs,
            } => {
                assert_eq!(len, 1);
                let result = (RichContent::embed(value.clone()), attrs.clone());
                self.finish_current();
                Some(result)
            }
            _ => None,
        }
    }

    fn advance_partial_insert(&mut self, scalar_len: usize, next_byte_offset: usize) {
        debug_assert!(scalar_len < self.remaining);
        self.byte_offset = next_byte_offset;
        self.remaining -= scalar_len;
    }

    fn finish_current(&mut self) {
        self.index += 1;
        self.reset_current();
        self.load();
    }

    fn reset_current(&mut self) {
        self.offset = 0;
        self.byte_offset = 0;
        self.remaining = 0;
    }

    fn load(&mut self) {
        while let Some(op) = self.ops.get(self.index) {
            self.remaining = match op {
                RichTextOp::Retain { len, .. } | RichTextOp::Delete(len) => {
                    len.saturating_sub(self.offset)
                }
                RichTextOp::Insert {
                    content: RichContent::Text(text),
                    ..
                } => text.len(),
                RichTextOp::Insert {
                    content: RichContent::Embed(_),
                    ..
                } => 1usize.saturating_sub(self.offset),
            };
            if self.remaining > 0 {
                break;
            }
            self.index += 1;
            self.reset_current();
        }
    }
}

pub(super) fn text_prefix(text: &str, len: usize) -> &str {
    &text[..char_prefix_bytes(text, len)]
}
