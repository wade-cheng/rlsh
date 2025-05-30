mod game;
mod job_list;

use job_list::State;

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
    NonBuiltin,
}

struct CommandData<'a> {
    command: &'a str,
    args: Vec<&'a str>,
    infile: Option<&'a str>,
    outfile: Option<&'a str>,
    command_type: Builtin,
    state: State,
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

    /// Parses the input and matches on it. If the input is a builtin, we do the
    /// corresponding operation. If we could not recognize a builtin, we pass it
    /// to the rlsh game's parser. Finally, if that returns an error, pass the
    /// string to execve (or whatever the command is).
    fn eval(input: &str) {
        let command = Self::parse(input);
        match command.command_type {
            Builtin::Exit => exit(0),
            Builtin::Jobs => println!("Jobsing"),
            Builtin::Bg => println!("Bging"),
            Builtin::Fg => println!("Fging"),
            Builtin::Noop => {}
            Builtin::NonBuiltin => match game::parse(command.command) {
                Ok(()) => {}
                Err(()) => Self::run_command(command),
            },
        }
    }

    fn run_command(command: CommandData) {
        // cd is supposed to be a shell builtin. it breaks on windows when we feed it
        // to Command.
        // TODO: implement this more formally.
        dbg!(command.state);
        dbg!(command.infile);
        dbg!(command.outfile);
        dbg!(command.command);
        dbg!(command.args.clone());
        if command.command == "cd" {
            match command.args.len() {
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
            env::set_current_dir(command.args[0]).unwrap_or_else(|_| println!("cd errored"));
            return;
        }

        match Command::new(command.command).args(command.args).status() {
            Ok(_) => (),
            Err(error) => match error.kind() {
                ErrorKind::NotFound => println!("{} not found.", command.command),
                _ => println!("unknown error."), // uhhh. todo.
            },
        };
    }

    /// Parses a command line input into a `Command`.
    ///
    /// Note that we match for builtins on the first word.
    /// This means `fg`, `fg sidjf`, and `fg --help` will return `Command::Fg`,
    /// but `fg___` will not.

    fn parse(input: &str) -> CommandData {
        let mut input: Vec<&str> = input.split_whitespace().collect();

        // first check if this is a foreground or background job
        let last_word = input.last();
        let state = match last_word {
            Some(&"&") => {
                input.pop();
                State::BG
            }
            _ => State::FG,
        };

        // Check for specified stdout and stdin
        let (infile, mut input) = match input.iter().position(|x| x == &"<") {
            Some(i) => {
                let mut new_input = input.split_off(i);
                new_input.remove(0);
                (input.last().map(|v| *v), new_input)
            }
            None => (None, input),
        };

        let outfile = match input.iter().position(|x| x == &">") {
            Some(i) => {
                let outvec = input.split_off(i);
                outvec.get(1).map(|v| *v)
            }
            None => None,
        };

        // if empty then return no op
        if input.len() == 0 {
            return CommandData {
                command: &"",
                args: vec![],
                infile,
                outfile,
                command_type: Builtin::Noop,
                state,
            };
        }

        // extract command
        let command = input.remove(0);

        let command_type = match command {
            "fg" => Builtin::Fg,
            "bg" => Builtin::Bg,
            "jobs" => Builtin::Jobs,
            "exit" => Builtin::Exit,
            _ => Builtin::NonBuiltin,
        };

        CommandData {
            command,
            args: input,
            infile,
            outfile,
            command_type,
            state,
        }
    }

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
