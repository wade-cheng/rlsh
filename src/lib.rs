use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::exit,
};

enum Command<'a> {
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
                Ok(_) => {
                    Self::eval(&input_buffer);
                }
                Err(_) => panic!(),
            }

            input_buffer.clear();
        }
    }

    fn eval(input: &str) {
        let command = Self::parse(input);
        match command {
            Command::Clear => {
                // use magic control sequence to clear the screen and position the
                // cursor at 1,1.
                // TODO: I've been a fool; clear is a terminal command, not a builtin :0
                // TODO: remove once we actually implement more of the shell.
                print!("\x1B[2J\x1B[1;1H");
            }
            Command::Exit => exit(0),
            Command::Jobs => println!("Jobsing"),
            Command::Bg => println!("Bging"),
            Command::Fg => println!("Fging"),
            Command::Noop => {}
            Command::NonBuiltin(s) => println!("NonBuiltining: {s}"),
        }
    }

    /// Parses a command line input into a `Command`.
    ///
    /// Note that we match for builtins on the first word.
    /// This means `fg`, `fg sidjf`, and `fg --help` will return `Command::Fg`,
    /// but `fg___` will not.
    fn parse(input: &str) -> Command {
        let first_word = input.split_whitespace().next();
        match first_word {
            Some("clear") => Command::Clear,
            Some("fg") => Command::Fg,
            Some("bg") => Command::Bg,
            Some("jobs") => Command::Jobs,
            Some("exit") => Command::Exit,
            Some(_) => Command::NonBuiltin(input),
            None => Command::Noop,
        }
    }
}
