use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use assert_cmd::Command;
use tempfile::TempDir;
use tiny_http::{Header, ListenAddr, Response, Server};

// The self-update integration tests copy the real binary and then execute the
// copy. On Linux this can race with concurrent copies/executions in the same
// process and produce `ETXTBSY` ("Text file busy"). Serialize the tests to keep
// the file operations from stepping on each other.
static UPDATE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn spawn_mock_server(
    tmp: &TempDir,
    release_json: &str,
    tampered_hash: Option<&str>,
) -> (u16, String) {
    let server = Server::http("127.0.0.1:0").unwrap();
    let port = match server.server_addr() {
        ListenAddr::IP(addr) => addr.port(),
        _ => panic!("expected TCP socket address"),
    };
    let base = tmp.path().to_path_buf();

    let asset_name = tag_cli::commands::update::asset_name("99.99.99").unwrap();
    let asset_path = base.join(&asset_name);
    fs::write(&asset_path, b"new binary contents").unwrap();

    let hash = tampered_hash.map(String::from).unwrap_or_else(|| {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"new binary contents");
        format!("{:x}", hasher.finalize())
    });
    fs::write(base.join("SHA256SUMS"), format!("{hash}  {asset_name}\n")).unwrap();

    let json_path = base.join("release.json");
    fs::write(&json_path, release_json).unwrap();

    let asset_name_for_thread = asset_name.clone();
    thread::spawn(move || {
        let json_header =
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();

        for request in server.incoming_requests() {
            let url = request.url().to_string();
            let response: Response<std::io::Cursor<Vec<u8>>> = if url == "/release.json" {
                Response::from_string(fs::read_to_string(&json_path).unwrap())
                    .with_header(json_header.clone())
            } else if url == "/SHA256SUMS" {
                Response::from_string(fs::read_to_string(base.join("SHA256SUMS")).unwrap())
            } else if url == format!("/{asset_name_for_thread}") {
                Response::from_data(fs::read(&asset_path).unwrap())
            } else {
                Response::from_string("not found").with_status_code(404)
            };
            request.respond(response).ok();
        }
    });

    (port, asset_name)
}

fn copy_test_binary(tmp: &TempDir) -> PathBuf {
    let cargo_path = PathBuf::from(Command::cargo_bin("tag-cli").unwrap().get_program());
    let ext = cargo_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let tmp_bin = if ext.is_empty() {
        tmp.path().join("tag-cli")
    } else {
        tmp.path().join(format!("tag-cli.{ext}"))
    };
    fs::copy(&cargo_path, &tmp_bin).unwrap();
    tmp_bin
}

fn clear_proxy_env(cmd: &mut Command) {
    cmd.env_remove("HTTP_PROXY")
        .env_remove("http_proxy")
        .env_remove("HTTPS_PROXY")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY")
        .env_remove("all_proxy")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy");
}

#[test]
fn update_detects_new_version_and_downloads() {
    let _guard = UPDATE_TEST_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let (port, asset_name) = spawn_mock_server(&tmp, r#"{"tag_name":"v99.99.99"}"#, None);

    let tmp_bin = copy_test_binary(&tmp);

    let mut cmd = Command::new(&tmp_bin);
    clear_proxy_env(&mut cmd);
    cmd.arg("update")
        .env(
            "TAG_CLI_UPDATE_API_URL",
            format!("http://127.0.0.1:{port}/release.json"),
        )
        .env(
            "TAG_CLI_UPDATE_DOWNLOAD_BASE",
            format!("http://127.0.0.1:{port}"),
        );

    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("Current version"), "stderr: {stderr}");
    assert!(stderr.contains("Latest version"), "stderr: {stderr}");
    assert!(
        stderr.contains(&format!("Downloading {asset_name}")),
        "stderr: {stderr}"
    );

    let replaced = fs::read(&tmp_bin).unwrap();
    assert_eq!(replaced, b"new binary contents");
}

#[test]
fn update_rejects_checksum_mismatch() {
    let _guard = UPDATE_TEST_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let (port, _asset_name) =
        spawn_mock_server(&tmp, r#"{"tag_name":"v99.99.99"}"#, Some(&"0".repeat(64)));

    let tmp_bin = copy_test_binary(&tmp);
    let original = fs::read(&tmp_bin).unwrap();

    let mut cmd = Command::new(&tmp_bin);
    clear_proxy_env(&mut cmd);
    cmd.arg("update")
        .env(
            "TAG_CLI_UPDATE_API_URL",
            format!("http://127.0.0.1:{port}/release.json"),
        )
        .env(
            "TAG_CLI_UPDATE_DOWNLOAD_BASE",
            format!("http://127.0.0.1:{port}"),
        );

    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("checksum mismatch"), "stderr: {stderr}");

    let after = fs::read(&tmp_bin).unwrap();
    assert_eq!(after, original);
}

#[test]
fn update_already_up_to_date() {
    let _guard = UPDATE_TEST_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let current_version = env!("CARGO_PKG_VERSION");
    let json = format!("{{\"tag_name\":\"v{current_version}\"}}");
    let (port, _asset_name) = spawn_mock_server(&tmp, &json, None);

    let tmp_bin = copy_test_binary(&tmp);

    let mut cmd = Command::new(&tmp_bin);
    clear_proxy_env(&mut cmd);
    cmd.arg("update")
        .env(
            "TAG_CLI_UPDATE_API_URL",
            format!("http://127.0.0.1:{port}/release.json"),
        )
        .env(
            "TAG_CLI_UPDATE_DOWNLOAD_BASE",
            format!("http://127.0.0.1:{port}"),
        );

    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("is already the latest version"),
        "stdout: {stdout}"
    );
}

#[test]
fn update_fails_when_proxy_is_unreachable() {
    let _guard = UPDATE_TEST_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let (port, _asset_name) = spawn_mock_server(&tmp, r#"{"tag_name":"v99.99.99"}"#, None);

    let tmp_bin = copy_test_binary(&tmp);

    let mut cmd = Command::new(&tmp_bin);
    clear_proxy_env(&mut cmd);
    cmd.arg("update")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env(
            "TAG_CLI_UPDATE_API_URL",
            format!("http://127.0.0.1:{port}/release.json"),
        )
        .env(
            "TAG_CLI_UPDATE_DOWNLOAD_BASE",
            format!("http://127.0.0.1:{port}"),
        );

    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("proxy")
            || stderr.contains("Proxy")
            || stderr.contains("Connection refused"),
        "expected proxy-related error, got stderr: {stderr}"
    );
}

#[test]
fn update_bypasses_proxy_for_no_proxy_hosts() {
    let _guard = UPDATE_TEST_LOCK.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let (port, asset_name) = spawn_mock_server(&tmp, r#"{"tag_name":"v99.99.99"}"#, None);

    let tmp_bin = copy_test_binary(&tmp);

    let mut cmd = Command::new(&tmp_bin);
    clear_proxy_env(&mut cmd);
    cmd.arg("update")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env(
            "TAG_CLI_UPDATE_API_URL",
            format!("http://127.0.0.1:{port}/release.json"),
        )
        .env(
            "TAG_CLI_UPDATE_DOWNLOAD_BASE",
            format!("http://127.0.0.1:{port}"),
        );

    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains(&format!("Downloading {asset_name}")),
        "stderr: {stderr}"
    );

    let replaced = fs::read(&tmp_bin).unwrap();
    assert_eq!(replaced, b"new binary contents");
}
