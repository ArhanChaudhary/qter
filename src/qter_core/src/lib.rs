#![warn(clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_panics_doc
)]

pub mod architectures;
mod shared_facelet_detection;
pub mod table_encoding;

mod span;
pub use span::*;

mod runtime;
pub use runtime::*;

mod math;
pub use math::*;
