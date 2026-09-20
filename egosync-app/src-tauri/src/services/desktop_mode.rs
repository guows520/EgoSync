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

/// 读模式（fail-safe）：缺失/损坏/未知值 ⇒ [`DesktopMode::Local`]。
///
/// 引导热路径（每次进程启动调用两次量级）——错误只 warn 不上抛：模式
/// 源不可读时回退本地（有数据的一侧），不让桌面因模式文件拒启。
pub fn read_mode(app_data_dir: &Path) -> DesktopMode {
    match read_mode_file(app_data_dir) {
        Some(file) => parse_mode(&file.mode),
        None => DesktopMode::Local,
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
}
