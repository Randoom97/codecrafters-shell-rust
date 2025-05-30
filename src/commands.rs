use std::{
    env,
    fs::File,
    io::Write,
    path::PathBuf,
    process::{exit, Stdio},
    str::FromStr,
};

use tokio::net::unix::pipe::{Receiver, Sender};

use crate::parser::RedirectType;

#[derive(Debug)]
pub enum Command {
    Exit,
    Echo(Vec<String>),
    Type(Vec<Command>),
    PWD,
    CD(Vec<String>),
    Executable(PathBuf, Vec<String>),
    InvalidCommand(String),
    Pipe(Box<Command>, Box<Command>),
    Redirect(RedirectType, RedirectType, Box<Command>),
}

impl Command {
    pub async fn run(&self) {
        self.run_with_io(IO::Default, IO::Default, IO::Default)
            .await
            .wait()
            .await;
    }

    /** Runs a command with the given io (in, out, err). Returns a run result to be waited on. */
    async fn run_with_io(&self, mut iin: IO, mut out: IO, mut err: IO) -> RunResult {
        match self {
            Command::Exit => exit(0),
            Command::Echo(args) => out.writeln(args.join(" ")).await,
            Command::Type(commands) => {
                for command in commands {
                    out.writeln(command.r#type()).await;
                }
            }
            Command::PWD => {
                out.writeln(env::current_dir().unwrap().display().to_string())
                    .await
            }
            Command::CD(args) => {
                if args.len() > 2 {
                    err.writeln(format!("{}: too many arguments", self.name()))
                        .await;
                    return RunResult::None;
                }

                let path_str = args.get(0).map(|cp| cp.clone()).unwrap_or_else(|| {
                    let home = env::var_os("HOME").unwrap();
                    home.into_string().unwrap()
                });

                let path = PathBuf::from_str(&path_str).unwrap();
                if !path.exists() {
                    err.writeln(format!(
                        "{}: {}: No such file or directory",
                        self.name(),
                        path_str
                    ))
                    .await;
                    return RunResult::None;
                }
                if !path.is_dir() {
                    err.writeln(format!("{}: {}: Not a directory", self.name(), path_str))
                        .await;
                    return RunResult::None;
                }
                env::set_current_dir(path).unwrap();
            }
            Command::Executable(_, args) => {
                let mut pcommand = tokio::process::Command::new(self.name());
                // let mut pcommand = process::Command::new(self.name());
                pcommand
                    .args(args)
                    .stdin(iin.as_stdin())
                    .stdout(out.as_stdio())
                    .stderr(err.as_stdio());
                let child = pcommand.spawn().unwrap();
                return RunResult::Child(child);
            }
            Command::InvalidCommand(input) => {
                err.writeln(format!("{}: command not found", input.trim()))
                    .await;
            }
            Command::Pipe(left_command, right_command) => {
                let (sender, receiver) = tokio::net::unix::pipe::pipe().unwrap();
                let out_pipe = IO::Pipe(Some(sender), None);
                let in_pipe = IO::Pipe(None, Some(receiver));
                let mut left_child =
                    Box::pin(left_command.run_with_io(iin, out_pipe, err.clone())).await;
                let mut right_child = Box::pin(right_command.run_with_io(in_pipe, out, err)).await;

                // important to spawn the children before awaiting to avoid blocking the data passing through the pipe
                left_child.wait().await;
                right_child.wait().await;
            }
            Command::Redirect(out_path, err_path, command) => {
                let out = out_path.as_io();
                let err = err_path.as_io();
                Box::pin(command.run_with_io(iin, out, err))
                    .await
                    .wait()
                    .await;
            }
        }
        return RunResult::None;
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

enum RunResult {
    None,
    Child(tokio::process::Child),
}

impl RunResult {
    pub async fn wait(&mut self) {
        match self {
            RunResult::None => {}
            RunResult::Child(child) => {
                child.wait().await.unwrap();
            }
        }
    }
}

pub enum IO {
    Default,
    File(File),
    Pipe(Option<Sender>, Option<Receiver>),
}

impl IO {
    pub async fn writeln(&mut self, data: String) {
        self.write(data + "\n").await;
    }

    pub async fn write(&mut self, data: String) {
        match self {
            IO::Default => print!("{}", data),
            IO::File(file) => write!(file, "{}", data).unwrap(),
            IO::Pipe(sender, _) => {
                sender.as_ref().unwrap().writable().await.unwrap();
                sender.as_ref().unwrap().try_write(data.as_bytes()).unwrap();
            }
        }
    }

    pub fn as_stdin(&mut self) -> Stdio {
        match self {
            IO::Default => Stdio::inherit(),
            IO::File(file) => file.try_clone().unwrap().into(),
            IO::Pipe(_, receiver) => receiver.take().unwrap().into_blocking_fd().unwrap().into(),
        }
    }

    /** Used to create a output handle (out, err) */
    pub fn as_stdio(&mut self) -> Stdio {
        match self {
            IO::Default => Stdio::inherit(),
            IO::File(file) => file.try_clone().unwrap().into(),
            // take is really awkward, but the resulting Stdio has to be owned, and into_blocking_fd() can't be used on a reference
            IO::Pipe(sender, _) => sender.take().unwrap().into_blocking_fd().unwrap().into(),
        }
    }
}

impl Clone for IO {
    fn clone(&self) -> Self {
        match self {
            Self::Default => Self::Default,
            Self::File(file) => Self::File(file.try_clone().unwrap()),
            Self::Pipe(..) => panic!("Can't clone a pipe"),
        }
    }
}
