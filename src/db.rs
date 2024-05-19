use std::path::Path;

use color_eyre::Result;
use tokio_rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).await?;

        Ok(Self { conn })
    }

    pub async fn create_tables(&self) -> Result<()> {
        self.conn
            .call(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS test (
                        id INTEGER PRIMARY KEY,
                        desc TEXT NOT NULL,
                        creation_date TEXT NOT NULL,
                        last_used_date TEXT,
                        times_used INTEGER NOT NULL DEFAULT 0
                    );

                    CREATE TABLE IF NOT EXISTS step (
                        id INTEGER PRIMARY KEY,
                        test_id INTEGER NOT NULL,
                        FOREIGN KEY (test_id) REFERENCES test (id)
                    )
                    "#,
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }
}
