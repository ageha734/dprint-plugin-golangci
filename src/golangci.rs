use anyhow::Result;
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Deserialize)]
pub struct LintOutput {
    #[serde(rename = "Issues")]
    pub issues: Option<Vec<LintIssue>>,
}

#[derive(Deserialize)]
pub struct LintIssue {
    #[serde(rename = "FromLinter")]
    pub from_linter: String,
    #[serde(rename = "Text")]
    pub text: String,
    #[serde(rename = "Pos")]
    pub pos: LintPosition,
}

#[derive(Deserialize)]
pub struct LintPosition {
    #[serde(rename = "Filename")]
    pub filename: String,
    #[serde(rename = "Line")]
    pub line: u32,
    #[serde(rename = "Column")]
    pub column: u32,
}

pub fn format_issues(issues: &[LintIssue]) -> String {
    issues
        .iter()
        .map(|issue| {
            format!(
                "{}:{}:{}: [{}] {}",
                issue.pos.filename, issue.pos.line, issue.pos.column, issue.from_linter, issue.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn detect_version(binary_path: &Path) -> Result<Version> {
    let output = Command::new(binary_path)
        .arg("version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("version v2") || stdout.contains("version 2.") {
        Ok(Version::V2)
    } else {
        Ok(Version::V1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Version {
    V1,
    V2,
}

pub fn build_args(
    version: Version,
    fix: bool,
    config_path: Option<&str>,
    file_path: &str,
) -> Vec<String> {
    let mut args = vec!["run".to_string()];

    if fix {
        args.push("--fix".to_string());
    }

    if let Some(path) = config_path {
        args.push(format!("--config={}", path));
    }

    match version {
        Version::V1 => args.push("--out-format=json".to_string()),
        Version::V2 => {
            args.push("--output.json.path".to_string());
            args.push("stdout".to_string());
        }
    }

    args.push(file_path.to_string());
    args
}

pub fn parse_output(stdout: &str) -> Option<Vec<LintIssue>> {
    let output: LintOutput = serde_json::from_str(stdout).ok()?;
    output.issues.filter(|issues| !issues.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_v1_default() {
        let args = build_args(Version::V1, true, None, "main.go");
        assert_eq!(args, vec!["run", "--fix", "--out-format=json", "main.go"]);
    }

    #[test]
    fn build_args_v2_default() {
        let args = build_args(Version::V2, true, None, "main.go");
        assert_eq!(
            args,
            vec!["run", "--fix", "--output.json.path", "stdout", "main.go"]
        );
    }

    #[test]
    fn build_args_with_config() {
        let args = build_args(Version::V2, false, Some(".golangci.yml"), "pkg/foo.go");
        assert_eq!(
            args,
            vec![
                "run",
                "--config=.golangci.yml",
                "--output.json.path",
                "stdout",
                "pkg/foo.go"
            ]
        );
    }

    #[test]
    fn parse_output_with_issues() {
        let json = r#"{"Issues":[{"FromLinter":"unused","Text":"func `foo` is unused","Pos":{"Filename":"main.go","Offset":0,"Line":5,"Column":6}}]}"#;
        let issues = parse_output(json).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].from_linter, "unused");
        assert_eq!(issues[0].pos.line, 5);
    }

    #[test]
    fn parse_output_no_issues() {
        let json = r#"{"Issues":[]}"#;
        let issues = parse_output(json);
        assert!(issues.is_none());
    }

    #[test]
    fn parse_output_invalid_json() {
        let issues = parse_output("not json");
        assert!(issues.is_none());
    }

    #[test]
    fn format_issues_display() {
        let issues = vec![
            LintIssue {
                from_linter: "unused".to_string(),
                text: "func `foo` is unused".to_string(),
                pos: LintPosition {
                    filename: "main.go".to_string(),
                    line: 5,
                    column: 6,
                },
            },
            LintIssue {
                from_linter: "errcheck".to_string(),
                text: "error return value not checked".to_string(),
                pos: LintPosition {
                    filename: "main.go".to_string(),
                    line: 10,
                    column: 2,
                },
            },
        ];
        let output = format_issues(&issues);
        assert_eq!(
            output,
            "main.go:5:6: [unused] func `foo` is unused\nmain.go:10:2: [errcheck] error return value not checked"
        );
    }
}
