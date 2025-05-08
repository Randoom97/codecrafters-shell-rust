use std::{
    env,
    fs::{File, OpenOptions},
};

use crate::commands::Command;

pub fn parse_command(input: &str) -> Option<Command> {
    let mut command_parts = transform_input(input);

    if command_parts.len() < 1 {
        return None;
    }

    let mut out_path = RedirectType::None;
    let mut err_path = RedirectType::None;
    let mut keep = Vec::new();
    let mut keep_next = true;
    for (i, command_part) in command_parts.iter().enumerate() {
        let next = command_parts.get(i + 1);
        match command_part.as_str() {
            ">" | "1>" => out_path = RedirectType::Truncate(next.unwrap().clone()),
            ">>" | "1>>" => out_path = RedirectType::Append(next.unwrap().clone()),
            "2>" => err_path = RedirectType::Truncate(next.unwrap().clone()),
            "2>>" => err_path = RedirectType::Append(next.unwrap().clone()),
            _ => {
                keep.push(keep_next);
                keep_next = true;
                continue;
            }
        }
        keep.push(false);
        keep_next = false;
    }
    let mut keep_iter = keep.iter();
    command_parts.retain(|_| *keep_iter.next().unwrap());

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

pub enum RedirectType {
    None,
    Truncate(String),
    Append(String),
}

impl RedirectType {
    pub fn is_some(&self) -> bool {
        match self {
            RedirectType::None => false,
            _ => true,
        }
    }

    pub fn as_file(&self) -> Option<File> {
        match self {
            RedirectType::None => None,
            RedirectType::Truncate(path) => Some(File::create(path).unwrap()),
            RedirectType::Append(path) => Some(
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .unwrap(),
            ),
        }
    }
}

#[derive(PartialEq)]
enum QuoteState {
    None,
    Single,
    Double,
}

pub fn transform_input(input: &str) -> Vec<String> {
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
