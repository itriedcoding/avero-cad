//! Player-count samples captured from live API responses.

use crate::db::Database;
use crate::error::AppResult;
use crate::models::MetricPoint;
use rusqlite::params;

impl Database {
    pub fn insert_metric(
        &self,
        captured_at: i64,
        player_count: i64,
        max_players: i64,
        staff_count: i64,
        latency_ms: Option<i64>,
    ) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO metric_samples (captured_at, player_count, max_players, staff_count, latency_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![captured_at, player_count, max_players, staff_count, latency_ms],
        )?;
        Ok(())
    }

    pub fn recent_metrics(&self, limit: usize) -> AppResult<Vec<MetricPoint>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT captured_at, player_count, max_players, latency_ms
             FROM metric_samples ORDER BY captured_at DESC LIMIT ?1",
        )?;
        let mut points = stmt
            .query_map(params![limit as i64], |row| {
                Ok(MetricPoint {
                    captured_at: row.get(0)?,
                    player_count: row.get(1)?,
                    max_players: row.get(2)?,
                    latency_ms: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        points.reverse();
        Ok(points)
    }

    pub fn prune_metrics(&self, keep: usize) -> AppResult<()> {
        let conn = self.lock()?;
        conn.execute(
            "DELETE FROM metric_samples WHERE id NOT IN
                (SELECT id FROM metric_samples ORDER BY captured_at DESC LIMIT ?1)",
            params![keep as i64],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_orders_samples() {
        let db = Database::open_in_memory().unwrap();
        db.insert_metric(1000, 5, 40, 1, Some(120)).unwrap();
        db.insert_metric(2000, 7, 40, 2, Some(90)).unwrap();
        let points = db.recent_metrics(10).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].player_count, 5);
        assert_eq!(points[1].player_count, 7);
        assert_eq!(points[1].latency_ms, Some(90));
    }

    #[test]
    fn prunes_old_samples() {
        let db = Database::open_in_memory().unwrap();
        for i in 0..10 {
            db.insert_metric(1000 + i, i, 40, 0, None).unwrap();
        }
        db.prune_metrics(5).unwrap();
        let points = db.recent_metrics(100).unwrap();
        assert_eq!(points.len(), 5);
        assert_eq!(points[4].player_count, 9);
    }
}
