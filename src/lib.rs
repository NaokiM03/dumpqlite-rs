use std::io;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Rusqlite(rusqlite::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Io(err) => core::fmt::Display::fmt(err, f),
            Error::Rusqlite(err) => core::fmt::Display::fmt(err, f),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Rusqlite(err) => Some(err),
        }
    }
}

impl core::convert::From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io(source)
    }
}

impl core::convert::From<rusqlite::Error> for Error {
    fn from(source: rusqlite::Error) -> Self {
        Error::Rusqlite(source)
    }
}

pub trait ConnectionExt {
    fn dump<W: io::Write>(&self, writer: &mut W) -> Result<(), crate::Error>;
}

impl ConnectionExt for rusqlite::Connection {
    fn dump<W: io::Write>(&self, writer: &mut W) -> Result<(), crate::Error> {
        writeln!(writer, "PRAGMA foreign_keys=OFF;")?;
        writeln!(writer, "BEGIN TRANSACTION;")?;

        let mut stmt = self.prepare(
            r#"
                SELECT name, sql
                FROM sqlite_schema
                WHERE sql NOT NULL
                    AND type == 'table'
                    AND name NOT LIKE 'sqlite_%';"#,
        )?;
        let tables = stmt
            .query_map([], |row| {
                let table_name: String = row.get(0)?;
                let create_sql: String = row.get(1)?;

                Ok((table_name, create_sql))
            })?
            .filter_map(Result::ok);

        for (table_name, create_sql) in tables {
            writeln!(writer, "{create_sql};")?;

            let (columns, column_count) = {
                let mut stmt = self.prepare(&format!("PRAGMA table_info({table_name});"))?;
                let columns: Vec<String> = stmt
                    .query_map([], |row| row.get(1))?
                    .filter_map(Result::ok)
                    .collect();

                (columns.join(", "), columns.len())
            };

            let mut stmt = self.prepare(&format!("SELECT {columns} FROM {table_name};"))?;
            stmt.query_map([], |row| {
                let values = (0..column_count)
                    .map(|i| row.get_ref(i))
                    .filter_map(Result::ok)
                    .map(|value| match value {
                        rusqlite::types::ValueRef::Null => "NULL".to_owned(),
                        rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                        rusqlite::types::ValueRef::Real(f) => f.to_string(),
                        rusqlite::types::ValueRef::Text(t) => {
                            format!("'{}'", String::from_utf8_lossy(t))
                        }

                        rusqlite::types::ValueRef::Blob(_b) => {
                            // let hex = _b.iter().fold(String::new(), |mut output, b| {
                            //     let _ = fmt::Write::write_fmt(&mut output, format_args!("{b:02x}"));
                            //     output
                            // });
                            // format!("X'{hex}'")

                            todo!()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(",");
                Ok(values)
            })?
            .filter_map(Result::ok)
            .try_for_each(|values| {
                writeln!(writer, "INSERT INTO {table_name} VALUES({values});")
            })?;
        }

        writeln!(writer, "DELETE FROM sqlite_sequence;")?;

        let mut stmt = self.prepare("SELECT name, seq FROM sqlite_sequence;")?;
        stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let seq: i64 = row.get(1)?;

            let values = format!("'{name}',{seq}");
            Ok(values)
        })?
        .filter_map(Result::ok)
        .try_for_each(|values| writeln!(writer, "INSERT INTO sqlite_sequence VALUES({values});"))?;

        writeln!(writer, "COMMIT;")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectionExt;

    #[test]
    fn test_dump() -> Result<(), crate::Error> {
        let mut conn = rusqlite::Connection::open_in_memory()?;

        let tx = conn.transaction()?;

        let sql = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL
);

-- Tasks table (simplified)
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    completed INTEGER DEFAULT 0,
    user_id INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Insert sample users
INSERT INTO
    users (username)
VALUES
    ('alice'),
    ('bob');

-- Insert sample tasks (no description or due_date)
INSERT INTO
    tasks (title, completed, user_id)
VALUES
    ('Buy groceries', 0, 1),
    ('Finish project report', 1, 1),
    ('Book dentist appointment', 0, 2);"#;

        tx.execute_batch(sql)?;
        tx.commit()?;

        let mut writer = Vec::new();
        conn.dump(&mut writer)?;

        let result = std::str::from_utf8(&writer).unwrap().trim();

        // SQLite CLI dump result
        let expected = r#"
PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL
);
INSERT INTO users VALUES(1,'alice');
INSERT INTO users VALUES(2,'bob');
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    completed INTEGER DEFAULT 0,
    user_id INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
INSERT INTO tasks VALUES(1,'Buy groceries',0,1);
INSERT INTO tasks VALUES(2,'Finish project report',1,1);
INSERT INTO tasks VALUES(3,'Book dentist appointment',0,2);
DELETE FROM sqlite_sequence;
INSERT INTO sqlite_sequence VALUES('users',2);
INSERT INTO sqlite_sequence VALUES('tasks',3);
COMMIT;
"#
        .trim();

        assert_eq!(expected, result);

        Ok(())
    }
}
