use std::{
    env,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    process::{self, exit},
    str::FromStr,
};

pub fn parse_command(input: &str) -> Option<Command> {
    let mut command_parts = transform_input(input);

    if command_parts.len() < 1 {
        return None;
    }

    let out_redirect_index = command_parts
        .iter()
        .position(|cp| *cp == ">" || *cp == "1>");
    let err_redirect_index = command_parts.iter().position(|cp| *cp == "2>");

    let mut out_path = None;
    let mut err_path = None;

    if out_redirect_index.is_some() {
        command_parts.remove(out_redirect_index.unwrap());
        out_path = Some(command_parts.remove(out_redirect_index.unwrap()));
    }
    if err_redirect_index.is_some() {
        command_parts.remove(err_redirect_index.unwrap());
        err_path = Some(command_parts.remove(err_redirect_index.unwrap()));
    }

    let command: Command = match command_parts[0].as_str() {
        "exit" => Command::Exit, // might need the input later to change the exit code
        "echo" => Command::Echo(command_parts[1..].iter().cloned().collect()),
        "type" => Command::Type(
            command_parts[1..]
                .iter()
                .map(|cp| parse_command(&cp).unwrap())
                .collect(),
        ),
        "pwd" => Command::PWD,
        "cd" => Command::CD(command_parts[1..].iter().cloned().collect()),
        _ => {
            let paths = env::var_os("PATH").unwrap();
            let mut found_command = None;
            for path in env::split_paths(&paths) {
                let exec_path = path.join(command_parts[0].as_str());
                if exec_path.is_file() {
                    found_command = Some(Command::Executable(
                        exec_path,
                        command_parts[1..].iter().map(|s| s.to_string()).collect(),
                    ));
                    break;
                }
            }
            found_command.unwrap_or_else(|| Command::InvalidCommand(command_parts[0].clone()))
        }
    };

    if out_path.is_some() || err_path.is_some() {
        return Some(Command::Redirect(out_path, err_path, Box::new(command)));
    }

    return Some(command);
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
                if escaped {
                    current_string.push(char);
                    escaped = false;
                    continue;
                }

                if char::is_ascii_whitespace(&char) {
                    if current_string.len() > 0 {
                        output.push(current_string);
                        current_string = String::new();
                    }
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
    Redirect(Option<String>, Option<String>, Box<Command>),
}

impl Command {
    pub fn run(&self, out_file: &mut Option<File>, err_file: &mut Option<File>) {
        let out: &mut dyn Write = if out_file.is_some() {
            out_file.as_mut().unwrap()
        } else {
            &mut io::stdout()
        };
        let err: &mut dyn Write = if err_file.is_some() {
            err_file.as_mut().unwrap()
        } else {
            &mut io::stderr()
        };

        match self {
            Command::Exit => exit(0),
            Command::Echo(args) => writeln!(out, "{}", args.join(" ")).unwrap(),
            Command::Type(commands) => {
                for command in commands {
                    writeln!(out, "{}", command.r#type()).unwrap();
                }
            }
            Command::PWD => writeln!(out, "{}", env::current_dir().unwrap().display()).unwrap(),
            Command::CD(args) => {
                if args.len() > 2 {
                    writeln!(err, "{}: too many arguments", self.name()).unwrap();
                    return;
                }

                let path_str = args.get(0).map(|cp| cp.clone()).unwrap_or_else(|| {
                    let home = env::var_os("HOME").unwrap();
                    home.into_string().unwrap()
                });

                let path = PathBuf::from_str(&path_str).unwrap();
                if !path.exists() {
                    writeln!(
                        err,
                        "{}: {}: No such file or directory",
                        self.name(),
                        path_str
                    )
                    .unwrap();
                    return;
                }
                if !path.is_dir() {
                    writeln!(err, "{}: {}: Not a directory", self.name(), path_str).unwrap();
                    return;
                }
                env::set_current_dir(path).unwrap();
            }
            Command::Executable(_, args) => {
                let mut command = process::Command::new(self.name());
                command.args(args);
                if out_file.is_some() {
                    command.stdout(out_file.as_ref().unwrap().try_clone().unwrap());
                }
                if err_file.is_some() {
                    command.stderr(err_file.as_ref().unwrap().try_clone().unwrap());
                }
                command.spawn().unwrap().wait().unwrap();
            }
            Command::InvalidCommand(input) => {
                writeln!(err, "{}: command not found", input.trim()).unwrap()
            }
            Command::Redirect(out_path, err_path, command) => {
                let mut out_file = out_path.as_ref().map(|op| File::create(op).unwrap());
                let mut err_file = err_path.as_ref().map(|ep| File::create(ep).unwrap());
                command.run(&mut out_file, &mut err_file);
            }
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
            _ => panic!("Invalid command for type!"),
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
            _ => panic!("Invalid command for name!"),
        };
    }
}
