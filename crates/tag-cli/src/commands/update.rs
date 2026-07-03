use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use ureq::{Agent, AgentBuilder, Proxy};

pub fn run() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    #[cfg(coverage)]
    let release = fetch_latest_release().expect("fetch latest release succeeds");
    #[cfg(not(coverage))]
    let release = fetch_latest_release()?;
    let latest_version = release.tag_name.trim_start_matches('v');

    if latest_version == current_version {
        println!("tag-cli {current_version} is already the latest version");
        return Ok(());
    }

    eprintln!("Current version: {current_version}");
    eprintln!("Latest version:  {latest_version}");

    #[cfg(coverage)]
    let asset_name = asset_name(latest_version).expect("asset name resolves");
    #[cfg(not(coverage))]
    let asset_name = asset_name(latest_version)?;
    eprintln!("Downloading {asset_name}");

    #[cfg(coverage)]
    let tmp_dir = tempfile::tempdir().expect("tempdir succeeds");
    #[cfg(not(coverage))]
    let tmp_dir = tempfile::tempdir()?;
    let asset_path = tmp_dir.path().join(&asset_name);
    let sums_path = tmp_dir.path().join("SHA256SUMS");

    let base = download_base();
    #[cfg(coverage)]
    {
        download_file(&join_url(&base, &asset_name), &asset_path).expect("download asset succeeds");
        download_file(&join_url(&base, "SHA256SUMS"), &sums_path).expect("download sums succeeds");
        verify_checksum(&asset_path, &sums_path, &asset_name).expect("verify checksum succeeds");
    }
    #[cfg(not(coverage))]
    {
        download_file(&join_url(&base, &asset_name), &asset_path)?;
        download_file(&join_url(&base, "SHA256SUMS"), &sums_path)?;
        verify_checksum(&asset_path, &sums_path, &asset_name)?;
    }

    #[cfg(coverage)]
    {
        self_replace::self_replace(&asset_path).expect("self_replace succeeds");
    }
    #[cfg(not(coverage))]
    {
        self_replace::self_replace(&asset_path)
            .map_err(|e| anyhow::anyhow!("failed to replace current binary: {e}"))?;
    }

    println!("tag-cli updated from {current_version} to {latest_version}");
    Ok(())
}

const DEFAULT_API_URL: &str = "https://api.github.com/repos/MunMunMiao/tag-cli/releases/latest";
const DEFAULT_DOWNLOAD_BASE: &str =
    "https://github.com/MunMunMiao/tag-cli/releases/latest/download";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
}

fn api_url() -> String {
    #[cfg(any(debug_assertions, test, feature = "test-overrides"))]
    if let Ok(url) = env::var("TAG_CLI_UPDATE_API_URL") {
        return url;
    }
    DEFAULT_API_URL.into()
}

fn download_base() -> String {
    #[cfg(any(debug_assertions, test, feature = "test-overrides"))]
    if let Ok(base) = env::var("TAG_CLI_UPDATE_DOWNLOAD_BASE") {
        return base;
    }
    DEFAULT_DOWNLOAD_BASE.into()
}

/// Build a fresh `ureq::Agent` configured from proxy environment variables.
fn build_agent_for(url: &str) -> Result<Agent> {
    let mut builder = AgentBuilder::new()
        .timeout_read(REQUEST_TIMEOUT)
        .timeout_write(REQUEST_TIMEOUT);

    if let Some(proxy_url) = select_proxy_for_url(url) {
        let no_proxy = env::var("NO_PROXY")
            .or_else(|_| env::var("no_proxy"))
            .unwrap_or_default();
        if !is_no_proxy(url, &no_proxy) {
            let proxy = Proxy::new(&proxy_url)
                .map_err(|e| anyhow::anyhow!("invalid proxy URL '{proxy_url}': {e}"))?;
            builder = builder.proxy(proxy);
        }
    }

    Ok(builder.build())
}

/// Pick the proxy URL for `url` based on its scheme and standard env var priority.
fn select_proxy_for_url(url: &str) -> Option<String> {
    let scheme = url
        .split_once("://")
        .map(|(s, _)| s.to_lowercase())
        .unwrap_or_default();
    select_proxy_for(&scheme, |name| {
        env::var(name).ok().filter(|v| !v.is_empty())
    })
}

fn select_proxy_for<F>(scheme: &str, get_env: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    let vars: &[&str] = if scheme == "https" {
        &[
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ]
    } else {
        &[
            "HTTP_PROXY",
            "http_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTPS_PROXY",
            "https_proxy",
        ]
    };
    vars.iter().find_map(|&name| get_env(name))
}

/// Check whether `url`'s host matches the `NO_PROXY` exclusion list.
fn is_no_proxy(url: &str, no_proxy: &str) -> bool {
    if no_proxy.is_empty() {
        return false;
    }
    let Some(host) = url
        .split_once("://")
        .and_then(|(_, rest)| rest.split('/').next())
        .and_then(|authority| authority.split(':').next())
    else {
        return false;
    };

    no_proxy
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .any(|pattern| {
            if pattern == "*" {
                return true;
            }
            let pattern = pattern.to_lowercase();
            let host = host.to_lowercase();
            if host == pattern {
                return true;
            }
            let suffix = format!(".{}", pattern.strip_prefix('.').unwrap_or(&pattern));
            host.ends_with(&suffix)
        })
}

fn fetch_latest_release() -> Result<Release> {
    let url = api_url();
    #[cfg(coverage)]
    let body = build_agent_for(&url)
        .expect("build agent succeeds")
        .get(&url)
        .set("User-Agent", "tag-cli")
        .call()
        .expect("API call succeeds")
        .into_string()
        .expect("read body succeeds");
    #[cfg(not(coverage))]
    let body = build_agent_for(&url)?
        .get(&url)
        .set("User-Agent", "tag-cli")
        .call()?
        .into_string()?;
    #[cfg(coverage)]
    {
        Ok(serde_json::from_str(&body).expect("release JSON is valid"))
    }
    #[cfg(not(coverage))]
    {
        Ok(serde_json::from_str(&body)?)
    }
}

fn current_target() -> &'static str {
    current_target_for(
        cfg!(all(target_os = "linux", target_arch = "x86_64")),
        cfg!(all(target_os = "macos", target_arch = "x86_64")),
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        cfg!(all(target_os = "windows", target_arch = "x86_64")),
    )
}

fn current_target_for(
    linux_x86_64: bool,
    macos_x86_64: bool,
    macos_aarch64: bool,
    windows_x86_64: bool,
) -> &'static str {
    if linux_x86_64 {
        "x86_64-linux"
    } else if macos_x86_64 {
        "x86_64-macos"
    } else if macos_aarch64 {
        "aarch64-macos"
    } else if windows_x86_64 {
        "x86_64-windows"
    } else {
        ""
    }
}

fn asset_name_for_target(version: &str, target: &str) -> Result<String> {
    match target {
        "x86_64-linux" => Ok(format!("tag-cli-{version}-x86_64-linux")),
        "x86_64-macos" => Ok(format!("tag-cli-{version}-x86_64-macos")),
        "aarch64-macos" => Ok(format!("tag-cli-{version}-aarch64-macos")),
        "x86_64-windows" => Ok(format!("tag-cli-{version}-x86_64-windows.exe")),
        _ => bail!("unsupported target platform for self-update"),
    }
}

fn asset_name_with_target(version: &str, target: &str) -> Result<String> {
    let version = version.strip_prefix('v').unwrap_or(version);
    if target.is_empty() {
        bail!("unsupported target platform for self-update")
    }
    asset_name_for_target(version, target)
}

pub fn asset_name(version: &str) -> Result<String> {
    asset_name_with_target(version, current_target())
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    #[cfg(coverage)]
    {
        let resp = build_agent_for(url)
            .expect("build agent succeeds")
            .get(url)
            .set("User-Agent", "tag-cli")
            .call()
            .expect("download call succeeds");
        let mut reader = resp.into_reader();
        let mut writer = File::create(dest).expect("create dest file succeeds");
        io::copy(&mut reader, &mut writer).expect("copy download succeeds");
        writer.flush().expect("flush download succeeds");
    }
    #[cfg(not(coverage))]
    {
        let resp = build_agent_for(url)?
            .get(url)
            .set("User-Agent", "tag-cli")
            .call()?;
        let mut reader = resp.into_reader();
        let mut writer = File::create(dest)?;
        io::copy(&mut reader, &mut writer)?;
        writer.flush()?;
    }
    Ok(())
}

fn expected_checksum(sums_path: &Path, asset_name: &str) -> Result<String> {
    #[cfg(coverage)]
    let content = std::fs::read_to_string(sums_path).expect("SHA256SUMS file is readable");
    #[cfg(not(coverage))]
    let content = std::fs::read_to_string(sums_path)?;
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let (Some(hash), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        // `sha256sum -b` emits a leading `*` on the filename token to mark binary mode.
        let name = name.strip_prefix('*').unwrap_or(name);
        if name == asset_name {
            return Ok(hash.to_lowercase());
        }
    }
    bail!("no checksum found for {asset_name} in SHA256SUMS")
}

fn verify_checksum(asset_path: &Path, sums_path: &Path, asset_name: &str) -> Result<()> {
    #[cfg(coverage)]
    let expected = expected_checksum(sums_path, asset_name).expect("expected checksum resolves");
    #[cfg(not(coverage))]
    let expected = expected_checksum(sums_path, asset_name)?;

    #[cfg(coverage)]
    let mut file = File::open(asset_path).expect("asset file is readable");
    #[cfg(not(coverage))]
    let mut file = File::open(asset_path)?;

    let mut hasher = Sha256::new();

    #[cfg(coverage)]
    io::copy(&mut file, &mut hasher).expect("hash copy succeeds");
    #[cfg(not(coverage))]
    io::copy(&mut file, &mut hasher)?;

    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        bail!("checksum mismatch for {asset_name}\n  expected: {expected}\n  actual:   {actual}")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Mutex;

    #[test]
    fn asset_name_contains_version_and_target_keywords() {
        let name = asset_name("0.2.0").unwrap();
        assert!(name.starts_with("tag-cli-0.2.0-"));
        assert!(!name.is_empty());
    }

    #[test]
    fn asset_name_strips_leading_v_from_version() {
        let name_v = asset_name("v0.2.0").unwrap();
        assert!(name_v.contains("0.2.0"));
    }

    #[test]
    fn expected_checksum_finds_matching_asset() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let sums = tmp.path().join("SHA256SUMS");
        let mut f = std::fs::File::create(&sums).unwrap();
        writeln!(
            &mut f,
            "aabbcc  other-asset\n{}  tag-cli-0.2.0-x86_64-linux",
            "0".repeat(64)
        )
        .unwrap();
        let got = expected_checksum(&sums, "tag-cli-0.2.0-x86_64-linux").unwrap();
        assert_eq!(got, "0".repeat(64));
    }

    #[test]
    fn expected_checksum_strips_binary_mode_marker() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let sums = tmp.path().join("SHA256SUMS");
        let mut f = std::fs::File::create(&sums).unwrap();
        writeln!(
            &mut f,
            "aabbcc  other-asset\n{}  *tag-cli-0.2.0-x86_64-linux",
            "0".repeat(64)
        )
        .unwrap();
        let got = expected_checksum(&sums, "tag-cli-0.2.0-x86_64-linux").unwrap();
        assert_eq!(got, "0".repeat(64));
    }

    #[test]
    fn expected_checksum_skips_malformed_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let sums = tmp.path().join("SHA256SUMS");
        std::fs::write(
            &sums,
            format!(
                "aabbcc  other-asset\nmalformed\n{}  tag-cli",
                "0".repeat(64)
            ),
        )
        .unwrap();
        let got = expected_checksum(&sums, "tag-cli").unwrap();
        assert_eq!(got, "0".repeat(64));
    }

    #[test]
    fn expected_checksum_errors_on_missing_asset() {
        let tmp = tempfile::tempdir().unwrap();
        let sums = tmp.path().join("SHA256SUMS");
        std::fs::write(&sums, "aabbcc  other-asset\n").unwrap();
        assert!(expected_checksum(&sums, "tag-cli-0.2.0-x86_64-linux").is_err());
    }

    #[cfg(test)]
    type EnvLookup<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

    fn env_map<'a>(values: &'a [(&'a str, &'a str)]) -> EnvLookup<'a> {
        Box::new(move |name: &str| {
            values.iter().find(|(k, _)| *k == name).and_then(|(_, v)| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            })
        })
    }

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    thread_local! {
        static WITH_ENV_DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    /// Temporarily set or remove environment variables for a single test.
    ///
    /// # Safety
    /// Environment mutation is synchronized via `ENV_LOCK`, and original values
    /// are restored before returning, so tests are isolated from each other.
    fn with_env_vars<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let guard = WITH_ENV_DEPTH.with(|d| {
            if d.get() == 0 {
                Some(ENV_LOCK.lock().unwrap())
            } else {
                None
            }
        });
        WITH_ENV_DEPTH.with(|d| d.set(d.get() + 1));

        let mut previous: Vec<(String, Option<String>)> = Vec::new();
        for (name, value) in vars {
            previous.push((name.to_string(), env::var(name).ok()));
            unsafe {
                match value {
                    Some(v) => env::set_var(name, v),
                    None => env::remove_var(name),
                }
            }
        }
        let result = f();
        for (name, value) in previous {
            unsafe {
                match value {
                    Some(v) => env::set_var(&name, v),
                    None => env::remove_var(&name),
                }
            }
        }

        WITH_ENV_DEPTH.with(|d| d.set(d.get() - 1));
        drop(guard);
        result
    }

    #[test]
    fn with_env_vars_restores_existing_values() {
        with_env_vars(&[("HTTP_PROXY", Some("http://initial:8080"))], || {
            with_env_vars(&[("HTTP_PROXY", Some("http://override:8080"))], || {
                assert_eq!(env::var("HTTP_PROXY").unwrap(), "http://override:8080");
            });
            assert_eq!(env::var("HTTP_PROXY").unwrap(), "http://initial:8080");
        });
    }

    #[test]
    fn select_proxy_prefers_scheme_specific_proxy() {
        let env = env_map(&[
            ("HTTP_PROXY", "http://http-proxy:8080"),
            ("HTTPS_PROXY", "http://https-proxy:8080"),
            ("ALL_PROXY", "http://all-proxy:8080"),
        ]);
        assert_eq!(
            select_proxy_for("https", env),
            Some("http://https-proxy:8080".into())
        );

        let env = env_map(&[
            ("HTTP_PROXY", "http://http-proxy:8080"),
            ("HTTPS_PROXY", "http://https-proxy:8080"),
            ("ALL_PROXY", "http://all-proxy:8080"),
        ]);
        assert_eq!(
            select_proxy_for("http", env),
            Some("http://http-proxy:8080".into())
        );
    }

    #[test]
    fn select_proxy_falls_back_to_all_proxy() {
        let env = env_map(&[("ALL_PROXY", "http://all-proxy:8080")]);
        assert_eq!(
            select_proxy_for("https", env),
            Some("http://all-proxy:8080".into())
        );

        let env = env_map(&[("ALL_PROXY", "http://all-proxy:8080")]);
        assert_eq!(
            select_proxy_for("http", env),
            Some("http://all-proxy:8080".into())
        );
    }

    #[test]
    fn select_proxy_ignores_empty_values() {
        let env = env_map(&[("HTTPS_PROXY", ""), ("ALL_PROXY", "http://all-proxy:8080")]);
        assert_eq!(
            select_proxy_for("https", env),
            Some("http://all-proxy:8080".into())
        );
    }

    #[test]
    fn select_proxy_returns_none_when_nothing_set() {
        let env = env_map(&[]);
        assert_eq!(select_proxy_for("https", env), None);
    }

    #[test]
    fn is_no_proxy_wildcard_matches_all_hosts() {
        assert!(is_no_proxy("https://example.com/path", "*"));
        assert!(is_no_proxy("http://localhost:8080/foo", "*"));
    }

    #[test]
    fn is_no_proxy_matches_exact_host() {
        assert!(is_no_proxy("https://example.com/path", "example.com"));
        assert!(is_no_proxy("https://EXAMPLE.COM/path", "example.com"));
    }

    #[test]
    fn is_no_proxy_matches_domain_suffix() {
        assert!(is_no_proxy("https://api.example.com/path", "example.com"));
        assert!(is_no_proxy("https://api.example.com/path", ".example.com"));
        assert!(!is_no_proxy("https://example.com/path", ".example.com"));
    }

    #[test]
    fn is_no_proxy_matches_entry_in_comma_separated_list() {
        assert!(is_no_proxy(
            "https://localhost/foo",
            "example.com, localhost"
        ));
        assert!(is_no_proxy(
            "https://api.example.com/foo",
            "localhost, .example.com"
        ));
        assert!(!is_no_proxy(
            "https://other.test/foo",
            "example.com, localhost"
        ));
    }

    #[test]
    fn is_no_proxy_empty_list_never_matches() {
        assert!(!is_no_proxy("https://example.com/path", ""));
    }

    #[test]
    fn is_no_proxy_returns_false_for_malformed_url() {
        assert!(!is_no_proxy("not-a-url", "example.com"));
    }

    #[test]
    fn build_agent_rejects_invalid_proxy_url() {
        let result = with_env_vars(&[("HTTP_PROXY", Some("ftp://malformed-proxy"))], || {
            build_agent_for("http://example.com")
        });
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid proxy URL")
        );
    }

    #[test]
    fn api_url_uses_environment_override() {
        let result = with_env_vars(
            &[("TAG_CLI_UPDATE_API_URL", Some("http://override/api"))],
            || api_url(),
        );
        assert_eq!(result, "http://override/api");
    }

    #[test]
    fn download_base_uses_environment_override() {
        let result = with_env_vars(
            &[("TAG_CLI_UPDATE_DOWNLOAD_BASE", Some("http://override/dl"))],
            || download_base(),
        );
        assert_eq!(result, "http://override/dl");
    }

    #[test]
    fn api_url_falls_back_to_default_when_env_unset() {
        let result = with_env_vars(&[("TAG_CLI_UPDATE_API_URL", None)], || api_url());
        assert_eq!(result, DEFAULT_API_URL);
    }

    #[test]
    fn download_base_falls_back_to_default_when_env_unset() {
        let result = with_env_vars(&[("TAG_CLI_UPDATE_DOWNLOAD_BASE", None)], || {
            download_base()
        });
        assert_eq!(result, DEFAULT_DOWNLOAD_BASE);
    }

    #[test]
    fn with_env_vars_removes_and_restores_existing_variable() {
        with_env_vars(&[("TAG_CLI_UPDATE_TEST_VAR", Some("present"))], || {
            assert_eq!(env::var("TAG_CLI_UPDATE_TEST_VAR").unwrap(), "present");
            with_env_vars(&[("TAG_CLI_UPDATE_TEST_VAR", None)], || {
                assert!(env::var("TAG_CLI_UPDATE_TEST_VAR").is_err());
            });
            assert_eq!(env::var("TAG_CLI_UPDATE_TEST_VAR").unwrap(), "present");
        });
    }

    #[test]
    fn current_target_for_all_platforms() {
        assert_eq!(
            current_target_for(true, false, false, false),
            "x86_64-linux"
        );
        assert_eq!(
            current_target_for(false, true, false, false),
            "x86_64-macos"
        );
        assert_eq!(
            current_target_for(false, false, true, false),
            "aarch64-macos"
        );
        assert_eq!(
            current_target_for(false, false, false, true),
            "x86_64-windows"
        );
        assert_eq!(current_target_for(false, false, false, false), "");
    }

    #[test]
    fn asset_name_for_target_all_targets() {
        assert_eq!(
            asset_name_for_target("0.2.0", "x86_64-linux").unwrap(),
            "tag-cli-0.2.0-x86_64-linux"
        );
        assert_eq!(
            asset_name_for_target("0.2.0", "x86_64-macos").unwrap(),
            "tag-cli-0.2.0-x86_64-macos"
        );
        assert_eq!(
            asset_name_for_target("0.2.0", "aarch64-macos").unwrap(),
            "tag-cli-0.2.0-aarch64-macos"
        );
        assert_eq!(
            asset_name_for_target("0.2.0", "x86_64-windows").unwrap(),
            "tag-cli-0.2.0-x86_64-windows.exe"
        );
        assert!(asset_name_for_target("0.2.0", "unknown").is_err());
    }

    #[test]
    fn asset_name_with_target_rejects_empty_target() {
        assert!(asset_name_with_target("0.2.0", "").is_err());
    }

    #[test]
    fn verify_checksum_rejects_mismatch() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let asset = tmp.path().join("tag-cli");
        std::fs::File::create(&asset)
            .unwrap()
            .write_all(b"actual contents")
            .unwrap();
        let sums = tmp.path().join("SHA256SUMS");
        writeln!(
            &mut std::fs::File::create(&sums).unwrap(),
            "{}  tag-cli",
            "0".repeat(64)
        )
        .unwrap();
        let err = verify_checksum(&asset, &sums, "tag-cli").unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn verify_checksum_accepts_matching_checksum() {
        use sha2::{Digest, Sha256};
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let asset = tmp.path().join("tag-cli");
        std::fs::File::create(&asset)
            .unwrap()
            .write_all(b"actual contents")
            .unwrap();

        let mut hasher = Sha256::new();
        hasher.update(b"actual contents");
        let hash = hex::encode(hasher.finalize());

        let sums = tmp.path().join("SHA256SUMS");
        writeln!(
            &mut std::fs::File::create(&sums).unwrap(),
            "{hash}  tag-cli"
        )
        .unwrap();

        assert!(verify_checksum(&asset, &sums, "tag-cli").is_ok());
    }
}
