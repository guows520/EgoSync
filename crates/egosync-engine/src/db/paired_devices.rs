//! `paired_devices` 表读写（Story 12.2）。
//!
//! 单对单语义（NFR-M6）：桌面同一时刻只信任一台手机，故
//! [`upsert_single_device`] 在写入新记录前先清空旧记录。

use sqlx::SqlitePool;

use crate::error::AppError;
use crate::models::companion::PairedDevice;

pub async fn upsert_single_device(
    pool: &SqlitePool,
    device: &PairedDevice,
) -> Result<(), AppError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::DbError(format!("开启配对设备事务失败: {}", e)))?;
    // 单对单：新记录写入前清空旧记录（同公钥 upsert 时等价于更新自身）
    sqlx::query("DELETE FROM paired_devices WHERE device_pubkey != ?1")
        .bind(&device.device_pubkey)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DbError(format!("清理旧配对设备失败: {}", e)))?;
    sqlx::query(
        "INSERT INTO paired_devices (id, device_name, device_pubkey, paired_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(device_pubkey) DO UPDATE SET
            device_name = excluded.device_name,
            paired_at = excluded.paired_at,
            last_seen_at = excluded.last_seen_at",
    )
    .bind(&device.id)
    .bind(&device.device_name)
    .bind(&device.device_pubkey)
    .bind(&device.paired_at)
    .bind(&device.last_seen_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::DbError(format!("写入配对设备失败: {}", e)))?;
    tx.commit()
        .await
        .map_err(|e| AppError::DbError(format!("提交配对设备事务失败: {}", e)))?;
    Ok(())
}

pub async fn get_all(pool: &SqlitePool) -> Result<Vec<PairedDevice>, AppError> {
    sqlx::query_as::<_, PairedDevice>(
        "SELECT id, device_name, device_pubkey, paired_at, last_seen_at
         FROM paired_devices ORDER BY paired_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::DbError(format!("查询配对设备失败: {}", e)))
}

pub async fn get_by_pubkey(
    pool: &SqlitePool,
    pubkey: &str,
) -> Result<Option<PairedDevice>, AppError> {
    sqlx::query_as::<_, PairedDevice>(
        "SELECT id, device_name, device_pubkey, paired_at, last_seen_at
         FROM paired_devices WHERE device_pubkey = ?1",
    )
    .bind(pubkey)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::DbError(format!("按公钥查询配对设备失败: {}", e)))
}

pub async fn update_last_seen(pool: &SqlitePool, pubkey: &str) -> Result<(), AppError> {
    let now = crate::db::settings::chrono_now_pub();
    sqlx::query("UPDATE paired_devices SET last_seen_at = ?1 WHERE device_pubkey = ?2")
        .bind(&now)
        .bind(pubkey)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("更新配对设备最后在线时间失败: {}", e)))?;
    Ok(())
}

pub async fn remove(pool: &SqlitePool, device_id: &str) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM paired_devices WHERE id = ?1")
        .bind(device_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("删除配对设备失败: {}", e)))?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!(
            "配对设备不存在: {}",
            device_id
        )));
    }
    Ok(())
}

/// 清空全部配对设备（数据销毁路径由 MAIN_DB_TABLES 覆盖，此函数供 remove_all 语义完整性使用）。
pub async fn remove_all(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("DELETE FROM paired_devices")
        .execute(pool)
        .await
        .map_err(|e| AppError::DbError(format!("清空配对设备失败: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pool::init_db;
    use tempfile::tempdir;

    async fn test_pool() -> SqlitePool {
        let dir = tempdir().expect("create temp dir");
        // into_path() 放弃自动清理，目录存活到进程结束——池的生命周期长于 TempDir guard
        let db_dir = dir.into_path();
        init_db(&db_dir.join("egosync.db"))
            .await
            .expect("init db with migrations")
    }

    fn device(id: &str, name: &str, pubkey: &str) -> PairedDevice {
        PairedDevice {
            id: id.to_string(),
            device_name: name.to_string(),
            device_pubkey: pubkey.to_string(),
            paired_at: "2026-08-28T10:00:00Z".to_string(),
            last_seen_at: "2026-08-28T10:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn upsert_single_device_replaces_previous_device() {
        // WHY: 单对单（NFR-M6）——若第二条记录静默追加，第三台设备可在用户
        // 不知情时与桌面并列持有信任，"唯一事实源"的配对边界即被打破。
        let pool = test_pool().await;
        upsert_single_device(&pool, &device("d1", "旧手机", "pub-old"))
            .await
            .expect("first upsert");
        upsert_single_device(&pool, &device("d2", "新手机", "pub-new"))
            .await
            .expect("second upsert");

        let all = get_all(&pool).await.expect("get all");
        assert_eq!(all.len(), 1, "新设备写入后必须只剩一条记录");
        assert_eq!(all[0].device_pubkey, "pub-new");
    }

    #[tokio::test]
    async fn upsert_same_pubkey_updates_in_place() {
        // WHY: 免配对重连写 last_seen/改名时不能产生第二条记录——
        // 同公钥必须是更新而非插入（UNIQUE 约束之上的语义保证）。
        let pool = test_pool().await;
        upsert_single_device(&pool, &device("d1", "手机", "pub-a"))
            .await
            .expect("first upsert");
        upsert_single_device(&pool, &device("d1", "改名手机", "pub-a"))
            .await
            .expect("same-pubkey upsert");

        let all = get_all(&pool).await.expect("get all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].device_name, "改名手机");
    }

    #[tokio::test]
    async fn get_by_pubkey_and_update_last_seen_roundtrip() {
        let pool = test_pool().await;
        upsert_single_device(&pool, &device("d1", "手机", "pub-a"))
            .await
            .expect("upsert");

        let found = get_by_pubkey(&pool, "pub-a").await.expect("get by pubkey");
        assert!(found.is_some(), "已写入公钥必须可查");
        assert_eq!(found.unwrap().id, "d1");

        update_last_seen(&pool, "pub-a").await.expect("update last seen");
        let updated = get_by_pubkey(&pool, "pub-a")
            .await
            .expect("get after update")
            .expect("record exists");
        assert_ne!(
            updated.last_seen_at, "2026-08-28T10:00:00Z",
            "update_last_seen 必须实际刷新 last_seen_at"
        );

        let missing = get_by_pubkey(&pool, "pub-none").await.expect("query missing");
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn remove_deletes_record_and_errors_on_missing() {
        // WHY: "移除即拒绝"（用户主权）依赖 remove 真正删掉记录；
        // 删除不存在的设备必须显式报错而非静默成功（显式失败原则）。
        let pool = test_pool().await;
        upsert_single_device(&pool, &device("d1", "手机", "pub-a"))
            .await
            .expect("upsert");

        remove(&pool, "d1").await.expect("remove existing");
        assert!(get_all(&pool).await.expect("get all").is_empty());

        let err = remove(&pool, "d1").await.expect_err("remove missing must fail");
        assert!(matches!(err, AppError::NotFound(_)), "实际: {err:?}");
    }

    #[tokio::test]
    async fn remove_all_clears_everything() {
        let pool = test_pool().await;
        upsert_single_device(&pool, &device("d1", "手机", "pub-a"))
            .await
            .expect("upsert");
        remove_all(&pool).await.expect("remove all");
        assert!(get_all(&pool).await.expect("get all").is_empty());
    }
}
