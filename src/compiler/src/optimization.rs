use std::{collections::VecDeque, iter::from_fn, marker::PhantomData, sync::Arc};

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

// TODO: Remove when https://doc.rust-lang.org/beta/unstable-book/language-features/deref-patterns.html is stable
macro_rules! primitive_match {
    ($pattern:pat = $val:expr) => {
        primitive_match!($pattern = $val; else { return None; });
    };

    ($pattern:pat = $val:expr; else $else:block) => {
        let OptimizingCodeComponent::Instruction(instr, _) = $val else $else;
        let $pattern = &**instr else $else;
    }
}

// TODO:
// IMPORTANT:
// - Dead code erasure
// - Removing labels that are never jumped to
// - `solve` instruction
//
// NICETIES:
// - Remove jumps to next instruction
// - Coalesce solved-gotos to the same label
// - Coalesce adjacent labels
// - Strength reduction of `solved-goto` after a `repeat until` or `solve` that guarantees whether or not it succeeds

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

    let rx = add_stage::<RemoveDeadCode>(&executor, rx);
    let rx = add_stage::<Peephole<RemoveUselessJumps>>(&executor, rx);
    let rx = add_stage::<CoalesceAdds>(&executor, rx);
    let rx = add_stage::<Peephole<RepeatUntil1>>(&executor, rx);
    let rx = add_stage::<Peephole<RepeatUntil2>>(&executor, rx);
    let rx = add_stage::<Peephole<RepeatUntil3>>(&executor, rx);

    from_fn(move || future::block_on(executor.run(rx.recv())).ok())
}

/// Any non-label instructions that come immedately after an unconditional goto are unreachable and can be removed
#[derive(Default)]
struct RemoveDeadCode {
    goto: Option<WithSpan<OptimizingCodeComponent>>,
}

impl Rewriter for RemoveDeadCode {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match self.goto.take() {
            Some(goto) => {
                if matches!(&*component, OptimizingCodeComponent::Label(_)) {
                    return vec![goto, component];
                }

                // Otherwise throw out the instruction
                self.goto = Some(goto);

                Vec::new()
            }
            None => {
                primitive_match!(OptimizingPrimitive::Goto { .. } = &*component; else { return vec![component]; });

                self.goto = Some(component);

                Vec::new()
            }
        }
    }

    fn eof(self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match self.goto {
            Some(goto) => vec![goto],
            None => Vec::new(),
        }
    }
}

#[derive(Default)]
struct RemoveUselessJumps;

impl PeepholeRewriter for RemoveUselessJumps {
    const MAX_WINDOW_SIZE: usize = 2;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(label) = &**window.get(1)? else {
            return None;
        };

        primitive_match!(
            (OptimizingPrimitive::SolvedGoto {
                label: jumps_to,
                ..
            } | OptimizingPrimitive::Goto { label: jumps_to }) = &**window.front()?
        );

        if jumps_to.name == label.name && jumps_to.block_id == label.maybe_block_id.unwrap() {
            window.pop_front().unwrap();
        }

        None
    }
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

    fn merge_effects(
        effect1: &mut Vec<(usize, Option<Int<U>>, WithSpan<Int<U>>)>,
        effect2: &[(usize, Option<Int<U>>, WithSpan<Int<U>>)],
    ) {
        'next_effect: for new_effect in effect2 {
            for effect in &mut *effect1 {
                if effect.0 == new_effect.0 {
                    *effect.2 += *new_effect.2;
                    continue 'next_effect;
                }
            }

            effect1.push(new_effect.to_owned());
        }
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
                            CoalesceAdds::merge_effects(&mut puzzle.2, &amts);

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

struct Peephole<R: PeepholeRewriter> {
    window: VecDeque<WithSpan<OptimizingCodeComponent>>,
    phantom_: PhantomData<R>,
}

impl<R: PeepholeRewriter> Peephole<R> {
    fn do_try_match(&mut self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match R::try_match(&mut self.window) {
            Some(mut v) => {
                let again = self.do_try_match();
                v.extend(again);
                v
            }
            None => {
                if self.window.len() >= R::MAX_WINDOW_SIZE {
                    vec![self.window.pop_front().unwrap()]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

impl<R: PeepholeRewriter> Default for Peephole<R> {
    fn default() -> Self {
        Peephole {
            window: VecDeque::new(),
            phantom_: PhantomData,
        }
    }
}

trait PeepholeRewriter {
    const MAX_WINDOW_SIZE: usize;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>>;
}

impl<R: PeepholeRewriter> Rewriter for Peephole<R> {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.window.push_back(component);

        self.do_try_match()
    }

    fn eof(mut self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let mut out = Vec::new();

        loop {
            out.extend(self.do_try_match());

            match self.window.pop_front() {
                Some(first) => out.push(first),
                _ => return out,
            }
        }
    }
}

/*
Transforms
```
spot1:
    solved-goto <positions> spot2
    <algorithm>
    goto spot1
spot2:
```
into
```
spot1:
    repeat until <positions> solved <algorithm>
spot2:
```
*/
struct RepeatUntil1;

impl PeepholeRewriter for RepeatUntil1 {
    const MAX_WINDOW_SIZE: usize = 5;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(spot1) = &**window.front()? else {
            return None;
        };

        primitive_match!(
            OptimizingPrimitive::SolvedGoto {
                label: spot2,
                register,
            } = &**window.get(1)?
        );

        primitive_match!(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**window.get(2)?);

        primitive_match!(OptimizingPrimitive::Goto { label } = &**window.get(3)?);

        if label.name != spot1.name || label.block_id != spot1.maybe_block_id.unwrap() {
            return None;
        }

        let OptimizingCodeComponent::Label(real_spot2) = &**window.get(4)? else {
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
        values.push(window.pop_front().unwrap());

        let span = window
            .drain(0..3)
            .map(|v| v.span().clone())
            .reduce(|a, v| a.merge(&v))
            .unwrap();

        values.push(span.with(repeat_until));

        Some(values)
    }
}

/*
Transforms
```
    goto spot2
spot1:
    <algorithm>
spot2:
    solved-goto <positions> spot3
    goto spot1
spot3:
```
into
```
    goto spot2
spot1:
    <algorithm>
spot2:
    repeat until <positions> solved <algorithm>
spot3:
```
*/
struct RepeatUntil2;

impl PeepholeRewriter for RepeatUntil2 {
    const MAX_WINDOW_SIZE: usize = 7;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        primitive_match!(OptimizingPrimitive::Goto { label: spot2 } = &**window.front()?);

        let OptimizingCodeComponent::Label(spot1) = &**window.get(1)? else {
            return None;
        };

        primitive_match!(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**window.get(2)?);

        let OptimizingCodeComponent::Label(real_spot2) = &**window.get(3)? else {
            return None;
        };

        if spot2.name != real_spot2.name || spot2.block_id != real_spot2.maybe_block_id.unwrap() {
            return None;
        }

        primitive_match!(
            OptimizingPrimitive::SolvedGoto {
                label: spot3,
                register,
            } = &**window.get(4)?
        );

        primitive_match!(OptimizingPrimitive::Goto { label: maybe_spot1 } = &**window.get(5)?);

        if spot1.name != maybe_spot1.name || spot1.maybe_block_id.unwrap() != maybe_spot1.block_id {
            return None;
        }

        let OptimizingCodeComponent::Label(real_spot3) = &**window.get(6)? else {
            return None;
        };

        if spot3.name != real_spot3.name || spot3.block_id != real_spot3.maybe_block_id.unwrap() {
            return None;
        }

        let repeat_until = OptimizingCodeComponent::Instruction(
            Box::new(OptimizingPrimitive::RepeatUntil {
                puzzle: *puzzle,
                arch: Arc::clone(arch),
                amts: amts.to_owned(),
                register: register.to_owned(),
            }),
            spot3.block_id,
        );

        let mut out = Vec::new();

        out.extend(window.drain(0..4));

        let span = window
            .drain(0..2)
            .map(|v| v.span().clone())
            .reduce(|a, v| a.merge(&v))
            .unwrap();

        out.push(span.with(repeat_until));

        Some(out)
    }
}

/*
Transforms
```
spot1:
    <algorithm>
<optional label>:
    solved-goto <positions> spot2
    <optional algorithm>
    goto spot1
spot2:
```
into
```
spot1:
    <algorithm>
<optional label>:
    repeat until <positions> solved <optional algorithm> <algorithm>
spot2:
```
*/
struct RepeatUntil3;

impl PeepholeRewriter for RepeatUntil3 {
    const MAX_WINDOW_SIZE: usize = 7;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(spot1) = &**window.front()? else {
            return None;
        };

        primitive_match!(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**window.get(1)?);

        let optional_label = usize::from(matches!(&*window[2], OptimizingCodeComponent::Label(_)));

        primitive_match!(
            OptimizingPrimitive::SolvedGoto {
                label: spot2,
                register,
            } = &**window.get(2 + optional_label)?
        );

        let maybe_algorithm = match &**window.get(3 + optional_label)? {
            OptimizingCodeComponent::Instruction(optimizing_primitive, _) => {
                match &**optimizing_primitive {
                    OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } => {
                        Some((puzzle, arch, amts))
                    }
                    _ => None,
                }
            }
            OptimizingCodeComponent::Label(_) => None,
        };

        let is_alg = usize::from(maybe_algorithm.is_some());

        primitive_match!(
            OptimizingPrimitive::Goto { label: maybe_spot1 } =
                &**window.get(3 + optional_label + is_alg)?
        );

        if maybe_spot1.name != spot1.name || maybe_spot1.block_id != spot1.maybe_block_id.unwrap() {
            return None;
        }

        let OptimizingCodeComponent::Label(real_spot2) =
            &**window.get(4 + optional_label + is_alg)?
        else {
            return None;
        };

        if spot2.name != real_spot2.name || spot2.block_id != real_spot2.maybe_block_id.unwrap() {
            return None;
        }

        let mut amts = amts.to_owned();

        if let Some((_, _, effect)) = maybe_algorithm {
            CoalesceAdds::merge_effects(&mut amts, effect);
        }

        let repeat_until = OptimizingCodeComponent::Instruction(
            Box::new(OptimizingPrimitive::RepeatUntil {
                puzzle: *puzzle,
                arch: Arc::clone(arch),
                amts,
                register: register.to_owned(),
            }),
            spot2.block_id,
        );

        let mut out = Vec::new();

        out.extend(window.drain(0..2 + optional_label));

        let span = window
            .drain(0..2 + is_alg)
            .map(|v| v.span().clone())
            .reduce(|a, v| a.merge(&v))
            .unwrap();

        out.push(span.with(repeat_until));

        Some(out)
    }
}
