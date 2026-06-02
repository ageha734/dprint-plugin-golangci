use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
use dprint_plugin_golangci::configuration::resolve_config;
use dprint_plugin_golangci::golangci::{self, Version};

#[test]
fn configuration_resolve_defaults() {
    let config = ConfigKeyMap::new();
    let global = GlobalConfiguration::default();
    let (resolved, diagnostics) = resolve_config(config, &global);

    assert!(diagnostics.is_empty());
    assert!(resolved.fix);
    assert!(resolved.config_path.is_none());
    assert!(resolved.version.is_none());
}

#[test]
fn configuration_resolve_with_version() {
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
fn configuration_resolve_with_config_path() {
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
fn build_args_v1() {
    let args = golangci::build_args(Version::V1, true, Some(".golangci.yml"), "main.go");
    assert_eq!(
        args,
        vec![
            "run",
            "--fix",
            "--config=.golangci.yml",
            "--out-format=json",
            "main.go"
        ]
    );
}

#[test]
fn build_args_v2() {
    let args = golangci::build_args(Version::V2, true, Some(".golangci.yml"), "main.go");
    assert_eq!(
        args,
        vec![
            "run",
            "--fix",
            "--config=.golangci.yml",
            "--output.json.path",
            "stdout",
            "main.go"
        ]
    );
}

#[test]
fn build_args_no_fix() {
    let args = golangci::build_args(Version::V2, false, None, "src/lib.go");
    assert_eq!(args, vec!["run", "--output.json.path", "stdout", "src/lib.go"]);
}

#[test]
fn parse_and_format_issues() {
    let json = r#"{"Issues":[{"FromLinter":"unused","Text":"func `foo` is unused","Pos":{"Filename":"main.go","Offset":0,"Line":5,"Column":6}}]}"#;
    let issues = golangci::parse_output(json).unwrap();
    let output = golangci::format_issues(&issues);
    assert_eq!(output, "main.go:5:6: [unused] func `foo` is unused");
}

#[test]
fn parse_output_no_issues() {
    let json = r#"{"Issues":[]}"#;
    assert!(golangci::parse_output(json).is_none());
}
