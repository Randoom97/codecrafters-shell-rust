use std::env;

mod commands;
mod parser;

fn main() {
    let mut editor = rustyline::Editor::new().unwrap();
    editor.set_helper(Some(Completer::new()));

    loop {
        let input = editor.readline("$ ").unwrap();

        let command = parser::parse_command(&input);
        if command.is_some() {
            command.unwrap().run(&mut None, &mut None);
        }
    }
}

#[derive(rustyline::Helper, rustyline::Highlighter, rustyline::Hinter, rustyline::Validator)]
struct Completer {
    complete_options: Vec<String>,
}
impl Completer {
    pub fn new() -> Completer {
        let mut complete_options: Vec<String> = Vec::new();

        // TODO Tie this more closely with the enum in commands.rs
        let builtins = vec!["echo", "exit", "type", "pwd", "cd"];
        complete_options.append(&mut builtins.iter().map(|s| s.to_string()).collect());

        let paths = env::var_os("PATH").unwrap();
        for path in env::split_paths(&paths) {
            if path.is_file() {
                complete_options.push(path.file_name().unwrap().to_str().unwrap().to_string());
                continue;
            }
            if path.is_dir() {
                let dir = path.read_dir().unwrap();
                for entry_option in dir {
                    let entry = entry_option.unwrap();
                    if entry.metadata().unwrap().is_file() {
                        complete_options.push(entry.file_name().into_string().unwrap());
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
                options.push(complete_option.clone() + " ");
            }
        }

        return Ok((0, options));
    }
}
