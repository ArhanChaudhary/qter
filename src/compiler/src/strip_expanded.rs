use std::{collections::HashMap, iter::Peekable, sync::Arc};

use internment::ArcIntern;
use itertools::Itertools;
use pest::error::Error;
use qter_core::{
    architectures::{Architecture, PermutationGroup},
    mk_error, Facelets, Instruction, Int, PermutePuzzle, Program, RegisterGenerator, WithSpan, U,
};

use crate::{
    parsing::Rule, BlockID, Expanded, ExpandedCode, LabelReference, Primitive, RegisterReference,
};

#[derive(Clone, Debug)]
enum RegisterIdx {
    Theoretical,
    Real { idx: usize, arch: Arc<Architecture> },
}

impl RegisterIdx {
    fn facelets(&self) -> Facelets {
        match self {
            RegisterIdx::Theoretical => Facelets::Theoretical,
            RegisterIdx::Real { idx, arch } => Facelets::Puzzle {
                facelets: arch.registers()[*idx].signature_facelets(),
            },
        }
    }

    fn generator(&self) -> RegisterGenerator {
        match self {
            RegisterIdx::Theoretical => RegisterGenerator::Theoretical,
            RegisterIdx::Real { idx, arch } => RegisterGenerator::Puzzle {
                generator: PermutePuzzle::new_from_effect(arch, vec![(*idx, Int::<U>::one())]),
                facelets: arch.registers()[*idx].signature_facelets(),
            },
        }
    }
}

fn coalesce_adds<V: Iterator<Item = WithSpan<ExpandedCode>>>(
    iter: &mut Peekable<V>,
    global_regs: &GlobalRegs,
) -> Option<Vec<WithSpan<CoalescedAdds>>> {
    let mut adds = HashMap::new();

    while let Some(ExpandedCode::Instruction(Primitive::Add { amt, register }, _)) =
        iter.peek().map(|v| &**v)
    {
        let reg = global_regs.get_reg(register);
        adds.entry(reg.1)
            .or_insert(Vec::new())
            .push((reg.0, amt.to_owned()));
        iter.next();
    }

    if adds.is_empty() {
        return iter
            .next()
            .map(|next| vec![next.map(CoalescedAdds::Instruction)]);
    }

    Some(
        adds.into_iter()
            .sorted_unstable_by_key(|v| v.0)
            .map(|(puzzle, adds)| {
                let span = adds
                    .iter()
                    .map(|v| v.1.span().to_owned())
                    .reduce(|a, v| a.merge(&v))
                    .unwrap();

                WithSpan::new(
                    match &adds[0].0 {
                        RegisterIdx::Theoretical => CoalescedAdds::AddTheoretical(
                            puzzle,
                            adds.iter().map(|v| *v.1).sum::<Int<U>>(),
                        ),
                        RegisterIdx::Real { idx: _, arch } => CoalescedAdds::AddPuzzle(
                            puzzle,
                            PermutePuzzle::new_from_effect(
                                arch,
                                adds.iter()
                                    .map(|v| {
                                        (
                                            match &v.0 {
                                                RegisterIdx::Theoretical => unreachable!(),
                                                RegisterIdx::Real { idx, arch: _ } => *idx,
                                            },
                                            *v.1,
                                        )
                                    })
                                    .collect_vec(),
                            ),
                        ),
                    },
                    span,
                )
            })
            .collect_vec(),
    )
}

enum CoalescedAdds {
    AddPuzzle(usize, PermutePuzzle),
    AddTheoretical(usize, Int<U>),
    Instruction(ExpandedCode),
}

enum CoalescedAddsRemovedLabels {
    AddPuzzle(usize, PermutePuzzle),
    AddTheoretical(usize, Int<U>),
    Instruction(Primitive),
}

struct GlobalRegs {
    register_table: HashMap<ArcIntern<str>, (RegisterIdx, usize)>,
    theoretical: Vec<WithSpan<Int<U>>>,
    puzzles: Vec<WithSpan<Arc<PermutationGroup>>>,
}

impl GlobalRegs {
    fn get_reg(&self, reference: &RegisterReference) -> (RegisterIdx, usize) {
        match reference.block == BlockID(0) {
            true => self.register_table.get(&reference.name).unwrap().to_owned(),
            false => todo!(),
        }
    }
}

pub fn strip_expanded(expanded: Expanded) -> Result<Program, Box<Error<Rule>>> {
    let mut label_locations = HashMap::new();

    let mut global_regs = GlobalRegs {
        register_table: HashMap::new(),
        theoretical: vec![],
        puzzles: vec![],
    };

    if let Some(decl) = &expanded.block_info.0.get(&BlockID(0)).unwrap().registers {
        for puzzle in &decl.puzzles {
            match puzzle {
                crate::Puzzle::Theoretical { name, order } => {
                    global_regs.register_table.insert(
                        ArcIntern::clone(name),
                        (RegisterIdx::Theoretical, global_regs.theoretical.len()),
                    );

                    global_regs.theoretical.push(order.to_owned());
                }
                crate::Puzzle::Real { architectures } => {
                    // TODO: Support for architecture switching
                    // Just take the first architecture
                    let (names, architecture) = &architectures[0];
                    for (i, reg) in names
                        .iter()
                        .zip(architecture.registers().iter())
                        .enumerate()
                    {
                        global_regs.register_table.insert(
                            ArcIntern::clone(reg.0),
                            (
                                RegisterIdx::Real {
                                    idx: i,
                                    arch: Arc::clone(architecture),
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
    };

    // TODO: Coalesce add instructions

    let mut program_counter = 0;

    let instructions = expanded
        .code
        .into_iter()
        .peekable()
        .batching(|iter| coalesce_adds(iter, &global_regs))
        .flatten()
        .filter_map(|v| {
            let span = v.span().to_owned();

            match v.into_inner() {
                CoalescedAdds::Instruction(ExpandedCode::Instruction(primitive, _block)) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::Instruction(primitive))
                }
                CoalescedAdds::Instruction(ExpandedCode::Label(label)) => {
                    label_locations.insert(
                        LabelReference {
                            name: label.name,
                            block: label.block.unwrap(),
                        },
                        program_counter,
                    );
                    None
                }
                CoalescedAdds::AddPuzzle(puzzle, permutation) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::AddPuzzle(puzzle, permutation))
                }
                CoalescedAdds::AddTheoretical(idx, amt) => {
                    program_counter += 1;
                    Some(CoalescedAddsRemovedLabels::AddTheoretical(idx, amt))
                }
            }
            .map(|v| WithSpan::new(v, span))
        })
        .collect_vec();

    let instructions = instructions
        .into_iter()
        .map(|v| {
            let span = v.span().to_owned();
            let instruction = match v.into_inner() {
                CoalescedAddsRemovedLabels::AddPuzzle(puzzle, permutation) => {
                    return Ok(WithSpan::new(
                        Instruction::PermutePuzzle {
                            puzzle_idx: puzzle,
                            permute_puzzle: permutation,
                        },
                        span,
                    ))
                }
                CoalescedAddsRemovedLabels::AddTheoretical(idx, amt) => {
                    return Ok(WithSpan::new(
                        Instruction::AddTheoretical {
                            register_idx: idx,
                            amount: amt,
                        },
                        span,
                    ))
                }
                CoalescedAddsRemovedLabels::Instruction(v) => v,
            };

            let instruction = match instruction {
                Primitive::Add {
                    amt: _,
                    register: _,
                } => {
                    unreachable!()
                }
                Primitive::Goto { label } => {
                    let label = match expanded.block_info.label_scope(&label) {
                        Some(v) => v,
                        None => {
                            return Err(mk_error("Could not find label in scope", label.span()));
                        }
                    };

                    Instruction::Goto {
                        instruction_idx: *label_locations.get(&label).unwrap(),
                    }
                }
                Primitive::SolvedGoto { register, label } => {
                    let label = match expanded.block_info.label_scope(&label) {
                        Some(v) => v,
                        None => {
                            return Err(mk_error("Could not find label in scope", label.span()));
                        }
                    };

                    let (reg, idx) = global_regs.get_reg(&register);

                    Instruction::SolvedGoto {
                        instruction_idx: *label_locations.get(&label).unwrap(),
                        register_idx: idx,
                        facelets: reg.facelets(),
                    }
                }
                Primitive::Input { message, register } => {
                    let (reg, idx) = global_regs.get_reg(&register);

                    Instruction::Input {
                        message: message.into_inner(),
                        register_idx: idx,
                        register: reg.generator(),
                    }
                }
                Primitive::Halt { message, register } => match register {
                    Some(register) => {
                        let (reg, idx) = global_regs.get_reg(&register);

                        Instruction::Halt {
                            message: message.into_inner(),
                            register_idx: Some(idx),
                            register: Some(reg.generator()),
                        }
                    }
                    None => Instruction::Halt {
                        message: message.into_inner(),
                        register_idx: None,
                        register: None,
                    },
                },
                Primitive::Print { message, register } => match register {
                    Some(register) => {
                        let (reg, idx) = global_regs.get_reg(&register);
                        Instruction::Print {
                            message: message.into_inner(),
                            register_idx: Some(idx),
                            register: Some(reg.generator()),
                        }
                    }
                    None => Instruction::Print {
                        message: message.into_inner(),
                        register_idx: None,
                        register: None,
                    },
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
