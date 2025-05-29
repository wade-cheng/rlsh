mod game;

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::exit,
};

/// Any string can be parsed into one of these variants.
///
/// These include the builtin commands for the shell, and a catch-all
/// NonBuiltin variant that contains the string.
enum Builtin<'a> {
    Clear,
    Exit,
    Jobs,
    Bg,
    Fg,
    Noop,
    NonBuiltin(&'a str),
}

pub struct App;

impl App {
    pub fn new() -> Self {
        App
    }

    /// Prints the prompt for the shell.
    ///
    /// That is, the thing that looks like `user@device ~/... $`.
    fn print_prompt() {
        print!(
            "{}@{} {} $ ",
            whoami::username(),
            whoami::devicename(),
            env::current_dir().unwrap_or(PathBuf::from("?")).display()
        );
        io::stdout().flush().unwrap();
    }

    pub fn run(self) {
        let mut input_buffer = String::new();
        loop {
            Self::print_prompt();

            match io::stdin().read_line(&mut input_buffer) {
                Ok(0) => return, // exit on EOF (CTRL-D)
                Ok(_) => Self::eval(&input_buffer),
                Err(_) => panic!(),
            }

            input_buffer.clear();
        }
    }

    /// Parses the input and matches on it. If the input is a builtin, we do the
    /// corresponding operation. If we could not recognize a builtin, we pass it
    /// to the rlsh game's parser. Finally, if that returns an error, pass the
    /// string to execve (or whatever the command is).
    fn eval(input: &str) {
        let command = Self::parse(input);
        match command {
            Builtin::Clear => {
                // use magic control sequence to clear the screen and position the
                // cursor at 1,1.
                // TODO: I've been a fool; clear is a terminal command, not a builtin :0
                // TODO: remove once we actually implement more of the shell.
                print!("\x1B[2J\x1B[1;1H");
            }
            Builtin::Exit => exit(0),
            Builtin::Jobs => println!("Jobsing"),
            Builtin::Bg => println!("Bging"),
            Builtin::Fg => println!("Fging"),
            Builtin::Noop => {}
            Builtin::NonBuiltin(s) => match game::parse(s) {
                Ok(()) => {}
                Err(()) => println!("NonBuiltining"),
            },
        }
    }

    /// Parses a command line input into a `Command`.
    ///
    /// Note that we match for builtins on the first word.
    /// This means `fg`, `fg sidjf`, and `fg --help` will return `Command::Fg`,
    /// but `fg___` will not.
    fn parse(input: &str) -> Builtin {
        let first_word = input.split_whitespace().next();
        match first_word {
            Some("clear") => Builtin::Clear,
            Some("fg") => Builtin::Fg,
            Some("bg") => Builtin::Bg,
            Some("jobs") => Builtin::Jobs,
            Some("exit") => Builtin::Exit,
            Some(_) => Builtin::NonBuiltin(input),
            None => Builtin::Noop,
        }
    }
}
