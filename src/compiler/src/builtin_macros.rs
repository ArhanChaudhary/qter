use internment::ArcIntern;
use pest::error::{Error, ErrorVariant};
use qter_core::{Span, WithSpan};

use crate::{
    parsing::Rule, BlockID, Code, ExpansionInfo, Instruction, LabelReference, Macro, Primitive,
    RegisterReference, Value,
};

use std::collections::HashMap;

fn expect_reg(
    reg: WithSpan<Value>,
    syntax: &ExpansionInfo,
    block: BlockID,
) -> Result<RegisterReference, Box<Error<Rule>>> {
    match &*reg {
        Value::Word(name) => match syntax.block_info.get_register(&RegisterReference {
            block,
            name: WithSpan::new(ArcIntern::clone(name), reg.span().to_owned()),
        }) {
            Some(v) => Ok(v.0),
            None => {
                return Err(Box::new(Error::new_from_span(
                    ErrorVariant::CustomError {
                        message: format!("The register {name} does not exist"),
                    },
                    reg.span().pest(),
                )));
            }
        },
        _ => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: "Expected a register".to_string(),
                },
                reg.span().pest(),
            )));
        }
    }
}

fn expect_label(
    label: WithSpan<Value>,
    block: BlockID,
) -> Result<WithSpan<LabelReference>, Box<Error<Rule>>> {
    match &*label {
        Value::Word(word) => Ok(WithSpan::new(
            LabelReference {
                name: ArcIntern::clone(word),
                block,
            },
            label.span().to_owned(),
        )),
        _ => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: "Expected a label".to_string(),
                },
                label.span().pest(),
            )));
        }
    }
}

fn print_like(
    syntax: &ExpansionInfo,
    mut args: WithSpan<Vec<WithSpan<Value>>>,
    block: BlockID,
) -> Result<(RegisterReference, WithSpan<String>), Box<Error<Rule>>> {
    if args.len() != 2 {
        return Err(Box::new(Error::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Expected two arguments, found {}", args.len()),
            },
            args.span().pest(),
        )));
    }

    let register = expect_reg(args.pop().unwrap(), syntax, block)?;

    let message = args.pop().unwrap();
    let message_span = message.span().to_owned();
    let message = match message.into_inner() {
        Value::Word(v) => WithSpan::new(v.trim_matches('"').to_owned(), message_span),
        _ => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: "Expected a message".to_string(),
                },
                message_span.pest(),
            )));
        }
    };

    Ok((register, message))
}

pub fn builtin_macros(
    prelude: ArcIntern<String>,
) -> HashMap<(ArcIntern<String>, ArcIntern<String>), WithSpan<Macro>> {
    let mut macros = HashMap::new();

    let s = Span::new(ArcIntern::from_ref(" "), 0, 0);

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("add")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 2 {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("Expected two arguments, found {}", args.len()),
                        },
                        args.span().pest(),
                    )));
                }

                let reg = expect_reg(args.pop().unwrap(), syntax, block)?;

                let num = args.pop().unwrap();
                let num = match &*num {
                    Value::Int(int) => WithSpan::new(*int, num.span().to_owned()),
                    _ => {
                        return Err(Box::new(Error::new_from_span(
                            ErrorVariant::CustomError {
                                message: "Expected a number".to_string(),
                            },
                            num.span().pest(),
                        )));
                    }
                };

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Add {
                    amt: num,
                    register: reg,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("goto")),
        WithSpan::new(
            Macro::Builtin(|_syntax, mut args, block| {
                if args.len() != 1 {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("Expected one argument, found {}", args.len()),
                        },
                        args.span().pest(),
                    )));
                }

                let label = expect_label(args.pop().unwrap(), block)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Goto {
                    label,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("solved-goto")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 2 {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("Expected two arguments, found {}", args.len()),
                        },
                        args.span().pest(),
                    )));
                }

                let label = expect_label(args.pop().unwrap(), block)?;
                let register = expect_reg(args.pop().unwrap(), syntax, block)?;

                Ok(vec![Instruction::Code(Code::Primitive(
                    Primitive::SolvedGoto { register, label },
                ))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("input")),
        WithSpan::new(
            Macro::Builtin(|syntax, args, block| {
                let (register, message) = print_like(syntax, args, block)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Input {
                    register,
                    message,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("halt")),
        WithSpan::new(
            Macro::Builtin(|syntax, args, block| {
                let (register, message) = print_like(syntax, args, block)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Halt {
                    register,
                    message,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from_ref("print")),
        WithSpan::new(
            Macro::Builtin(|syntax, args, block| {
                let (register, message) = print_like(syntax, args, block)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Print {
                    register,
                    message,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros
}
