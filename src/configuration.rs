use dprint_core::configuration::{ConfigKeyMap, ConfigurationDiagnostic, GlobalConfiguration};
use serde::Serialize;

#[derive(Clone, Serialize, Default)]
pub struct Configuration {
    pub config_path: Option<String>,
    pub fix: bool,
}

pub fn resolve_config(
    config: ConfigKeyMap,
    _global_config: &GlobalConfiguration,
) -> (Configuration, Vec<ConfigurationDiagnostic>) {
    let mut diagnostics = Vec::new();
    let mut resolved = Configuration {
        config_path: None,
        fix: true,
    };

    for (key, value) in &config {
        match key.as_str() {
            "configPath" => {
                if let serde_json::Value::String(s) = value {
                    resolved.config_path = Some(s.clone());
                } else {
                    diagnostics.push(ConfigurationDiagnostic {
                        property_name: key.clone(),
                        message: "Expected a string value for configPath".to_string(),
                    });
                }
            }
            "fix" => {
                if let serde_json::Value::Bool(b) = value {
                    resolved.fix = *b;
                } else {
                    diagnostics.push(ConfigurationDiagnostic {
                        property_name: key.clone(),
                        message: "Expected a boolean value for fix".to_string(),
                    });
                }
            }
            _ => {
                diagnostics.push(ConfigurationDiagnostic {
                    property_name: key.clone(),
                    message: format!("Unknown configuration key: {}", key),
                });
            }
        }
    }

    (resolved, diagnostics)
}

impl Configuration {
    pub fn to_args(&self, file_path: &str) -> Vec<String> {
        let mut args = vec!["run".to_string()];

        if self.fix {
            args.push("--fix".to_string());
        }

        if let Some(ref path) = self.config_path {
            args.push(format!("--config={}", path));
        }

        args.push("--out-format=json".to_string());
        args.push(file_path.to_string());

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_default_config() {
        let config = ConfigKeyMap::new();
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert!(resolved.fix);
        assert!(resolved.config_path.is_none());
    }

    #[test]
    fn resolve_with_config_path() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "configPath".to_string(),
            serde_json::Value::String(".golangci.yml".to_string()),
        );
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert_eq!(resolved.config_path, Some(".golangci.yml".to_string()));
    }

    #[test]
    fn resolve_with_fix_disabled() {
        let mut config = ConfigKeyMap::new();
        config.insert("fix".to_string(), serde_json::Value::Bool(false));
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert!(!resolved.fix);
    }

    #[test]
    fn resolve_unknown_key_produces_diagnostic() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "unknown".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        let global = GlobalConfiguration::default();
        let (_, diagnostics) = resolve_config(config, &global);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].property_name, "unknown");
    }

    #[test]
    fn to_args_default() {
        let config = Configuration {
            config_path: None,
            fix: true,
        };
        let args = config.to_args("main.go");

        assert_eq!(args, vec!["run", "--fix", "--out-format=json", "main.go"]);
    }

    #[test]
    fn to_args_with_config_path() {
        let config = Configuration {
            config_path: Some("/path/.golangci.yml".to_string()),
            fix: true,
        };
        let args = config.to_args("main.go");

        assert_eq!(
            args,
            vec![
                "run",
                "--fix",
                "--config=/path/.golangci.yml",
                "--out-format=json",
                "main.go"
            ]
        );
    }

    #[test]
    fn to_args_no_fix() {
        let config = Configuration {
            config_path: None,
            fix: false,
        };
        let args = config.to_args("main.go");

        assert_eq!(args, vec!["run", "--out-format=json", "main.go"]);
    }
}
