//! 桌面模式事实源（Story 16.3，FR-48）：`desktop-mode.json` 读写 + keyring
//! 远程令牌存取。
//!
//! **模式文件在 DB 外**（先有鸡问题）：`app_settings` 表在 egosync.db 内，
//! 而引擎启动前就要知道模式（远程模式不打开 egosync.db）——
//! `app_data_dir/desktop-mode.json` 为最小事实源。
//!
//! fail-safe 语义（冻结款）：文件缺失/损坏/未知 mode 值 ⇒ 回退
//! `Local`（数据在本地——回退到有数据的一侧；远程态无本地数据兜底）。
//!
//! 令牌只入 keyring（键 `remote_instance_token`）——不落明文磁盘；
//! 复用既有 `services::secret_store` 自由函数（keyring 唯一直接引用点）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 模式文件名（app_data_dir 下）。
pub const MODE_FILE_NAME: &str = "desktop-mode.json";
/// 远程实例令牌的 keyring 键。
pub const REMOTE_TOKEN_KEY: &str = "remote_instance_token";

/// 模式文件内容（camelCase 序列化——与前端 boot config 同形）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopModeFile {
    /// `"local"` | `"remote"`（未知值按损坏处理 ⇒ local）。
    pub mode: String,
    /// 远程实例 base URL（http/https；local 态可保留作下次切换预填）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
}

/// 解析后的桌面模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMode {
    /// 本地模式（默认）：完整引擎装配。
    Local,
    /// 远程模式：桌面 = 远端实例客户端（本地引擎零装配零写入）。
    Remote,
}

impl DesktopMode {
    pub fn is_remote(self) -> bool {
        self == Self::Remote
    }
}

impl std::fmt::Display for DesktopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Local => "local",
            Self::Remote => "remote",
        })
    }
}

/// 读模式（fail-safe）：缺失/损坏/未知值/remote 缺 URL ⇒ [`DesktopMode::Local`]。
///
/// 引导热路径（每次进程启动调用两次量级）——错误只 warn 不上抛：模式
/// 源不可读时回退本地（有数据的一侧），不让桌面因模式文件拒启。
///
/// 裁决统一经 [`resolve_mode`]（[T2 修订]——与 `desktop_get_boot_config`
/// 同源，杜绝两层分歧）。
pub fn read_mode(app_data_dir: &Path) -> DesktopMode {
    match read_mode_file(app_data_dir) {
        Some(file) => resolve_mode(&file),
        None => DesktopMode::Local,
    }
}

/// 读侧统一裁决（[T2 修订]）：mode=remote 而 remoteUrl 缺失/空白 ⇒ 按
/// local 处理。
///
/// 触发面 = 手改/半写模式文件（写路径必验 URL）。若 Rust 只看 mode 字段
/// 走远程 builder（引擎零装配、仅壳命令），而前端回退 TauriTransport
/// （getAuthStatus 直通 authenticated:true ⇒ gate ready ⇒ 业务 invoke
/// 命中未注册命令）——两层分歧的砖死会话。`read_mode`（进程引导）与
/// `commands::desktop_mode::desktop_get_boot_config`（前端引导）同用本
/// 裁决，两层恒同源。
pub fn resolve_mode(file: &DesktopModeFile) -> DesktopMode {
    match parse_mode(&file.mode) {
        DesktopMode::Remote => {
            let has_url = file
                .remote_url
                .as_deref()
                .map(str::trim)
                .is_some_and(|u| !u.is_empty());
            if has_url {
                DesktopMode::Remote
            } else {
                tracing::warn!(
                    "desktop-mode.json 为 remote 但 remoteUrl 缺失/空白（读侧统一裁决：按 local 处理——fail-safe 有数据的一侧）"
                );
                DesktopMode::Local
            }
        }
        mode => mode,
    }
}

/// 读模式文件（缺失/损坏 ⇒ None）。
pub fn read_mode_file(app_data_dir: &Path) -> Option<DesktopModeFile> {
    let path = app_data_dir.join(MODE_FILE_NAME);
    let content = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<DesktopModeFile>(&content) {
        Ok(file) => Some(file),
        Err(e) => {
            tracing::warn!(
                "desktop-mode.json 损坏（回退本地模式 fail-safe）: {}",
                e
            );
            None
        }
    }
}

/// mode 字符串解析（未知值 ⇒ local——与文件损坏同 fail-safe 语义）。
pub fn parse_mode(mode: &str) -> DesktopMode {
    match mode {
        "remote" => DesktopMode::Remote,
        "local" => DesktopMode::Local,
        other => {
            tracing::warn!("desktop-mode.json 未知 mode 值（回退本地）: {}", other);
            DesktopMode::Local
        }
    }
}

/// 校验远程实例 URL：非空 + http/https scheme（切换守卫——拼装 Bearer
/// base URL 的前置条件）。
pub fn validate_remote_url(url: &str) -> Result<(), AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("远程实例地址不能为空".to_string()));
    }
    let has_scheme = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    if !has_scheme {
        return Err(AppError::ValidationError(
            "远程实例地址必须以 http:// 或 https:// 开头".to_string(),
        ));
    }
    Ok(())
}

/// 写模式文件（原子写：同目录 temp + rename——与 agent_config 写
/// opencode.json 同款范式；app_data_dir 不存在时自建（首装场景））。
pub fn write_mode_file(app_data_dir: &Path, file: &DesktopModeFile) -> Result<(), AppError> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| AppError::SidecarError(format!("创建应用数据目录失败: {}", e)))?;
    let path = app_data_dir.join(MODE_FILE_NAME);
    let content = serde_json::to_string_pretty(file)
        .map_err(|e| AppError::ValidationError(format!("模式文件序列化失败: {}", e)))?;
    let tmp_path = app_data_dir.join(format!("{}.tmp", MODE_FILE_NAME));
    std::fs::write(&tmp_path, content)
        .map_err(|e| AppError::SidecarError(format!("写入 desktop-mode.json.tmp 失败: {}", e)))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| AppError::SidecarError(format!("rename desktop-mode.json 失败: {}", e)))?;
    tracing::info!(mode = %file.mode, "桌面模式已持久化: {}", path.display());
    Ok(())
}

// ── keyring 令牌存取（复用既有自由函数——keyring 唯一直接引用点） ──

/// 保存请求校验与归一（[T12 修订] 纯函数——命令层校验分支的单测直测面）。
///
/// 返回 `(归一后的模式文件, 待写 keyring 的令牌)`：
/// - 未知 mode 值 ⇒ `ValidationError`（写路径与读路径 fail-safe 语义不同：
///   写错方向比读错方向危险——显式拒绝而非静默落 local；`parse_mode` 的
///   未知值回退 local 使「解析结果与请求不一致」成为未知值的判别式）；
/// - `remote`：URL 必填（trim 后非空）且须 http/https scheme；token 提供
///   即须非空（trim 后）⇒ 归一后交付 keyring 写入（keyring I/O 留在命令
///   层——本函数保持纯函数可直测）；
/// - `local`：URL 归一保留（trim、空 ⇒ None——下次切换预填）；token 不
///   触碰（切回本地不清除 keyring 令牌——用户自己的钥匙串，切回远程
///   免重录）。
pub fn validate_save_request(
    mode: &str,
    remote_url: Option<&str>,
    token: Option<&str>,
) -> Result<(DesktopModeFile, Option<String>), AppError> {
    let parsed = parse_mode(mode);
    if parsed.to_string() != mode {
        return Err(AppError::ValidationError(format!(
            "未知桌面模式: {}（仅支持 local / remote）",
            mode
        )));
    }

    let normalized_url = remote_url
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(|s| s.to_string());

    if parsed.is_remote() {
        let url = normalized_url.as_deref().ok_or_else(|| {
            AppError::ValidationError("切换到远程模式需要实例地址".to_string())
        })?;
        validate_remote_url(url)?;
        let token_to_save = match token.map(str::trim) {
            Some(trimmed) if !trimmed.is_empty() => Some(trimmed.to_string()),
            // 空白令牌显式拒绝（调用方传了令牌却不可用——静默丢弃会让
            // 用户误以为已保存）
            Some(_) => {
                return Err(AppError::ValidationError(
                    "远程实例令牌不能为空".to_string(),
                ))
            }
            None => None,
        };
        Ok((
            DesktopModeFile {
                mode: parsed.to_string(),
                remote_url: normalized_url,
            },
            token_to_save,
        ))
    } else {
        Ok((
            DesktopModeFile {
                mode: parsed.to_string(),
                remote_url: normalized_url,
            },
            None,
        ))
    }
}

/// 保存远程实例令牌到系统钥匙串（已存在则覆写）。
pub fn save_remote_token(token: &str) -> Result<(), AppError> {
    super::secret_store::save_secret(REMOTE_TOKEN_KEY, token)
}

/// 读取远程实例令牌（无记录 ⇒ None；keyring 不可用 ⇒ Err 如实上抛）。
pub fn load_remote_token() -> Result<Option<String>, AppError> {
    super::secret_store::load_secret(REMOTE_TOKEN_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 临时目录（TempDir keep 范式与 server 测试一致——本crate 用
    /// tempfile dev-dependency）。
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = tempfile::tempdir().expect("创建临时目录");
        dir.keep().join(tag)
    }

    #[test]
    fn missing_file_falls_back_to_local() {
        let dir = temp_dir("missing");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Local);
    }

    #[test]
    fn corrupt_file_falls_back_to_local() {
        let dir = temp_dir("corrupt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MODE_FILE_NAME), "{ not json !!!").unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Local, "损坏 ⇒ fail-safe 本地");
    }

    #[test]
    fn unknown_mode_value_falls_back_to_local() {
        assert_eq!(parse_mode("remote"), DesktopMode::Remote);
        assert_eq!(parse_mode("local"), DesktopMode::Local);
        assert_eq!(parse_mode("weird-mode"), DesktopMode::Local, "未知值 ⇒ 本地");
    }

    #[test]
    fn mode_file_roundtrip_local_and_remote() {
        let dir = temp_dir("roundtrip");
        std::fs::create_dir_all(&dir).unwrap();

        // local：缺 remoteUrl
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "local".to_string(),
                remote_url: None,
            },
        )
        .unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Local);
        assert_eq!(
            read_mode_file(&dir),
            Some(DesktopModeFile {
                mode: "local".to_string(),
                remote_url: None,
            })
        );

        // remote：携 remoteUrl
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "remote".to_string(),
                remote_url: Some("https://instance.example.com".to_string()),
            },
        )
        .unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Remote);
        let file = read_mode_file(&dir).expect("读回");
        assert_eq!(
            file.remote_url.as_deref(),
            Some("https://instance.example.com")
        );

        // 序列化形状：camelCase remoteUrl（与前端 boot config 同形契约）
        let raw = std::fs::read_to_string(dir.join(MODE_FILE_NAME)).unwrap();
        assert!(raw.contains("\"remoteUrl\""), "camelCase 字段名: {}", raw);
        assert!(raw.contains("\"mode\""));
    }

    #[test]
    fn local_mode_with_retained_remote_url_still_local() {
        // 切回本地保留 remoteUrl（预填）——mode 字段为唯一裁决源
        let dir = temp_dir("local-with-url");
        std::fs::create_dir_all(&dir).unwrap();
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "local".to_string(),
                remote_url: Some("https://instance.example.com".to_string()),
            },
        )
        .unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Local);
    }

    #[test]
    fn write_creates_parent_dir_if_missing() {
        // app_data_dir 尚不存在（首装场景）：写入须自建目录
        let dir = temp_dir("no-dir").join("nested");
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "remote".to_string(),
                remote_url: Some("https://x.example.com".to_string()),
            },
        )
        .unwrap();
        assert_eq!(read_mode(&dir), DesktopMode::Remote);
    }

    #[test]
    fn remote_url_validation() {
        assert!(validate_remote_url("https://instance.example.com").is_ok());
        assert!(validate_remote_url("http://192.168.1.10:8080").is_ok());
        assert!(validate_remote_url("").is_err());
        assert!(validate_remote_url("   ").is_err());
        assert!(validate_remote_url("instance.example.com").is_err(), "无 scheme 拒绝");
        assert!(validate_remote_url("ftp://x").is_err(), "非 http(s) scheme 拒绝");
    }

    // ── [T2 修订] 读侧统一裁决：remote 缺 URL ⇒ local（两层同源） ──

    #[test]
    fn remote_mode_without_url_falls_back_to_local() {
        let dir = temp_dir("remote-no-url");
        std::fs::create_dir_all(&dir).unwrap();
        // 手改/半写模式文件形态：mode=remote 而 remoteUrl 缺失
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "remote".to_string(),
                remote_url: None,
            },
        )
        .unwrap();
        assert_eq!(
            read_mode(&dir),
            DesktopMode::Local,
            "remote 缺 URL ⇒ local（杜绝 Rust 远程 builder / 前端 TauriTransport 直通的两层分歧）"
        );
    }

    #[test]
    fn remote_mode_with_blank_url_falls_back_to_local() {
        let dir = temp_dir("remote-blank-url");
        std::fs::create_dir_all(&dir).unwrap();
        write_mode_file(
            &dir,
            &DesktopModeFile {
                mode: "remote".to_string(),
                remote_url: Some("   ".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            read_mode(&dir),
            DesktopMode::Local,
            "remote 空白 URL ⇒ local（与缺失同语义）"
        );
    }

    #[test]
    fn resolve_mode_adjudicates_remote_with_url_as_remote() {
        let file = DesktopModeFile {
            mode: "remote".to_string(),
            remote_url: Some("https://instance.example.com".to_string()),
        };
        assert_eq!(resolve_mode(&file), DesktopMode::Remote);
    }

    // ── [T12 修订] 命令层校验纯函数直测（原为零测试的校验分支） ──

    #[test]
    fn validate_save_request_rejects_unknown_mode() {
        let err = validate_save_request("weird-mode", Some("https://x.example.com"), None)
            .expect_err("未知 mode 必须拒绝（写路径不 fail-safe）");
        assert!(matches!(err, AppError::ValidationError(_)));
        // 大小写敏感：Remote ≠ remote
        assert!(validate_save_request("Remote", None, None).is_err());
    }

    #[test]
    fn validate_save_request_remote_requires_valid_url() {
        // 缺 URL 拒绝
        assert!(validate_save_request("remote", None, Some("tk")).is_err());
        // 空白 URL 拒绝
        assert!(validate_save_request("remote", Some("  "), Some("tk")).is_err());
        // 非 http(s) scheme 拒绝
        assert!(validate_save_request("remote", Some("ftp://x"), Some("tk")).is_err());
        // 合法 URL + 令牌 ⇒ 归一（trim）+ 待写 keyring 令牌
        let (file, token) =
            validate_save_request("remote", Some("  https://x.example.com/  "), Some("  tk  "))
                .expect("合法 remote 请求");
        assert_eq!(file.mode, "remote");
        assert_eq!(file.remote_url.as_deref(), Some("https://x.example.com/"));
        assert_eq!(token.as_deref(), Some("tk"));
    }

    #[test]
    fn validate_save_request_remote_rejects_empty_token() {
        // 提供了令牌但为空白 ⇒ 拒绝（不静默丢弃——用户会误以为已保存）
        let err = validate_save_request("remote", Some("https://x.example.com"), Some("   "))
            .expect_err("空白令牌必须拒绝");
        assert!(matches!(err, AppError::ValidationError(_)));
        // 未提供令牌 ⇒ 放行（切换前的令牌校验归前端测试连接守卫）
        let (file, token) =
            validate_save_request("remote", Some("https://x.example.com"), None).unwrap();
        assert_eq!(file.mode, "remote");
        assert_eq!(token, None);
    }

    #[test]
    fn validate_save_request_local_preserves_url_and_skips_token() {
        // local 保留 URL（下次切换预填——归一 trim、空 ⇒ None）
        let (file, token) =
            validate_save_request("local", Some("  https://saved.example.com  "), None).unwrap();
        assert_eq!(file.mode, "local");
        assert_eq!(
            file.remote_url.as_deref(),
            Some("https://saved.example.com"),
            "local 保留 URL 作预填"
        );
        assert_eq!(token, None, "local 不触碰 keyring 令牌（保留不清除）");
        // 空白 URL ⇒ None
        let (file, _) = validate_save_request("local", Some("  "), None).unwrap();
        assert_eq!(file.remote_url, None);
    }
}
