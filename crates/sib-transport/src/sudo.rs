use sib_core::transport::shell_quote;

pub fn wrap_passwordless(command: &str) -> String {
    format!("sudo -n sh -c {}", shell_quote(command))
}

pub fn wrap_with_password(command: &str) -> String {
    format!("sudo -S -p '' sh -c {}", shell_quote(command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_passwordless_quotes_command() {
        assert_eq!(
            wrap_passwordless("cat /etc/shadow"),
            "sudo -n sh -c 'cat /etc/shadow'"
        );
    }

    #[test]
    fn wrap_with_password_uses_stdin_flag() {
        assert!(wrap_with_password("id").starts_with("sudo -S -p '' sh -c"));
    }
}
