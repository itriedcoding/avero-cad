//! Aggregated activity statistics.
//!
//! Every number here is computed from rows this installation actually stored:
//! CAD calls, moderation records, executed commands, duty shifts and recorded
//! metric samples. Nothing is generated or extrapolated.

use crate::db::Database;
use crate::error::AppResult;
use crate::models::{DailyStatView, HourStatView, StatsOverviewView, TopModeratorView};
use crate::util::now_ms;
use chrono::{Duration as ChronoDuration, Local, TimeZone};
use rusqlite::{params, Connection};

fn local_date_key(now: i64) -> chrono::NaiveDate {
    Local
        .timestamp_millis_opt(now)
        .single()
        .map(|value| value.date_naive())
        .unwrap_or_else(|| Local::now().date_naive())
}

fn daily_series(days: i64, now: i64) -> Vec<DailyStatView> {
    let today = local_date_key(now);
    (0..days)
        .rev()
        .map(|offset| DailyStatView {
            date: (today - ChronoDuration::days(offset))
                .format("%Y-%m-%d")
                .to_string(),
            calls_opened: 0,
            calls_closed: 0,
            moderation: 0,
            commands: 0,
            peak_players: 0,
        })
        .collect()
}

fn hour_series() -> Vec<HourStatView> {
    (0..24)
        .map(|hour| HourStatView {
            hour,
            calls: 0,
            moderation: 0,
            commands: 0,
        })
        .collect()
}

fn bump_daily(series: &mut [DailyStatView], date: String, apply: impl FnOnce(&mut DailyStatView)) {
    if let Some(entry) = series.iter_mut().find(|entry| entry.date == date) {
        apply(entry);
    }
}

impl Database {
    pub fn stats_overview(&self, days: i64) -> AppResult<StatsOverviewView> {
        let now = now_ms();
        let days = days.clamp(1, 365);
        let since = now - days * 86_400_000;
        let conn = self.lock()?;

        let (calls_opened, calls_closed, calls_open_now): (i64, i64, i64) = conn.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN closed_at IS NOT NULL THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN closed_at IS NULL THEN 1 ELSE 0 END), 0)
             FROM dispatch_calls WHERE created_at >= ?1",
            params![since],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let avg_close_ms: Option<f64> = conn.query_row(
            "SELECT AVG(closed_at - created_at) FROM dispatch_calls
             WHERE closed_at IS NOT NULL AND closed_at >= ?1",
            params![since],
            |row| row.get(0),
        )?;

        let (moderation_total, moderation_failed): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0)
             FROM moderation_records WHERE created_at >= ?1",
            params![since],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let mut kicks = 0;
        let mut bans = 0;
        let mut unbans = 0;
        let mut internal_bans = 0;
        {
            let mut stmt = conn.prepare(
                "SELECT action, COUNT(*) FROM moderation_records
                 WHERE created_at >= ?1 GROUP BY action",
            )?;
            let rows = stmt.query_map(params![since], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (action, count) = row?;
                match action.as_str() {
                    "kick" => kicks = count,
                    "ban" => bans = count,
                    "unban" => unbans = count,
                    "internal_ban" | "internal_unban" => internal_bans += count,
                    _ => {}
                }
            }
        }

        let (commands_total, commands_failed): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN status <> 'success' THEN 1 ELSE 0 END), 0)
             FROM command_history WHERE created_at >= ?1",
            params![since],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let (bolos_active, bolos_cleared): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'cleared' THEN 1 ELSE 0 END), 0)
             FROM bolo_reports",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let players_tracked: i64 =
            conn.query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))?;
        let players_new: i64 = conn.query_row(
            "SELECT COUNT(*) FROM players WHERE first_seen_at >= ?1",
            params![since],
            |row| row.get(0),
        )?;

        let (duty_ms, duty_shifts): (i64, i64) = conn.query_row(
            "SELECT COALESCE(SUM(COALESCE(ended_at, ?1) - started_at), 0), COUNT(*)
             FROM duty_shifts WHERE COALESCE(ended_at, ?1) >= ?2",
            params![now, since],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let on_duty_now: i64 = conn.query_row(
            "SELECT COUNT(*) FROM duty_shifts WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )?;

        let mut daily = daily_series(days, now);
        let mut stmt = conn.prepare(
            "SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS day, COUNT(*)
             FROM dispatch_calls WHERE created_at >= ?1 GROUP BY day",
        )?;
        let rows = stmt.query_map(params![since], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (date, opened) = row?;
            bump_daily(&mut daily, date, |entry| entry.calls_opened = opened);
        }

        closed_calls_by_day(&conn, since, &mut daily)?;
        moderation_by_day(&conn, since, &mut daily)?;
        commands_by_day(&conn, since, &mut daily)?;
        peaks_by_day(&conn, since, &mut daily)?;

        let mut hours = hour_series();
        bump_hours(
            &conn,
            "SELECT CAST(strftime('%H', created_at / 1000, 'unixepoch', 'localtime') AS INTEGER) AS hour,
                    COUNT(*) FROM dispatch_calls WHERE created_at >= ?1 GROUP BY hour",
            since,
            &mut hours,
            |entry, count| entry.calls = count,
        )?;
        bump_hours(
            &conn,
            "SELECT CAST(strftime('%H', created_at / 1000, 'unixepoch', 'localtime') AS INTEGER) AS hour,
                    COUNT(*) FROM moderation_records WHERE created_at >= ?1 GROUP BY hour",
            since,
            &mut hours,
            |entry, count| entry.moderation = count,
        )?;
        bump_hours(
            &conn,
            "SELECT CAST(strftime('%H', created_at / 1000, 'unixepoch', 'localtime') AS INTEGER) AS hour,
                    COUNT(*) FROM command_history WHERE created_at >= ?1 GROUP BY hour",
            since,
            &mut hours,
            |entry, count| entry.commands = count,
        )?;

        let mut stmt = conn.prepare(
            "SELECT moderator_name, COUNT(*),
                    COALESCE(SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END), 0)
             FROM moderation_records WHERE created_at >= ?1
             GROUP BY moderator_name ORDER BY COUNT(*) DESC, moderator_name LIMIT 5",
        )?;
        let top_moderators = stmt
            .query_map(params![since], |row| {
                Ok(TopModeratorView {
                    name: row.get(0)?,
                    actions: row.get(1)?,
                    failed: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(StatsOverviewView {
            range_days: days,
            since,
            calls_opened,
            calls_closed,
            calls_open_now,
            avg_close_ms: avg_close_ms.map(|value| value.round() as i64),
            moderation_total,
            moderation_failed,
            kicks,
            bans,
            unbans,
            internal_bans,
            commands_total,
            commands_failed,
            bolos_active,
            bolos_cleared,
            players_tracked,
            players_new,
            duty_ms,
            duty_shifts,
            on_duty_now,
            daily,
            hours,
            top_moderators,
        })
    }
}

fn closed_calls_by_day(
    conn: &Connection,
    since: i64,
    daily: &mut [DailyStatView],
) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT date(closed_at / 1000, 'unixepoch', 'localtime') AS day, COUNT(*)
         FROM dispatch_calls WHERE closed_at IS NOT NULL AND closed_at >= ?1 GROUP BY day",
    )?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (date, count) = row?;
        bump_daily(daily, date, |entry| entry.calls_closed += count);
    }
    Ok(())
}

fn moderation_by_day(conn: &Connection, since: i64, daily: &mut [DailyStatView]) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS day, COUNT(*)
         FROM moderation_records WHERE created_at >= ?1 GROUP BY day",
    )?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (date, count) = row?;
        bump_daily(daily, date, |entry| entry.moderation = count);
    }
    Ok(())
}

fn commands_by_day(conn: &Connection, since: i64, daily: &mut [DailyStatView]) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS day, COUNT(*)
         FROM command_history WHERE created_at >= ?1 GROUP BY day",
    )?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (date, count) = row?;
        bump_daily(daily, date, |entry| entry.commands = count);
    }
    Ok(())
}

fn peaks_by_day(conn: &Connection, since: i64, daily: &mut [DailyStatView]) -> AppResult<()> {
    let mut stmt = conn.prepare(
        "SELECT date(captured_at / 1000, 'unixepoch', 'localtime') AS day, MAX(player_count)
         FROM metric_samples WHERE captured_at >= ?1 GROUP BY day",
    )?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (date, count) = row?;
        bump_daily(daily, date, |entry| entry.peak_players = count);
    }
    Ok(())
}

fn bump_hours(
    conn: &Connection,
    sql: &str,
    since: i64,
    hours: &mut [HourStatView],
    apply: impl Fn(&mut HourStatView, i64),
) -> AppResult<()> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (hour, count) = row?;
        if let Some(entry) = hours.iter_mut().find(|entry| entry.hour == hour) {
            apply(entry, count);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(db: &Database) {
        let conn = db.lock().unwrap();
        let now = now_ms();
        conn.execute(
            "INSERT INTO dispatch_calls
             (call_number, title, status, source, created_by_name, created_at, updated_at, closed_at)
             VALUES (1001, 'Fire', 'closed', 'cad', 'Owner', ?1, ?1, ?1),
                    (1002, 'Theft', 'open', 'cad', 'Owner', ?1, ?1, NULL)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO moderation_records
             (username, action, source, moderator_name, success, created_at)
             VALUES ('Troll', 'kick', 'erlc_api', 'Mod A', 1, ?1),
                    ('Troll', 'ban', 'erlc_api', 'Mod A', 0, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO command_history (actor_name, command, status, duration_ms, created_at)
             VALUES ('Mod A', ':h hi', 'success', 20, ?1), ('Mod A', ':h hi', 'failed', 20, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bolo_reports (kind, subject_name, created_by_name, created_at)
             VALUES ('person', 'Suspect', 'Owner', ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO metric_samples (captured_at, player_count, max_players)
             VALUES (?1, 17, 40)",
            params![now],
        )
        .unwrap();
    }

    #[test]
    fn overview_counts_real_rows() {
        let db = Database::open_in_memory().unwrap();
        seed(&db);
        let stats = db.stats_overview(7).unwrap();
        assert_eq!(stats.calls_opened, 2);
        assert_eq!(stats.calls_closed, 1);
        assert_eq!(stats.calls_open_now, 1);
        assert_eq!(stats.moderation_total, 2);
        assert_eq!(stats.moderation_failed, 1);
        assert_eq!(stats.kicks, 1);
        assert_eq!(stats.bans, 1);
        assert_eq!(stats.commands_total, 2);
        assert_eq!(stats.commands_failed, 1);
        assert_eq!(stats.bolos_active, 1);
        assert_eq!(stats.top_moderators.len(), 1);
        assert_eq!(stats.top_moderators[0].name, "Mod A");
        assert_eq!(stats.top_moderators[0].actions, 2);
        assert_eq!(stats.daily.len(), 7);
        assert_eq!(stats.hours.len(), 24);
    }

    #[test]
    fn daily_buckets_cover_the_whole_range() {
        let db = Database::open_in_memory().unwrap();
        seed(&db);
        let stats = db.stats_overview(7).unwrap();
        let total_commands: i64 = stats.daily.iter().map(|d| d.commands).sum();
        assert_eq!(total_commands, 2);
        let peak: i64 = stats
            .daily
            .iter()
            .map(|d| d.peak_players)
            .max()
            .unwrap_or(0);
        assert_eq!(peak, 17);
        let call_hours: i64 = stats.hours.iter().map(|h| h.calls).sum();
        assert_eq!(call_hours, 2);
    }

    #[test]
    fn empty_database_reports_zeros_without_error() {
        let db = Database::open_in_memory().unwrap();
        let stats = db.stats_overview(14).unwrap();
        assert_eq!(stats.range_days, 14);
        assert_eq!(stats.calls_opened, 0);
        assert_eq!(stats.avg_close_ms, None);
        assert!(stats.daily.iter().all(|day| day.calls_opened == 0));
        assert!(stats.top_moderators.is_empty());
    }

    #[test]
    fn range_is_clamped() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.stats_overview(0).unwrap().range_days, 1);
        assert_eq!(db.stats_overview(9_999).unwrap().range_days, 365);
    }
}
