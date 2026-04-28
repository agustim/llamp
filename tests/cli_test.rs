#[cfg(test)]
mod tests {
    use std::process::Command;
    use assert_cmd::prelude::*;

    #[test]
    fn test_cli_help() {
        let mut cmd = Command::cargo_bin("llamp").unwrap();
        cmd.arg("--help");
        cmd.assert().success();
    }
}