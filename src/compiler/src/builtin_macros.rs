use internment::ArcIntern;
use pest::error::Error;
use qter_core::{Span, WithSpan, mk_error};

use crate::{
    BlockID, Code, ExpansionInfo, Instruction, LabelReference, Macro, Primitive, RegisterReference,
    Value, parsing::Rule,
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
            None => Err(mk_error(
                format!("The register {name} does not exist"),
                reg.span(),
            )),
        },
        _ => Err(mk_error("Expected a register", reg.span())),
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
        _ => Err(mk_error("Expected a label", label.span())),
    }
}

fn print_like(
    syntax: &ExpansionInfo,
    mut args: WithSpan<Vec<WithSpan<Value>>>,
    block: BlockID,
) -> Result<(Option<RegisterReference>, WithSpan<String>), Box<Error<Rule>>> {
    if args.len() > 2 {
        return Err(mk_error(
            format!("Expected one or two arguments, found {}", args.len()),
            args.span(),
        ));
    }

    let register = if args.len() == 2 {
        Some(expect_reg(args.pop().unwrap(), syntax, block)?)
    } else {
        None
    };

    let message = args.pop().unwrap();
    let message_span = message.span().to_owned();
    let message = match message.into_inner() {
        Value::Word(v) => {
            if !v.starts_with('"') || !v.ends_with('"') {
                return Err(mk_error("The message must be quoted", message_span));
            }

            let v = v.strip_prefix('"').unwrap_or(&v);
            let v = v.strip_suffix('"').unwrap_or(v);

            WithSpan::new(v.to_owned(), message_span)
        }
        _ => {
            return Err(mk_error("Expected a message", message_span));
        }
    };

    Ok((register, message))
}

pub fn builtin_macros(
    prelude: ArcIntern<str>,
) -> HashMap<(ArcIntern<str>, ArcIntern<str>), WithSpan<Macro>> {
    let mut macros = HashMap::new();

    let s = Span::new(ArcIntern::from(" "), 0, 0);

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("add")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
                }

                let num = args.pop().unwrap();
                let num = match &*num {
                    Value::Int(int) => WithSpan::new(*int, num.span().to_owned()),
                    _ => {
                        return Err(mk_error("Expected a number", num.span()));
                    }
                };

                let reg = expect_reg(args.pop().unwrap(), syntax, block)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Add {
                    amt: num,
                    register: reg,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("goto")),
        WithSpan::new(
            Macro::Builtin(|_syntax, mut args, block| {
                if args.len() != 1 {
                    return Err(mk_error(
                        format!("Expected one argument, found {}", args.len()),
                        args.span(),
                    ));
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
        (prelude.to_owned(), ArcIntern::from("solved-goto")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
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
        (prelude.to_owned(), ArcIntern::from("input")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
                }

                let register = expect_reg(args.pop().unwrap(), syntax, block)?;

                let message = args.pop().unwrap();
                let message_span = message.span().to_owned();
                let message = match message.into_inner() {
                    Value::Word(v) => WithSpan::new(v.trim_matches('"').to_owned(), message_span),
                    _ => {
                        return Err(mk_error("Expected a message", message_span));
                    }
                };

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Input {
                    register,
                    message,
                }))])
            }),
            s.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("halt")),
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
        (prelude.to_owned(), ArcIntern::from("print")),
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
