use std::{
    env,
    path::PathBuf,
    process::{self, exit},
    str::FromStr,
};

pub fn parse_command(input: &str) -> Option<Command> {
    let command_parts = transform_input(input);

    if command_parts.len() < 1 {
        return None;
    }

    match command_parts[0].as_str() {
        "exit" => Some(Command::Exit), // might need the input later to change the exit code
        "echo" => Some(Command::Echo(command_parts[1..].iter().cloned().collect())),
        "type" => Some(Command::Type(
            command_parts[1..]
                .iter()
                .map(|cp| parse_command(&cp).unwrap())
                .collect(),
        )),
        "pwd" => Some(Command::PWD),
        "cd" => Some(Command::CD(command_parts[1..].iter().cloned().collect())),
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
            return Some(Command::InvalidCommand(command_parts[0].clone()));
        }
    }
}

#[derive(PartialEq)]
enum QuoteState {
    None,
    Single,
    Double,
}

fn transform_input(input: &str) -> Vec<String> {
    let home = env::var_os("HOME").unwrap();

    let mut output: Vec<String> = Vec::new();
    let mut current_string = String::new();
    let mut quote_state = QuoteState::None;
    let mut escaped = false;

    for char in input.trim().chars() {
        match quote_state {
            QuoteState::None => {
                if char::is_ascii_whitespace(&char) {
                    if current_string.len() > 0 {
                        output.push(current_string);
                        current_string = String::new();
                    }
                    continue;
                }

                if escaped {
                    current_string.push(char);
                    escaped = false;
                    continue;
                }

                match char {
                    '\'' => quote_state = QuoteState::Single,
                    '"' => quote_state = QuoteState::Double,
                    '~' => current_string.push_str(home.to_str().unwrap()),
                    '\\' => escaped = true,
                    _ => current_string.push(char),
                }
            }
            QuoteState::Single => {
                if char == '\'' {
                    quote_state = QuoteState::None;
                } else {
                    current_string.push(char);
                }
            }
            QuoteState::Double => {
                if escaped {
                    match char {
                        // fallthrough to adding the char
                        '"' | '\\' => (),
                        // need to add the \ because it didn't escape anything
                        _ => current_string.push('\\'),
                    }
                    current_string.push(char);
                    escaped = false;
                    continue;
                }

                match char {
                    '"' => quote_state = QuoteState::None,
                    '\\' => escaped = true,
                    _ => current_string.push(char),
                }
            }
        }
    }
    if current_string.len() > 0 {
        output.push(current_string);
    }
    return output;
}

pub enum Command {
    Exit,
    Echo(Vec<String>),
    Type(Vec<Command>),
    PWD,
    CD(Vec<String>),
    Executable(PathBuf, Vec<String>),
    InvalidCommand(String),
}

impl Command {
    pub fn run(&self) {
        match self {
            Command::Exit => exit(0),
            Command::Echo(args) => println!("{}", args.join(" ")),
            Command::Type(commands) => {
                for command in commands {
                    println!("{}", command.r#type());
                }
            }
            Command::PWD => println!("{}", env::current_dir().unwrap().display()),
            Command::CD(args) => {
                if args.len() > 2 {
                    println!("{}: too many arguments", self.name());
                    return;
                }

                let path_str = args.get(0).map(|cp| cp.clone()).unwrap_or_else(|| {
                    let home = env::var_os("HOME").unwrap();
                    home.into_string().unwrap()
                });

                let path = PathBuf::from_str(&path_str).unwrap();
                if !path.exists() {
                    println!("{}: {}: No such file or directory", self.name(), path_str);
                    return;
                }
                if !path.is_dir() {
                    println!("{}: {}: Not a directory", self.name(), path_str);
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
