use std::{collections::HashMap, rc::Rc};

use internment::ArcIntern;
use lua::LuaMacros;
use mlua::Lua;
use pest::error::Error;
use pest_derive::Parser;
use qter_core::{Int, Program, WithSpan, U};

mod lua;

#[derive(Parser)]
#[grammar = "./qat.pest"]
struct Parser;

pub fn compile(
    qat: Rc<str>,
    find_import: impl Fn(&str) -> Result<Rc<str>, String>,
) -> Result<Program, Box<Error<Rule>>> {
    todo!()
}

#[derive(Hash, PartialEq, Eq)]
struct Label {
    name: String,
    blocks: Vec<usize>,
}

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

enum Value {
    Int(Int<U>),
    Constant(ArcIntern<String>),
    Word(ArcIntern<String>),
    Block(Vec<WithSpan<Instruction>>),
}

struct MacroCall {
    name: WithSpan<ArcIntern<String>>,
    arguments: Vec<WithSpan<Value>>,
}

enum Code {
    Primitive(Primitive),
    Macro(MacroCall),
}

enum Instruction {
    Label(ArcIntern<Label>),
    Code(Code),
    Constant(ArcIntern<String>),
    LuaCall {
        function_name: WithSpan<ArcIntern<String>>,
        args: Vec<WithSpan<Value>>,
    },
}

#[derive(Clone, Copy, Debug)]
enum PatternArgTy {
    Int,
    Reg,
    Block,
    Ident,
}

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

struct Pattern(Vec<WithSpan<PatternComponent>>);

impl Pattern {
    /// Returns `None` if the patterns do not conflict, otherwise returns a counterexample that would match both patterns.
    fn conflicts_with(&self, other: &Pattern) -> Option<String> {
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
    }
}

struct MacroBranch {
    pattern: WithSpan<Pattern>,
    code: Vec<WithSpan<Instruction>>,
}

struct Macro {
    branches: Vec<WithSpan<MacroBranch>>,
    imported_from: ArcIntern<String>,
}

#[derive(Hash, PartialEq, Eq)]
struct Scope {
    levels: Vec<usize>,
}

struct Define {
    name: ArcIntern<String>,
    scope: ArcIntern<Scope>,
    value: Value,
}

struct ParsedSyntax {
    scope_counter: usize,
    macros: HashMap<ArcIntern<String>, WithSpan<Macro>>,
    defines: Vec<Define>,
    lua_macros: LuaMacros,
    code: Vec<WithSpan<(Instruction, ArcIntern<Scope>)>>,
}
