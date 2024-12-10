use std::{collections::HashMap, rc::Rc};

use bnum::types::U512;
use internment::ArcIntern;
use mlua::Lua;
use pest::error::Error;
use pest_derive::Parser;
use qter_core::{Program, WithSpan};

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
        amt: WithSpan<U512>,
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
    Int(U512),
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

enum PatternArg {
    Int,
    Reg,
    Block,
    Word(ArcIntern<String>),
}

struct Pattern(Vec<WithSpan<PatternArg>>);

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
    lua_scripts: Lua,
    code: Vec<WithSpan<(Instruction, ArcIntern<Scope>)>>,
}
