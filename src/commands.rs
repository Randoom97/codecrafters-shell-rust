use std::{
    env,
    path::PathBuf,
    process::{self, exit},
    str::FromStr,
};

pub fn parse_command(input: &str) -> Option<Command> {
    let home = env::var_os("HOME").unwrap();
    let true_input = quote_aware_replace(input, "~", home.to_str().unwrap());
    let command_parts = quote_aware_split(&true_input);

    if command_parts.len() < 1 {
        return None;
    }

    match command_parts[0].as_str() {
        "exit" => Some(Command::Exit), // might need the input later to change the exit code
        "echo" => Some(Command::Echo(
            command_parts[1..].iter().map(|s| s.to_owned()).collect(),
        )),
        "type" => Some(Command::Type(
            parse_command(true_input.trim()[4..].trim()).map(|sc| Box::new(sc)),
        )),
        "pwd" => Some(Command::PWD),
        "cd" => Some(Command::CD(true_input.trim()[2..].trim().to_owned())),
        _ => {
            let paths = env::var_os("PATH").unwrap();
            for path in env::split_paths(&paths) {
                let exec_path = path.join(command_parts[0].as_str());
                if exec_path.is_file() {
                    return Some(Command::Executable(
                        exec_path,
                        command_parts[1..].iter().map(|s| s.to_string()).collect(),
                    ));
                }
            }
            return Some(Command::InvalidCommand(true_input.trim().to_owned()));
        }
    }
}

fn quote_aware_replace(string: &str, pattern: &str, replacement: &str) -> String {
    let mut output = String::new();
    let mut section = String::new();
    let mut in_single_quote = false;
    for char in string.trim().chars() {
        section.push(char);
        if char == '\'' {
            if in_single_quote {
                output.push_str(&section);
            } else {
                let new_section = section.replace(pattern, replacement);
                output.push_str(&new_section);
            }
            section.clear();
            in_single_quote = !in_single_quote;
        }
    }
    output.push_str(&section.replace(pattern, replacement));
    return output;
}

fn quote_aware_split(string: &str) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let mut current_string = String::new();
    let mut in_single_quote = false;
    for char in string.trim().chars() {
        if char == '\'' {
            in_single_quote = !in_single_quote;
            continue;
        }
        if !in_single_quote && char::is_ascii_whitespace(&char) {
            if current_string.len() > 0 {
                output.push(current_string);
                current_string = String::new();
            }
            continue;
        }
        current_string.push(char);
    }
    if current_string.len() > 0 {
        output.push(current_string);
    }
    return output;
}

pub enum Command {
    Exit,
    Echo(Vec<String>),
    Type(Option<Box<Command>>),
    PWD,
    CD(String),
    Executable(PathBuf, Vec<String>),
    InvalidCommand(String),
}

impl Command {
    pub fn run(&self) {
        match self {
            Command::Exit => exit(0),
            Command::Echo(args) => println!("{}", args.join(" ")),
            Command::Type(subcommand) => {
                if subcommand.is_some() {
                    println!("{}", subcommand.as_ref().unwrap().r#type());
                }
            }
            Command::PWD => println!("{}", env::current_dir().unwrap().display()),
            Command::CD(input) => {
                if input.split_whitespace().count() > 1 {
                    println!("{}: too many arguments", self.name());
                    return;
                }

                let path = PathBuf::from_str(input).unwrap();
                if !path.exists() {
                    println!("{}: {}: No such file or directory", self.name(), input);
                    return;
                }
                if !path.is_dir() {
                    println!("{}: {}: Not a directory", self.name(), input);
                    return;
                }
                env::set_current_dir(path).unwrap();
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
            Command::Echo(..)
            | Command::Exit
            | Command::Type(..)
            | Command::PWD
            | Command::CD(..) => {
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
            Command::PWD => "pwd",
            Command::CD(..) => "cd",
            Command::Executable(path, _) => path.file_name().unwrap().to_str().unwrap(),
            Command::InvalidCommand(..) => "invalid_command",
        };
    }
}
