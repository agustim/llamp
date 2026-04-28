#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*;
    use std::process::Command;

    #[test]
    fn test_cli_help() {
        let mut cmd = Command::cargo_bin("llamp").unwrap();
        cmd.arg("--help");
        cmd.assert().success();
    }
}
