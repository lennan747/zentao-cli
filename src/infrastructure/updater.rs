//! zentao-cli 自更新引擎：查询最新版本、下载校验、解包、替换当前可执行文件。
//!
//! 与一键安装脚本同源约定：资产名 `zentao-cli-<target>.tar.gz`（Linux/macOS）
//! 或 `.zip`（Windows），发布页附带 `SHA256SUMS`（含全部平台行）。

use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// 当前版本（编译期写入，与 `--version` 一致）。
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 默认发布仓库。
pub const DEFAULT_REPO: &str = "lennan747/zentao-cli";

/// GitHub API 基地址；可用 `ZENTAO_CLI_UPDATE_API` 覆盖（镜像/测试）。
pub fn release_api_base() -> String {
    std::env::var("ZENTAO_CLI_UPDATE_API").unwrap_or_else(|_| "https://api.github.com".into())
}

/// 发布资产下载基地址；可用 `ZENTAO_CLI_UPDATE_DOWNLOAD` 覆盖（镜像/测试）。
pub fn release_download_base() -> String {
    std::env::var("ZENTAO_CLI_UPDATE_DOWNLOAD").unwrap_or_else(|_| "https://github.com".into())
}

/// 发布资产：文件名、压缩包内二进制名、是否为 zip。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetTarget {
    pub asset: String,
    pub binary_name: String,
    pub is_zip: bool,
}

/// 识别当前平台对应的发布资产；不支持的平台返回 `None`。
pub fn asset_target() -> Option<AssetTarget> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some(AssetTarget {
            asset: "zentao-cli-x86_64-unknown-linux-gnu.tar.gz".into(),
            binary_name: "zentao-cli".into(),
            is_zip: false,
        }),
        ("macos", "aarch64") => Some(AssetTarget {
            asset: "zentao-cli-aarch64-apple-darwin.tar.gz".into(),
            binary_name: "zentao-cli".into(),
            is_zip: false,
        }),
        ("windows", "x86_64") => Some(AssetTarget {
            asset: "zentao-cli-x86_64-pc-windows-msvc.zip".into(),
            binary_name: "zentao-cli.exe".into(),
            is_zip: true,
        }),
        _ => None,
    }
}

/// 解析语义化版本（可选 `v` 前缀、可带预发布后缀），返回 `(major, minor, patch)`。
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let trimmed = v.trim().strip_prefix('v').unwrap_or(v.trim());
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|s| s.split(['-', '+']).next().and_then(|s| s.parse().ok()))
        .unwrap_or(0);
    Some((major, minor, patch))
}

/// `a > b` 是否成立（版本号无法解析时视为不更新）。
pub fn is_newer(a: &str, b: &str) -> bool {
    match (parse_version(a), parse_version(b)) {
        (Some(x), Some(y)) => x > y,
        _ => false,
    }
}

/// 规范化版本标签：确保带 `v` 前缀（GitHub 下载地址使用原始 tag）。
pub fn normalize_tag(v: &str) -> String {
    let v = v.trim();
    if v.starts_with('v') {
        v.to_string()
    } else {
        format!("v{v}")
    }
}

/// 从 `SHA256SUMS` 内容中提取指定资产行的哈希（小写）。
pub fn expected_sha256(sums: &str, asset: &str) -> Option<String> {
    for line in sums.lines() {
        let trimmed = line.trim();
        if !(trimmed.ends_with(asset) || trimmed.ends_with(&format!("*{asset}"))) {
            continue;
        }
        let hash = trimmed.split_whitespace().next()?;
        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

/// 计算文件的 SHA256（小写十六进制）。
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("打开文件失败 {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取失败 {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 查询最新 release 标签（如 `v0.1.1`）。
pub async fn latest_tag(client: &reqwest::Client, repo: &str) -> Result<String, String> {
    let url = format!("{}/repos/{repo}/releases/latest", release_api_base());
    let resp = client
        .get(&url)
        .header("User-Agent", "zentao-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("查询最新版本失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("查询最新版本失败: HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析最新版本响应失败: {e}"))?;
    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| "解析最新版本失败：响应缺少 tag_name".to_string())
}

/// 下载 URL 内容到本地文件。
pub async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    let resp = client
        .get(url)
        .header("User-Agent", "zentao-cli")
        .send()
        .await
        .map_err(|e| format!("下载失败 {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载失败 {url}: HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("下载失败 {url}: {e}"))?;
    std::fs::write(dest, &bytes).map_err(|e| format!("写入文件失败 {}: {e}", dest.display()))
}

/// 下载 `SHA256SUMS` 与平台资产，校验哈希后解包，返回新二进制路径（位于 `temp_dir` 内）。
pub async fn fetch_and_extract(
    client: &reqwest::Client,
    repo: &str,
    tag: &str,
    temp_dir: &Path,
) -> Result<PathBuf, String> {
    let target = asset_target().ok_or_else(|| {
        "当前平台不支持一键更新（仅支持 Linux x86_64 / macOS arm64 / Windows x86_64）".to_string()
    })?;
    let base = format!("{}/{repo}/releases/download/{tag}", release_download_base());
    let asset_url = format!("{base}/{}", target.asset);
    let sums_url = format!("{base}/SHA256SUMS");

    let asset_path = temp_dir.join(&target.asset);
    let sums_path = temp_dir.join("SHA256SUMS");
    download(client, &sums_url, &sums_path).await?;
    download(client, &asset_url, &asset_path).await?;

    let sums =
        std::fs::read_to_string(&sums_path).map_err(|e| format!("读取 SHA256SUMS 失败: {e}"))?;
    let expected = expected_sha256(&sums, &target.asset)
        .ok_or_else(|| format!("SHA256SUMS 中未找到 {} 的校验值", target.asset))?;
    let actual = sha256_file(&asset_path)?;
    if actual != expected {
        return Err("SHA256 校验失败，已中止更新（请勿绕过校验）".to_string());
    }
    extract(&asset_path, &target, temp_dir)
}

/// 解包压缩资产，返回包内二进制路径。
pub fn extract(archive: &Path, target: &AssetTarget, dest: &Path) -> Result<PathBuf, String> {
    if target.is_zip {
        extract_zip(archive, target, dest)
    } else {
        extract_tar_gz(archive, target, dest)
    }
}

fn extract_zip(archive: &Path, target: &AssetTarget, dest: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("打开压缩包失败: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("解析 zip 失败: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("读取 zip 条目失败: {e}"))?;
        let name = entry.name().to_string();
        if !name.ends_with(&target.binary_name) || name.contains("__MACOSX") {
            continue;
        }
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&target.binary_name);
        let out_path = dest.join(file_name);
        let mut out = std::fs::File::create(&out_path).map_err(|e| format!("解包失败: {e}"))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("解包失败: {e}"))?;
        return Ok(out_path);
    }
    Err("压缩包内未找到可执行文件".to_string())
}

fn extract_tar_gz(archive: &Path, target: &AssetTarget, dest: &Path) -> Result<PathBuf, String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("打开压缩包失败: {e}"))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut ar = tar::Archive::new(gz);
    for entry in ar.entries().map_err(|e| format!("解析 tar 失败: {e}"))? {
        let mut entry = entry.map_err(|e| format!("读取 tar 条目失败: {e}"))?;
        let name = entry
            .path()
            .ok()
            .and_then(|p| p.file_name().and_then(|s| s.to_str().map(str::to_string)))
            .unwrap_or_default();
        if name != target.binary_name {
            continue;
        }
        let out_path = dest.join(&name);
        entry
            .unpack(&out_path)
            .map_err(|e| format!("解包失败: {e}"))?;
        return Ok(out_path);
    }
    Err("压缩包内未找到可执行文件".to_string())
}

/// 用新二进制替换当前运行的可执行文件。
///
/// 先复制到与当前可执行文件同目录的暂存文件（避免跨文件系统 rename），再替换：
/// - Unix：直接原子 rename；
/// - Windows：先把当前程序改名为 `.exe.old`，再放新文件（运行中的 exe 无法覆盖）。
pub fn replace_current_exe(new_bin: &Path) -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| format!("无法定位当前程序: {e}"))?;
    replace_file(new_bin, &current)
}

/// 通用替换（可注入路径，便于测试）：`current` 最终内容为 `new_bin` 的内容。
pub fn replace_file(new_bin: &Path, current: &Path) -> Result<(), String> {
    let dir = current
        .parent()
        .ok_or_else(|| "无法定位安装目录".to_string())?;
    let file_name = current
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("zentao-cli");
    let staged = dir.join(format!(".{file_name}.update-{}", std::process::id()));
    std::fs::copy(new_bin, &staged).map_err(|e| format!("写入暂存文件失败: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755));
        std::fs::rename(&staged, current).map_err(|e| format!("替换失败: {e}"))?;
    }
    #[cfg(windows)]
    {
        let old = current.with_extension("exe.old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(current, &old).map_err(|e| format!("备份当前程序失败: {e}"))?;
        if let Err(e) = std::fs::rename(&staged, current) {
            let _ = std::fs::rename(&old, current);
            return Err(format!("替换失败: {e}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_versions() {
        assert_eq!(parse_version("0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("v0.1.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("v1"), Some((1, 0, 0)));
        assert_eq!(parse_version("0.1.1-rc.1"), Some((0, 1, 1)));
        assert_eq!(parse_version("abc"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("nonsense", "0.1.1"));
    }

    #[test]
    fn normalizes_tag() {
        assert_eq!(normalize_tag("0.1.1"), "v0.1.1");
        assert_eq!(normalize_tag("v0.1.1"), "v0.1.1");
        assert_eq!(normalize_tag(" v0.2.0 "), "v0.2.0");
    }

    #[test]
    fn extracts_expected_hash_line() {
        let sums = "abc123  zentao-cli-aarch64-apple-darwin.tar.gz\n\
                    4f06310e0e2b3f105af8384c476888251593562b4b15a56b90ca5c79dc04eb43  zentao-cli-x86_64-unknown-linux-gnu.tar.gz\n\
                    deadbeef  zentao-cli-x86_64-pc-windows-msvc.zip\n";
        assert_eq!(
            expected_sha256(sums, "zentao-cli-x86_64-unknown-linux-gnu.tar.gz"),
            Some("4f06310e0e2b3f105af8384c476888251593562b4b15a56b90ca5c79dc04eb43".to_string())
        );
        assert_eq!(expected_sha256(sums, "nope.tar.gz"), None);
    }

    #[test]
    fn sha256_known_vector() {
        let dir = std::env::temp_dir().join(format!("zentao-cli-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            sha256_file(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extracts_tar_gz() {
        let dir = std::env::temp_dir().join(format!("zentao-cli-tar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.tar.gz");
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(gz);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "./zentao-cli", &b"data"[..])
            .unwrap();
        builder.into_inner().unwrap().finish().unwrap();

        let target = AssetTarget {
            asset: "zentao-cli-x86_64-unknown-linux-gnu.tar.gz".into(),
            binary_name: "zentao-cli".into(),
            is_zip: false,
        };
        let out = extract(&archive, &target, &dir).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"data");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extracts_zip() {
        let dir = std::env::temp_dir().join(format!("zentao-cli-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("a.zip");
        let file = std::fs::File::create(&archive).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        zw.start_file("zentao-cli.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"pe-binary").unwrap();
        zw.finish().unwrap();

        let target = AssetTarget {
            asset: "zentao-cli-x86_64-pc-windows-msvc.zip".into(),
            binary_name: "zentao-cli.exe".into(),
            is_zip: true,
        };
        let out = extract(&archive, &target, &dir).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"pe-binary");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_target_file() {
        let dir = std::env::temp_dir().join(format!("zentao-cli-rep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let new_bin = dir.join("new-bin");
        let current = dir.join("zentao-cli");
        std::fs::write(&new_bin, b"new-version").unwrap();
        std::fs::write(&current, b"old-version").unwrap();
        replace_file(&new_bin, &current).unwrap();
        assert_eq!(std::fs::read(&current).unwrap(), b"new-version");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
