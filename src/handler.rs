use anyhow::{Result, anyhow};
use dprint_core::async_runtime::LocalBoxFuture;
use dprint_core::async_runtime::async_trait;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::{
    AsyncPluginHandler, FileMatchingInfo, FormatRequest, FormatResult, HostFormatRequest,
    PluginInfo, PluginResolveConfigurationResult,
};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use crate::configuration::{Configuration, resolve_config};
use crate::golangci;
use crate::installer;

pub struct GolangciHandler;

#[async_trait(?Send)]
impl AsyncPluginHandler for GolangciHandler {
    type Configuration = Configuration;

    fn plugin_info(&self) -> PluginInfo {
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            config_key: "golangci".to_string(),
            help_url: env!("CARGO_PKG_REPOSITORY").to_string(),
            config_schema_url: "".to_string(),
            update_url: None,
        }
    }

    fn license_text(&self) -> String {
        include_str!("../LICENSE").to_string()
    }

    async fn resolve_config(
        &self,
        config: ConfigKeyMap,
        global_config: GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        let (resolved, diagnostics) = resolve_config(config, &global_config);
        PluginResolveConfigurationResult {
            config: resolved,
            diagnostics,
            file_matching: FileMatchingInfo {
                file_extensions: vec!["go".to_string()],
                file_names: vec![],
            },
        }
    }

    async fn format(
        &self,
        request: FormatRequest<Self::Configuration>,
        _format_with_host: impl FnMut(HostFormatRequest) -> LocalBoxFuture<'static, FormatResult>
        + 'static,
    ) -> FormatResult {
        if request.range.is_some() {
            return Ok(None);
        }

        let file_text = String::from_utf8_lossy(&request.file_bytes);
        format_bytes(&file_text, &request.file_path, &request.config).await
    }
}

pub async fn format_bytes(
    file_text: &str,
    file_path: &std::path::Path,
    config: &Configuration,
) -> Result<Option<Vec<u8>>> {
    let binary_path = resolve_binary(config).await?;
    let version = golangci::detect_version(&binary_path).await?;

    let file_name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string_lossy().to_string());
    let args = golangci::build_args(
        version,
        config.fix,
        config.config_path.as_deref(),
        &file_name,
    );

    let work_dir = file_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let child = Command::new(&binary_path)
        .args(&args)
        .current_dir(work_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn golangci-lint: {}", e))?;

    let output = child.wait_with_output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if let Some(issues) = golangci::parse_output(&stdout) {
        let formatted = golangci::format_issues(&issues);
        return Err(anyhow!("{}", formatted));
    }

    if output.status.success() {
        if config.fix {
            let fixed_content = tokio::fs::read_to_string(file_path).await?;
            if fixed_content == file_text {
                return Ok(None);
            }
            tokio::fs::write(file_path, file_text.as_bytes()).await?;
            return Ok(Some(fixed_content.into_bytes()));
        }
        return Ok(None);
    }

    if !stderr.trim().is_empty() {
        return Err(anyhow!("golangci-lint error:\n{}", stderr));
    }

    Ok(None)
}

async fn resolve_binary(config: &Configuration) -> Result<PathBuf> {
    let version = match &config.version {
        Some(v) => v.clone(),
        None => installer::resolve_latest_version().await?,
    };
    installer::ensure_golangci_lint(&version).await
}
