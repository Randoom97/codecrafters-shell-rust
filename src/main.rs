#[allow(unused_imports)]
use std::io::{self, Write};

use commands::parse_command;

mod commands;
mod parser;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        // Wait for user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let command = parser::parse_command(&input);
        if command.is_some() {
            command.unwrap().run(&mut None, &mut None);
        }
    }
}
