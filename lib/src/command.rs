#![allow(unused_imports)] //TODO: remove

use derive_more::Display;
use std::{
    collections::HashMap,
    fmt::Display,
    fmt::{self, Debug, Formatter},
    str::FromStr,
    sync::Arc,
};

use crate::user::User;
use anyhow::{anyhow, Result};

pub type ConstStr = Arc<str>;

#[derive(Debug, Clone)]
pub struct Command {
    pub name: ConstStr,
    pub description: ConstStr,
    pub usage: ConstStr,
    operation: fn(&User, &str) -> Result<ConstStr>,
}

impl Command {
    pub fn new(
        name: &str,
        description: &str,
        usage: &str,
        operation: fn(&User, &str) -> Result<ConstStr>,
    ) -> Command {
        return Command {
            name: name.into(),
            description: description.into(),
            usage: usage.into(),
            operation,
        };
    }

    pub fn execute(&self, caller: &User, args: &str) -> Result<ConstStr> {
        return (self.operation)(caller, args);
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

pub struct CommandRegistry {
    commands: HashMap<ConstStr, Command>,
}

impl CommandRegistry {
    pub fn new() -> CommandRegistry {
        return CommandRegistry {
            commands: HashMap::new(),
        };
    }
    pub fn register(&mut self, command: Command) -> Result<()> {
        let name = command.name.clone();
        match self.commands.contains_key(&name) {
            true => Err(anyhow!("Command {} already registered", name)),
            false => {
                self.commands.insert(name, command);
                Ok(())
            }
        }
    }
    pub fn remove(&mut self, command_name: &str) {
        self.commands.remove(command_name);
    }
    pub fn get(&self, command_name: &str) -> Option<&Command> {
        return self.commands.get(command_name);
    }
}

// pub struct Echo {
//     payload: String,
// }

// impl Echo {
//     pub fn new(payload: String) -> Self {
//         Self { payload }
//     }
// }
//
// impl FromStr for Echo {
//     type Err = anyhow::Error;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         Ok(Self::new(s.to_string()))
//     }
// }

// impl Command for Echo {
//     const NAME: &'static str = "echo";
//     const DESCRIPTION: &'static str = "Echoes the given text.";
//     const USAGE: &'static str = "/echo <text>";
//
//     /// Echoes the given text, specified by the rest of the command.
//     /// ```rust
//     /// # use lib::commands::{Command, Echo};
//     /// # use std::str::FromStr;
//     /// # use anyhow::Result;
//     /// # fn main() -> Result<()> {
//     /// println!("{}", Echo::describe());
//     /// assert_ne!(Echo::describe(), "");
//     /// let output = Echo::from_str("hello")?.execute()?;
//     /// assert_eq!(output, "hello");
//     /// # Ok(())
//     /// # }
//     /// ```
//     fn execute(self) -> Result<String> {
//         Ok(self.payload)
//     }
// }
