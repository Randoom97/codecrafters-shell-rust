use std::{collections::HashSet, env};

use rustyline::config::Configurer;

mod commands;
mod parser;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut editor = rustyline::Editor::new().unwrap();
    editor.set_helper(Some(Completer::new()));
    editor.set_completion_type(rustyline::CompletionType::List);

    loop {
        let input = editor.readline("$ ").unwrap();

        let command = parser::parse_input(&input);
        if command.is_some() {
            command.unwrap().run();
        }
    }
}

#[derive(rustyline::Helper, rustyline::Highlighter, rustyline::Hinter, rustyline::Validator)]
struct Completer {
    complete_options: HashSet<String>,
}
impl Completer {
    pub fn new() -> Completer {
        let mut complete_options: HashSet<String> = HashSet::new();

        // TODO Tie this more closely with the enum in commands.rs
        let builtins = vec!["echo", "exit", "type", "pwd", "cd"];
        builtins.iter().for_each(|b| {
            complete_options.insert(b.to_string());
        });

        let paths = env::var_os("PATH").unwrap();
        for path in env::split_paths(&paths) {
            if path.is_file() {
                complete_options.insert(path.file_name().unwrap().to_str().unwrap().to_string());
                continue;
            }
            if path.is_dir() {
                let dir = path.read_dir().unwrap();
                for entry_option in dir {
                    let entry = entry_option.unwrap();
                    if entry.file_type().unwrap().is_file() {
                        complete_options.insert(entry.file_name().into_string().unwrap());
                    }
                }
            }
        }
        Completer { complete_options }
    }
}
impl rustyline::completion::Completer for Completer {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<String>), rustyline::error::ReadlineError> {
        let mut options = Vec::new();
        for complete_option in &self.complete_options {
            if complete_option.starts_with(line) {
                options.push(complete_option.clone());
            }
        }

        options.sort_unstable();

        // we want a space when it completes in place for some reason
        if options.len() == 1 {
            options[0] += " ";
        }

        return Ok((0, options));
    }
}
