//! 服务端 SecretStore 适配器（Story 15.4，OQ1 裁决 A——架构 ④ 完整落地）。
//!
//! 双通道、**文件优先、env 兜底**：
//! - 数据目录 `secrets.json`（0600 明文）：运行时主存储（唯一可写
//!   事实源——save/delete 永远落此文件）；
//! - env `EGOSYNC_SECRET_{api_key_ref}`：原样区分大小写拼接（ref 段零
//!   大小写转换）。**对全部 key 生效**——包括 UUID 形态的
//!   `llm_{uuid}_api_key`（epic AC 的 LLM Key 引导示例即 UUID 形态，
//!   云端首启可 `EGOSYNC_SECRET_llm_<uuid>_api_key=sk-...` 注入后再
//!   前端重录持久化），不只限固定名 secret。env 不构成运行时遮蔽源
//!   ——文件存在该键时 env 值不生效（防 env 静默遮蔽文件值）。
//!
//! 写入（save/delete）永远落 secrets.json 0600；禁止 InMemory 占位
//! （静默丢 Key 的已知坏状态）。并发写经进程内互斥锁串行化。
//! 行为由 `tests/secret_store_test.rs` 四断言钉死（读序 / env 键
//! 区分大小写 / 0600 / 原子写），本模块行为面 Story 17.1 零改动。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use egosync_engine::error::AppError;
use egosync_engine::services::secret_store::SecretStore;

/// secrets.json 文件名（数据目录下）。
const SECRETS_FILE: &str = "secrets.json";

/// 服务端密钥存储：secrets.json 文件 + env 兜底。
pub struct ServerSecretStore {
    data_dir: PathBuf,
    /// 读改写串行化（进程内单实例；跨进程写并发由 0600 权限与部署形态约束）。
    lock: Mutex<()>,
}

impl ServerSecretStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            lock: Mutex::new(()),
        }
    }

    fn secrets_path(&self) -> PathBuf {
        self.data_dir.join(SECRETS_FILE)
    }

    /// 读取 secrets.json 全量键值（文件不存在 ⇒ 空表；损坏 ⇒ 显式报错）。
    fn read_file(&self) -> Result<BTreeMap<String, String>, AppError> {
        let path = self.secrets_path();
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| {
            AppError::KeyringError(format!("读取 secrets.json 失败: {}", e))
        })?;
        serde_json::from_str(&text)
            .map_err(|e| AppError::KeyringError(format!("secrets.json 解析失败: {}", e)))
    }

    /// 全量覆写 secrets.json（0600；临时文件 + rename 原子落盘）。
    fn write_file(&self, map: &BTreeMap<String, String>) -> Result<(), AppError> {
        let path = self.secrets_path();
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(map)
            .map_err(|e| AppError::KeyringError(format!("secrets.json 序列化失败: {}", e)))?;
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp)
                .map_err(|e| AppError::KeyringError(format!("创建 secrets.json 失败: {}", e)))?;
            set_owner_only_permissions(&file)?;
            file.write_all(text.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|e| AppError::KeyringError(format!("写入 secrets.json 失败: {}", e)))?;
        }
        std::fs::rename(&tmp, &path)
            .map_err(|e| AppError::KeyringError(format!("secrets.json 落盘失败: {}", e)))
    }
}

/// 0600 归一（仅属主可读写）；非 Unix 平台为 no-op（Windows ACL 语义另计）。
fn set_owner_only_permissions(file: &std::fs::File) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file
            .metadata()
            .map_err(|e| AppError::KeyringError(format!("读取 secrets.json 元数据失败: {}", e)))?
            .permissions();
        perms.set_mode(0o600);
        file.set_permissions(perms)
            .map_err(|e| AppError::KeyringError(format!("设置 secrets.json 权限失败: {}", e)))?;
    }
    #[cfg(not(unix))]
    let _ = file;
    Ok(())
}

impl SecretStore for ServerSecretStore {
    fn save_secret(&self, key: &str, value: &str) -> Result<(), AppError> {
        let _guard = self.lock.lock().expect("secrets.json 写锁中毒");
        let mut map = self.read_file()?;
        map.insert(key.to_string(), value.to_string());
        self.write_file(&map)
    }

    fn load_secret(&self, key: &str) -> Result<Option<String>, AppError> {
        let _guard = self.lock.lock().expect("secrets.json 写锁中毒");
        // 文件优先
        if let Some(value) = self.read_file()?.get(key) {
            return Ok(Some(value.clone()));
        }
        // env 兜底：EGOSYNC_SECRET_{key} 原样区分大小写拼接
        Ok(std::env::var(format!("EGOSYNC_SECRET_{}", key)).ok())
    }

    fn delete_secret(&self, key: &str) -> Result<(), AppError> {
        let _guard = self.lock.lock().expect("secrets.json 写锁中毒");
        let mut map = self.read_file()?;
        map.remove(key);
        self.write_file(&map)
    }
}
