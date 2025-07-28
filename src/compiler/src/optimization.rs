use std::{iter::from_fn, mem, sync::Arc};

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
    found: Vec<WithSpan<OptimizingCodeComponent>>,
    spot_1: Option<LabelReference>,
    spot_2: Option<LabelReference>,
    register: Option<RegisterReference>,
    addition: Option<(
        PuzzleIdx,
        Arc<Architecture>,
        Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
    )>,
    stage: SolvedGoto1Stage,
}

impl RepeatUntil1 {
    fn reset(
        &mut self,
        found_instr: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.stage = SolvedGoto1Stage::None;

        let mut ret = mem::take(&mut self.found);
        if ret.is_empty() {
            vec![found_instr]
        } else {
            ret.extend(self.rewrite(found_instr));
            ret
        }
    }
}

#[derive(Default)]
enum SolvedGoto1Stage {
    #[default]
    None,
    Spot1,
    SolvedGoto,
    AddPuzzle,
    Goto,
}

impl Rewriter for RepeatUntil1 {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match self.stage {
            SolvedGoto1Stage::None => match &*component {
                OptimizingCodeComponent::Instruction(_, _) => self.reset(component),
                OptimizingCodeComponent::Label(label) => {
                    self.spot_1 = Some(LabelReference {
                        name: ArcIntern::clone(&label.name),
                        block_id: label.maybe_block_id.unwrap(),
                    });
                    self.found.push(component);
                    self.stage = SolvedGoto1Stage::Spot1;
                    Vec::new()
                }
            },
            SolvedGoto1Stage::Spot1 => match &*component {
                OptimizingCodeComponent::Instruction(primitive, _) => match &**primitive {
                    OptimizingPrimitive::SolvedGoto { label, register } => {
                        self.register = Some(register.clone());
                        self.spot_2 = Some((**label).clone());
                        self.found.push(component);
                        self.stage = SolvedGoto1Stage::SolvedGoto;
                        Vec::new()
                    }
                    _ => self.reset(component),
                },
                OptimizingCodeComponent::Label(_) => self.reset(component),
            },
            SolvedGoto1Stage::SolvedGoto => match &*component {
                OptimizingCodeComponent::Instruction(primitive, _) => match &**primitive {
                    OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } => {
                        self.addition = Some((*puzzle, Arc::clone(arch), amts.clone()));
                        self.found.push(component);
                        self.stage = SolvedGoto1Stage::AddPuzzle;
                        Vec::new()
                    }
                    _ => self.reset(component),
                },
                OptimizingCodeComponent::Label(_) => self.reset(component),
            },
            SolvedGoto1Stage::AddPuzzle => match &*component {
                OptimizingCodeComponent::Instruction(primitive, _) => match &**primitive {
                    OptimizingPrimitive::Goto { label } => {
                        if &**label == self.spot_1.as_ref().unwrap() {
                            self.found.push(component);
                            self.stage = SolvedGoto1Stage::Goto;
                            Vec::new()
                        } else {
                            self.reset(component)
                        }
                    }
                    _ => self.reset(component),
                },
                OptimizingCodeComponent::Label(_) => self.reset(component),
            },
            SolvedGoto1Stage::Goto => match &*component {
                OptimizingCodeComponent::Instruction(_, _) => self.reset(component),
                OptimizingCodeComponent::Label(label) => {
                    let spot_2 = self.spot_2.as_ref().unwrap();

                    if spot_2.name == label.name && spot_2.block_id == label.maybe_block_id.unwrap()
                    {
                        let span = self
                            .found
                            .drain(1..)
                            .map(|v| v.span().clone())
                            .reduce(|a, v| a.merge(&v))
                            .unwrap();

                        let addition = self.addition.take().unwrap();

                        self.found
                            .push(span.with(OptimizingCodeComponent::Instruction(
                                Box::new(OptimizingPrimitive::RepeatUntil {
                                    puzzle: addition.0,
                                    arch: addition.1,
                                    amts: addition.2,
                                    register: self.register.take().unwrap(),
                                }),
                                spot_2.block_id,
                            )));
                    }

                    self.reset(component)
                }
            },
        }
    }

    fn eof(self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.found
    }
}
