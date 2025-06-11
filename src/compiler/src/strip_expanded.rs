use std::{collections::HashMap, iter::Peekable, sync::Arc};

use internment::ArcIntern;
use itertools::Itertools;
use pest::error::Error;
use qter_core::{
    ByPuzzleType, Halt, Input, Instruction, Int, Print, Program, PuzzleIdx, RegisterGenerator,
    TheoreticalIdx, U, WithSpan,
    architectures::{Algorithm, Architecture, CycleGeneratorSubcycle, PermutationGroup},
    mk_error,
};

use crate::{
    BlockID, ExpandedCode, ExpandedCodeComponent, LabelReference, Primitive, Puzzle,
    RegisterReference, parsing::Rule,
};

#[derive(Clone, Debug)]
enum RegisterIdx {
    Theoretical,
    Real {
        idx: usize,
        arch: Arc<Architecture>,
        modulus: Option<Int<U>>,
    },
}

impl RegisterIdx {
    fn generator(&self) -> ByPuzzleType<RegisterGenerator> {
        match self {
            RegisterIdx::Theoretical => ByPuzzleType::Theoretical(()),
            &RegisterIdx::Real {
                idx,
                ref arch,
                modulus: _,
            } => ByPuzzleType::Puzzle((
                Algorithm::new_from_effect(arch, vec![(idx, Int::<U>::one())]),
                arch.registers()[idx].signature_facelets(),
            )),
        }
    }
}

fn coalesce_adds<V: Iterator<Item = WithSpan<ExpandedCodeComponent>>>(
    code_components_iter: &mut Peekable<V>,
    global_regs: &GlobalRegs,
) -> Option<Vec<WithSpan<CoalescedAdds>>> {
    let mut adds = HashMap::new();

    while let Some(ExpandedCodeComponent::Instruction(b, _)) =
        code_components_iter.peek().map(|v| &**v)
    {
        let Primitive::Add { amt, register } = &**b else {
            break;
        };
        let (reg_idx, puzzle_idx) = global_regs.get_reg(register);
        adds.entry(puzzle_idx)
            .or_insert(Vec::new())
            .push((reg_idx, amt.to_owned()));
        code_components_iter.next();
    }

    if adds.is_empty() {
        return code_components_iter
            .next()
            .map(|next| vec![next.map(CoalescedAdds::Instruction)]);
    }

    Some(
        adds.into_iter()
            .sorted_unstable_by_key(|&(puzzle_idx, _)| puzzle_idx)
            .map(|(puzzle_idx, adds)| {
                let merged_adds = adds
                    .iter()
                    .map(|(_, add)| add.span().to_owned())
                    .reduce(|acc, add| acc.merge(&add))
                    .unwrap();

                WithSpan::new(
                    match &adds[0].0 {
                        RegisterIdx::Theoretical => CoalescedAdds::AddTheoretical(
                            TheoreticalIdx(puzzle_idx),
                            adds.iter().map(|(_, amt)| **amt).sum::<Int<U>>(),
                        ),
                        RegisterIdx::Real {
                            idx: _,
                            arch,
                            modulus: _,
                        } => CoalescedAdds::AddPuzzle(
                            PuzzleIdx(puzzle_idx),
                            Algorithm::new_from_effect(
                                arch,
                                adds.iter()
                                    .map(|(reg_idx, add)| {
                                        (
                                            match reg_idx {
                                                RegisterIdx::Theoretical => unreachable!(),
                                                RegisterIdx::Real {
                                                    idx,
                                                    arch: _,
                                                    modulus: _,
                                                } => *idx,
                                            },
                                            **add,
                                        )
                                    })
                                    .collect_vec(),
                            ),
                        ),
                    },
                    merged_adds,
                )
            })
            .collect_vec(),
    )
}

// all usize here is puzzle index

enum CoalescedAdds {
    AddPuzzle(PuzzleIdx, Algorithm),
    AddTheoretical(TheoreticalIdx, Int<U>),
    Instruction(ExpandedCodeComponent),
}

enum CoalescedAddsRemovedLabels {
    AddPuzzle(PuzzleIdx, Algorithm),
    AddTheoretical(TheoreticalIdx, Int<U>),
    Instruction(Primitive),
}

struct GlobalRegs {
    register_table: HashMap<ArcIntern<str>, (RegisterIdx, usize)>,
    theoretical: Vec<WithSpan<Int<U>>>,
    puzzles: Vec<WithSpan<Arc<PermutationGroup>>>,
}

impl GlobalRegs {
    fn get_reg(&self, reference: &RegisterReference) -> (RegisterIdx, usize) {
        if reference.block_id == BlockID(0) {
            let mut reg = self
                .register_table
                .get(&reference.reg_name)
                .unwrap()
                .to_owned();

            if let Some(mod_) = reference.modulus {
                match &mut reg.0 {
                    RegisterIdx::Theoretical => todo!(),
                    RegisterIdx::Real {
                        idx: _,
                        arch: _,
                        modulus,
                    } => *modulus = Some(mod_),
                }
            }

            reg
        } else {
            todo!()
        }
    }
}

pub fn strip_expanded(expanded: ExpandedCode) -> Result<Program, Box<Error<Rule>>> {
    let mut label_locations = HashMap::new();

    let mut global_regs = GlobalRegs {
        register_table: HashMap::new(),
        theoretical: vec![],
        puzzles: vec![],
    };

    if let Some(decl) = &expanded.block_info.0.get(&BlockID(0)).unwrap().registers {
        for puzzle in &decl.puzzles {
            match puzzle {
                Puzzle::Theoretical { name, order } => {
                    global_regs.register_table.insert(
                        ArcIntern::clone(name),
                        (RegisterIdx::Theoretical, global_regs.theoretical.len()),
                    );

                    global_regs.theoretical.push(order.to_owned());
                }
                Puzzle::Real { architectures } => {
                    // TODO: Support for architecture switching
                    // Just take the first architecture
                    let (names, architecture) = &architectures[0];
                    for (i, name) in names.iter().enumerate() {
                        global_regs.register_table.insert(
                            ArcIntern::clone(name),
                            (
                                RegisterIdx::Real {
                                    idx: i,
                                    arch: Arc::clone(architecture),
                                    modulus: None,
                                },
                                global_regs.puzzles.len(),
                            ),
                        );
                    }

                    global_regs.puzzles.push(WithSpan::new(
                        architecture.group_arc(),
                        architecture.span().to_owned(),
                    ));
                }
            }
        }
    }

    // TODO: Coalesce add instructions

    let mut program_counter = 0;

    let instructions = expanded
        .expanded_code_components
        .into_iter()
        .peekable()
        .batching(|code_components_iter| coalesce_adds(code_components_iter, &global_regs))
        .flatten()
        .filter_map(|coalesced_adds| {
            let span = coalesced_adds.span().to_owned();

            match coalesced_adds.into_inner() {
                CoalescedAdds::Instruction(ExpandedCodeComponent::Instruction(primitive, _)) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::Instruction(*primitive))
                }
                CoalescedAdds::Instruction(ExpandedCodeComponent::Label(label)) => {
                    label_locations.insert(
                        LabelReference {
                            name: label.name,
                            block_id: label.maybe_block_id.unwrap(),
                        },
                        program_counter,
                    );
                    None
                }
                CoalescedAdds::AddPuzzle(puzzle_idx, alg) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::AddPuzzle(puzzle_idx, alg))
                }
                CoalescedAdds::AddTheoretical(puzzle_idx, amt) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::AddTheoretical(puzzle_idx, amt))
                }
            }
            .map(|v| WithSpan::new(v, span))
        })
        .collect_vec();

    let instructions = instructions
        .into_iter()
        .map(|fully_simplified| {
            let span = fully_simplified.span().to_owned();

            let prim = match fully_simplified.into_inner() {
                CoalescedAddsRemovedLabels::AddPuzzle(puzzle_idx, alg) => {
                    return Ok(WithSpan::new(
                        Instruction::PerformAlgorithm(ByPuzzleType::Puzzle((puzzle_idx, alg))),
                        span,
                    ));
                }
                CoalescedAddsRemovedLabels::AddTheoretical(puzzle_idx, amt) => {
                    return Ok(WithSpan::new(
                        Instruction::PerformAlgorithm(ByPuzzleType::Theoretical(( puzzle_idx , amt))),
                        span,
                    ));
                }
                CoalescedAddsRemovedLabels::Instruction(v) => v,
            };

            let instruction = match prim {
                Primitive::Add {
                    amt: _,
                    register: _,
                } => {
                    unreachable!()
                }
                Primitive::Goto { label } => {
                    let Some(label) = expanded.block_info.label_scope(&label) else {
                        return Err(mk_error("Could not find label in scope", label.span()));
                    };

                    Instruction::Goto {
                        instruction_idx: *label_locations.get(&label).unwrap(),
                    }
                }
                Primitive::SolvedGoto { register, label } => {
                    let Some(label) = expanded.block_info.label_scope(&label) else {
                        return Err(mk_error("Could not find label in scope", label.span()));
                    };

                    let (reg_idx, puzzle_idx) = global_regs.get_reg(&register);

                    let solved_goto = qter_core::SolvedGoto { instruction_idx: *label_locations.get(&label).unwrap() };

                    Instruction::SolvedGoto(match reg_idx {
                        RegisterIdx::Theoretical => ByPuzzleType::Theoretical((solved_goto, TheoreticalIdx(puzzle_idx))),
                        RegisterIdx::Real { idx, arch, modulus } => {
                            let facelets = match modulus {
                                Some(modulus) => if let Some(v) = arch.registers()[idx].signature_facelets_mod(modulus) { v } else {
                                    let cycles = arch.registers()[idx]
                                        .unshared_cycles()
                                        .iter()
                                        .map(CycleGeneratorSubcycle::chromatic_order)
                                        .sorted()
                                        .dedup()
                                        .collect_vec();

                                    return Err(mk_error(format!("Could not find a set of pieces for solved-goto that encode the given modulus. The available moduli are the LCM of any combination of the following piece subcycles: {}", cycles.into_iter().join(", ")), register.reg_name.span()))
                                },
                                None => {
                                    arch.registers()[idx].signature_facelets()
                                },
                            };

                            ByPuzzleType::Puzzle((solved_goto, PuzzleIdx(puzzle_idx), facelets))
                        },
                    })
                }
                Primitive::Input { message, register } => {
                    let (reg_idx, puzzle_idx) = global_regs.get_reg(&register);

                    let input = Input { message: message.into_inner() };

                    Instruction::Input(match reg_idx.generator() {
                        ByPuzzleType::Theoretical(()) => ByPuzzleType::Theoretical((input,TheoreticalIdx(puzzle_idx))),
                        ByPuzzleType::Puzzle ( (generator, solved_goto_facelets) ) => ByPuzzleType::Puzzle((input, PuzzleIdx(puzzle_idx), generator, solved_goto_facelets)),
                    })
                }
                Primitive::Halt { message, register } => {
                    let halt = Halt { message: message.into_inner() };
                    Instruction::Halt(match register {
                        Some(register) => {
                            let (reg_idx, puzzle_idx) = global_regs.get_reg(&register);

                            match reg_idx.generator() {
                                ByPuzzleType::Theoretical(()) => ByPuzzleType::Theoretical((halt, Some(TheoreticalIdx(puzzle_idx)))),
                                ByPuzzleType::Puzzle ( (generator, solved_goto_facelets) ) => ByPuzzleType::Puzzle((halt, Some((PuzzleIdx(puzzle_idx), generator, solved_goto_facelets)))),
                            }
                        }
                        None => ByPuzzleType::Puzzle((halt, None)),
                    })
                },
                Primitive::Print { message, register } => {
                    let print = Print { message: message.into_inner() };
                    Instruction::Print(match register {
                        Some(register) => {
                            let (reg_idx, puzzle_idx) = global_regs.get_reg(&register);

                            match reg_idx.generator() {
                                ByPuzzleType::Theoretical(()) => ByPuzzleType::Theoretical((print, Some(TheoreticalIdx(puzzle_idx)))),
                                ByPuzzleType::Puzzle (( generator, solved_goto_facelets )) => ByPuzzleType::Puzzle((print, Some((PuzzleIdx(puzzle_idx), generator, solved_goto_facelets)))),
                            }
                        }
                        None => ByPuzzleType::Puzzle((print, None)),
                    })
                },
            };

            Ok(WithSpan::new(instruction, span))
        })
        .try_collect::<_, Vec<_>, _>()?;

    Ok(Program {
        theoretical: global_regs.theoretical,
        puzzles: global_regs.puzzles,
        instructions,
    })
}
