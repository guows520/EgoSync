//! ServerSecretStore 单元测试（评审修复 #12：文件优先/env 兜底此前零
//! 覆盖，api_test.rs 头注释曾虚报——此处补齐行为契约）。
//!
//! 四断言：①文件命中遮蔽 env ②文件 miss 回退 env 且 ref 段大小写
//! 原样拼接 ③delete 后 env 值重新可见 ④secrets.json 落盘 0600 权限。
//!
//! env 操作用 `EGOSYNC_SECRET_TEST_FF_*` 独立键名——并行测试不互踩
//!（std::env 是进程级全局，共享键名会竞态）。

use egosync_engine::services::secret_store::SecretStore;

use egosync_server::secret_store::ServerSecretStore;

/// 独立键（本测试文件专属前缀，防并行互踩）。
const FILE_SHADOW_ENV_KEY: &str = "TEST_FF_FILE_SHADOW";
const ENV_FALLBACK_KEY: &str = "TEST_FF_ENV_FALLBACK";
const ENV_CASE_KEY: &str = "TEST_FF_CaseSense";
const DELETE_REVEALS_ENV_KEY: &str = "TEST_FF_DELETE_REVEAL";

fn temp_store(tag: &str) -> (ServerSecretStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let data_dir = dir.path().join(tag);
    std::fs::create_dir_all(&data_dir).expect("创建数据子目录");
    let store = ServerSecretStore::new(data_dir);
    (store, dir)
}

#[test]
fn file_value_shadows_env_value() {
    let (store, _dir) = temp_store("shadow");
    let key = format!("EGOSYNC_SECRET_{}", FILE_SHADOW_ENV_KEY);
    // 独立 env 键——测试结束后还原（不影响其他测试）
    let prev = std::env::var(&key).ok();
    std::env::set_var(&key, "env-value-must-be-shadowed");
    // 文件写入后必须遮蔽 env
    store
        .save_secret(FILE_SHADOW_ENV_KEY, "file-value-wins")
        .expect("文件写入");
    let loaded = store
        .load_secret(FILE_SHADOW_ENV_KEY)
        .expect("读取（文件优先）");
    assert_eq!(
        loaded.as_deref(),
        Some("file-value-wins"),
        "文件命中必须遮蔽 env 值"
    );
    // 清理：delete 后 env 兜底重新生效（由测试 3 专测，此处只还原 env）
    store.delete_secret(FILE_SHADOW_ENV_KEY).expect("清理");
    match prev {
        Some(v) => std::env::set_var(&key, v),
        None => std::env::remove_var(&key),
    }
}

#[test]
fn env_fallback_with_exact_case_concatenation() {
    let (store, _dir) = temp_store("fallback");
    // 无文件值 ⇒ 回退 env；ref 段大小写原样拼接（零转换）
    let env_name = format!("EGOSYNC_SECRET_{}", ENV_FALLBACK_KEY);
    let prev = std::env::var(&env_name).ok();
    std::env::set_var(&env_name, "env-fallback-value");
    let loaded = store
        .load_secret(ENV_FALLBACK_KEY)
        .expect("读取（env 兜底）");
    assert_eq!(loaded.as_deref(), Some("env-fallback-value"));
    match prev {
        Some(v) => std::env::set_var(&env_name, v),
        None => std::env::remove_var(&env_name),
    }

    // 大小写原样：ref 含大写字母，env 变量名同样带大写（区分大小写拼接）
    let env_case_name = format!("EGOSYNC_SECRET_{}", ENV_CASE_KEY);
    let prev_case = std::env::var(&env_case_name).ok();
    std::env::set_var(&env_case_name, "case-exact-value");
    // 文件中无该键（store 为新目录）⇒ 走 env 兜底
    let loaded = store.load_secret(ENV_CASE_KEY).expect("读取（大小写原样）");
    assert_eq!(
        loaded.as_deref(),
        Some("case-exact-value"),
        "ref 段大小写原样拼接：EGOSYNC_SECRET_{} 必须命中", ENV_CASE_KEY
    );
    match prev_case {
        Some(v) => std::env::set_var(&env_case_name, v),
        None => std::env::remove_var(&env_case_name),
    }
}

#[test]
fn delete_reveals_env_value_again() {
    let (store, _dir) = temp_store("reveal");
    let env_name = format!("EGOSYNC_SECRET_{}", DELETE_REVEALS_ENV_KEY);
    let prev = std::env::var(&env_name).ok();
    std::env::set_var(&env_name, "env-revealed-after-delete");

    // 先文件写入（遮蔽 env），再删除——env 兜底应重新可见
    store
        .save_secret(DELETE_REVEALS_ENV_KEY, "file-transient")
        .expect("文件写入");
    store.delete_secret(DELETE_REVEALS_ENV_KEY).expect("文件删除");
    let loaded = store
        .load_secret(DELETE_REVEALS_ENV_KEY)
        .expect("读取（delete 后 env 重新可见）");
    assert_eq!(
        loaded.as_deref(),
        Some("env-revealed-after-delete"),
        "delete 只删文件面——env 引导通道必须重新兜底"
    );
    match prev {
        Some(v) => std::env::set_var(&env_name, v),
        None => std::env::remove_var(&env_name),
    }
}

#[test]
fn secrets_file_is_owner_only_0600() {
    let (store, dir) = temp_store("perms");
    store
        .save_secret("any-key", "any-value")
        .expect("写入");
    let path = dir.path().join("perms").join("secrets.json");
    let meta = std::fs::metadata(&path).expect("secrets.json 必须存在");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            meta.permissions().mode() & 0o777,
            0o600,
            "secrets.json 必须为 0600（仅属主读写）"
        );
    }
    #[cfg(not(unix))]
    let _ = meta;
}
