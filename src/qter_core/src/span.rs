use std::{ops::{Deref, DerefMut}, sync::OnceLock};

use internment::ArcIntern;
use pest::{Position, RuleType};

pub fn mk_error<Rule: RuleType>(
    message: impl Into<String>,
    loc: impl AsPestLoc,
) -> Box<pest::error::Error<Rule>> {
    let err = pest::error::ErrorVariant::CustomError {
        message: message.into(),
    };

    Box::new(match loc.as_pest_loc() {
        SpanOrPos::Span(span) => pest::error::Error::new_from_span(err, span),
        SpanOrPos::Pos(pos) => pest::error::Error::new_from_pos(err, pos),
    })
}

pub enum SpanOrPos<'a> {
    Span(pest::Span<'a>),
    Pos(pest::Position<'a>),
}

pub trait AsPestLoc {
    fn as_pest_loc(&self) -> SpanOrPos<'_>;
}

impl AsPestLoc for pest::Span<'_> {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Span(self.to_owned())
    }
}

impl AsPestLoc for Span {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Span(self.pest())
    }
}

impl AsPestLoc for Position<'_> {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        SpanOrPos::Pos(self.to_owned())
    }
}

impl<T: AsPestLoc> AsPestLoc for &T {
    fn as_pest_loc(&self) -> SpanOrPos<'_> {
        (*self).as_pest_loc()
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
    pub fn from_span(span: pest::Span) -> Span {
        Span::new(ArcIntern::from(span.get_input()), span.start(), span.end())
    }

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

    pub fn after(mut self) -> Span {
        self.start = self.end;
        self
    }

    pub fn source(&self) -> ArcIntern<str> {
        ArcIntern::clone(&self.source)
    }

    pub fn merge(self, other: &Span) -> Span {
        assert_eq!(self.source, other.source);

        Span {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line_and_col: OnceLock::new(),
        }
    }

    fn pest(&self) -> pest::Span<'_> {
        pest::Span::new(&self.source, self.start, self.end).unwrap()
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slice())
    }
}

impl From<pest::Span<'_>> for Span {
    fn from(value: pest::Span) -> Self {
        Span::from_span(value)
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
        self.value.hash(state)
    }
}
