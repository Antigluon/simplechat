use anyhow::{anyhow, Result};
use std::collections::HashSet;

pub enum User {
    Registered(RegisteredUser),
    Guest(GuestUser),
}

impl User {
    pub fn username(&self) -> &str {
        match self {
            User::Registered(user) => &user.username,
            User::Guest(user) => &user.username,
        }
    }
}

pub struct GuestUser {
    username: String,
}

impl GuestUser {
    pub fn new(user_registry: &HashSet<String>, username: &str) -> Result<GuestUser> {
        let username = format_username(username);
        let username = validate_username(&user_registry, username)?;
        Ok(GuestUser { username })
    }
}

pub struct RegisteredUser {
    username: String,
}

/// Formats a username.
/// - Removes leading and trailing whitespace.
/// - Converts to lowercase.
/// ```
/// # use lib::user::format_username;
/// assert_eq!(format_username("  Hello World  "), "hello world")
/// ```
pub fn format_username(username: &str) -> String {
    username.trim().to_lowercase().to_string()
}

/// Checks if the (formatted) username is valid.
/// - Must be [3, 30] characters long.
/// - Must not be the name of an existing user.
/// ```
/// # use lib::user::validate_username;
/// # use std::collections::HashSet;
/// let mut existing_names = HashSet::new();
/// existing_names.insert("alice".to_string());
/// validate_username(&existing_names, "bob".to_string()).unwrap();
/// validate_username(&existing_names, "bob".to_string()).unwrap_err();
/// validate_username(&existing_names, "alice".to_string()).unwrap_err();
/// validate_username(&existing_names, "a".to_string()).unwrap_err();
/// validate_username(&existing_names, "fartoolong".repeat(4)).unwrap_err();
/// ```
pub fn validate_username(all_users: &HashSet<String>, username: String) -> Result<String> {
    let name_length = username.len();
    if !(3..=30).contains(&name_length) {
        return Err(anyhow!(
            "Username must be between 3 and 30 characters long. 
        {username} is {name_length} characters long."
        ));
    }
    if all_users.contains(&username) {
        return Err(anyhow!("Username {username} is already taken."));
    }
    Ok(username)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn name_processing() {
        let names = HashSet::new();
        let username = "  Hello World  ";
        let formatted_username = format_username(username);
        let result = validate_username(&names, formatted_username)
            .expect("Failed to validate valid username.");
        assert_eq!(result, "hello world");
    }
}
