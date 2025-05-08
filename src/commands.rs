use std::{
    env,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    process::{self, exit},
    str::FromStr,
};

use crate::parser::RedirectType;

pub enum Command {
    Exit,
    Echo(Vec<String>),
    Type(Vec<Command>),
    PWD,
    CD(Vec<String>),
    Executable(PathBuf, Vec<String>),
    InvalidCommand(String),
    Redirect(RedirectType, RedirectType, Box<Command>),
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
                let mut out_file = out_path.as_file();
                let mut err_file = err_path.as_file();
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
