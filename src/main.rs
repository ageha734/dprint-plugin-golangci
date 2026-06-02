use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::process::Command;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "getPluginInfo")]
    GetPluginInfo,
    #[serde(rename = "getLicenseText")]
    GetLicenseText,
    #[serde(rename = "getResolvedConfig")]
    GetResolvedConfig,
    #[serde(rename = "getConfigDiagnostics")]
    GetConfigDiagnostics,
    #[serde(rename = "formatText")]
    FormatText(FormatTextRequest),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatTextRequest {
    file_path: String,
    file_text: String,
    #[serde(default)]
    _override_config: serde_json::Value,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Response {
    #[serde(rename = "pluginInfo")]
    PluginInfo(PluginInfo),
    #[serde(rename = "licenseText")]
    LicenseText { text: String },
    #[serde(rename = "resolvedConfig")]
    ResolvedConfig { config: serde_json::Value },
    #[serde(rename = "configDiagnostics")]
    ConfigDiagnostics {
        diagnostics: Vec<serde_json::Value>,
    },
    #[serde(rename = "formatted")]
    Formatted { text: String },
    #[serde(rename = "noChange")]
    NoChange,
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginInfo {
    name: String,
    version: String,
    config_key: String,
    help_url: String,
    file_extensions: Vec<String>,
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                send_response(
                    &stdout,
                    &Response::Error {
                        message: format!("Failed to parse request: {}", e),
                    },
                );
                continue;
            }
        };

        let response = handle_request(request);
        send_response(&stdout, &response);
    }
}

fn handle_request(request: Request) -> Response {
    match request {
        Request::GetPluginInfo => Response::PluginInfo(PluginInfo {
            name: "dprint-plugin-golangci".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_key: "golangci".to_string(),
            help_url:
                "https://github.com/ageha734/dprint-plugin-golangci".to_string(),
            file_extensions: vec!["go".to_string()],
        }),
        Request::GetLicenseText => Response::LicenseText {
            text: include_str!("../LICENSE").to_string(),
        },
        Request::GetResolvedConfig => Response::ResolvedConfig {
            config: serde_json::json!({
                "linters": [
                    "errcheck",
                    "govet",
                    "staticcheck",
                    "bodyclose",
                    "forcetypeassert",
                    "gosec",
                    "misspell",
                    "testpackage",
                    "unconvert",
                    "unparam",
                    "unused"
                ]
            }),
        },
        Request::GetConfigDiagnostics => Response::ConfigDiagnostics {
            diagnostics: vec![],
        },
        Request::FormatText(req) => run_lint(&req),
    }
}

fn run_lint(req: &FormatTextRequest) -> Response {
    if !req.file_path.ends_with(".go") {
        return Response::NoChange;
    }

    let output = Command::new("golangci-lint")
        .args(["run", "--fix", "--out-format=json", &req.file_path])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                match std::fs::read_to_string(&req.file_path) {
                    Ok(fixed_content) => {
                        if fixed_content == req.file_text {
                            Response::NoChange
                        } else {
                            Response::Formatted { text: fixed_content }
                        }
                    }
                    Err(_) => Response::NoChange,
                }
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if stderr.contains("issues") {
                    Response::Error {
                        message: format!(
                            "golangci-lint found issues in {}:\n{}",
                            req.file_path,
                            String::from_utf8_lossy(&result.stdout)
                        ),
                    }
                } else {
                    Response::NoChange
                }
            }
        }
        Err(e) => Response::Error {
            message: format!("Failed to run golangci-lint: {}", e),
        },
    }
}

fn send_response(stdout: &io::Stdout, response: &Response) {
    let json = serde_json::to_string(response).unwrap();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", json).unwrap();
    handle.flush().unwrap();
}
