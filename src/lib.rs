use std::io::{self, Write};

pub struct App {
    /// The prompt for the shell.
    /// For reference, this is `pc@user MINGW64 pwd $ ` on git bash.
    /// TODO: we should implement better than a static string.
    /// It should at least be able to read from `pwd`.
    prompt: String,
}

impl App {
    pub fn new() -> Self {
        App {
            prompt: "> ".to_string(),
        }
    }

    pub fn run(self) {
        let mut input_buffer = String::new();
        loop {
            // print prompt
            print!("{}", &self.prompt);
            io::stdout().flush().unwrap();

            match io::stdin().read_line(&mut input_buffer) {
                Ok(0) => return, // exit on EOF (CTRL-D)
                Ok(_) => {
                    println!("got {}", input_buffer);
                }
                Err(_) => panic!(),
            }
        }

        unreachable!();
    }
}
