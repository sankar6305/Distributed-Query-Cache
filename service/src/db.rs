use duckdb::Connection;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Database {
    /// Opens an in-memory DuckDB instance and seeds it with a synthetic
    /// analytical table, so there's something non-trivial to query and cache.
    pub fn open_seeded() -> duckdb::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE events AS
             SELECT
                i AS id,
                (i % 50) AS region_id,
                round((random() * 1000)::DOUBLE, 2) AS amount,
                DATE '2024-01-01' + INTERVAL (i % 365) DAY AS event_date
             FROM range(2000000) AS t(i);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Runs arbitrary SQL against the embedded database. DuckDB's Rust API is
    /// synchronous, so the query runs on a blocking thread instead of the
    /// async executor.
    pub async fn run_query(&self, sql: String) -> Result<QueryResult, String> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

            // DuckDB only knows the result schema *after* the statement has
            // executed, so column names come from the first row (via
            // `row.as_ref()`), not from `stmt` before `query()` runs —
            // calling `stmt.column_names()` beforehand panics.
            let mut columns: Vec<String> = Vec::new();
            let mut rows_out = Vec::new();

            let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                if columns.is_empty() {
                    columns = row.as_ref().column_names();
                }
                let mut r = Vec::with_capacity(columns.len());
                for i in 0..columns.len() {
                    let value: duckdb::types::Value = row.get(i).map_err(|e| e.to_string())?;
                    r.push(format_value(&value));
                }
                rows_out.push(r);
            }

            if columns.is_empty() {
                // Zero-row result: the statement still executed, just
                // nothing came back, so `stmt` is safe to query directly
                // once `rows` (which borrows it) is out of scope.
                drop(rows);
                columns = stmt.column_names();
            }

            Ok(QueryResult {
                columns,
                rows: rows_out,
            })
        })
        .await
        .map_err(|e| e.to_string())?
    }

    pub fn is_healthy(&self) -> bool {
        self.conn
            .lock()
            .map(|c| c.execute_batch("SELECT 1").is_ok())
            .unwrap_or(false)
    }
}

fn format_value(value: &duckdb::types::Value) -> String {
    use duckdb::types::Value;
    match value {
        Value::Null => String::new(),
        Value::Text(s) => s.clone(),
        other => format!("{other:?}"),
    }
}
