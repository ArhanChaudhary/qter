mod puzzles;
pub use puzzles::*;

mod puzzle_geometry;
pub use puzzle_geometry::*;

mod defaults;
mod options;

type PuzzleDescriptionString<'a> = &'a str;