use std::{
    ops::{Deref, DerefMut},
    sync::OnceLock,
};

use chumsky::{container::Container, extra::Full, input::ValueInput, prelude::*};
use internment::ArcIntern;

pub type Extra = Full<Rich<'static, char, Span>, (), ()>;

#[derive(Clone)]
pub struct File(ArcIntern<str>);

impl File {
    #[must_use]
    pub fn inner(&self) -> ArcIntern<str> {
        ArcIntern::clone(&self.0)
    }
}

impl<T: Into<ArcIntern<str>>> From<T> for File {
    fn from(value: T) -> Self {
        File(value.into())
    }
}

impl Input<'_> for File {
    type Span = Span;
    type Token = char;
    type MaybeToken = char;
    type Cursor = usize;
    type Cache = Self;

    fn begin(self) -> (Self::Cursor, Self::Cache) {
        (0, self)
    }

    fn cursor_location(cursor: &Self::Cursor) -> usize {
        *cursor
    }

    unsafe fn next_maybe(
        this: &mut Self::Cache,
        cursor: &mut Self::Cursor,
    ) -> Option<Self::MaybeToken> {
        let c = this.0.get(*cursor..)?.chars().next()?;

        *cursor += c.len_utf8();

        Some(c)
    }

    unsafe fn span(this: &mut Self::Cache, range: std::ops::Range<&Self::Cursor>) -> Self::Span {
        Span::new(ArcIntern::clone(&this.0), *range.start, *range.end)
    }
}

impl ValueInput<'_> for File {
    unsafe fn next(cache: &mut Self::Cache, cursor: &mut Self::Cursor) -> Option<Self::Token> {
        // SAFETY: Guarantees required by this are upheld by the caller
        unsafe { File::next_maybe(cache, cursor) }
    }
}

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
        assert!(start <= source.len());
        assert!(end <= source.len());

        Span {
            source,
            start,
            end,
            line_and_col: OnceLock::new(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn from_static(str: &'static str) -> Span {
        Span::new(ArcIntern::from(str), 0, str.len())
    }

    pub fn slice(&self) -> &str {
        &self.source[self.start..self.end]
    }

    pub fn line_and_col(&self) -> (usize, usize) {
        *self.line_and_col.get_or_init(|| {
            let mut current_line = 1;
            let mut current_col = 1;

            let mut taken = 0;

            for c in self.source.chars() {
                if taken > self.start() {
                    break;
                }

                taken += c.len_utf8();

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

    pub fn source(&self) -> ArcIntern<str> {
        self.source.clone()
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

    pub fn with<T>(self, v: T) -> WithSpan<T> {
        WithSpan::new(v, self)
    }
}

impl AsRef<str> for Span {
    fn as_ref(&self) -> &str {
        self.slice()
    }
}

impl ariadne::Span for Span {
    type SourceId = ();

    fn source(&self) -> &Self::SourceId {
        &()
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl chumsky::span::Span for Span {
    type Context = ArcIntern<str>;
    type Offset = usize;

    fn new(source: Self::Context, range: std::ops::Range<Self::Offset>) -> Self {
        Span::new(source, range.start, range.end)
    }

    fn context(&self) -> Self::Context {
        self.source()
    }

    fn start(&self) -> Self::Offset {
        ariadne::Span::start(self)
    }

    fn end(&self) -> Self::Offset {
        ariadne::Span::end(self)
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slice())
    }
}

impl core::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start(), self.end())
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

impl<T> WithSpan<MaybeErr<T>> {
    pub fn spanspose(self) -> MaybeErr<WithSpan<T>> {
        self.value.map(|v| self.span.with(v))
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

pub enum MaybeErr<T> {
    Some(T),
    None,
}

impl<T> MaybeErr<T> {
    pub fn map<X>(self, f: impl FnOnce(T) -> X) -> MaybeErr<X> {
        match self {
            MaybeErr::Some(v) => MaybeErr::Some(f(v)),
            MaybeErr::None => MaybeErr::None,
        }
    }

    pub fn option(self) -> Option<T> {
        match self {
            MaybeErr::Some(v) => Some(v),
            MaybeErr::None => None,
        }
    }
}

impl<T> MaybeErr<MaybeErr<T>> {
    pub fn flatten(self) -> MaybeErr<T> {
        match self {
            MaybeErr::Some(v) => v,
            MaybeErr::None => MaybeErr::None,
        }
    }
}

impl<X, T: Container<X>> Container<MaybeErr<X>> for MaybeErr<T> {
    fn push(&mut self, item: MaybeErr<X>) {
        match (self, item) {
            (MaybeErr::Some(container), MaybeErr::Some(item)) => container.push(item),
            (this, _) => *this = MaybeErr::None,
        }
    }
}

impl<T: Default> Default for MaybeErr<T> {
    fn default() -> Self {
        MaybeErr::Some(T::default())
    }
}
