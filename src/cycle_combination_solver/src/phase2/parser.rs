//! A parser used to parse scramble sequences into a Rust representation.

use super::cube::*;
use regex::Regex;
use std::str::FromStr;

fn parse_base_move(token: &str) -> Result<BaseMoveToken, strum::ParseError> {
    BaseMoveToken::from_str(token)
}

/// Parses a scramble sequence from a string.
///
/// Returns a Result object indicating whether the parse was successful.
pub fn parse_scramble(scramble: &str) -> Result<Vec<MoveInstance>, strum::ParseError> {
    let mut parsed = vec![];
    let re_normal = Regex::new(r"^([UDLRFB])$").unwrap();
    let re_prime = Regex::new(r"^([UDLRFB])'").unwrap();
    let re_double = Regex::new(r"^([UDLRFB])2").unwrap();
    let mut parse_error = "";
    'tokens: for token in scramble.split_whitespace() {
        if re_normal.is_match(token) {
            for cap in re_normal.captures_iter(token) {
                let basemove = parse_base_move(&cap[1])?;
                parsed.push(MoveInstance {
                    basemove,
                    dir: Direction::Normal,
                })
            }
        } else if re_prime.is_match(token) {
            for cap in re_prime.captures_iter(token) {
                let basemove = parse_base_move(&cap[1])?;
                parsed.push(MoveInstance {
                    basemove,
                    dir: Direction::Prime,
                })
            }
        } else if re_double.is_match(token) {
            for cap in re_double.captures_iter(token) {
                let basemove = parse_base_move(&cap[1])?;
                parsed.push(MoveInstance {
                    basemove,
                    dir: Direction::Double,
                })
            }
        } else {
            parse_error = token;
            break 'tokens;
        }
    }
    if parse_error.is_empty() {
        Ok(parsed)
    } else {
        Err(strum::ParseError::VariantNotFound)
    }
}
