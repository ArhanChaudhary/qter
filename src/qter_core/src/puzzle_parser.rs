use std::{collections::HashMap, rc::Rc};

use internment::ArcIntern;
use itertools::Itertools;
use pest::{error::Error, Parser};
use pest_derive::Parser;

use crate::{
    architectures::{Architecture, Permutation, PermutationGroup, PuzzleDefinition},
    Int, U,
};

#[derive(Parser)]
#[grammar = "./puzzle.pest"]
struct SpecParser;

pub fn parse(spec: &str) -> Result<PuzzleDefinition, Box<Error<Rule>>> {
    let mut parsed = SpecParser::parse(Rule::description, spec)?
        .next()
        .unwrap()
        .into_inner();

    let colors_pair = parsed.next().unwrap();

    let mut colors_map = HashMap::<String, Vec<usize>>::new();

    let mut min_facelet = usize::MAX;
    let mut max_facelet = usize::MIN;

    let colors_span = colors_pair.as_span();

    for pair in colors_pair.into_inner() {
        let mut pairs = pair.into_inner();
        let color = pairs.next().unwrap().as_str();

        let mut facelets = vec![];

        for pair in pairs {
            let facelet = pair.as_str().parse::<usize>().unwrap();

            if min_facelet > facelet {
                min_facelet = facelet;
            }

            if max_facelet < facelet {
                max_facelet = facelet;
            }

            facelets.push(facelet);
        }

        colors_map.insert(color.to_owned(), facelets);
    }

    let empty_string = ArcIntern::new(String::new());
    let mut colors = vec![empty_string; max_facelet - min_facelet + 1];

    // Make facelets zero based
    for (color, facelets) in colors_map {
        let color = ArcIntern::new(color);

        for facelet in facelets {
            colors[facelet - min_facelet] = ArcIntern::clone(&color);
        }
    }

    for color in colors.iter() {
        if color.is_empty() {
            return Err(Box::new(Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: "Didn't define the color for every facelet".to_owned(),
                },
                colors_span,
            )));
        }
    }

    let generators_pair = parsed.next().unwrap().into_inner();

    let mut generators = HashMap::new();

    for pair in generators_pair {
        let mut pairs = pair.into_inner();

        let name = pairs.next().unwrap().as_str();

        let mut cycles = vec![];

        for cycle_pair in pairs {
            let mut cycle = vec![];

            for value in cycle_pair.into_inner() {
                let facelet_span = value.as_span();
                let facelet = value.as_str().parse::<usize>().unwrap();

                if min_facelet > facelet || max_facelet < facelet {
                    return Err(Box::new(Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: "Facelet is out of range".to_owned(),
                        },
                        facelet_span,
                    )));
                }

                cycle.push(facelet - min_facelet);
            }

            cycles.push(cycle);
        }

        let mut permutation = Permutation::from_cycles(cycles);

        permutation.facelet_count = max_facelet - min_facelet + 1;

        generators.insert(ArcIntern::from_ref(name), permutation);
    }

    let derived_pair = parsed.next().unwrap().into_inner();

    for pair in derived_pair {
        let mut pairs = pair.into_inner();

        let name = pairs.next().unwrap().as_str();

        let permutation_name = pairs.next().unwrap();
        let mut permutation = match generators.get(&ArcIntern::from_ref(permutation_name.as_str()))
        {
            Some(v) => v.to_owned(),
            None => {
                return Err(Box::new(Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: format!(
                            "The permutation {} doesn't exist",
                            permutation_name.as_str(),
                        ),
                    },
                    permutation_name.as_span(),
                )))
            }
        };

        for pair in pairs {
            let next_permutation = match generators.get(&ArcIntern::from_ref(pair.as_str())) {
                Some(v) => v,
                None => {
                    return Err(Box::new(Error::new_from_span(
                        pest::error::ErrorVariant::CustomError {
                            message: format!(
                                "The permutation {} doesn't exist",
                                permutation_name.as_str(),
                            ),
                        },
                        permutation_name.as_span(),
                    )))
                }
            };

            permutation.compose(next_permutation);
        }

        generators.insert(ArcIntern::from_ref(name), permutation);
    }

    let group = Rc::new(PermutationGroup::new(colors, generators));

    let presets_pairs = parsed.next().unwrap().into_inner();

    let mut presets = vec![];

    for preset_pair in presets_pairs {
        let algorithm_span = preset_pair.as_span();
        let mut preset_pairs = preset_pair.into_inner();

        let orders_pair = preset_pairs.next().unwrap();
        let mut orders = vec![];

        for order in orders_pair.into_inner() {
            orders.push(order.as_str().parse::<Int<U>>().unwrap());
        }

        let mut algorithms = vec![];

        for algorithm_pair in preset_pairs {
            let mut moves = vec![];

            for action in algorithm_pair.into_inner() {
                moves.push(ArcIntern::from_ref(action.as_str()));
            }

            algorithms.push(moves);
        }

        let architecture = match Architecture::new(Rc::clone(&group), algorithms) {
            Ok(v) => Rc::new(v),
            Err(e) => {
                return Err(Box::new(Error::new_from_span(
                    pest::error::ErrorVariant::CustomError {
                        message: format!("Generator doesn't exist: {e}"),
                    },
                    algorithm_span,
                )))
            }
        };

        for (register, order) in architecture.registers().iter().zip(orders.into_iter()) {
            if register.order() != order {
                return Err(Box::new(Error::new_from_span(
                    pest::error::ErrorVariant::CustomError { message: format!("The algorithm {} has an incorrect order. Expected order {order} but found order {}.", register.generator_sequence.iter().join(" "), register.order()) },
                    algorithm_span,
                )));
            }
        }

        presets.push(architecture);
    }

    Ok(PuzzleDefinition { group, presets })
}
