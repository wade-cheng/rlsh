mod game;

use std::{
    env,
    fs::{self, DirEntry},
    io::{self, Error, Write},
    path::PathBuf,
    process::{Command, exit},
};

/// Any string can be parsed into one of these variants.
///
/// These include the builtin commands for the shell, and a catch-all
/// NonBuiltin variant that contains the string.
enum Builtin<'a> {
    /// ls can be called with no args or one arg pointing to the directory to examine.
    Ls(Option<&'a str>),
    /// cd can be called with no args or one arg pointing to the directory to change to.
    Cd(Option<&'a str>),
    Exit,
    Jobs,
    Bg,
    Fg,
    Noop,
    NonBuiltin(&'a str),
}

impl<'a> Builtin<'a> {
    fn eval(self) {
        match self {
            Builtin::Ls(file) => {
                if let Err(error) = Self::ls(file) {
                    println!("ls errored: {error}")
                }
            }
            Builtin::Cd(dest) => Self::cd(dest),
            Builtin::Exit => exit(0),
            Builtin::Jobs => println!("Jobsing"),
            Builtin::Bg => println!("Bging"),
            Builtin::Fg => println!("Fging"),
            Builtin::Noop => {}
            Builtin::NonBuiltin(s) => Self::run_command(s),
        }
    }

    fn ls(file: Option<&str>) -> Result<(), Error> {
        let mut path = env::current_dir()?;
        path.push(file.unwrap_or("."));

        let entries = fs::read_dir(path)?;
        let mut files: Vec<DirEntry> = Vec::new();
        for e in entries {
            files.push(e?);
        }
        files.sort_by_key(|entry| !entry.file_type().unwrap().is_dir());

        for file in files {
            // ignore dotfiles.
            if let Some('.') = file
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .chars()
                .next()
            {
                continue;
            }
            if file.file_type().unwrap().is_dir() {
                print!("\x1b[1;34m"); // 1: bold text; 34: blue foreground
            }
            print!("{}", file.path().file_name().unwrap().display());
            if file.file_type().unwrap().is_dir() {
                print!("\x1b[0m"); // reset text styling
            }
            print!("\n");
            io::stdout().flush().unwrap();
        }

        Ok(())
    }

    fn cd(dest: Option<&str>) {
        // TODO: this computes homedir every call. we only need to when dest = None
        // I'd like to avoid creating a whole string because it's unneccessary, but
        // it's hard to get a string slice without such ownership without the borrow
        // checker complaining.
        let homedir = dirs::home_dir().unwrap();
        let dest = dest.unwrap_or(homedir.to_str().unwrap());
        env::set_current_dir(dest).unwrap_or_else(|error| println!("cd errored: {error}"));
    }

    fn run_command(s: &str) {
        let words: Vec<&str> = s.split_whitespace().collect();
        let (command, args) = (words[0], words.get(1..).unwrap_or(&[]));

        if let Err(error) = Command::new(command).args(args).status() {
            println!("{command} errored: {error}")
        }
    }
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
                    let command = Self::parse(&input_buffer);
                    command.eval();
                }
                Err(_) => panic!(),
            }

            input_buffer.clear();
        }
    }

    /// Parses a command line input into a `Command`.
    ///
    /// Note that we match for builtins on the first word.
    /// This means `fg`, `fg sidjf`, and `fg --help` will return `Command::Fg`,
    /// but `fg___` will not.
    fn parse(input: &str) -> Builtin {
        let words: Vec<&str> = input.split_whitespace().collect();
        match words.get(0) {
            Some(&"ls") => {
                if words.len() > 2 {
                    println!("ls: too many arguments");
                    return Builtin::Noop;
                }
                Builtin::Ls(words.get(1).map(|v| *v))
            }
            Some(&"cd") => {
                if words.len() > 2 {
                    println!("cd: too many arguments");
                    return Builtin::Noop;
                }
                Builtin::Cd(words.get(1).map(|v| *v))
            }
            Some(&"fg") => Builtin::Fg,
            Some(&"bg") => Builtin::Bg,
            Some(&"jobs") => Builtin::Jobs,
            Some(&"exit") => Builtin::Exit,
            Some(_) => Builtin::NonBuiltin(input),
            None => Builtin::Noop,
        }
    }
}
