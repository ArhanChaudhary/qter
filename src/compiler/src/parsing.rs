use std::{collections::HashMap, fmt::Debug, rc::Rc};

use internment::ArcIntern;
use pest::{
    error::{
        Error,
        ErrorVariant::{self, CustomError},
    },
    iterators::Pair,
    Parser,
};
use pest_derive::Parser;
use qter_core::{
    architectures::{puzzle_by_name, Architecture},
    Int, WithSpan, U,
};

use crate::{lua::LuaMacros, Block, BlockID, Code, Cube, Define, DefinedValue, Label, LuaCall, MacroCall, ParsedSyntax, RegisterDecl, Value};

use super::Instruction;

#[derive(Parser)]
#[grammar = "./qat.pest"]
struct QatParser;

fn parse(qat: Rc<str>) -> Result<ParsedSyntax, Box<Error<Rule>>> {
    let program = QatParser::parse(Rule::program, &qat)?.next().unwrap();
    let zero_pos = program.as_span().start_pos();
    let mut program = program.into_inner();

    let lua = match LuaMacros::new() {
        Ok(v) => v,
        Err(e) => return Err(Box::new(Error::new_from_pos(ErrorVariant::CustomError { message: e.to_string()}, zero_pos))),
    };

    let mut macros = HashMap::new();
    let mut instructions = Vec::new();

    for pair in program {
        if let Rule::EOI = pair.as_rule() {
            break;
        }
        
        let span = pair.as_span();
        match parse_statement(pair)? {
            Statement::Macro => todo!(),
            Statement::Instruction(instruction) => {
                let span = instruction.span().to_owned();

                instructions.push(WithSpan::new((instruction.value, BlockID(0)), span))
            },
            Statement::LuaBlock(code) => if let Err(e) = lua.add_chunk(code) {
                return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: e.to_string() }, span)))
            },
            Statement::Import(_) => todo!(),
        }
    }

    let mut block_parent = HashMap::new();
    block_parent.insert(BlockID(0), None);

    Ok(ParsedSyntax { block_counter: 1, block_parent, registers: HashMap::new(), macros, defines: Vec::new(), lua_macros: lua, code: instructions })
}

fn parse_registers(pair: Pair<'_, Rule>) -> Result<RegisterDecl, Box<Error<Rule>>> {
    let mut cubes = Vec::new();

    for decl in pair.into_inner() {
        cubes.push(match decl.as_rule() {
            Rule::unswitchable => parse_declaration(decl)?,
            Rule::switchable => {
                let mut decls = Vec::new();

                for pair in decl.into_inner() {
                    let span = pair.as_span();

                    match parse_declaration(pair)? {
                        Cube::Theoretical { name: _, order: _ } => return Err(Box::new(Error::new_from_span(
                            ErrorVariant::CustomError {
                                message:
                                    "Cannot create a switchable cube with a theoretical register"
                                        .to_owned(),
                            },
                            span,
                        ))),
                        Cube::Real { architectures } => decls.extend_from_slice(&architectures),
                    }
                }

                // TODO: Verify that the architectures are compatible with each other
                
                Cube::Real { architectures: decls }
            }
            rule => unreachable!("{rule:?}, {}", decl.as_str()),
        });
    }

    Ok(RegisterDecl { cubes, block: None })
}

fn parse_declaration(pair: Pair<'_, Rule>) -> Result<Cube, Box<Error<Rule>>> {
    let span = pair.as_span();
    let mut pairs = pair.into_inner();

    let mut regs = Vec::new();

    let mut arch = None;

    for pair in pairs.by_ref() {
        if let Rule::ident = pair.as_rule() {
            regs.push(WithSpan::new(
                ArcIntern::<String>::from_ref(pair.as_str()),
                pair.as_span().into(),
            ));
        } else {
            arch = Some(pair);
            break;
        }
    }

    let arch = arch.unwrap();

    match arch.as_rule() {
        Rule::theoretical_architecture => {
            if regs.len() > 1 {
                return Err(Box::new(Error::new_from_span(
                    CustomError {
                        message: format!(
                            "Expected one register name for a theoretical architecture, found {}",
                            regs.len()
                        ),
                    },
                    span,
                )));
            }

            let number = arch.into_inner().next().unwrap();

            Ok(Cube::Theoretical {
                name: regs.pop().unwrap(),
                order: WithSpan::new(
                    number.as_str().parse::<Int<U>>().unwrap(),
                    number.as_span().into(),
                ),
            })
        }
        Rule::real_architecture => {
            let arch = arch.into_inner().next().unwrap();
            let rule = arch.as_rule();
            let span = arch.as_span();
            let mut arch = arch.into_inner();

            let puzzle_name = arch.next().unwrap();
            let puzzle = match puzzle_by_name(puzzle_name.as_str()) {
                Some(v) => v,
                None => {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: "Unknown puzzle".to_string(),
                        },
                        puzzle_name.as_span(),
                    )))
                }
            };

            let decoded_arch = match rule {
                Rule::builtin_architecture => {
                    let mut orders = Vec::new();

                    for order in arch {
                        orders.push(order.as_str().parse::<Int<U>>().unwrap());
                    }

                    match puzzle.get_preset(&orders) {
                        Some(arch) => arch,
                        None => return Err(Box::new(Error::new_from_span(ErrorVariant::CustomError { message: "Could not find a builtin architecture for the given puzzle with the given orders".to_string() }, span))),
                    }
                }
                Rule::custom_architecture => {
                    let mut algorithms = Vec::new();

                    for algorithm in arch {
                        let mut generators = Vec::new();

                        for generator in algorithm.into_inner() {
                            generators.push(ArcIntern::<String>::from_ref(generator.as_str()));
                        }

                        algorithms.push(generators);
                    }

                    match Architecture::new(puzzle.group, algorithms) {
                        Ok(v) => Rc::new(v),
                        Err(e) => {
                            return Err(Box::new(Error::new_from_span(
                                ErrorVariant::CustomError {
                                    message: format!(
                                        "The generator `{e}` isn't defined for the given puzzle"
                                    ),
                                },
                                span,
                            )));
                        }
                    }
                }
                rule => unreachable!("{rule:?}"),
            };

            let cube = Cube::Real {
                architectures: vec![(regs, WithSpan::new(decoded_arch, span.into()))],
            };

            Ok(cube)
        }
        rule => unreachable!("{rule:?}"),
    }
}

enum Statement<'a> {
    Macro,
    Instruction(WithSpan<Instruction>),
    LuaBlock(&'a str),
    Import(&'a str),
}

fn parse_statement(pair: Pair<'_, Rule>) -> Result<Statement<'_>, Box<Error<Rule>>> {
    let rule = pair.as_rule();

    match rule {
        Rule::r#macro => todo!(),
        Rule::instruction => Ok(Statement::Instruction(parse_instruction(pair)?)),
        Rule::lua_code => {
            Ok(Statement::LuaBlock(pair.as_str()))
        },
        Rule::import => todo!(),
        _ => unreachable!("{rule:?}"),
    }
}

fn parse_instruction(pair: Pair<'_, Rule>) -> Result<WithSpan<Instruction>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();

    Ok(WithSpan::new(match rule {
        Rule::label => Instruction::Label(Label { name: ArcIntern::<String>::from_ref(pair.into_inner().next().unwrap().as_str()), block: None } ),
        Rule::code => {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();
            let name = WithSpan::new(ArcIntern::<String>::from_ref(name.as_str()), name.as_span().into());

            let arguments = pairs.map(|v| parse_value(v)).collect::<Result<Vec<_>, _>>()?;
            
            Instruction::Code(Code::Macro(MacroCall { name, arguments }))
        },
        Rule::constant => Instruction::Constant(ArcIntern::<String>::from_ref(pair.into_inner().next().unwrap().as_str())),
        Rule::lua_call => {
            Instruction::LuaCall(parse_lua_call(pair)?)
        },
        Rule::define=> {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();

            let definition = pairs.next().unwrap();

            let value = match definition.as_rule() {
                Rule::value => DefinedValue::Value(parse_value(definition)?),
                Rule::lua_call => {
                    let span = definition.as_span();

                    DefinedValue::LuaCall(WithSpan::new(parse_lua_call(definition)?, span.into()))
                },
                rule => unreachable!("{rule:?}"),
            };

            Instruction::Define(Define { name: WithSpan::new(ArcIntern::from_ref(name.as_str()), name.as_span().into()), block: None, value })
        },
        Rule::registers=> Instruction::Registers(parse_registers(pair)?),
        _ => unreachable!("{rule:?}")
    }, span))
}

fn parse_value(pair: Pair<'_, Rule>) -> Result<WithSpan<Value>, Box<Error<Rule>>> {
    let pair = pair.into_inner().next().unwrap();
    let rule = pair.as_rule();
    let span = pair.as_span().into();
    
    Ok(WithSpan::new(match rule {
        Rule::number => Value::Int(pair.as_str().parse::<Int<U>>().unwrap()),
        Rule::constant=> Value::Constant(ArcIntern::from_ref(pair.as_str())),
        Rule::ident => Value::Word(ArcIntern::from_ref(pair.as_str())),
        Rule::block=> Value::Block ( parse_block(pair)? ),
        _ => unreachable!("{rule:?}")
    }, span))
}

fn parse_block(pair: Pair<'_, Rule>) -> Result<Block, Box<Error<Rule>>> {
    Ok(Block { code: 
    pair.into_inner().map(|v| parse_instruction(v)).collect::<Result<Vec<_>, _>>()?
        , block: None })
}

fn parse_lua_call(pair: Pair<'_, Rule>) -> Result<LuaCall, Box<Error<Rule>>> {
            let mut pairs = pair.into_inner();

            let name = pairs.next().unwrap();
            
            Ok(LuaCall { function_name: WithSpan::new(ArcIntern::from_ref(name.as_str()), name.as_span().into()), args: pairs.map(|v| parse_value(v)).collect::<Result<_, _>>()? })
    
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::parse;

    #[test]
    fn bruh() {
        let code = "
            .registers {
                a, b ← 3x3 builtin (90, 90)
                (
                    c, d ← 3x3 builtin (210, 24)
                    d, e ← 3x3 builtin (30, 30, 30)
                )
                f ← theoretical 90
                g, h ← 3x3 (U, D)
            }

            .start-lua
                function bruh()
                    print(\"skibidi\")
                end
            end-lua

            bruh:
            add 1 a
            goto bruh

            lua bruh(1, 2, 3)

            .define yeet lua bruh(1, 2, 3)
            .define pog 4
        ";

        match parse(Rc::from(code)) {
            Ok(_) => {}
            Err(e) => panic!("{e}"),
        }
    }
}
