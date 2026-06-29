use tauri::State;

use crate::db::app_settings;
use crate::db::pool::DbPool;
use crate::error::AppError;

const DEFAULT_BRIEFING_TIME: &str = "08:00";
pub const DEFAULT_REVIEW_DAY: &str = "7";
pub const DEFAULT_REVIEW_TIME: &str = "20:00";
pub const DEFAULT_BIGROCK_REMINDER_DAY: &str = "1";
pub const DEFAULT_BIGROCK_REMINDER_TIME: &str = "09:00";

const KEY_BRIEFING_TIME: &str = "briefing_time";
pub const KEY_REVIEW_DAY: &str = "review_day";
pub const KEY_REVIEW_TIME: &str = "review_time";
pub const KEY_BIGROCK_REMINDER_DAY: &str = "bigrock_reminder_day";
pub const KEY_BIGROCK_REMINDER_TIME: &str = "bigrock_reminder_time";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleConfig {
    pub briefing_time: String,
    pub review_day: String,
    pub review_time: String,
    pub bigrock_reminder_day: String,
    pub bigrock_reminder_time: String,
}

fn validate_time(value: &str, field_name: &str) -> Result<(), AppError> {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() || !bytes[3].is_ascii_digit() || !bytes[4].is_ascii_digit() {
        return Err(AppError::ValidationError(format!(
            "{} 格式无效，应为 HH:MM",
            field_name
        )));
    }
    let hour: u32 = (bytes[0] - b'0') as u32 * 10 + (bytes[1] - b'0') as u32;
    let minute: u32 = (bytes[3] - b'0') as u32 * 10 + (bytes[4] - b'0') as u32;
    if hour > 23 || minute > 59 {
        return Err(AppError::ValidationError(format!(
            "{} 超出有效范围 00:00~23:59",
            field_name
        )));
    }
    Ok(())
}

fn validate_review_day(value: &str) -> Result<(), AppError> {
    match value {
        "1" | "2" | "3" | "4" | "5" | "6" | "7" => Ok(()),
        _ => Err(AppError::ValidationError(
            "周复盘星期无效，应为 1~7".to_string(),
        )),
    }
}

fn validate_bigrock_reminder_day(value: &str) -> Result<(), AppError> {
    match value {
        "1" | "2" => Ok(()),
        _ => Err(AppError::ValidationError(
            "大石头规划提醒星期无效，仅可选周一(1)或周二(2)".to_string(),
        )),
    }
}

async fn read_schedule_config(pool: &DbPool) -> Result<ScheduleConfig, AppError> {
    let briefing_time = app_settings::get_setting(pool, KEY_BRIEFING_TIME)
        .await?
        .unwrap_or_else(|| DEFAULT_BRIEFING_TIME.to_string());
    let review_day = app_settings::get_setting(pool, KEY_REVIEW_DAY)
        .await?
        .unwrap_or_else(|| DEFAULT_REVIEW_DAY.to_string());
    let review_time = app_settings::get_setting(pool, KEY_REVIEW_TIME)
        .await?
        .unwrap_or_else(|| DEFAULT_REVIEW_TIME.to_string());
    let bigrock_reminder_day = app_settings::get_setting(pool, KEY_BIGROCK_REMINDER_DAY)
        .await?
        .unwrap_or_else(|| DEFAULT_BIGROCK_REMINDER_DAY.to_string());
    let bigrock_reminder_time = app_settings::get_setting(pool, KEY_BIGROCK_REMINDER_TIME)
        .await?
        .unwrap_or_else(|| DEFAULT_BIGROCK_REMINDER_TIME.to_string());

    Ok(ScheduleConfig {
        briefing_time,
        review_day,
        review_time,
        bigrock_reminder_day,
        bigrock_reminder_time,
    })
}

#[tauri::command]
pub async fn settings_get_schedule(pool: State<'_, DbPool>) -> Result<ScheduleConfig, AppError> {
    read_schedule_config(&pool).await
}

#[tauri::command]
pub async fn settings_update_schedule(
    briefing_time: Option<String>,
    review_day: Option<String>,
    review_time: Option<String>,
    bigrock_reminder_day: Option<String>,
    bigrock_reminder_time: Option<String>,
    pool: State<'_, DbPool>,
) -> Result<ScheduleConfig, AppError> {
    // 先全量校验所有传入字段，避免部分写入后才发现非法值导致数据不一致
    if let Some(ref v) = briefing_time {
        validate_time(v, "晨间简报时间")?;
    }
    if let Some(ref v) = review_day {
        validate_review_day(v)?;
    }
    if let Some(ref v) = review_time {
        validate_time(v, "周复盘时间")?;
    }
    if let Some(ref v) = bigrock_reminder_day {
        validate_bigrock_reminder_day(v)?;
    }
    if let Some(ref v) = bigrock_reminder_time {
        validate_time(v, "大石头规划提醒时间")?;
    }

    // 校验全部通过后再统一写入
    if let Some(ref v) = briefing_time {
        app_settings::set_setting(&pool, KEY_BRIEFING_TIME, v).await?;
    }
    if let Some(ref v) = review_day {
        app_settings::set_setting(&pool, KEY_REVIEW_DAY, v).await?;
    }
    if let Some(ref v) = review_time {
        app_settings::set_setting(&pool, KEY_REVIEW_TIME, v).await?;
    }
    if let Some(ref v) = bigrock_reminder_day {
        app_settings::set_setting(&pool, KEY_BIGROCK_REMINDER_DAY, v).await?;
    }
    if let Some(ref v) = bigrock_reminder_time {
        app_settings::set_setting(&pool, KEY_BIGROCK_REMINDER_TIME, v).await?;
    }

    read_schedule_config(&pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> DbPool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create test db");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT,
                updated_at TEXT NOT NULL DEFAULT '2026-01-01T00:00:00Z'
            )",
        )
        .execute(&pool)
        .await
        .expect("failed to create table");

        pool
    }

    #[tokio::test]
    async fn test_get_schedule_returns_defaults() {
        let pool = setup_test_db().await;
        let config = read_schedule_config(&pool).await.unwrap();
        assert_eq!(config.briefing_time, "08:00");
        assert_eq!(config.review_day, "7");
        assert_eq!(config.review_time, "20:00");
        assert_eq!(config.bigrock_reminder_day, "1");
        assert_eq!(config.bigrock_reminder_time, "09:00");
    }

    #[tokio::test]
    async fn test_update_partial_only_review_day() {
        let pool = setup_test_db().await;
        app_settings::set_setting(&pool, KEY_REVIEW_DAY, "3")
            .await
            .unwrap();
        let config = read_schedule_config(&pool).await.unwrap();
        assert_eq!(config.review_day, "3");
        assert_eq!(config.briefing_time, "08:00");
        assert_eq!(config.review_time, "20:00");
        assert_eq!(config.bigrock_reminder_day, "1");
        assert_eq!(config.bigrock_reminder_time, "09:00");
    }

    #[tokio::test]
    async fn test_update_all_fields() {
        let pool = setup_test_db().await;
        app_settings::set_setting(&pool, KEY_BRIEFING_TIME, "07:30")
            .await
            .unwrap();
        app_settings::set_setting(&pool, KEY_REVIEW_DAY, "1")
            .await
            .unwrap();
        app_settings::set_setting(&pool, KEY_REVIEW_TIME, "19:00")
            .await
            .unwrap();
        app_settings::set_setting(&pool, KEY_BIGROCK_REMINDER_DAY, "2")
            .await
            .unwrap();
        app_settings::set_setting(&pool, KEY_BIGROCK_REMINDER_TIME, "10:00")
            .await
            .unwrap();
        let config = read_schedule_config(&pool).await.unwrap();
        assert_eq!(config.briefing_time, "07:30");
        assert_eq!(config.review_day, "1");
        assert_eq!(config.review_time, "19:00");
        assert_eq!(config.bigrock_reminder_day, "2");
        assert_eq!(config.bigrock_reminder_time, "10:00");
    }

    #[test]
    fn test_validate_review_day_invalid() {
        assert!(validate_review_day("8").is_err());
        assert!(validate_review_day("0").is_err());
        assert!(validate_review_day("abc").is_err());
    }

    #[test]
    fn test_validate_review_day_valid() {
        for d in 1..=7 {
            assert!(validate_review_day(&d.to_string()).is_ok());
        }
    }

    #[test]
    fn test_validate_bigrock_reminder_day_invalid() {
        assert!(validate_bigrock_reminder_day("3").is_err());
        assert!(validate_bigrock_reminder_day("0").is_err());
        assert!(validate_bigrock_reminder_day("7").is_err());
    }

    #[test]
    fn test_validate_bigrock_reminder_day_valid() {
        assert!(validate_bigrock_reminder_day("1").is_ok());
        assert!(validate_bigrock_reminder_day("2").is_ok());
    }

    #[test]
    fn test_validate_time_invalid_format() {
        assert!(validate_time("25:00", "test").is_err());
        assert!(validate_time("12:60", "test").is_err());
        assert!(validate_time("abc", "test").is_err());
        assert!(validate_time("8:00", "test").is_err());
        assert!(validate_time("1200", "test").is_err());
    }

    #[test]
    fn test_validate_time_valid() {
        assert!(validate_time("00:00", "test").is_ok());
        assert!(validate_time("23:59", "test").is_ok());
        assert!(validate_time("08:30", "test").is_ok());
    }
}
