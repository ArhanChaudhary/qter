use std::{collections::VecDeque, iter::from_fn, mem, sync::Arc};

use qter_core::WithSpan;

use crate::{optimization::OptimizingCodeComponent, strip_expanded::GlobalRegs};

pub trait Rewriter: Default {
    /// Feed an instruction into the rewriter and return any instructions that are known to be optimized as best as possible under the given rule
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>>;

    /// Dump all instructions out of the rewriter
    fn eof(self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>>;
}

pub fn push_to_pull<R: Rewriter + 'static>(
    rewriter: R,
    mut iter: impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + 'static,
    global_regs: Arc<GlobalRegs>,
) -> impl Iterator<Item = WithSpan<OptimizingCodeComponent>> + 'static {
    let mut rewriter = Some(rewriter);
    let mut output_so_far = VecDeque::new();

    from_fn(move || {
        loop {
            if let Some(output) = output_so_far.pop_front() {
                return Some(output);
            }

            match iter.next() {
                Some(component) => output_so_far
                    .extend(rewriter.as_mut().unwrap().rewrite(component, &global_regs)),
                None => match rewriter.take() {
                    Some(this) => output_so_far.extend(Box::new(this).eof(&global_regs)),
                    None => return None,
                },
            }
        }
    })
}

impl<R1: Rewriter, R2: Rewriter> Rewriter for (R1, R2) {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let first_out = self.0.rewrite(component, global_regs);

        let mut out = Vec::new();

        for component in first_out {
            out.extend(self.1.rewrite(component, global_regs));
        }

        out
    }

    fn eof(mut self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let first_out = self.0.eof(global_regs);

        let mut out = Vec::new();

        for component in first_out {
            self.1.rewrite(component, global_regs);
        }

        out.extend(self.1.eof(global_regs));

        out
    }
}

#[derive(Default)]
struct Stages<R: Rewriter> {
    // INVARIANT: The `VecDeque` in each stage is empty before and after every `rewrite` call
    stages: VecDeque<(R, VecDeque<WithSpan<OptimizingCodeComponent>>)>,
}

impl<R: Rewriter> Rewriter for Stages<R> {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        if self.stages.is_empty() {
            return vec![component];
        }

        self.stages[0].1.push_front(component);

        let mut current = 0;

        let mut output_found = Vec::new();

        while current != 0 || !self.stages[0].1.is_empty() {
            if self.stages[current].1.is_empty() {
                current -= 1;
                continue;
            }

            while let Some(instr) = self.stages[current].1.pop_front() {
                let output = self.stages[current].0.rewrite(instr, global_regs);

                current += 1;

                if current == self.stages.len() {
                    output_found.extend(output);
                    break;
                }

                self.stages[current + 1].1.extend(output);
            }
        }

        output_found
    }

    fn eof(mut self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let mut output = Vec::new();

        while let Some(stage) = self.stages.pop_front() {
            let in_buffer = stage.0.eof(global_regs);

            for component in in_buffer {
                output.extend(self.rewrite(component, global_regs));
            }
        }

        output
    }
}

#[derive(Default)]
pub struct RepeatUntilConvergence<R: Rewriter> {
    stages: Stages<R>,
    input_so_far: VecDeque<WithSpan<OptimizingCodeComponent>>,
    last_stage: R,
    output_so_far: Vec<WithSpan<OptimizingCodeComponent>>,
    correct_until: usize,
}

impl<R: Rewriter> RepeatUntilConvergence<R> {
    fn check_equality_and_maybe_new_stage(&mut self, global_regs: &GlobalRegs) {
        while !self.input_so_far.is_empty() && self.correct_until < self.output_so_far.len() {
            if self.input_so_far.pop_front().unwrap() != self.output_so_far[self.correct_until] {
                self.stages
                    .stages
                    .push_back((mem::take(&mut self.last_stage), VecDeque::new()));
                self.input_so_far = VecDeque::new();
                self.correct_until = 0;

                for component in mem::take(&mut self.output_so_far) {
                    self.input_so_far.push_back(component.clone());
                    self.output_so_far
                        .extend(self.last_stage.rewrite(component, global_regs));
                }

                self.check_equality_and_maybe_new_stage(global_regs);

                break;
            }

            self.correct_until += 1;
        }
    }
}

impl<R: Rewriter> Rewriter for RepeatUntilConvergence<R> {
    fn rewrite(
        &mut self,
        component: WithSpan<OptimizingCodeComponent>,
        global_regs: &GlobalRegs,
    ) -> Vec<WithSpan<OptimizingCodeComponent>> {
        let stages_output = self.stages.rewrite(component, global_regs);

        for component in &stages_output {
            self.output_so_far
                .extend(self.last_stage.rewrite(component.clone(), global_regs));
        }

        self.input_so_far.extend(stages_output);
        self.check_equality_and_maybe_new_stage(global_regs);

        vec![]
    }

    fn eof(mut self, global_regs: &GlobalRegs) -> Vec<WithSpan<OptimizingCodeComponent>> {
        while !self.stages.stages.is_empty() {
            let (stage, _) = self.stages.stages.pop_back().unwrap();
            let components = stage.eof(global_regs);

            for component in components {
                self.rewrite(component, global_regs);
            }
        }

        let mut input = self.input_so_far.into();
        let mut output = mem::take(&mut self.output_so_far);
        output.extend(mem::take(&mut self.last_stage).eof(global_regs));

        while input != output {
            input = output;
            output = Vec::new();

            for component in input.clone() {
                output.extend(self.last_stage.rewrite(component, global_regs));
            }

            output.extend(mem::take(&mut self.last_stage).eof(global_regs));
        }

        output
    }
}
