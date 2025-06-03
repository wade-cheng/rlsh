pub mod game;
mod job_list;

use job_list::{JobList, State};

use std::{
    env,
    fs::{self, DirEntry, File},
    io::{self, Error, Write},
    path::PathBuf,
    process::Stdio,
    time::SystemTime,
};

use tokio::{process::Command, task};

/// Any string can be parsed into one of these variants.
///
/// These include the builtin commands for the shell, and a catch-all
/// NonBuiltin variant that contains the string.
enum Executable {
    /// ls can be called with no args or one arg pointing to the directory to examine.
    Ls(LsData),
    /// cd can be called with no args or one arg pointing to the directory to change to.
    Cd(Option<String>),
    Exit,
    Jobs(Option<String>),
    Noop,
    TempDebugSpawnEnemy(String),
    TempDebugAttackEnemy(String),
    NonBuiltin(NonBuiltInData),
}

struct NonBuiltInData {
    // String that contains the command to pass to exec
    command: String,
    // Vector of string arguments to pass as the arguments to exec
    args: Vec<String>,
    // Weather this is to be run as a foreground or background job
    state: State,
    // The exact command that was entered into the command line
    cmdline: String,
    // An option that either contains a string to the file to replace stdin
    // or none if stdin should be inherrited
    infile: Option<String>,
    // An option that either contains a string to the file to replace stdout
    // or none if stdout should be inherrited
    outfile: Option<String>,
}

/// We attempt to mimic the GNU coreutils args as much as possible. This helps
/// users with familiarity with the terminal.
#[derive(Debug, Clone)]
struct LsData {
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
    // An option that either contains a string to the file to replace stdout
    // or none if stdout should be inherrited
    outfile: Option<String>,
}

impl Executable {
    async fn eval(self, job_list: &JobList) -> bool {
        match self {
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
            Executable::Jobs(outfile) => match job_list.list_jobs(outfile) {
                Ok(()) => (),
                Err(err) => println!("Error printing jobs: {err}"),
            },
            Executable::Exit => return false,
            Executable::Noop => {}
            Executable::NonBuiltin(data) => Self::run_command(data, job_list.clone()).await,
        };

        return true;
    }

    fn ls(mut data: LsData) -> Result<(), Error> {
        let mut outfile: Box<dyn Write> = match &data.outfile {
            Some(path) => Box::new(File::create(path)?),
            None => Box::new(io::stdout().lock()),
        };

        let path = env::current_dir()?;

        if data.files.len() == 0 {
            data.files.push(".".to_string());
        }

        let dirs = data.files.iter().map(|s| {
            let mut dir = path.clone();
            dir.push(s);
            dir
        });

        for dir in dirs {
            if data.files.len() != 1 {
                writeln!(outfile, "{}:", dir.file_name().unwrap().display())?;
            }

            let entries = fs::read_dir(dir)?;
            let mut files: Vec<DirEntry> = Vec::new();
            for e in entries {
                files.push(e?);
            }
            files.sort_by_key(|entry| {
                let is_dir = !entry.file_type().unwrap().is_dir();
                let fname = entry.file_name();
                if data.sort_time {
                    let x = entry.metadata().unwrap().modified().unwrap();
                    return (x, is_dir, fname);
                }
                (SystemTime::UNIX_EPOCH, is_dir, fname)
            });

            // Need Box hack because `iter` and `rev` have differently typed outputs.
            // An `either` crate exists for this use case, but we can cut down on
            // crate usage.
            let file_order: Box<dyn Iterator<Item = &DirEntry>> = if data.reverse {
                Box::new(files.iter().rev())
            } else {
                Box::new(files.iter())
            };
            for file in file_order {
                // ignore dotfiles. NOTE: let chains would help this look nicer, but are nightly.
                if !data.all {
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

                let (prefix, suffix) = if let None = data.outfile {
                    if file.file_type().unwrap().is_dir() {
                        ("\x1b[1;34m".to_string(), "\x1b[0m") // 1: bold text; 34: blue foreground; 0: reset
                    } else if let Ok(_) = game::get_entity(file.path()) {
                        ("\x1b[31m".to_string() + game::PERSON_ICON + " ", "\x1b[0m") // 31: red foreground; 0: reset
                    } else {
                        ("".to_string(), "")
                    }
                } else {
                    ("".to_string(), "")
                };

                write!(
                    outfile,
                    "{}{}{}",
                    prefix,
                    file.path().file_name().unwrap().display(),
                    suffix
                )?;

                if data.long {
                    write!(outfile, "\n")?;
                } else {
                    write!(outfile, "  ")?;
                }
                outfile.flush().unwrap();
            }

            if data.files.len() != 1 && !data.long {
                writeln!(outfile, "")?;
            }
        }

        if !data.long && data.files.len() <= 1 {
            writeln!(outfile, "")?;
        }

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

    // Runs a non built in command
    async fn run_command(data: NonBuiltInData, job_list: JobList) {
        // Calculate the infile
        let infile: Stdio = match data.infile {
            Some(path) => match File::create(path) {
                Ok(file) => file.into(),
                Err(err) => {
                    println!("Error opening file: {err}");
                    return;
                }
            },
            None => {
                if let State::FG = data.state {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                }
            }
        };

        // Calculate the outfile
        let outfile: Stdio = match data.outfile {
            Some(path) => match File::create(path) {
                Ok(file) => file.into(),
                Err(err) => {
                    println!("Error opening file: {err}");
                    return;
                }
            },
            None => {
                if let State::FG = data.state {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                }
            }
        };

        match Command::new(&data.command)
            .args(data.args)
            .stdin(infile)
            .stdout(outfile)
            .spawn()
        {
            Err(error) => println!("{} errored: {error}", data.command),
            Ok(mut child) => {
                let pid = child.id().unwrap_or(0);
                match job_list.add(pid, data.state, data.cmdline) {
                    Ok(jid) => {
                        if let State::FG = data.state {
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

    fn parse(input: &str) -> Executable {
        let cmdline = input.to_string();

        let mut input: Vec<&str> = input.split_whitespace().collect();
        if let Some(&"spawn") = input.get(0) {
            return Executable::TempDebugSpawnEnemy(String::from(
                input.get(1..).unwrap_or(&["goblin"]).join(" "),
            ));
        }

        if let Some(&"attack") = input.get(0) {
            return Executable::TempDebugAttackEnemy(String::from(
                input.get(1..).unwrap_or(&["goblin"]).join(" "),
            ));
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
            return Executable::Noop;
        }

        // extract command

        match input.remove(0) {
            "ls" => Self::parse_ls(input, outfile),
            "cd" => {
                if input.len() > 1 {
                    println!("cd: too many arguments");
                    Executable::Noop
                } else {
                    Executable::Cd(input.get(0).map(|v| v.to_string()))
                }
            }
            "jobs" => Executable::Jobs(outfile),
            "exit" => Executable::Exit,
            x => Executable::NonBuiltin(NonBuiltInData {
                command: x.to_string(),
                args: input.iter().map(|v| v.to_string()).collect(),
                state,
                cmdline,
                infile,
                outfile,
            }),
        }
    }

    fn parse_ls(mut input: Vec<&str>, outfile: Option<String>) -> Executable {
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
        let data = LsData {
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
            outfile,
        };

        if !arg_list.is_empty() {
            println!(
                "ls: could not recognize these arguments: {}",
                arg_list.join(" ")
            );
            Executable::Noop
        } else {
            Executable::Ls(data)
        }
    }
}
