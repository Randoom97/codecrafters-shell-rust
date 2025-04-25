#[allow(unused_imports)]
use std::io::{self, Write};
use std::process::exit;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        // Wait for user input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let command_parts: Vec<&str> = input.trim().split_whitespace().collect();

        if command_parts.len() < 1 {
            continue;
        }

        match command_parts[0] {
            "exit" => exit(0),
            _ => println!("{}: command not found", input.trim()),
        }
    }
}
