use std::{
    collections::{HashSet, VecDeque},
    iter::from_fn,
    marker::PhantomData,
    sync::Arc,
};

use itertools::Itertools;
use qter_core::{
    ByPuzzleType, Int, PuzzleIdx, TheoreticalIdx, U, WithSpan, architectures::Architecture,
};
use smol::{
    Executor,
    channel::{Receiver, bounded},
    future,
};

use crate::{
    BlockID, optimization::OptimizingPrimitive, primitive_match, strip_expanded::GlobalRegs,
};

use super::OptimizingCodeComponent;

trait Rewriter {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>>;

    fn eof(self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>>;
}

fn add_stage<R: Rewriter + Default + Send>(
    executor: &Executor,
    rx: Receiver<WithSpan<OptimizingCodeComponent>>,
    global_regs: Arc<GlobalRegs>,
) -> Receiver<WithSpan<OptimizingCodeComponent>> {
    let (tx, new_rx) = bounded(32);

    executor
        .spawn(async move {
            let mut rewriter = R::default();

            while let Ok(instruction) = rx.recv().await {
                let new = rewriter.rewrite(instruction, &global_regs);

                for new_instr in new {
                    tx.send(new_instr).await.unwrap();
                }
            }

            let new = rewriter.eof(&global_regs);

            for new_instr in new {
                tx.send(new_instr).await.unwrap();
            }
        })
        .detach();

    new_rx
}

pub fn do_local_optimization(
    instructions: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + Send + 'static,
    global_regs: Arc<GlobalRegs>,
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

    let rx = add_stage::<RemoveDeadCode>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<Peephole<RemoveUselessJumps>>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<CoalesceAdds>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<Peephole<RepeatUntil1>>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<Peephole<RepeatUntil2>>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<Peephole<RepeatUntil3>>(&executor, rx, Arc::clone(&global_regs));
    let rx = add_stage::<TransformSolve>(&executor, rx, global_regs);

    from_fn(move || future::block_on(executor.run(rx.recv())).ok())
}

/// Any non-label instructions that come immedately after an unconditional goto or halt are unreachable and can be removed
#[derive(Default)]
struct RemoveDeadCode {
    diverging: Option<WithSpan<OptimizingCodeComponent>>,
}

impl Rewriter for RemoveDeadCode {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        _: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match self.diverging.take() {
            Some(goto) => {
                if matches!(&*component, OptimizingCodeComponent::Label(_)) {
                    return vec![goto, component];
                }

                // Otherwise throw out the instruction
                self.diverging = Some(goto);

                Vec::new()
            }
            None => {
                primitive_match!((OptimizingPrimitive::Goto { .. } | OptimizingPrimitive::Halt { .. }) = &*component; else { return vec![component]; });

                self.diverging = Some(component);

                Vec::new()
            }
        }
    }

    fn eof(self, _: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match self.diverging {
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
        _: &GlobalRegs,
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
        _: &GlobalRegs,
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

    fn eof(mut self, _: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.dump_state()
    }
}

struct Peephole<R: PeepholeRewriter> {
    window: VecDeque<WithSpan<OptimizingCodeComponent>>,
    phantom_: PhantomData<R>,
}

impl<R: PeepholeRewriter> Peephole<R> {
    fn do_try_match(&mut self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        match R::try_match(&mut self.window, global_regs) {
            Some(mut v) => {
                let again = self.do_try_match(global_regs);
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
        global_regs: &GlobalRegs,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>>;
}

impl<R: PeepholeRewriter> Rewriter for Peephole<R> {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.window.push_back(component);

        self.do_try_match(global_regs)
    }

    fn eof(mut self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let mut out = Vec::new();

        loop {
            out.extend(self.do_try_match(global_regs));

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
        global_regs: &GlobalRegs,
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

        if match global_regs.get_reg(register) {
            qter_core::ByPuzzleType::Theoretical(_) => true,
            qter_core::ByPuzzleType::Puzzle((idx, _)) => idx != *puzzle,
        } {
            return None;
        }

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
spot1:
    <algorithm>
<optional label>:
    solved-goto <positions> spot3
    goto spot1
spot3:
```
into
```
spot1:
    <algorithm>
<optional label>:
    repeat until <positions> solved <algorithm>
spot3:
```
*/
struct RepeatUntil2;

impl PeepholeRewriter for RepeatUntil2 {
    const MAX_WINDOW_SIZE: usize = 6;

    fn try_match(
        window: &mut VecDeque<WithSpan<OptimizingCodeComponent>>,
        global_regs: &GlobalRegs,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(spot1) = &**window.front()? else {
            return None;
        };

        primitive_match!(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**window.get(1)?);

        let optional_label = usize::from(matches!(
            window.get(2).map(|v| &**v),
            Some(OptimizingCodeComponent::Label(_))
        ));

        primitive_match!(
            OptimizingPrimitive::SolvedGoto {
                label: spot3,
                register,
            } = &**window.get(2 + optional_label)?
        );

        if match global_regs.get_reg(register) {
            qter_core::ByPuzzleType::Theoretical(_) => true,
            qter_core::ByPuzzleType::Puzzle((idx, _)) => idx != *puzzle,
        } {
            return None;
        }

        primitive_match!(
            OptimizingPrimitive::Goto { label: maybe_spot1 } = &**window.get(3 + optional_label)?
        );

        if spot1.name != maybe_spot1.name || spot1.maybe_block_id.unwrap() != maybe_spot1.block_id {
            return None;
        }

        let OptimizingCodeComponent::Label(real_spot3) = &**window.get(4 + optional_label)? else {
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

        out.extend(window.drain(0..2 + optional_label));

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
        global_regs: &GlobalRegs,
    ) -> Option<Vec<WithSpan<OptimizingCodeComponent>>> {
        let OptimizingCodeComponent::Label(spot1) = &**window.front()? else {
            return None;
        };

        primitive_match!(OptimizingPrimitive::AddPuzzle { puzzle, arch, amts } = &**window.get(1)?);

        let optional_label = usize::from(matches!(
            window.get(2).map(|v| &**v),
            Some(OptimizingCodeComponent::Label(_))
        ));

        primitive_match!(
            OptimizingPrimitive::SolvedGoto {
                label: spot2,
                register,
            } = &**window.get(2 + optional_label)?
        );

        if match global_regs.get_reg(register) {
            qter_core::ByPuzzleType::Theoretical(_) => true,
            qter_core::ByPuzzleType::Puzzle((idx, _)) => idx != *puzzle,
        } {
            return None;
        }

        let maybe_algorithm = match &**window.get(3 + optional_label)? {
            OptimizingCodeComponent::Instruction(optimizing_primitive, _) => {
                match &**optimizing_primitive {
                    OptimizingPrimitive::AddPuzzle {
                        puzzle: new_puzzle,
                        arch,
                        amts,
                    } => {
                        if puzzle != new_puzzle {
                            return None;
                        }

                        Some((new_puzzle, arch, amts))
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

#[derive(Default)]
struct TransformSolve {
    instrs: VecDeque<(WithSpan<OptimizingCodeComponent>, Option<usize>)>,
    puzzle_idx: Option<PuzzleIdx>,
    guaranteed_zeroed: HashSet<usize>,
}

impl TransformSolve {
    fn dump(&mut self) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.guaranteed_zeroed = HashSet::new();
        self.instrs.drain(..).map(|(instr, _)| instr).collect_vec()
    }

    fn dump_with(
        &mut self,
        instr: WithSpan<OptimizingCodeComponent>,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let mut instrs = self.dump();
        instrs.push(instr);
        instrs
    }
}

impl Rewriter for TransformSolve {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let OptimizingCodeComponent::Instruction(instr, block_id) = &*component else {
            return self.dump_with(component);
        };

        let OptimizingPrimitive::RepeatUntil {
            puzzle,
            arch: _,
            amts,
            register,
        } = &**instr
        else {
            return self.dump_with(component);
        };

        let mut dumped = Vec::new();

        if self.puzzle_idx.is_some() && self.puzzle_idx != Some(*puzzle) {
            dumped.extend(self.dump());
        }

        self.puzzle_idx = Some(*puzzle);

        let ByPuzzleType::Puzzle((puzzle_idx, (reg_idx, arch, modulus))) =
            global_regs.get_reg(register)
        else {
            dumped.extend(self.dump_with(component));
            return dumped;
        };

        assert_eq!(*puzzle, puzzle_idx);

        let mut broken = HashSet::new();

        for amt in amts {
            broken.insert(amt.0);
        }

        if let Some((i, _)) = self
            .instrs
            .iter()
            .enumerate()
            .rev()
            .find(|v| v.1.1.is_some_and(|v| broken.contains(&v)))
        {
            dumped.extend(self.instrs.drain(0..i).map(|v| v.0));
        }

        for thingy in broken {
            self.guaranteed_zeroed.remove(&thingy);
        }

        // If we have a modulus, then it is possible for the whole register not to be zeroed in the end
        let zeroes_out = modulus.is_none_or(|modulus| modulus == arch.registers()[reg_idx].order());

        if zeroes_out {
            self.guaranteed_zeroed.insert(reg_idx);
        }

        if self.guaranteed_zeroed.len() == arch.registers().len() {
            let span = self
                .instrs
                .drain(..)
                .map(|v| v.0.span().clone())
                .reduce(|a, v| a.merge(&v))
                .unwrap();

            self.guaranteed_zeroed = HashSet::new();
            dumped.push(span.with(OptimizingCodeComponent::Instruction(
                Box::new(OptimizingPrimitive::Solve {
                    puzzle: ByPuzzleType::Puzzle(self.puzzle_idx.unwrap()),
                }),
                *block_id,
            )));
        } else {
            self.instrs
                .push_back((component, zeroes_out.then_some(reg_idx)));
        }

        dumped
    }

    fn eof(mut self, _: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        self.dump()
    }
}
