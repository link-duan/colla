use super::{Error, ErrorCode, Result};
use cocodec::{Decode, Encode, WriteExt};
use std::{cell::RefCell, fmt, str::FromStr};

/// Stable identity, independent of content, location, and request identity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementId {
    namespace: [u8; 16],
    sequence: u64,
}

impl ElementId {
    /// Returns the independent 128-bit allocation namespace.
    pub fn namespace(self) -> [u8; 16] {
        self.namespace
    }
    /// Returns the positive monotonic sequence number.
    pub fn sequence(self) -> u64 {
        self.sequence
    }
    /// Allocates from this thread's allocator, creating its random namespace on
    /// first use. Entropy failure and sequence exhaustion are reported as errors.
    pub(crate) fn try_fresh() -> Result<Self> {
        ALLOCATOR.with(|current| {
            let mut current = current.borrow_mut();
            match current.as_mut() {
                Some(allocator) => allocator.allocate(),
                None => {
                    let mut allocator = IdAllocator::random()?;
                    let id = allocator.allocate();
                    *current = Some(allocator);
                    id
                }
            }
        })
    }
    /// Infallible variant for constructors whose public signatures cannot fail.
    /// Panics only when the platform has no secure entropy source or after
    /// 2^64 allocations in one namespace.
    pub(crate) fn fresh() -> Self {
        Self::try_fresh().unwrap_or_else(|error| panic!("element ID allocation failed: {error}"))
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.namespace {
            write!(f, "{byte:02x}")?;
        }
        write!(f, "{:016x}", self.sequence)
    }
}
impl fmt::Debug for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl FromStr for ElementId {
    type Err = Error;
    fn from_str(input: &str) -> Result<Self> {
        if input.len() != 48
            || !input
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::new(
                ErrorCode::InvalidValue,
                "expected a canonical 48-digit element ID",
            ));
        }
        let mut namespace = [0; 16];
        for (i, byte) in namespace.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&input[i * 2..i * 2 + 2], 16).unwrap();
        }
        let sequence = u64::from_str_radix(&input[32..], 16).unwrap();
        if sequence == 0 {
            return Err(Error::new(
                ErrorCode::InvalidValue,
                "element ID sequence must be positive",
            ));
        }
        Ok(Self {
            namespace,
            sequence,
        })
    }
}
impl Encode for ElementId {
    fn encode<W: cocodec::Write>(&self, w: &mut W) -> std::result::Result<(), cocodec::Error> {
        w.write_bytes(&self.namespace)?;
        self.sequence.encode(w)
    }
}
impl Decode for ElementId {
    fn decode<R: cocodec::Read>(
        d: &mut cocodec::Decoder<R>,
    ) -> std::result::Result<Self, cocodec::Error> {
        let offset = d.offset();
        let namespace = d
            .bytes()?
            .try_into()
            .map_err(|_| cocodec::Error::NonCanonical {
                offset,
                reason: "ID namespace must contain 16 bytes",
            })?;
        let sequence = u64::decode(d)?;
        if sequence == 0 {
            return Err(cocodec::Error::NonCanonical {
                offset,
                reason: "ID sequence must be positive",
            });
        }
        Ok(Self {
            namespace,
            sequence,
        })
    }
}

/// Monotonic allocation. Aborted edits never roll this allocator back.
pub struct IdAllocator {
    namespace: [u8; 16],
    next: Option<u64>,
}
impl IdAllocator {
    /// Creates an allocator with a fresh namespace from the platform secure random source.
    pub fn random() -> Result<Self> {
        let mut namespace = [0; 16];
        getrandom::getrandom(&mut namespace).map_err(|e| {
            Error::new(
                ErrorCode::InvalidState,
                format!("secure element ID entropy unavailable: {e}"),
            )
        })?;
        Ok(Self::deterministic(namespace))
    }
    /// Creates an allocator in an explicit test namespace, starting at sequence one.
    pub fn deterministic(namespace: [u8; 16]) -> Self {
        Self {
            namespace,
            next: Some(1),
        }
    }
    /// Runs synchronous construction/editing with this thread's allocator overridden.
    /// Allocated sequences remain consumed even when the callback fails or unwinds.
    /// Use a distinct deterministic namespace per test or a fresh random allocator.
    pub fn scope<T>(&mut self, callback: impl FnOnce() -> T) -> T {
        struct Restore<'a> {
            allocator: &'a mut IdAllocator,
            outer: Option<IdAllocator>,
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                ALLOCATOR.with(|current| {
                    let inner = std::mem::replace(&mut *current.borrow_mut(), self.outer.take());
                    if let Some(inner) = inner {
                        *self.allocator = inner;
                    }
                });
            }
        }
        let inner = std::mem::replace(self, Self::exhausted());
        let outer = ALLOCATOR.with(|current| current.borrow_mut().replace(inner));
        let _restore = Restore {
            allocator: self,
            outer,
        };
        callback()
    }
    fn exhausted() -> Self {
        Self {
            namespace: [0; 16],
            next: None,
        }
    }
    /// Allocates the next ID without reuse; namespace exhaustion returns an error.
    pub fn allocate(&mut self) -> Result<ElementId> {
        let sequence = self
            .next
            .ok_or_else(|| Error::new(ErrorCode::LimitExceeded, "element ID sequence exhausted"))?;
        self.next = sequence.checked_add(1);
        Ok(ElementId {
            namespace: self.namespace,
            sequence,
        })
    }
}
thread_local! {
    // Created lazily so entropy failure surfaces as an error from fallible constructors.
    static ALLOCATOR: RefCell<Option<IdAllocator>> = const { RefCell::new(None) };
}
