use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};

use crate::error::StorageError;

const RAW_RETENTION: Duration = Duration::hours(48);
const MINUTE_RETENTION: Duration = Duration::days(30);
const HOUR_RETENTION: Duration = Duration::days(365);
const MINUTE: i64 = 60;
const HOUR: i64 = 3600;

pub fn run(connection: &mut Connection, now: DateTime<Utc>) -> Result<(), StorageError> {
    let transaction = connection.transaction()?;
    let raw_cutoff = (now - RAW_RETENTION).timestamp();
    transaction.execute(
        "INSERT OR REPLACE INTO samples_1m (server, key, at, avg, min, max)
         SELECT server, key, (at / ?1) * ?1, AVG(value), MIN(value), MAX(value)
         FROM samples WHERE at < ?2 GROUP BY server, key, at / ?1",
        params![MINUTE, raw_cutoff],
    )?;
    transaction.execute("DELETE FROM samples WHERE at < ?1", params![raw_cutoff])?;
    let minute_cutoff = (now - MINUTE_RETENTION).timestamp();
    transaction.execute(
        "INSERT OR REPLACE INTO samples_1h (server, key, at, avg, min, max)
         SELECT server, key, (at / ?1) * ?1, AVG(avg), MIN(min), MAX(max)
         FROM samples_1m WHERE at < ?2 GROUP BY server, key, at / ?1",
        params![HOUR, minute_cutoff],
    )?;
    transaction.execute(
        "DELETE FROM samples_1m WHERE at < ?1",
        params![minute_cutoff],
    )?;
    let hour_cutoff = (now - HOUR_RETENTION).timestamp();
    transaction.execute("DELETE FROM samples_1h WHERE at < ?1", params![hour_cutoff])?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::sample::StoredSample;

    use super::*;

    fn sample(at: DateTime<Utc>, value: f64) -> StoredSample {
        StoredSample {
            server: "neo".into(),
            key: "cpu.total".into(),
            at,
            value,
        }
    }

    #[test]
    fn old_raw_samples_are_folded_into_minutes() {
        let mut db = Database::in_memory().expect("db");
        let now = Utc::now();
        let old = now - Duration::hours(50);
        db.insert_batch(&[
            sample(old, 10.0),
            sample(old + Duration::seconds(2), 30.0),
            sample(now, 5.0),
        ])
        .expect("insert");
        db.run_maintenance(now).expect("maintenance");
        assert_eq!(db.count("samples").expect("count"), 1);
        assert_eq!(db.count("samples_1m").expect("count"), 1);
    }

    #[test]
    fn very_old_minutes_are_folded_into_hours() {
        let mut db = Database::in_memory().expect("db");
        let now = Utc::now();
        let old = now - Duration::days(40);
        db.insert_batch(&[sample(old, 1.0)]).expect("insert");
        db.run_maintenance(now).expect("first pass");
        db.run_maintenance(now).expect("second pass");
        assert_eq!(db.count("samples_1m").expect("count"), 0);
        assert_eq!(db.count("samples_1h").expect("count"), 1);
    }
}
