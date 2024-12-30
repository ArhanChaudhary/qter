use std::{collections::HashMap, sync::Arc};

use internment::ArcIntern;
use lua::LuaMacros;
use pest::error::Error;
use qter_core::{architectures::Architecture, Int, Program, WithSpan, U};

mod lua;
mod parsing;

pub fn compile(
    qat: Arc<str>,
    find_import: impl Fn(&str) -> Result<Arc<str>, String>,
) -> Result<Program, Box<Error<parsing::Rule>>> {
    todo!()
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct Label {
    name: ArcIntern<String>,
    block: Option<BlockID>,
}

#[derive(Clone, Debug)]
struct Block {
    code: Vec<WithSpan<Instruction>>,
    block: Option<BlockID>,
}

#[derive(Clone, Debug)]
enum Primitive {
    Add {
        amt: WithSpan<Int<U>>,
        register: WithSpan<ArcIntern<String>>,
    },
    Goto {
        label: WithSpan<ArcIntern<Label>>,
    },
    SolvedGoto {
        register: WithSpan<ArcIntern<String>>,
        label: WithSpan<ArcIntern<Label>>,
    },
    Input {
        message: WithSpan<String>,
        register: WithSpan<ArcIntern<String>>,
    },
    Halt {
        message: WithSpan<String>,
        register: WithSpan<ArcIntern<String>>,
    },
    Print {
        message: WithSpan<String>,
        register: WithSpan<ArcIntern<String>>,
    },
}

#[derive(Clone, Debug)]
enum Value {
    Int(Int<U>),
    Constant(ArcIntern<String>),
    Word(ArcIntern<String>),
    Block(Block),
}

#[derive(Clone, Debug)]
struct MacroCall {
    name: WithSpan<ArcIntern<String>>,
    arguments: Vec<WithSpan<Value>>,
}

#[derive(Clone, Debug)]
enum Code {
    Primitive(Primitive),
    Macro(MacroCall),
}

#[derive(Clone, Debug)]
struct LuaCall {
    function_name: WithSpan<ArcIntern<String>>,
    args: Vec<WithSpan<Value>>,
}

#[derive(Clone, Debug)]
enum Instruction {
    Label(Label),
    Code(Code),
    Constant(ArcIntern<String>),
    LuaCall(LuaCall),
    Define(Define),
    Registers(RegisterDecl),
}

#[derive(Clone, Copy, Debug)]
enum PatternArgTy {
    Int,
    Reg,
    Block,
    Ident,
}

#[derive(Clone, Debug)]
enum PatternComponent {
    Argument {
        name: WithSpan<ArcIntern<String>>,
        ty: WithSpan<PatternArgTy>,
    },
    Word(ArcIntern<String>),
}

impl PatternComponent {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    fn conflicts_with(&self, other: &PatternComponent) -> Option<ArcIntern<String>> {
        use PatternArgTy as A;
        use PatternComponent as P;

        match (self, other) {
            (P::Argument { name: _, ty: a }, P::Argument { name: _, ty: b }) => match (**a, **b) {
                (A::Int, A::Int) => Some(ArcIntern::from_ref("123")),
                (A::Reg, A::Reg)
                | (A::Ident, A::Reg)
                | (A::Reg, A::Ident)
                | (A::Ident, A::Ident) => Some(ArcIntern::from_ref("a")),
                (A::Block, A::Block) => Some(ArcIntern::from_ref("{ }")),
                _ => None,
            },
            (P::Argument { name: _, ty }, P::Word(word))
            | (P::Word(word), P::Argument { name: _, ty }) => match **ty {
                A::Ident | A::Reg => Some(ArcIntern::clone(word)),
                _ => None,
            },
            (P::Word(a), P::Word(b)) => (a == b).then(|| ArcIntern::clone(a)),
        }
    }
}

#[derive(Clone, Debug)]
struct Pattern(Vec<WithSpan<PatternComponent>>);

impl Pattern {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    pub fn conflicts_with(&self, macro_name: &str, other: &Pattern) -> Option<String> {
        if self.0.len() != other.0.len() {
            return None;
        }

        self.0
            .iter()
            .zip(other.0.iter())
            .map(|(a, b)| a.conflicts_with(b))
            .try_fold(String::new(), |mut a, v| {
                let v = v?;

                a.push(' ');
                a.push_str(&v);
                Some(a)
            })
            .map(|e| format!("{macro_name}{e}"))
    }
}

#[derive(Clone, Debug)]
struct MacroBranch {
    pattern: WithSpan<Pattern>,
    code: Vec<WithSpan<Instruction>>,
}

#[derive(Clone, Debug)]
struct Macro {
    branches: Vec<WithSpan<MacroBranch>>,
    after: Option<WithSpan<ArcIntern<String>>>,
}

#[derive(Clone, Debug)]
enum DefinedValue {
    Value(WithSpan<Value>),
    LuaCall(WithSpan<LuaCall>),
}

#[derive(Clone, Debug)]
struct Define {
    name: WithSpan<ArcIntern<String>>,
    value: DefinedValue,
}

#[derive(Clone, Debug)]
enum Cube {
    Theoretical {
        name: WithSpan<ArcIntern<String>>,
        order: WithSpan<Int<U>>,
    },
    Real {
        architectures: Vec<(
            Vec<WithSpan<ArcIntern<String>>>,
            WithSpan<Arc<Architecture>>,
        )>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BlockID(pub usize);

#[derive(Clone, Debug)]
struct RegisterDecl {
    cubes: Vec<Cube>,
    block: Option<BlockID>,
}

#[derive(Debug, Clone)]
struct BlockInfo {
    parent: Option<BlockID>,
    children: Vec<BlockID>,
    registers: Option<RegisterDecl>,
    defines: Vec<Define>,
}

#[derive(Clone, Debug)]
struct ParsedSyntax {
    // Each block gets an ID and `block_parent` maps a block ID to it's parent
    // The global scope is block zero and if the block/label hasn't been expanded its ID is None
    block_counter: usize,
    block_info: HashMap<BlockID, BlockInfo>,
    /// Map (file contents, macro name) to a macro
    macros: HashMap<(ArcIntern<String>, ArcIntern<String>), WithSpan<Macro>>,
    /// Map each macro name to the file that it's in
    available_macros: HashMap<ArcIntern<String>, ArcIntern<String>>,
    /// Each file has its own LuaMacros; use the file contents as the key
    lua_macros: HashMap<ArcIntern<String>, LuaMacros>,
    code: Vec<WithSpan<(Instruction, BlockID)>>,
}
