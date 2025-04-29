use std::{
    env,
    path::PathBuf,
    process::{self, exit},
};

pub fn parse_command(input: &str) -> Option<Command> {
    let command_parts: Vec<&str> = input.trim().split_whitespace().collect();

    if command_parts.len() < 1 {
        return None;
    }

    match command_parts[0] {
        "exit" => Some(Command::Exit), // might need the input later to change the exit code
        "echo" => Some(Command::Echo(input.trim()[4..].trim().to_owned())),
        "type" => Some(Command::Type(
            parse_command(input.trim()[4..].trim()).map(|sc| Box::new(sc)),
        )),
        _ => {
            let paths = env::var_os("PATH").unwrap();
            for path in env::split_paths(&paths) {
                let exec_path = path.join(command_parts[0]);
                if exec_path.is_file() {
                    return Some(Command::Executable(
                        exec_path,
                        command_parts[1..].iter().map(|s| s.to_string()).collect(),
                    ));
                }
            }
            return Some(Command::InvalidCommand(input.trim().to_owned()));
        }
    }
}

pub enum Command {
    Exit,
    Echo(String),
    Type(Option<Box<Command>>),
    Executable(PathBuf, Vec<String>),
    InvalidCommand(String),
}

impl Command {
    pub fn run(&self) {
        match self {
            Command::Exit => exit(0),
            Command::Echo(input) => println!("{}", input),
            Command::Type(subcommand) => {
                if subcommand.is_some() {
                    println!("{}", subcommand.as_ref().unwrap().r#type());
                }
            }
            Command::Executable(_, args) => {
                process::Command::new(self.name())
                    .args(args)
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            }
            Command::InvalidCommand(input) => println!("{}: command not found", input.trim()),
        }
    }

    fn r#type(&self) -> String {
        return match self {
            Command::Echo(..) | Command::Exit | Command::Type(..) => {
                format!("{} is a shell builtin", self.name())
            }
            Command::Executable(path, _) => format!("{} is {}", self.name(), path.display()),
            Command::InvalidCommand(input) => format!("{}: not found", input.trim()),
        };
    }

    fn name(&self) -> &str {
        return match self {
            Command::Exit => "exit",
            Command::Echo(..) => "echo",
            Command::Type(..) => "type",
            Command::Executable(path, _) => path.file_name().unwrap().to_str().unwrap(),
            Command::InvalidCommand(..) => "invalid_command",
        };
    }
}
