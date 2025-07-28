use std::{iter::from_fn, sync::Arc};

use qter_core::{
    ByPuzzleType, Int, PuzzleIdx, StateIdx, TheoreticalIdx, U, WithSpan,
    architectures::Architecture,
};
use smol::{Executor, channel::bounded, future};

use crate::{BlockID, Label, LabelReference, RegisterReference};

#[derive(Clone, Debug)]
pub enum OptimizingPrimitive {
    AddPuzzle {
        puzzle: PuzzleIdx,
        arch: Arc<Architecture>,
        // register idx, modulus, amt to add
        amts: Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
    },
    AddTheoretical {
        theoretical: TheoreticalIdx,
        amt: WithSpan<Int<U>>,
    },
    Goto {
        label: WithSpan<LabelReference>,
    },
    SolvedGoto {
        label: WithSpan<LabelReference>,
        register: RegisterReference,
    },
    RepeatUntil {
        puzzle: PuzzleIdx,
        arch: Arc<Architecture>,
        amts: Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
        register: RegisterReference,
    },
    Solve {
        puzzle: ByPuzzleType<'static, StateIdx>,
    },
    Input {
        message: WithSpan<String>,
        register: RegisterReference,
    },
    Halt {
        message: WithSpan<String>,
        register: Option<RegisterReference>,
    },
    Print {
        message: WithSpan<String>,
        register: Option<RegisterReference>,
    },
}

#[derive(Clone, Debug)]
pub enum OptimizingCodeComponent {
    Instruction(Box<OptimizingPrimitive>, BlockID),
    Label(Label),
}

trait Rewriter {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>>;

    fn eof(self) -> Vec<WithSpan<OptimizingCodeComponent>>;
}

pub fn do_optimization(
    instructions: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + Send + 'static,
) -> impl Iterator<Item = WithSpan<OptimizingCodeComponent>> {
    let executor = Executor::new();

    let (tx, rx) = bounded(32);

    let mut add_coalescer = CoalesceAdds::default();

    executor
        .spawn(async move {
            for instruction in instructions {
                let new = add_coalescer.rewrite(instruction);

                for instr in new {
                    tx.send(instr).await.unwrap();
                }
            }

            for instr in add_coalescer.eof() {
                tx.send(instr).await.unwrap();
            }
        })
        .detach();

    from_fn(move || future::block_on(executor.run(rx.recv())).ok())
}

#[derive(Default)]
struct CoalesceAdds {
    block_id: Option<BlockID>,
    theoreticals: Vec<WithSpan<(TheoreticalIdx, WithSpan<Int<U>>)>>,
    puzzles: Vec<
        WithSpan<(
            PuzzleIdx,
            Arc<Architecture>,
            Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
        )>,
    >,
}

impl CoalesceAdds {
    fn dump_state(&mut self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.theoreticals
            .drain(..)
            .map(|v| {
                v.map(|(theoretical, amt)| {
                    OptimizingCodeComponent::Instruction(
                        Box::new(OptimizingPrimitive::AddTheoretical { theoretical, amt }),
                        self.block_id.unwrap(),
                    )
                })
            })
            .chain(self.puzzles.drain(..).map(|v| {
                v.map(|(puzzle, arch, amts)| {
                    OptimizingCodeComponent::Instruction(
                        Box::new(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts }),
                        self.block_id.unwrap(),
                    )
                })
            }))
            .collect()
    }
}

impl Rewriter for CoalesceAdds {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let span = component.span().clone();

        match component.into_inner() {
            OptimizingCodeComponent::Instruction(instr, block_id) => match *instr {
                OptimizingPrimitive::AddTheoretical {
                    theoretical: theoretical_idx,
                    amt,
                } => {
                    self.block_id = Some(block_id);

                    for theoretical in &mut self.theoreticals {
                        if theoretical.0 == theoretical_idx {
                            *theoretical.1 += *amt;
                            return Vec::new();
                        }
                    }

                    self.theoreticals.push(span.with((theoretical_idx, amt)));

                    Vec::new()
                }
                OptimizingPrimitive::AddPuzzle {
                    puzzle: puzzle_idx,
                    arch,
                    amts,
                } => {
                    self.block_id = Some(block_id);

                    for puzzle in &mut self.puzzles {
                        if puzzle.0 == puzzle_idx {
                            'next_effect: for new_effect in &amts {
                                for effect in &mut puzzle.2 {
                                    if effect.0 == new_effect.0 {
                                        *effect.2 += *new_effect.2;
                                        continue 'next_effect;
                                    }
                                }

                                puzzle.2.push(new_effect.to_owned());
                            }

                            return Vec::new();
                        }
                    }

                    self.puzzles.push(span.with((puzzle_idx, arch, amts)));

                    Vec::new()
                }
                primitive => {
                    let mut instrs = self.dump_state();
                    instrs.push(span.with(OptimizingCodeComponent::Instruction(
                        Box::new(primitive),
                        block_id,
                    )));
                    instrs
                }
            },
            OptimizingCodeComponent::Label(label) => {
                let mut instrs = self.dump_state();
                instrs.push(span.with(OptimizingCodeComponent::Label(label)));
                instrs
            }
        }
    }

    fn eof(mut self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.dump_state()
    }
}
