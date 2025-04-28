use std::process::exit;

pub fn parse_command(input: &str) -> Option<Command> {
    let command_parts: Vec<&str> = input.trim().split_whitespace().collect();

    if command_parts.len() < 1 {
        return None;
    }

    return Some(match command_parts[0] {
        "exit" => Command::Exit, // might need the input later to change the exit code
        "echo" => Command::Echo(input.trim()[4..].trim().to_owned()),
        "type" => Command::Type(parse_command(input.trim()[4..].trim()).map(|sc| Box::new(sc))),
        _ => Command::InvalidCommand(input.trim().to_owned()),
    });
}

pub enum Command {
    Exit,
    Echo(String),
    Type(Option<Box<Command>>),
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
            Command::InvalidCommand(input) => println!("{}: command not found", input.trim()),
        }
    }

    fn r#type(&self) -> String {
        return match self {
            Command::Echo(..) | Command::Exit | Command::Type(..) => {
                format!("{} is a shell builtin", self.name())
            }
            Command::InvalidCommand(input) => format!("{}: not found", input.trim()),
        };
    }

    fn name(&self) -> &str {
        return match self {
            Command::Exit => "exit",
            Command::Echo(..) => "echo",
            Command::Type(..) => "type",
            Command::InvalidCommand(..) => "invalid_command",
        };
    }
}
