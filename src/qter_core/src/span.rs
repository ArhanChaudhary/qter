use std::{
    ops::{Deref, DerefMut},
    sync::OnceLock,
};

use internment::ArcIntern;

/// A slice of the original source code; to be attached to pieces of data for error reporting
#[derive(Clone)]
pub struct Span {
    source: ArcIntern<str>,
    start: usize,
    end: usize,
    line_and_col: OnceLock<(usize, usize)>,
}

impl Span {
    /// Creates a new `Span` from the given source and start/end positions
    ///
    /// # Panics
    ///
    /// Panics if the start or end positions are out of bounds, or if the start is greater than the end
    #[must_use]
    pub fn new(source: ArcIntern<str>, start: usize, end: usize) -> Span {
        assert!(start <= end);
        assert!(start < source.len());
        assert!(end <= source.len());

        Span {
            source,
            start,
            end,
            line_and_col: OnceLock::new(),
        }
    }

    pub fn slice(&self) -> &str {
        &self.source[self.start..self.end]
    }

    pub fn line_and_col(&self) -> (usize, usize) {
        *self.line_and_col.get_or_init(|| {
            let mut current_line = 1;
            let mut current_col = 1;

            for c in self.source.chars().take(self.start) {
                if c == '\n' {
                    current_line += 1;
                    current_col = 1;
                } else {
                    current_col += 1;
                }
            }

            (current_line, current_col)
        })
    }

    pub fn line(&self) -> usize {
        self.line_and_col().0
    }

    pub fn col(&self) -> usize {
        self.line_and_col().1
    }

    #[must_use]
    pub fn after(mut self) -> Span {
        self.start = self.end;
        self
    }

    /// Merges two spans into one, keeping the earliest start and latest end
    ///
    /// # Panics
    ///
    /// Panics if the two spans are from different sources
    #[must_use]
    pub fn merge(self, other: &Span) -> Span {
        assert_eq!(self.source, other.source);

        Span {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line_and_col: OnceLock::new(),
        }
    }
}

impl ariadne::Span for Span {
    type SourceId = ArcIntern<str>;

    fn source(&self) -> &Self::SourceId {
        &self.source
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slice())
    }
}

/// A value attached to a `Span`
#[derive(Clone)]
pub struct WithSpan<T> {
    pub value: T,
    span: Span,
}

impl<T: core::fmt::Debug> core::fmt::Debug for WithSpan<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(&self.value, f)
    }
}

impl<T> Deref for WithSpan<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for WithSpan<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> WithSpan<T> {
    pub fn new(value: T, span: Span) -> WithSpan<T> {
        WithSpan { value, span }
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn map<V>(self, f: impl FnOnce(T) -> V) -> WithSpan<V> {
        WithSpan {
            value: f(self.value),
            span: self.span,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn line(&self) -> usize {
        self.span().line()
    }
}

impl<T: PartialEq> PartialEq for WithSpan<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for WithSpan<T> {}

impl<T: core::hash::Hash> core::hash::Hash for WithSpan<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}
