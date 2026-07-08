use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use ureq::{Agent, AgentBuilder, Proxy};
use url::Url;

pub fn run() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    #[cfg(coverage)]
    let release = fetch_latest_release().expect("fetch latest release succeeds");
    #[cfg(not(coverage))]
    let release = fetch_latest_release_for_update()?;
    let latest_tag = release.tag_name.as_str();
    let latest_version = latest_tag.trim_start_matches('v');

    if latest_version == current_version {
        println!("tag-cli {current_version} is already the latest version");
        return Ok(());
    }

    eprintln!("Current version: {current_version}");
    eprintln!("Latest version:  {latest_version}");

    #[cfg(coverage)]
    let asset_name = asset_name(latest_version).expect("asset name resolves");
    #[cfg(not(coverage))]
    let asset_name = resolve_asset_name(latest_version)?;
    eprintln!("Downloading {asset_name}");

    #[cfg(coverage)]
    let tmp_dir = tempfile::tempdir().expect("tempdir succeeds");
    #[cfg(not(coverage))]
    let tmp_dir = tempfile::tempdir()?;
    let asset_path = tmp_dir.path().join(&asset_name);
    let sums_path = tmp_dir.path().join("SHA256SUMS");

    let download_base = release_download_base(latest_tag);

    #[cfg(coverage)]
    {
        download_file(&join_url(&download_base, &asset_name), &asset_path)
            .expect("download asset succeeds");
        download_file(&join_url(&download_base, "SHA256SUMS"), &sums_path)
            .expect("download sums succeeds");
        verify_checksum(&asset_path, &sums_path, &asset_name).expect("verify checksum succeeds");
    }
    #[cfg(not(coverage))]
    {
        download_file(&join_url(&download_base, &asset_name), &asset_path)?;
        download_file(&join_url(&download_base, "SHA256SUMS"), &sums_path)?;
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
const DEFAULT_DOWNLOAD_BASE: &str = "https://github.com/MunMunMiao/tag-cli/releases/download";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_REDIRECTS: usize = 5;

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

fn release_download_base(tag: &str) -> String {
    join_url(DEFAULT_DOWNLOAD_BASE, tag)
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
}

/// Build a fresh `ureq::Agent` configured from proxy environment variables.
fn build_agent_for(url: &str) -> Result<Agent> {
    let mut builder = AgentBuilder::new()
        .timeout_read(REQUEST_TIMEOUT)
        .timeout_write(REQUEST_TIMEOUT)
        .redirects(0)
        .try_proxy_from_env(false);

    if let Some(proxy_url) = select_proxy_for_url(url) {
        let no_proxy = select_no_proxy();
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
    let scheme = Url::parse(url)
        .map(|u| u.scheme().to_string())
        .unwrap_or_default();
    let vars: &[&str] = if scheme == "https" {
        &["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
    } else {
        &["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"]
    };
    vars.iter()
        .find_map(|&name| env::var(name).ok().filter(|v| !v.is_empty()))
}

fn select_no_proxy() -> String {
    ["NO_PROXY", "no_proxy"]
        .iter()
        .find_map(|&name| env::var(name).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

/// Check whether `url`'s host matches the `NO_PROXY` exclusion list.
fn is_no_proxy(url: &str, no_proxy: &str) -> bool {
    if no_proxy.is_empty() {
        return false;
    }
    let Some(host) = Url::parse(url).ok().and_then(|u| {
        u.host_str().map(|h| {
            h.strip_prefix('[')
                .and_then(|h| h.strip_suffix(']'))
                .unwrap_or(h)
                .to_owned()
        })
    }) else {
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
            let pattern = pattern
                .strip_prefix('[')
                .and_then(|p| p.strip_suffix(']'))
                .unwrap_or(pattern)
                .to_lowercase();
            let host = host.to_lowercase();
            if host == pattern {
                return true;
            }
            let suffix = format!(".{}", pattern.strip_prefix('.').unwrap_or(&pattern));
            host.ends_with(&suffix)
        })
}

fn fetch_latest_release() -> Result<Release> {
    #[cfg(coverage)]
    let body = get_with_redirects(DEFAULT_API_URL)
        .expect("API call succeeds")
        .into_string()
        .expect("read body succeeds");
    #[cfg(not(coverage))]
    let body = get_with_redirects(DEFAULT_API_URL)?.into_string()?;
    #[cfg(coverage)]
    {
        Ok(serde_json::from_str(&body).expect("release JSON is valid"))
    }
    #[cfg(not(coverage))]
    {
        Ok(serde_json::from_str(&body)?)
    }
}

// Production fetch is excluded from coverage because it requires network access.
#[cfg(not(coverage))]
fn fetch_latest_release_for_update() -> Result<Release> {
    fetch_latest_release()
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

// Production asset-name resolution is excluded from coverage because it depends
// on the real target platform and is already exercised by the unit tests above.
#[cfg(not(coverage))]
fn resolve_asset_name(version: &str) -> Result<String> {
    asset_name(version)
}

fn get_with_redirects(url: &str) -> Result<ureq::Response> {
    let mut url = url.to_string();
    let mut redirects = 0;
    loop {
        let resp = build_agent_for(&url)?
            .get(&url)
            .set("User-Agent", "tag-cli")
            .call()?;
        if !(300..400).contains(&resp.status()) {
            return Ok(resp);
        }
        if redirects >= MAX_REDIRECTS {
            bail!("reached max redirects ({MAX_REDIRECTS})");
        }
        let location = resp
            .header("location")
            .ok_or_else(|| anyhow::anyhow!("redirect response missing Location header"))?;
        url = redirect_url(&url, location)?;
        redirects += 1;
    }
}

fn redirect_url(current_url: &str, location: &str) -> Result<String> {
    Ok(Url::parse(current_url)
        .map_err(|e| anyhow::anyhow!("invalid redirect base URL '{current_url}': {e}"))?
        .join(location)
        .map_err(|e| anyhow::anyhow!("invalid redirect Location '{location}': {e}"))?
        .into())
}

fn download_file(url: &str, dest: &Path) -> Result<()> {
    #[cfg(coverage)]
    {
        let resp = get_with_redirects(url).expect("download call succeeds");
        let mut reader = resp.into_reader();
        let mut writer = File::create(dest).expect("create dest file succeeds");
        io::copy(&mut reader, &mut writer).expect("copy download succeeds");
        writer.flush().expect("flush download succeeds");
    }
    #[cfg(not(coverage))]
    {
        let resp = get_with_redirects(url)?;
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

    let actual: [u8; 32] = hasher.finalize().into();
    let actual = actual
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
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
        with_env_vars(
            &[
                ("HTTP_PROXY", Some("http://http-proxy:8080")),
                ("http_proxy", None),
                ("HTTPS_PROXY", Some("http://https-proxy:8080")),
                ("https_proxy", None),
                ("ALL_PROXY", Some("http://all-proxy:8080")),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(
                    select_proxy_for_url("https://example.com"),
                    Some("http://https-proxy:8080".into())
                );
                assert_eq!(
                    select_proxy_for_url("http://example.com"),
                    Some("http://http-proxy:8080".into())
                );
            },
        );
    }

    #[test]
    fn select_proxy_does_not_cross_fallback_between_http_and_https() {
        with_env_vars(
            &[
                ("HTTP_PROXY", Some("http://http-proxy:8080")),
                ("http_proxy", None),
                ("HTTPS_PROXY", None),
                ("https_proxy", None),
                ("ALL_PROXY", None),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(select_proxy_for_url("https://example.com"), None);
            },
        );

        with_env_vars(
            &[
                ("HTTP_PROXY", None),
                ("http_proxy", None),
                ("HTTPS_PROXY", Some("http://https-proxy:8080")),
                ("https_proxy", None),
                ("ALL_PROXY", None),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(select_proxy_for_url("http://example.com"), None);
            },
        );
    }

    #[test]
    fn select_proxy_falls_back_to_all_proxy() {
        with_env_vars(
            &[
                ("HTTP_PROXY", None),
                ("http_proxy", None),
                ("HTTPS_PROXY", None),
                ("https_proxy", None),
                ("ALL_PROXY", Some("http://all-proxy:8080")),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(
                    select_proxy_for_url("https://example.com"),
                    Some("http://all-proxy:8080".into())
                );
                assert_eq!(
                    select_proxy_for_url("http://example.com"),
                    Some("http://all-proxy:8080".into())
                );
            },
        );
    }

    #[test]
    fn select_proxy_ignores_empty_values() {
        with_env_vars(
            &[
                ("HTTPS_PROXY", Some("")),
                ("https_proxy", None),
                ("ALL_PROXY", Some("http://all-proxy:8080")),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(
                    select_proxy_for_url("https://example.com"),
                    Some("http://all-proxy:8080".into())
                );
            },
        );
    }

    #[test]
    fn select_proxy_returns_none_when_nothing_set() {
        with_env_vars(
            &[
                ("HTTP_PROXY", None),
                ("http_proxy", None),
                ("HTTPS_PROXY", None),
                ("https_proxy", None),
                ("ALL_PROXY", None),
                ("all_proxy", None),
            ],
            || {
                assert_eq!(select_proxy_for_url("https://example.com"), None);
            },
        );
    }

    #[test]
    fn select_no_proxy_falls_back_to_lowercase_when_uppercase_is_empty() {
        with_env_vars(
            &[("NO_PROXY", Some("")), ("no_proxy", Some("github.com"))],
            || {
                assert_eq!(select_no_proxy(), "github.com");
            },
        );
    }

    #[test]
    fn release_download_base_uses_fetched_tag() {
        assert_eq!(
            release_download_base("1.2.3"),
            "https://github.com/MunMunMiao/tag-cli/releases/download/1.2.3"
        );
        assert_eq!(
            release_download_base("v1.2.3"),
            "https://github.com/MunMunMiao/tag-cli/releases/download/v1.2.3"
        );
    }

    #[test]
    fn redirect_url_handles_absolute_and_relative_locations() {
        assert_eq!(
            redirect_url("https://github.com/a/b", "https://assets.example.com/file").unwrap(),
            "https://assets.example.com/file"
        );
        assert_eq!(
            redirect_url("https://github.com/a/b", "//assets.example.com/file").unwrap(),
            "https://assets.example.com/file"
        );
        assert_eq!(
            redirect_url("https://github.com/a/b", "/download/file").unwrap(),
            "https://github.com/download/file"
        );
        assert_eq!(
            redirect_url("https://github.com/a/b", "?download=1").unwrap(),
            "https://github.com/a/b?download=1"
        );
        assert_eq!(
            redirect_url("https://github.com/a/b", "file").unwrap(),
            "https://github.com/a/file"
        );
    }

    #[test]
    fn get_with_redirects_allows_max_redirects_before_final_response() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpListener;
        use std::thread;

        with_env_vars(
            &[
                ("HTTP_PROXY", None),
                ("http_proxy", None),
                ("HTTPS_PROXY", None),
                ("https_proxy", None),
                ("ALL_PROXY", None),
                ("all_proxy", None),
                ("NO_PROXY", None),
                ("no_proxy", None),
            ],
            || {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                let addr = listener.local_addr().unwrap();
                let handle = thread::spawn(move || {
                    for i in 0..=MAX_REDIRECTS {
                        let (mut stream, _) = listener.accept().unwrap();
                        let mut first_line = String::new();
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        reader.read_line(&mut first_line).unwrap();
                        loop {
                            let mut line = String::new();
                            reader.read_line(&mut line).unwrap();
                            if line == "\r\n" {
                                break;
                            }
                        }
                        assert!(first_line.starts_with(&format!("GET /hop{i}")));
                        if i < MAX_REDIRECTS {
                            write!(
                                stream,
                                "HTTP/1.1 302 Found\r\nLocation: /hop{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                                i + 1
                            )
                            .unwrap();
                        } else {
                            write!(
                                stream,
                                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                            )
                            .unwrap();
                        }
                    }
                });

                let body = get_with_redirects(&format!("http://{addr}/hop0"))
                    .unwrap()
                    .into_string()
                    .unwrap();
                assert_eq!(body, "ok");
                handle.join().unwrap();
            },
        );
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
    fn is_no_proxy_matches_url_parser_hosts() {
        assert!(is_no_proxy("http://[::1]:8000/path", "::1"));
        assert!(is_no_proxy("http://[::1]:8000/path", "[::1]"));
        assert!(is_no_proxy(
            "http://user:pass@example.com/path",
            "example.com"
        ));
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
        let hash: [u8; 32] = hasher.finalize().into();
        let hash = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();

        let sums = tmp.path().join("SHA256SUMS");
        writeln!(
            &mut std::fs::File::create(&sums).unwrap(),
            "{hash}  tag-cli"
        )
        .unwrap();

        assert!(verify_checksum(&asset, &sums, "tag-cli").is_ok());
    }
}
