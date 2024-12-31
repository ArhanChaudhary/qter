use internment::ArcIntern;
use pest::error::{Error, ErrorVariant};
use qter_core::{Span, WithSpan};

use crate::{
    parsing::Rule, BlockID, Code, Instruction, Label, Macro, ParsedSyntax, Primitive,
    RegisterReference, Value,
};

use std::collections::HashMap;

fn expect_reg(
    reg: WithSpan<Value>,
    syntax: &ParsedSyntax,
    block: BlockID,
) -> Result<RegisterReference, Box<Error<Rule>>> {
    match &*reg {
        Value::Word(name) => match syntax.get_register(
            WithSpan::new(ArcIntern::clone(name), reg.span().to_owned()),
            block,
        ) {
            Some(v) => Ok(v),
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
                    message: format!("Expected a register"),
                },
                reg.span().pest(),
            )));
        }
    }
}

fn expect_label(
    label: WithSpan<Value>,
    syntax: &ParsedSyntax,
    block: BlockID,
) -> Result<WithSpan<Label>, Box<Error<Rule>>> {
    match &*label {
        Value::Word(word) => match syntax.get_label(word, block) {
            Some(v) => Ok(WithSpan::new(v, label.span().to_owned())),
            None => {
                return Err(Box::new(Error::new_from_span(
                    ErrorVariant::CustomError {
                        message: format!("Label could not be found"),
                    },
                    label.span().pest(),
                )))
            }
        },
        _ => {
            return Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("Expected a label"),
                },
                label.span().pest(),
            )));
        }
    }
}

fn print_like(
    syntax: &ParsedSyntax,
    mut args: WithSpan<Vec<WithSpan<Value>>>,
    block: BlockID,
) -> Result<(RegisterReference, WithSpan<String>), Box<Error<Rule>>> {
    if args.is_empty() {
        return Err(Box::new(Error::new_from_span(
            ErrorVariant::CustomError {
                message: format!("Expected some arguments, found none"),
            },
            args.span().pest(),
        )));
    }

    args.reverse();

    let register = expect_reg(args.pop().unwrap(), syntax, block)?;

    let span = args.span().to_owned();

    let message = args
        .into_inner()
        .into_iter()
        .rev()
        .map(|v| match &*v {
            Value::Word(word) => Ok(WithSpan::new(String::clone(&word), v.span().to_owned())),
            _ => Err(Box::new(Error::new_from_span(
                ErrorVariant::CustomError {
                    message: format!("Expected an identifier"),
                },
                v.span().pest(),
            ))),
        })
        .reduce(|a, v| {
            let mut a = a?;
            let v = v?;

            a.push_str(&v);

            let span = a.span().to_owned().merge(v.span());

            Ok(WithSpan::new(a.into_inner(), span))
        })
        .transpose()?
        .unwrap_or_else(|| WithSpan::new(String::new(), span));

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
                                message: format!("Expected a number"),
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
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 1 {
                    return Err(Box::new(Error::new_from_span(
                        ErrorVariant::CustomError {
                            message: format!("Expected one argument, found {}", args.len()),
                        },
                        args.span().pest(),
                    )));
                }

                let label = expect_label(args.pop().unwrap(), syntax, block)?;

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

                let label = expect_label(args.pop().unwrap(), syntax, block)?;
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
