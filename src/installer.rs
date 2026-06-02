use anyhow::{anyhow, Result};
use std::path::PathBuf;

const GITHUB_REPO: &str = "golangci/golangci-lint";

pub async fn ensure_golangci_lint(version: &str) -> Result<PathBuf> {
    if let Some(path) = find_in_path() {
        return Ok(path);
    }

    let cache_dir = cache_directory()?;
    let binary_path = cached_binary_path(&cache_dir, version);

    if binary_path.exists() {
        return Ok(binary_path);
    }

    download_and_install(version, &cache_dir).await?;

    if binary_path.exists() {
        Ok(binary_path)
    } else {
        Err(anyhow!(
            "Failed to install golangci-lint v{}. Binary not found after download.",
            version
        ))
    }
}

pub async fn resolve_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "dprint-plugin-golangci")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch latest golangci-lint version: HTTP {}",
            resp.status()
        ));
    }

    let body: serde_json::Value = resp.json().await?;
    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow!("No tag_name in release response"))?;

    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn find_in_path() -> Option<PathBuf> {
    which("golangci-lint")
}

fn which(binary: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(binary);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

fn cache_directory() -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .ok_or_else(|| anyhow!("Could not determine cache directory"))?;
    let dir = base.join("dprint-plugin-golangci");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn cached_binary_path(cache_dir: &std::path::Path, version: &str) -> PathBuf {
    let binary_name = if cfg!(target_os = "windows") {
        "golangci-lint.exe"
    } else {
        "golangci-lint"
    };
    cache_dir.join(format!("v{}", version)).join(binary_name)
}

async fn download_and_install(version: &str, cache_dir: &std::path::Path) -> Result<()> {
    let (os, arch) = platform_info();
    let archive_name = format!("golangci-lint-{}-{}-{}.tar.gz", version, os, arch);
    let url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        GITHUB_REPO, version, archive_name
    );

    eprintln!("Downloading golangci-lint v{} from {}...", version, url);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "dprint-plugin-golangci")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to download golangci-lint v{}: HTTP {}",
            version,
            resp.status()
        ));
    }

    let bytes = resp.bytes().await?;
    let version_dir = cache_dir.join(format!("v{}", version));
    std::fs::create_dir_all(&version_dir)?;

    let decoder = flate2::read::GzDecoder::new(bytes.as_ref());
    let mut archive = tar::Archive::new(decoder);

    let binary_name = if cfg!(target_os = "windows") {
        "golangci-lint.exe"
    } else {
        "golangci-lint"
    };

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.file_name().and_then(|f| f.to_str()) == Some(binary_name) {
            let dest = version_dir.join(binary_name);
            entry.unpack(&dest)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
            }
            eprintln!("Installed golangci-lint v{} to {:?}", version, dest);
            return Ok(());
        }
    }

    Err(anyhow!(
        "golangci-lint binary not found in archive {}",
        archive_name
    ))
}

fn platform_info() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    };

    (os, arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_info() {
        let (os, arch) = platform_info();
        assert!(["linux", "darwin", "windows"].contains(&os));
        assert!(["amd64", "arm64"].contains(&arch));
    }

    #[test]
    fn test_cache_directory() {
        let dir = cache_directory().unwrap();
        assert!(dir.to_string_lossy().contains("dprint-plugin-golangci"));
    }

    #[test]
    fn test_cached_binary_path() {
        let cache_dir = std::path::Path::new("/tmp/cache");
        let path = cached_binary_path(cache_dir, "2.5.0");
        assert!(path.to_string_lossy().contains("v2.5.0"));
        assert!(path.to_string_lossy().contains("golangci-lint"));
    }
}
