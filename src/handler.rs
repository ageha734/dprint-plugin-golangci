use anyhow::{anyhow, Result};
use dprint_core::async_runtime::async_trait;
use dprint_core::async_runtime::LocalBoxFuture;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::{
    AsyncPluginHandler, FileMatchingInfo, FormatRequest, FormatResult, HostFormatRequest,
    PluginInfo, PluginResolveConfigurationResult,
};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::configuration::{resolve_config, Configuration};

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
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        let (resolved, diagnostics) = resolve_config(config, global_config);
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

        format_text(&request.file_text, &request.file_path, &request.config).await
    }
}

pub async fn format_text(
    file_text: &str,
    file_path: &std::path::Path,
    config: &Configuration,
) -> Result<Option<String>> {
    let file_path_str = file_path.to_string_lossy();
    let args = config.to_args(&file_path_str);

    let mut child = Command::new("golangci-lint")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn golangci-lint: {}. Is it installed?", e))?;

    let output = child.wait_with_output().await?;

    if output.status.success() {
        if config.fix {
            let fixed_content = tokio::fs::read_to_string(file_path).await?;
            if fixed_content == file_text {
                return Ok(None);
            }
            // restore original before returning the diff
            tokio::fs::write(file_path, file_text.as_bytes()).await?;
            return Ok(Some(fixed_content));
        }
        return Ok(None);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() && stderr.trim().is_empty() {
        return Ok(None);
    }

    Err(anyhow!(
        "golangci-lint reported issues:\n{}{}",
        stdout,
        if stderr.is_empty() {
            String::new()
        } else {
            format!("\nstderr: {}", stderr)
        }
    ))
}
