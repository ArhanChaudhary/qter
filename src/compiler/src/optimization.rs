use std::{collections::VecDeque, iter::from_fn, mem, sync::Arc};

use internment::ArcIntern;
use qter_core::{
    ByPuzzleType, Int, PuzzleIdx, StateIdx, TheoreticalIdx, U, WithSpan,
    architectures::Architecture,
};
use smol::{
    Executor,
    channel::{Receiver, bounded},
    future,
};

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

fn add_stage<R: Rewriter + Default + Send>(
    executor: &Executor,
    rx: Receiver<WithSpan<OptimizingCodeComponent>>,
) -> Receiver<WithSpan<OptimizingCodeComponent>> {
    let (tx, new_rx) = bounded(32);

    executor
        .spawn(async move {
            let mut rewriter = R::default();

            while let Ok(instruction) = rx.recv().await {
                let new = rewriter.rewrite(instruction);

                for new_instr in new {
                    tx.send(new_instr).await.unwrap();
                }
            }

            let new = rewriter.eof();

            for new_instr in new {
                tx.send(new_instr).await.unwrap();
            }
        })
        .detach();

    new_rx
}

pub fn do_optimization(
    instructions: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + Send + 'static,
) -> impl Iterator<Item = WithSpan<OptimizingCodeComponent>> {
    let executor = Executor::new();

    let (tx, rx) = bounded(32);

    executor
        .spawn(async move {
            for instruction in instructions {
                tx.send(instruction).await.unwrap();
            }
        })
        .detach();

    let rx = add_stage::<CoalesceAdds>(&executor, rx);
    let rx = add_stage::<RepeatUntil1>(&executor, rx);

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

/// Transforms
///
/// ```
/// spot1:
/// solved-goto spot2 <positions>
/// <algorithm>
/// goto spot1
/// spot2:
/// ```
/// into
/// ```
/// spot1:
/// repeat until <positions> solved <algorithm>
/// spot2:
/// ```
#[derive(Default)]
struct RepeatUntil1 {
    window: VecDeque<WithSpan<OptimizingCodeComponent>>,
}

impl RepeatUntil1 {
    fn try_match(&mut self) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(spot1) = &*self.window[0] else {
            return None;
        };

        let OptimizingCodeComponent::Instruction(instr, _) = &*self.window[1] else {
            return None;
        };
        let OptimizingPrimitive::SolvedGoto {
            label: spot2,
            register,
        } = &**instr
        else {
            return None;
        };

        let OptimizingCodeComponent::Instruction(instr, _) = &*self.window[2] else {
            return None;
        };
        let OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**instr else {
            return None;
        };

        let OptimizingCodeComponent::Instruction(instr, _) = &*self.window[3] else {
            return None;
        };
        let OptimizingPrimitive::Goto { label } = &**instr else {
            return None;
        };

        if label.name != spot1.name || label.block_id != spot1.maybe_block_id.unwrap() {
            return None;
        }

        let OptimizingCodeComponent::Label(real_spot2) = &*self.window[4] else {
            return None;
        };

        if spot2.name != real_spot2.name || spot2.block_id != real_spot2.maybe_block_id.unwrap() {
            return None;
        }

        let repeat_until = OptimizingCodeComponent::Instruction(
            Box::new(OptimizingPrimitive::RepeatUntil {
                puzzle: *puzzle,
                arch: Arc::clone(arch),
                amts: amts.to_owned(),
                register: register.to_owned(),
            }),
            spot2.block_id,
        );

        let mut values = Vec::new();
        values.push(self.window.pop_front().unwrap());

        let span = self
            .window
            .drain(0..3)
            .map(|v| v.span().clone())
            .reduce(|a, v| a.merge(&v))
            .unwrap();

        values.push(span.with(repeat_until));

        Some(values)
    }
}

impl Rewriter for RepeatUntil1 {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.window.push_back(component);

        if self.window.len() < 5 {
            return Vec::new();
        }

        match self.try_match() {
            Some(v) => v,
            None => vec![self.window.pop_front().unwrap()],
        }
    }

    fn eof(self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.window.into()
    }
}
