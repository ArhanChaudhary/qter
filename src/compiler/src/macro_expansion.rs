use pest::error::Error;

use crate::{Expanded, ParsedSyntax};

use super::parsing::Rule;

pub fn expand(parsed: ParsedSyntax) -> Result<Expanded, Box<Error<Rule>>> {
    todo!()
}
