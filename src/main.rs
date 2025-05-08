#[allow(unused_imports)]
use std::io::{self, Write};

mod commands;
mod parser;

fn main() {
    let mut editor = rustyline::Editor::new().unwrap();
    editor.set_helper(Some(Completer));

    loop {
        let input = editor.readline("$ ").unwrap();

        let command = parser::parse_command(&input);
        if command.is_some() {
            command.unwrap().run(&mut None, &mut None);
        }
    }
}

#[derive(rustyline::Helper, rustyline::Highlighter, rustyline::Hinter, rustyline::Validator)]
struct Completer;
impl rustyline::completion::Completer for Completer {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<String>), rustyline::error::ReadlineError> {
        // TODO Tie this more closely with the enum in commands.rs
        let builtins = vec!["echo", "exit", "type", "pwd", "cd"];

        let mut options = Vec::new();
        for builtin in builtins {
            if builtin.starts_with(line) {
                options.push(builtin.to_string() + " ");
            }
        }

        return Ok((0, options));
    }
}
