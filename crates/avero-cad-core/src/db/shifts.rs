//! Duty shift tracking.
//!
//! A shift is one row: `ended_at IS NULL` means the staff member is currently
//! on duty. Durations are computed by SQLite from the stored timestamps, so no
//! time is invented and an open shift keeps counting in real time.

use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::DutyShiftView;
use crate::util::{clamp_text, now_ms};
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};

const SELECT_SHIFT: &str = "SELECT id, user_id, display_name, started_at, ended_at, end_reason
                            FROM duty_shifts";

fn map_shift(row: &rusqlite::Row<'_>, now: i64) -> rusqlite::Result<DutyShiftView> {
    let started_at: i64 = row.get("started_at")?;
    let ended_at: Option<i64> = row.get("ended_at")?;
    Ok(DutyShiftView {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        display_name: row.get("display_name")?,
        started_at,
        ended_at,
        duration_ms: ended_at.unwrap_or(now).saturating_sub(started_at),
        on_duty: ended_at.is_none(),
        end_reason: row.get("end_reason")?,
    })
}

fn start_of_local_day(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight is a valid time");
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|value| value.timestamp_millis())
        .unwrap_or_else(|| Local::now().timestamp_millis())
}

pub fn local_day_start(now: i64) -> i64 {
    let date = Local
        .timestamp_millis_opt(now)
        .single()
        .map(|value| value.date_naive())
        .unwrap_or_else(|| Local::now().date_naive());
    start_of_local_day(date)
}

pub fn local_week_start(now: i64, monday_offset_days: i64) -> i64 {
    let date = Local
        .timestamp_millis_opt(now)
        .single()
        .map(|value| value.date_naive())
        .unwrap_or_else(|| Local::now().date_naive());
    let start = date - chrono::Duration::days(monday_offset_days);
    start_of_local_day(start)
}

fn overlap_ms(conn: &Connection, user_id: i64, since: i64, now: i64) -> AppResult<i64> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(COALESCE(ended_at, ?1) - max(started_at, ?2)), 0)
         FROM duty_shifts
         WHERE user_id = ?3 AND COALESCE(ended_at, ?1) > ?2",
        params![now, since, user_id],
        |row| row.get(0),
    )?;
    Ok(total.max(0))
}

impl Database {
    pub fn open_shift(&self, user_id: i64, display_name: &str) -> AppResult<DutyShiftView> {
        let conn = self.lock()?;
        let already = conn
            .query_row(
                "SELECT 1 FROM duty_shifts WHERE user_id = ?1 AND ended_at IS NULL",
                params![user_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if already {
            return Err(AppError::conflict("You are already clocked in."));
        }
        let now = now_ms();
        conn.execute(
            "INSERT INTO duty_shifts (user_id, display_name, started_at) VALUES (?1, ?2, ?3)",
            params![user_id, clamp_text(display_name, 80), now],
        )
        .map_err(|err| match err {
            rusqlite::Error::SqliteFailure(e, _)
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                AppError::conflict("You are already clocked in.")
            }
            other => other.into(),
        })?;
        let id = conn.last_insert_rowid();
        let shift = conn.query_row(
            &format!("{SELECT_SHIFT} WHERE id = ?1"),
            params![id],
            |row| map_shift(row, now),
        )?;
        Ok(shift)
    }

    pub fn current_shift(&self, user_id: i64) -> AppResult<Option<DutyShiftView>> {
        let conn = self.lock()?;
        let now = now_ms();
        Ok(conn
            .query_row(
                &format!("{SELECT_SHIFT} WHERE user_id = ?1 AND ended_at IS NULL"),
                params![user_id],
                |row| map_shift(row, now),
            )
            .optional()?)
    }

    pub fn close_shift(&self, shift_id: i64, reason: &str) -> AppResult<DutyShiftView> {
        let now = now_ms();
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE duty_shifts SET ended_at = ?1, end_reason = ?2
             WHERE id = ?3 AND ended_at IS NULL",
            params![now, clamp_text(reason, 200), shift_id],
        )?;
        if changed == 0 {
            return Err(AppError::not_found(
                "That shift has already ended or does not exist.",
            ));
        }
        let shift = conn.query_row(
            &format!("{SELECT_SHIFT} WHERE id = ?1"),
            params![shift_id],
            |row| map_shift(row, now),
        )?;
        Ok(shift)
    }

    /// End the open shift of another staff member (roster management).
    pub fn close_shift_for_user(&self, user_id: i64, reason: &str) -> AppResult<DutyShiftView> {
        let conn = self.lock()?;
        let shift_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM duty_shifts WHERE user_id = ?1 AND ended_at IS NULL",
                params![user_id],
                |row| row.get(0),
            )
            .optional()?;
        let shift_id =
            shift_id.ok_or_else(|| AppError::not_found("That staff member is not on duty."))?;
        drop(conn);
        self.close_shift(shift_id, reason)
    }

    pub fn on_duty_shifts(&self) -> AppResult<Vec<DutyShiftView>> {
        let conn = self.lock()?;
        let now = now_ms();
        let mut stmt = conn.prepare(&format!(
            "{SELECT_SHIFT} WHERE ended_at IS NULL ORDER BY started_at ASC"
        ))?;
        let rows = stmt
            .query_map([], |row| map_shift(row, now))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Completed shifts that overlap `since`, newest first.
    pub fn recent_shifts(&self, since: i64, limit: usize) -> AppResult<Vec<DutyShiftView>> {
        let conn = self.lock()?;
        let now = now_ms();
        let mut stmt = conn.prepare(&format!(
            "{SELECT_SHIFT}
             WHERE ended_at IS NOT NULL AND ended_at >= ?1
             ORDER BY started_at DESC LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![since, limit as i64], |row| map_shift(row, now))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn shift_history(
        &self,
        user_id: Option<i64>,
        limit: usize,
    ) -> AppResult<Vec<DutyShiftView>> {
        let conn = self.lock()?;
        let now = now_ms();
        let mut stmt = conn.prepare(&format!(
            "{SELECT_SHIFT}
             WHERE (?1 IS NULL OR user_id = ?1)
             ORDER BY started_at DESC LIMIT ?2"
        ))?;
        let rows = stmt
            .query_map(params![user_id, limit as i64], |row| map_shift(row, now))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn shift_today_ms(&self, user_id: i64) -> AppResult<i64> {
        let now = now_ms();
        let conn = self.lock()?;
        overlap_ms(&conn, user_id, local_day_start(now), now)
    }

    pub fn shift_week_ms(&self, user_id: i64) -> AppResult<i64> {
        let now = now_ms();
        let offset = Local
            .timestamp_millis_opt(now)
            .single()
            .map(|value| value.weekday().num_days_from_monday() as i64)
            .unwrap_or(0);
        let conn = self.lock()?;
        overlap_ms(&conn, user_id, local_week_start(now, offset), now)
    }

    pub fn shift_total_ms(&self, user_id: i64) -> AppResult<i64> {
        let now = now_ms();
        let conn = self.lock()?;
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(COALESCE(ended_at, ?1) - started_at), 0)
             FROM duty_shifts WHERE user_id = ?2",
            params![now, user_id],
            |row| row.get(0),
        )?;
        Ok(total.max(0))
    }

    pub fn shift_count(&self, user_id: i64) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM duty_shifts WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?)
    }

    pub fn count_on_duty(&self) -> AppResult<i64> {
        let conn = self.lock()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM duty_shifts WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_open_shifts(db: &Database) -> (i64, i64) {
        let conn = db.lock().unwrap();
        let now = now_ms();
        conn.execute(
            "INSERT INTO app_users (username, display_name, password_hash, role_id, created_at)
             VALUES ('a', 'A', 'x', 1, ?1), ('b', 'B', 'x', 1, ?1)",
            params![now],
        )
        .unwrap();
        drop(conn);
        let a = db.open_shift(1, "Alpha").unwrap();
        let b = db.open_shift(2, "Bravo").unwrap();
        (a.id, b.id)
    }

    #[test]
    fn opening_and_closing_a_shift_records_a_duration() {
        let db = Database::open_in_memory().unwrap();
        let (first, _) = two_open_shifts(&db);
        assert_eq!(db.count_on_duty().unwrap(), 2);
        let closed = db.close_shift(first, "End of patrol").unwrap();
        assert!(!closed.on_duty);
        assert!(closed.ended_at.is_some());
        assert_eq!(closed.end_reason.as_deref(), Some("End of patrol"));
        assert_eq!(db.count_on_duty().unwrap(), 1);
        assert!(db.close_shift(first, "again").is_err());
    }

    #[test]
    fn a_user_cannot_open_two_shifts() {
        let db = Database::open_in_memory().unwrap();
        two_open_shifts(&db);
        let err = db.open_shift(1, "Alpha").unwrap_err();
        assert_eq!(err.code(), "conflict");
    }

    #[test]
    fn totals_count_the_open_shift() {
        let db = Database::open_in_memory().unwrap();
        two_open_shifts(&db);
        assert!(db.shift_today_ms(1).unwrap() >= 0);
        assert!(db.shift_total_ms(1).unwrap() >= 0);
        assert_eq!(db.shift_count(1).unwrap(), 1);
        let today = db.shift_today_ms(1).unwrap();
        let week = db.shift_week_ms(1).unwrap();
        assert!(week >= today);
    }

    #[test]
    fn roster_and_history_are_consistent() {
        let db = Database::open_in_memory().unwrap();
        let (first, _) = two_open_shifts(&db);
        db.close_shift(first, "Break").unwrap();
        assert_eq!(db.on_duty_shifts().unwrap().len(), 1);
        assert_eq!(db.recent_shifts(0, 10).unwrap().len(), 1);
        assert_eq!(db.shift_history(None, 10).unwrap().len(), 2);
        assert_eq!(db.shift_history(Some(1), 10).unwrap().len(), 1);
    }

    #[test]
    fn local_day_and_week_starts_are_ordered() {
        let now = now_ms();
        let day = local_day_start(now);
        let week = local_week_start(now, 6);
        assert!(day <= now);
        assert!(week <= day);
    }

    #[test]
    fn closing_another_users_shift_requires_an_open_shift() {
        let db = Database::open_in_memory().unwrap();
        two_open_shifts(&db);
        let closed = db
            .close_shift_for_user(2, "Supervisor ended the shift")
            .unwrap();
        assert_eq!(closed.user_id, 2);
        assert_eq!(
            db.close_shift_for_user(2, "again").unwrap_err().code(),
            "not_found"
        );
    }
}
