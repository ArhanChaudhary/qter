use internment::ArcIntern;
use pest::error::Error;
use qter_core::{Span, WithSpan, mk_error};

use crate::{
    BlockID, Code, ExpansionInfo, Instruction, LabelReference, Macro, Primitive, RegisterReference,
    Value, parsing::Rule,
};

use std::collections::HashMap;

fn expect_reg(
    reg_value: WithSpan<Value>,
    syntax: &ExpansionInfo,
    block_id: BlockID,
) -> Result<RegisterReference, Box<Error<Rule>>> {
    match &*reg_value {
        Value::Ident(reg_name) => match syntax.block_info.get_register(
            &RegisterReference::parse(
                block_id,
                WithSpan::new(ArcIntern::clone(reg_name), reg_value.span().to_owned()),
            )
            .map_err(|e| {
                mk_error(
                    format!("Could not parse the modulus as a string: {e}"),
                    reg_value.span(),
                )
            })?,
        ) {
            Some((reg, _)) => Ok(reg),
            None => Err(mk_error(
                format!("The register {reg_name} does not exist"),
                reg_value.span(),
            )),
        },
        _ => Err(mk_error("Expected a register", reg_value.span())),
    }
}

fn expect_label(
    label_value: WithSpan<Value>,
    block_id: BlockID,
) -> Result<WithSpan<LabelReference>, Box<Error<Rule>>> {
    match &*label_value {
        Value::Ident(label_name) => Ok(WithSpan::new(
            LabelReference {
                name: ArcIntern::clone(label_name),
                block_id,
            },
            label_value.span().to_owned(),
        )),
        _ => Err(mk_error("Expected a label", label_value.span())),
    }
}

fn print_like(
    syntax: &ExpansionInfo,
    mut args: WithSpan<Vec<WithSpan<Value>>>,
    block_id: BlockID,
) -> Result<(Option<RegisterReference>, WithSpan<String>), Box<Error<Rule>>> {
    if args.len() > 2 {
        return Err(mk_error(
            format!("Expected one or two arguments, found {}", args.len()),
            args.span(),
        ));
    }

    let maybe_reg = if args.len() == 2 {
        Some(expect_reg(args.pop().unwrap(), syntax, block_id)?)
    } else {
        None
    };

    let message = args.pop().unwrap();
    let span = message.span().to_owned();
    let message = match message.into_inner() {
        Value::Ident(raw_message) => {
            if !raw_message.starts_with('"') || !raw_message.ends_with('"') {
                return Err(mk_error("The message must be quoted", span));
            }

            let raw_message = raw_message.strip_prefix('"').unwrap_or(&raw_message);
            let raw_message = raw_message.strip_suffix('"').unwrap_or(raw_message);

            WithSpan::new(raw_message.to_owned(), span)
        }
        _ => {
            return Err(mk_error("Expected a message", span));
        }
    };

    Ok((maybe_reg, message))
}

pub fn builtin_macros(
    prelude: ArcIntern<str>,
) -> HashMap<(ArcIntern<str>, ArcIntern<str>), WithSpan<Macro>> {
    let mut macros = HashMap::new();

    let dummy_span = Span::new(ArcIntern::from(" "), 0, 0);

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("add")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block_id| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
                }

                let second_arg = args.pop().unwrap();
                let amt = match *second_arg {
                    Value::Int(int) => WithSpan::new(int, second_arg.span().to_owned()),
                    _ => {
                        return Err(mk_error("Expected a number", second_arg.span()));
                    }
                };

                let register = expect_reg(args.pop().unwrap(), syntax, block_id)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Add {
                    amt,
                    register,
                }))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("goto")),
        WithSpan::new(
            Macro::Builtin(|_syntax, mut args, block_id| {
                if args.len() != 1 {
                    return Err(mk_error(
                        format!("Expected one argument, found {}", args.len()),
                        args.span(),
                    ));
                }

                let label = expect_label(args.pop().unwrap(), block_id)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Goto {
                    label,
                }))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("solved-goto")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block_id| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
                }

                let label = expect_label(args.pop().unwrap(), block_id)?;
                let register = expect_reg(args.pop().unwrap(), syntax, block_id)?;

                Ok(vec![Instruction::Code(Code::Primitive(
                    Primitive::SolvedGoto { register, label },
                ))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("input")),
        WithSpan::new(
            Macro::Builtin(|syntax, mut args, block_id| {
                if args.len() != 2 {
                    return Err(mk_error(
                        format!("Expected two arguments, found {}", args.len()),
                        args.span(),
                    ));
                }

                let register = expect_reg(args.pop().unwrap(), syntax, block_id)?;

                let second_arg = args.pop().unwrap();
                let span = second_arg.span().to_owned();
                let message = match second_arg.into_inner() {
                    Value::Ident(raw_message) => {
                        WithSpan::new(raw_message.trim_matches('"').to_owned(), span)
                    }
                    _ => {
                        return Err(mk_error("Expected a message", span));
                    }
                };

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Input {
                    register,
                    message,
                }))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("halt")),
        WithSpan::new(
            Macro::Builtin(|syntax, args, block_id| {
                let (register, message) = print_like(syntax, args, block_id)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Halt {
                    register,
                    message,
                }))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros.insert(
        (prelude.to_owned(), ArcIntern::from("print")),
        WithSpan::new(
            Macro::Builtin(|syntax, args, block_id| {
                let (register, message) = print_like(syntax, args, block_id)?;

                Ok(vec![Instruction::Code(Code::Primitive(Primitive::Print {
                    register,
                    message,
                }))])
            }),
            dummy_span.to_owned(),
        ),
    );

    macros
}
