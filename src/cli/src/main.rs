use std::{fs, io, path::PathBuf};

use clap::{ArgAction, Parser};
use color_eyre::{eyre::eyre, owo_colors::OwoColorize};
use compiler::compile;
use internment::ArcIntern;
use interpreter::{ExecutionState, Interpreter, PausedState};
use qter_core::{Int, I};

/// Compiles and interprets qter programs
#[derive(Parser)]
#[command(version, about)]
enum Commands {
    /// Compile a QAT file to Q
    Compile {
        /// Which file to compile; must be a .q file
        file: PathBuf,
    },
    /// Interpret a QAT or a Q file
    Interpret {
        /// Which file to interpret; must be a .qat or .q file
        file: PathBuf,
        /// The level of execution trace to send to stderr. Can be set zero to three times.
        #[arg(short, action = ArgAction::Count)]
        trace: u8,
    },
    /// Step through a QAT or a Q program
    Debug {
        /// Which file to interpret; must be a .qat or .q file
        file: PathBuf,
    },
    /// Evaluate unit tests in a QAT program
    Test {
        /// Which file to test; must be a .qat file
        file: PathBuf,
    },
}

fn main() -> color_eyre::Result<()> {
    let args = Commands::parse();

    match args {
        Commands::Compile { file: _ } => todo!(),
        Commands::Interpret { file, trace } => {
            let program = match file.extension().and_then(|v| v.to_str()) {
                Some("q") => todo!(),
                Some("qat") => {
                    let text = fs::read_to_string(file)?;

                    compile(&text, |name| {
                        let path = PathBuf::from(name);

                        if path.ancestors().count() > 1 {
                            // Easier not to implement relative paths and stuff
                            return Err("Imported files must be in the same path".to_owned());
                        }

                        match fs::read_to_string(path) {
                            Ok(s) => Ok(ArcIntern::from(s)),
                            Err(e) => Err(e.to_string()),
                        }
                    })?
                }
                _ => {
                    return Err(eyre!(
                        "The file {file:?} must have an extension of `.qat` or `.q`."
                    ));
                }
            };

            let interpreter = Interpreter::new(program);

            if trace == 0 {
                interpret_fast(interpreter)?;
            } else {
                interpret_slow(interpreter, trace)?;
            }
        }
        Commands::Debug { file: _ } => todo!(),
        Commands::Test { file: _ } => todo!(),
    }

    Ok(())
}

fn interpret_fast(mut interpreter: Interpreter) -> color_eyre::Result<()> {
    loop {
        let state = interpreter.step_until_halt();

        let is_input = matches!(
            state,
            PausedState::Input {
                max_input: _,
                register_idx: _,
                register: _
            }
        );

        while let Some(message) = interpreter.messages().pop_front() {
            println!("{message}");
        }

        if is_input {
            give_number_input(&mut interpreter)?;
        } else {
            break Ok(());
        }
    }
}

fn give_number_input(interpreter: &mut Interpreter) -> color_eyre::Result<()> {
    loop {
        let mut number = String::new();
        io::stdin().read_line(&mut number)?;
        match number.parse::<Int<I>>() {
            Ok(v) => match interpreter.give_input(v) {
                Ok(_) => break Ok(()),
                Err(e) => println!("{e}"),
            },
            Err(_) => println!("Please input an integer"),
        }
    }
}

fn interpret_slow(mut interpreter: Interpreter, trace: u8) -> color_eyre::Result<()> {
    let pad_amt = ((interpreter.program().instructions.len() - 1).ilog10() + 1) as usize;

    loop {
        if trace >= 3 {
            let mut string = interpreter.program_counter().to_string();

            while string.len() < pad_amt {
                string.push(' ');
            }

            eprint!("{} | ", string);
        }

        let action = interpreter.step();

        let mut should_give_input = false;

        match action {
            interpreter::ActionPerformed::None => {
                if trace >= 2 {
                    eprintln!("Printing");
                }
            }
            interpreter::ActionPerformed::Paused => {
                let is_input = matches!(
                    interpreter.execution_state(),
                    ExecutionState::Paused(PausedState::Input {
                        max_input: _,
                        register_idx: _,
                        register: _
                    })
                );

                if is_input {
                    if trace >= 2 {
                        eprintln!("Accepting input");
                    }

                    should_give_input = true;
                } else {
                    if trace >= 2 {
                        eprintln!("Halting");
                    }

                    break Ok(());
                }
            }
            interpreter::ActionPerformed::Goto { location: _ } => {
                if trace >= 3 {
                    eprintln!("Jumping");
                }
            }
            interpreter::ActionPerformed::FailedSolvedGoto {
                puzzle_idx,
                facelets: _,
            } => {
                if trace >= 2 {
                    eprintln!("Inspect puzzle {puzzle_idx} - {}", "NOT TAKEN".red());
                }
            }
            interpreter::ActionPerformed::SucceededSolvedGoto {
                puzzle_idx,
                facelets: _,
                location: _,
            } => {
                if trace >= 2 {
                    eprintln!("Inspect puzzle {puzzle_idx} - {}", "TAKEN".green());
                }
            }
            interpreter::ActionPerformed::AddedToTheoretical { register_idx, amt } => {
                eprintln!("Theoretical {register_idx} += {amt}");
            }
            interpreter::ActionPerformed::ExecutedAlgorithm {
                puzzle_idx,
                algorithm,
            } => {
                eprint!("Puzzle {puzzle_idx}:");

                for generator in algorithm.generators() {
                    eprint!(" {generator}");
                }

                eprintln!();
            }
            interpreter::ActionPerformed::Panicked => {
                eprintln!("{}", "Panicked!".red());
            }
        }

        while let Some(message) = interpreter.messages().pop_front() {
            println!("{message}");
        }

        if should_give_input {
            give_number_input(&mut interpreter)?;
        }
    }
}
