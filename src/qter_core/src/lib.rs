use std::ops::{Deref, DerefMut};

use bnum::types::U512;
// Use a huge integers for orders to allow crazy things like examinx

pub struct WithSpan<T> {
    pub value: T,
    line_num: usize,
}

impl<T> Deref for WithSpan<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for WithSpan<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> WithSpan<T> {
    pub fn new(value: T, line_num: usize) -> WithSpan<T> {
        WithSpan { value, line_num }
    }

    pub fn line_num(&self) -> usize {
        self.line_num
    }
}

pub enum RegisterRepresentation {
    Theoretical { name: String, order: U512 },
    // TODO: Registers based on a permutation group
}

pub enum Instruction {
    Goto {
        instruction_idx: usize,
    },
    SolvedGoto {
        instruction_idx: usize,
        register: String,
    },
    Input {
        message: String,
        register: String,
    },
    Halt {
        message: String,
        register: String,
    },
    Print {
        message: String,
        register: String,
    },
    AddTheoretical {
        register: String,
        amount: U512,
    },
    // TODO: Addition to registers based on a permutation group
}

pub struct Program {
    pub groups: Vec<WithSpan<RegisterRepresentation>>,
    pub instructions: Vec<WithSpan<Instruction>>,
}
