use chrono::{Duration, TimeZone, Utc};
use tempfile::tempdir;
use tsr_collector::{
    image_retention::{ImageRetentionPolicy, cleanup_expired_images},
    models::{HighResScreenshotMeta, ScreenshotMeta},
    storage::Store,
};

fn screenshot_meta(id: i64, captured_at: chrono::DateTime<Utc>, file_path: &str) -> ScreenshotMeta {
    ScreenshotMeta {
        id,
        captured_at,
        file_path: file_path.to_string(),
        width: 960,
        height: 540,
        process_name: Some("Code.exe".to_string()),
        window_title: Some("Time State Recorder".to_string()),
        capture_status: "ok".to_string(),
    }
}

fn high_res_meta(
    id: i64,
    captured_at: chrono::DateTime<Utc>,
    file_path: &str,
) -> HighResScreenshotMeta {
    HighResScreenshotMeta {
        id,
        captured_at,
        file_path: file_path.to_string(),
        width: 1600,
        height: 900,
        process_name: Some("Code.exe".to_string()),
        window_title: Some("Time State Recorder".to_string()),
        capture_status: "ok".to_string(),
    }
}

#[test]
fn cleanup_expired_images_deletes_old_files_and_marks_metadata_expired() {
    let temp = tempdir().unwrap();
    let screenshot_dir = temp.path().join("screenshots");
    let high_res_dir = temp.path().join("high-res-screenshots");
    std::fs::create_dir_all(screenshot_dir.join("2026-05-03")).unwrap();
    std::fs::create_dir_all(high_res_dir.join("2026-05-03")).unwrap();
    std::fs::create_dir_all(screenshot_dir.join("2026-05-30")).unwrap();

    let old_thumbnail = "2026-05-03/08-00.jpg";
    let old_high_res = "2026-05-03/08-00-00.jpg";
    let recent_thumbnail = "2026-05-30/08-00.jpg";
    std::fs::write(screenshot_dir.join(old_thumbnail), b"old-thumbnail").unwrap();
    std::fs::write(high_res_dir.join(old_high_res), b"old-high-res").unwrap();
    std::fs::write(screenshot_dir.join(recent_thumbnail), b"recent-thumbnail").unwrap();

    let now = Utc.with_ymd_and_hms(2026, 6, 4, 0, 0, 0).unwrap();
    let old_at = now - Duration::days(32);
    let recent_at = now - Duration::days(5);

    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("test", "config").unwrap();
    store
        .insert_screenshot_with_file_size(
            &session_id,
            &screenshot_meta(0, old_at, old_thumbnail),
            13,
        )
        .unwrap();
    store
        .insert_high_res_screenshot_with_file_size(
            &session_id,
            &high_res_meta(0, old_at, old_high_res),
            12,
        )
        .unwrap();
    store
        .insert_screenshot_with_file_size(
            &session_id,
            &screenshot_meta(0, recent_at, recent_thumbnail),
            16,
        )
        .unwrap();

    let result = cleanup_expired_images(
        &mut store,
        &screenshot_dir,
        &high_res_dir,
        now,
        ImageRetentionPolicy { retention_days: 30 },
    )
    .unwrap();

    assert_eq!(result.deleted_files, 2);
    assert_eq!(result.deleted_bytes, 25);
    assert!(!screenshot_dir.join(old_thumbnail).exists());
    assert!(!high_res_dir.join(old_high_res).exists());
    assert!(screenshot_dir.join(recent_thumbnail).exists());

    let old_visible = store
        .list_screenshots_between(
            old_at - Duration::minutes(1),
            old_at + Duration::minutes(1),
            10,
        )
        .unwrap();
    assert!(old_visible.is_empty());

    let stats = store.get_db_stats_with_retention(30).unwrap();
    assert_eq!(stats.screenshots, 1);
    assert_eq!(stats.high_res_screenshots, 0);
    assert_eq!(stats.image_retention.expired_files, 2);
    assert_eq!(stats.image_retention.expired_bytes, 25);
    assert_eq!(stats.image_retention.active_files, 1);
    assert_eq!(stats.image_retention.active_bytes, 16);
    assert!(stats.image_retention.pending_google_drive_upload);
}
