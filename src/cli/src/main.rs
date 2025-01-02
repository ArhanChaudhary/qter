use std::{fs, io, path::PathBuf};

use clap::Parser;
use color_eyre::eyre::eyre;
use compiler::compile;
use internment::ArcIntern;
use interpreter::{Interpreter, PausedState};
use qter_core::{Int, I, U};

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
        Commands::Interpret { file } => {
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
                            Ok(s) => Ok(ArcIntern::new(s)),
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

            let mut interpreter = Interpreter::new(program);

            loop {
                let state = interpreter.step_until_halt();

                let is_input = matches!(
                    state,
                    PausedState::Input {
                        message: _,
                        register_idx: _,
                        register: _
                    }
                );

                while let Some(message) = interpreter.messages().pop_front() {
                    println!("{message}");
                }

                if is_input {
                    loop {
                        let mut number = String::new();
                        io::stdin().read_line(&mut number)?;
                        match number.parse::<Int<I>>() {
                            Ok(v) => {
                                interpreter.give_input(v);
                                break;
                            }
                            Err(_) => println!("Please input an integer"),
                        }
                    }
                } else {
                    break;
                }
            }
        }
        Commands::Debug { file: _ } => todo!(),
        Commands::Test { file: _ } => todo!(),
    }

    Ok(())
}
