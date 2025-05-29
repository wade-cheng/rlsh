mod game;

use std::{
    env,
    io::{self, ErrorKind, Write},
    path::PathBuf,
    process::{Command, exit},
};

/// Any string can be parsed into one of these variants.
///
/// These include the builtin commands for the shell, and a catch-all
/// NonBuiltin variant that contains the string.
enum Builtin<'a> {
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
            Builtin::Exit => exit(0),
            Builtin::Jobs => println!("Jobsing"),
            Builtin::Bg => println!("Bging"),
            Builtin::Fg => println!("Fging"),
            Builtin::Noop => {}
            Builtin::NonBuiltin(s) => match game::parse(s) {
                Ok(()) => {}
                Err(()) => Self::run_command(s),
            },
        }
    }

    fn run_command(s: &str) {
        let words: Vec<&str> = s.split_whitespace().collect();
        let (command, args) = (words[0], words.get(1..).unwrap_or(&[]));

        // cd is supposed to be a shell builtin. it breaks on windows when we feed it
        // to Command.
        // TODO: implement this more formally.
        if command == "cd" {
            match args.len() {
                0 => {
                    println!("cd with 0 args unimplemented");
                    return;
                }
                1 => {}
                _ => {
                    println!("cd with more than one arg unimplemented");
                    return;
                }
            };
            env::set_current_dir(args[0]).unwrap_or_else(|_| println!("cd errored"));
            return;
        }

        match Command::new(command).args(args).status() {
            Ok(_) => (),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => println!("{} not found.", command),
                _ => println!("unknown error."), // uhhh. todo.
            },
        };
    }

    /// Parses a command line input into a `Command`.
    ///
    /// Note that we match for builtins on the first word.
    /// This means `fg`, `fg sidjf`, and `fg --help` will return `Command::Fg`,
    /// but `fg___` will not.
    fn parse(input: &str) -> Builtin {
        let first_word = input.split_whitespace().next();
        match first_word {
            Some("fg") => Builtin::Fg,
            Some("bg") => Builtin::Bg,
            Some("jobs") => Builtin::Jobs,
            Some("exit") => Builtin::Exit,
            Some(_) => Builtin::NonBuiltin(input),
            None => Builtin::Noop,
        }
    }
}
