use dprint_core::configuration::{
    ConfigKeyMap, ConfigKeyValue, ConfigurationDiagnostic, GlobalConfiguration,
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Configuration {
    pub config_path: Option<String>,
    pub fix: bool,
    pub version: Option<String>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            config_path: None,
            fix: true,
            version: None,
        }
    }
}

pub fn resolve_config(
    config: ConfigKeyMap,
    _global_config: &GlobalConfiguration,
) -> (Configuration, Vec<ConfigurationDiagnostic>) {
    let mut diagnostics = Vec::new();
    let mut resolved = Configuration::default();

    for (key, value) in &config {
        match key.as_str() {
            "configPath" => match value {
                ConfigKeyValue::String(s) => {
                    resolved.config_path = Some(s.clone());
                }
                _ => {
                    diagnostics.push(ConfigurationDiagnostic {
                        property_name: key.clone(),
                        message: "Expected a string value for configPath".to_string(),
                    });
                }
            },
            "fix" => match value {
                ConfigKeyValue::Bool(b) => {
                    resolved.fix = *b;
                }
                _ => {
                    diagnostics.push(ConfigurationDiagnostic {
                        property_name: key.clone(),
                        message: "Expected a boolean value for fix".to_string(),
                    });
                }
            },
            "version" => match value {
                ConfigKeyValue::String(s) => {
                    resolved.version = Some(s.clone());
                }
                _ => {
                    diagnostics.push(ConfigurationDiagnostic {
                        property_name: key.clone(),
                        message: "Expected a string value for version".to_string(),
                    });
                }
            },
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
        assert!(resolved.version.is_none());
    }

    #[test]
    fn resolve_with_config_path() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "configPath".to_string(),
            ConfigKeyValue::String(".golangci.yml".to_string()),
        );
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert_eq!(resolved.config_path, Some(".golangci.yml".to_string()));
    }

    #[test]
    fn resolve_with_fix_disabled() {
        let mut config = ConfigKeyMap::new();
        config.insert("fix".to_string(), ConfigKeyValue::Bool(false));
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert!(!resolved.fix);
    }

    #[test]
    fn resolve_with_version() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "version".to_string(),
            ConfigKeyValue::String("2.5.0".to_string()),
        );
        let global = GlobalConfiguration::default();
        let (resolved, diagnostics) = resolve_config(config, &global);

        assert!(diagnostics.is_empty());
        assert_eq!(resolved.version, Some("2.5.0".to_string()));
    }

    #[test]
    fn resolve_unknown_key_produces_diagnostic() {
        let mut config = ConfigKeyMap::new();
        config.insert(
            "unknown".to_string(),
            ConfigKeyValue::String("value".to_string()),
        );
        let global = GlobalConfiguration::default();
        let (_, diagnostics) = resolve_config(config, &global);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].property_name, "unknown");
    }
}
