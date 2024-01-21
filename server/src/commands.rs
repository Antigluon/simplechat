use anyhow::Result;
use lib::command::{Command, CommandRegistry, ConstStr};
use lib::user::User;

pub fn default_commands() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    let send = Command::new("send", "sends a given message", "/send <text>", echo);
    let _ = registry.register(send);
    registry
}

fn echo(_user: &User, args: &str) -> Result<ConstStr> {
    Ok(args.into())
}
