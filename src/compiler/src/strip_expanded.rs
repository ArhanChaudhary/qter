use std::{collections::HashMap, sync::Arc};

use chumsky::error::Rich;
use internment::ArcIntern;
use itertools::{Either, Itertools};
use qter_core::{
    ByPuzzleType, Halt, Input, Instruction, Int, Print, Program, PuzzleIdx, RegisterGenerator,
    Span, TheoreticalIdx, U, WithSpan,
    architectures::{Algorithm, Architecture, CycleGeneratorSubcycle, PermutationGroup},
};

use crate::{
    ExpandedCode, ExpandedCodeComponent, LabelReference, Primitive, Puzzle, RegisterReference,
    optimization::{OptimizingCodeComponent, OptimizingPrimitive, do_optimization},
};

#[derive(Clone, Debug)]
pub enum RegisterIdx {
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
    }
}

pub fn strip_expanded(expanded: ExpandedCode) -> Result<Program, Vec<Rich<'static, char, Span>>> {
    let mut label_locations = HashMap::new();

    let mut global_regs = GlobalRegs {
        register_table: HashMap::new(),
        theoretical: vec![],
        puzzles: vec![],
    };

    for puzzle in &expanded.registers.puzzles {
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

    let global_regs = Arc::new(global_regs);
    let global_regs_for_iter = Arc::clone(&global_regs);

    let instructions_iter = expanded.expanded_code_components.into_iter().map(move |v| {
        v.map(|v| match v {
            ExpandedCodeComponent::Instruction(primitive, block_id) => {
                OptimizingCodeComponent::Instruction(
                    Box::new(match *primitive {
                        Primitive::Add { amt, register } => {
                            match global_regs_for_iter.get_reg(&register) {
                                (RegisterIdx::Theoretical, theoretical_idx) => {
                                    OptimizingPrimitive::AddTheoretical {
                                        theoretical: TheoreticalIdx(theoretical_idx),
                                        amt,
                                    }
                                }
                                (
                                    RegisterIdx::Real {
                                        idx: reg_idx,
                                        arch,
                                        modulus,
                                    },
                                    puzzle_idx,
                                ) => OptimizingPrimitive::AddPuzzle {
                                    puzzle: PuzzleIdx(puzzle_idx),
                                    arch,
                                    amts: vec![(reg_idx, modulus, amt)],
                                },
                            }
                        }
                        Primitive::Goto { label } => OptimizingPrimitive::Goto { label },
                        Primitive::SolvedGoto { label, register } => {
                            OptimizingPrimitive::SolvedGoto { label, register }
                        }
                        Primitive::Input { message, register } => {
                            OptimizingPrimitive::Input { message, register }
                        }
                        Primitive::Halt { message, register } => {
                            OptimizingPrimitive::Halt { message, register }
                        }
                        Primitive::Print { message, register } => {
                            OptimizingPrimitive::Print { message, register }
                        }
                    }),
                    block_id,
                )
            }
            ExpandedCodeComponent::Label(label) => OptimizingCodeComponent::Label(label),
        })
    });

    let optimized_iter = do_optimization(instructions_iter);

    let mut program_counter = 0;

    let instructions = optimized_iter
        .filter_map(|component| {
            let span = component.span().to_owned();

            match component.into_inner() {
                OptimizingCodeComponent::Instruction(primitive, _) => {
                    program_counter += 1;

                    Some(match *primitive {
                        OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } => {
                            CoalescedAddsRemovedLabels::AddPuzzle(
                                puzzle,
                                Algorithm::new_from_effect(
                                    &arch,
                                    amts.into_iter()
                                        .map(|(idx, _, amt)| (idx, amt.into_inner()))
                                        .collect(),
                                ),
                            )
                        }
                        OptimizingPrimitive::AddTheoretical { theoretical, amt } => {
                            CoalescedAddsRemovedLabels::AddTheoretical(theoretical, *amt)
                        }
                        OptimizingPrimitive::Goto { label } => {
                            CoalescedAddsRemovedLabels::Instruction(Primitive::Goto { label })
                        }
                        OptimizingPrimitive::SolvedGoto { label, register } => {
                            CoalescedAddsRemovedLabels::Instruction(Primitive::SolvedGoto {
                                label,
                                register,
                            })
                        }
                        OptimizingPrimitive::Input { message, register } => {
                            CoalescedAddsRemovedLabels::Instruction(Primitive::Input {
                                message,
                                register,
                            })
                        }
                        OptimizingPrimitive::Halt { message, register } => {
                            CoalescedAddsRemovedLabels::Instruction(Primitive::Halt {
                                message,
                                register,
                            })
                        }
                        OptimizingPrimitive::Print { message, register } => {
                            CoalescedAddsRemovedLabels::Instruction(Primitive::Print {
                                message,
                                register,
                            })
                        }
                    })
                }
                OptimizingCodeComponent::Label(label) => {
                    label_locations.insert(
                        LabelReference {
                            name: label.name,
                            block_id: label.maybe_block_id.unwrap(),
                        },
                        program_counter,
                    );
                    None
                }
            }
            .map(|v| WithSpan::new(v, span))
        })
        .collect_vec();

    let (instructions, errors) = instructions
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
                        return Err(Rich::custom(label.span().clone(), "Could not find label in scope"));
                    };

                    Instruction::Goto {
                        instruction_idx: *label_locations.get(&label).unwrap(),
                    }
                }
                Primitive::SolvedGoto { register, label } => {
                    let Some(label) = expanded.block_info.label_scope(&label) else {
                        return Err(Rich::custom(label.span().clone(), "Could not find label in scope"));
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

                                    return Err(Rich::custom(register.reg_name.span().clone(), format!("Could not find a set of pieces for solved-goto that encode the given modulus. The available moduli are the LCM of any combination of the following piece subcycles: {}", cycles.into_iter().join(", "))))
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
        .partition_map::<Vec<_>, Vec<_>, _, _, _>(|res| match res {
            Ok(v) => Either::Left(v),
            Err(e) => Either::Right(e),
        });

    if !errors.is_empty() {
        return Err(errors);
    }

    let global_regs = Arc::into_inner(global_regs).unwrap();

    Ok(Program {
        theoretical: global_regs.theoretical,
        puzzles: global_regs.puzzles,
        instructions,
    })
}
