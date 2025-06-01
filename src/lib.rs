mod game;
mod job_list;

use job_list::{JobList, State};

use std::{
    env,
    fs::{self, DirEntry},
    io::{self, Error, Write},
    path::PathBuf,
    process::Stdio,
    time::SystemTime,
};

use tokio::{
    process::Command,
    task,
};

/// Any string can be parsed into one of these variants.
///
/// These include the builtin commands for the shell, and a catch-all
/// NonBuiltin variant that contains the string.
enum Executable {
    /// ls can be called with no args or one arg pointing to the directory to examine.
    Ls(LsArgs),
    /// cd can be called with no args or one arg pointing to the directory to change to.
    Cd(Option<String>),
    Exit,
    Jobs,
    Noop,
    TempDebugSpawnEnemy(String),
    TempDebugAttackEnemy(String),
    NonBuiltin {
        command: String,
        args: Vec<String>,
    },
}

/// We attempt to mimic the GNU coreutils args as much as possible. This helps
/// users with familiarity with the terminal.
#[derive(Debug, Clone)]
struct LsArgs {
    /// The files to list information from. Will default to the current working
    /// directory if `files.len() == 0`.
    files: Vec<String>,
    /// `-a`, `--all`.
    /// Whether to include entires starting with `.`.
    all: bool,
    /// `-l`.
    /// Whether to use a long listing format (separated by newlines instead of two spaces).
    long: bool,
    /// `-r`, `--reverse`.
    /// Whether to reverse order of listing.
    reverse: bool,
    /// `-t`.
    /// Whether to sort by time.
    sort_time: bool,
}

struct CommandData {
    command: Executable,
    infile: Option<String>,
    outfile: Option<String>,
    state: State,
    cmdline: String,
}

impl CommandData {
    async fn eval(self, job_list: &JobList) -> bool {
        match &self.command {
            Executable::TempDebugSpawnEnemy(s) => game::spawn(
                game::Entity {
                    components: Vec::from([
                        game::Component::Enemy,
                        game::Component::TakesDamage(5),
                    ]),
                },
                s,
            ),
            Executable::TempDebugAttackEnemy(s) => {
                if let Err(_) = game::attack(&s) {
                    println!("could not attack {s}??? weirdo...");
                }
            }
            Executable::Ls(args) => {
                if let Err(error) = Self::ls(args.clone()) {
                    println!("ls errored: {error}")
                }
            }
            Executable::Cd(dest) => Self::cd(&dest),
            Executable::Jobs => match job_list.list_jobs(self.outfile) {
                Ok(()) => (),
                Err(err) => println!("Error printing jobs: {err}"),
            },
            Executable::Exit => return false,
            Executable::Noop => {}
            Executable::NonBuiltin { command, args } => self.run_command(job_list.clone()).await,
        };

        return true;
    }

    fn ls(mut args: LsArgs) -> Result<(), Error> {
        let path = env::current_dir()?;

        if args.files.len() == 0 {
            args.files.push(".".to_string());
        }

        let dirs = args.files.iter().map(|s| {
            let mut dir = path.clone();
            dir.push(s);
            dir
        });

        for dir in dirs {
            if args.files.len() != 1 {
                println!("{}:", dir.file_name().unwrap().display())
            }

            let entries = fs::read_dir(dir)?;
            let mut files: Vec<DirEntry> = Vec::new();
            for e in entries {
                files.push(e?);
            }
            files.sort_by_key(|entry| {
                let is_dir = !entry.file_type().unwrap().is_dir();
                let fname = entry.file_name();
                if args.sort_time {
                    let x = entry.metadata().unwrap().modified().unwrap();
                    return (x, is_dir, fname);
                }
                (SystemTime::UNIX_EPOCH, is_dir, fname)
            });

            // Need Box hack because `iter` and `rev` have differently typed outputs.
            // An `either` crate exists for this use case, but we can cut down on
            // crate usage.
            let file_order: Box<dyn Iterator<Item = &DirEntry>> = if args.reverse {
                Box::new(files.iter().rev())
            } else {
                Box::new(files.iter())
            };
            for file in file_order {
                // ignore dotfiles. NOTE: let chains would help this look nicer, but are nightly.
                if !args.all {
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
                }

                let (prefix, suffix) = if file.file_type().unwrap().is_dir() {
                    ("\x1b[1;34m".to_string(), "\x1b[0m") // 1: bold text; 34: blue foreground; 0: reset
                } else if let Ok(_) = game::get_entity(file.path()) {
                    ("\x1b[31m".to_string() + game::PERSON_ICON + " ", "\x1b[0m") // 31: red foreground; 0: reset
                } else {
                    ("".to_string(), "")
                };
                print!(
                    "{}{}{}",
                    prefix,
                    file.path().file_name().unwrap().display(),
                    suffix
                );

                if args.long {
                    print!("\n");
                } else {
                    print!("  ");
                }
                io::stdout().flush().unwrap();
            }

            if args.files.len() != 1 {
                println!("");
            }
        }

        println!("");

        Ok(())
    }

    fn cd(dest: &Option<String>) {
        // TODO: this computes homedir every call. we only need to when dest = None
        // I'd like to avoid creating a whole string because it's unneccessary, but
        // it's hard to get a string slice without such ownership without the borrow
        // checker complaining.
        let homedir = dirs::home_dir().unwrap();
        let dest = dest.as_deref().unwrap_or(homedir.to_str().unwrap());
        env::set_current_dir(dest).unwrap_or_else(|error| println!("cd errored: {error}"));
    }

    async fn run_command(self, job_list: JobList) {
        if let Executable::NonBuiltin { command, args } = self.command {
            match Command::new(&command)
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Err(error) => println!("{command} errored: {error}"),
                Ok(mut child) => {
                    let pid = child.id().unwrap_or(0);
                    match job_list.add(pid, self.state, self.cmdline) {
                        Ok(jid) => {
                            if let State::FG = self.state {
                                child.wait().await.expect("Error waiting for child");
                                if !job_list.delete(jid) {
                                    eprintln!("Failed to remove job");
                                }
                            } else {
                                let cmdline = job_list.get_cmdline(jid).unwrap_or(String::new());
                                task::spawn(async move {
                                    print!("[{jid}] ({pid}) {}", cmdline);

                                    child.wait().await.expect("Error waiting for child");

                                    if !job_list.delete(jid) {
                                        eprintln!("Failed to remove job");
                                    }
                                    println!("\nJob [{jid}] ({pid}) terminated");
                                });
                            }
                        }
                        Err(error) => {
                            eprintln!("{error}");
                            child.kill().await.expect("Error killing child");
                            child.wait().await.expect("Error waiting for child");
                        }
                    }
                }
            };
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

    #[tokio::main]
    pub async fn run(self) {
        let mut input_buffer = String::new();
        let job_list = JobList::new();
        loop {
            Self::print_prompt();

            match io::stdin().read_line(&mut input_buffer) {
                Ok(0) => return, // exit on EOF (CTRL-D)
                Ok(_) => {
                    let command = Self::parse(&input_buffer);
                    if !command.eval(&job_list).await {
                        return;
                    }
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

    fn parse(input: &str) -> CommandData {
        let cmdline = input.to_string();

        let mut input: Vec<&str> = input.split_whitespace().collect();
        if let Some(&"spawn") = input.get(0) {
            return CommandData {
                command: Executable::TempDebugSpawnEnemy(String::from(
                    input.get(1..).unwrap_or(&["goblin"]).join(" "),
                )),
                infile: None,
                outfile: None,
                state: State::FG,
                cmdline,
            };
        }
        if let Some(&"attack") = input.get(0) {
            return CommandData {
                command: Executable::TempDebugAttackEnemy(String::from(
                    input.get(1..).unwrap_or(&["goblin"]).join(" "),
                )),
                infile: None,
                outfile: None,
                state: State::FG,
                cmdline,
            };
        }

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
                (input.last().map(|v| v.to_string()), new_input)
            }
            None => (None, input),
        };

        let outfile = match input.iter().position(|x| x == &">") {
            Some(i) => {
                let outvec = input.split_off(i);
                outvec.get(1).map(|v| v.to_string())
            }
            None => None,
        };

        // if empty then return no op
        if input.len() == 0 {
            return CommandData {
                command: Executable::Noop,
                infile,
                outfile,
                state,
                cmdline,
            };
        }

        // extract command

        let command = match input.remove(0) {
            "ls" => Self::parse_ls(input),
            "cd" => {
                if input.len() > 1 {
                    println!("cd: too many arguments");
                    Executable::Noop
                } else {
                    Executable::Cd(input.get(0).map(|v| v.to_string()))
                }
            }
            "jobs" => Executable::Jobs,
            "exit" => Executable::Exit,
            x => Executable::NonBuiltin {
                command: x.to_string(),
                args: input.iter().map(|v| v.to_string()).collect(),
            },
        };

        CommandData {
            command,
            infile,
            outfile,
            state,
            cmdline,
        }
    }

    fn parse_ls(mut input: Vec<&str>) -> Executable {
        let mut arg_list: Vec<String> = Vec::new();
        input.retain(|word| {
            // input was split by whitespace, guaranteeing that word is nonzero length
            let starts_with_dash = word.chars().nth(0).unwrap() == '-';
            if starts_with_dash && word.len() > 1 {
                if word.chars().nth(1).unwrap() == '-' {
                    // move --long-args to arg_list
                    arg_list.push(word.to_string());
                } else {
                    // move -args to arg_list as -a -r -g -s
                    arg_list.append(
                        &mut word[1..]
                            .chars()
                            .map(|c| {
                                let mut arg = String::from("-");
                                arg.push(c);
                                arg
                            })
                            .collect(),
                    );
                }
                return false;
            }
            true // keep `-` in FILES to ls through for gnu corelib parity. `-` is a valid dir after all.
            // TODO: testcase about it. also, pull this whole parsing code out into a module.
        });

        let mut old_arg_list_len;
        let args = LsArgs {
            all: {
                old_arg_list_len = arg_list.len();
                arg_list.retain(|word| !(*word == "-a" || *word == "--all"));
                old_arg_list_len > arg_list.len()
            },
            long: {
                old_arg_list_len = arg_list.len();
                arg_list.retain(|word| !(*word == "-l"));
                old_arg_list_len > arg_list.len()
            },
            reverse: {
                old_arg_list_len = arg_list.len();
                arg_list.retain(|word| !(*word == "-r" || *word == "--reverse"));
                old_arg_list_len > arg_list.len()
            },
            sort_time: {
                old_arg_list_len = arg_list.len();
                arg_list.retain(|word| !(*word == "-t"));
                old_arg_list_len > arg_list.len()
            },
            files: input.iter().map(|v| v.to_string()).collect(),
        };

        if !arg_list.is_empty() {
            println!(
                "ls: could not recognize these arguments: {}",
                arg_list.join(" ")
            );
            Executable::Noop
        } else {
            Executable::Ls(args)
        }
    }
}
