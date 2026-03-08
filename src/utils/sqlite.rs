use rusqlite::Connection;
use std::collections::HashMap;

/// (col_name, sqlite_type_affinity)
pub type ColumnInfo = Vec<(String, String)>;

/// Reads the full schema of a SQLite database.
/// Returns: table_name -> [(col_name, sqlite_type)]
pub fn read_schema(db_path: &str) -> Result<HashMap<String, ColumnInfo>, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("cannot open '{}': {}", db_path, e))?;

    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(|e| e.to_string())?;

    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut schema = HashMap::new();

    for table_name in table_names {
        let query = format!("PRAGMA table_info(\"{}\")", table_name);
        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let columns: ColumnInfo = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                let col_type: String = row.get(2)?;
                Ok((name, col_type))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        schema.insert(table_name, columns);
    }

    Ok(schema)
}

/// Maps a SQLite type affinity string to a Laz type name.
/// SQLite types: INTEGER, REAL, TEXT, BLOB, NUMERIC (+ variants)
#[allow(dead_code)]
pub fn sqlite_type_to_laz(sqlite_type: &str) -> &'static str {
    let upper = sqlite_type.to_uppercase();
    if upper.contains("INT") {
        "int"
    } else if upper.contains("REAL") || upper.contains("FLOAT") || upper.contains("DOUBLE") {
        "float"
    } else if upper.contains("BOOL") {
        "bool"
    } else {
        "string" // TEXT, BLOB, NUMERIC, empty, etc.
    }
}

/// Loads all rows from a SQLite table as strings.
/// Returns (headers, rows) — mirrors DataTable's format so existing SQL logic reuses it.
pub fn load_table_rows(
    db_path: &str,
    table_name: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("cannot open '{}': {}", db_path, e))?;

    // Column names from PRAGMA
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{}\")", table_name))
        .map_err(|e| e.to_string())?;
    let headers: Vec<String> = stmt
        .query_map([], |row| row.get(1))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let col_count = headers.len();

    // All rows
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM \"{}\"", table_name))
        .map_err(|e| e.to_string())?;

    let rows: Vec<Vec<String>> = stmt
        .query_map([], |row| {
            let mut row_data = Vec::new();
            for i in 0..col_count {
                let val: rusqlite::types::Value = row.get(i)?;
                let s = match val {
                    rusqlite::types::Value::Null => String::new(),
                    rusqlite::types::Value::Integer(v) => v.to_string(),
                    rusqlite::types::Value::Real(v) => v.to_string(),
                    rusqlite::types::Value::Text(v) => v,
                    rusqlite::types::Value::Blob(_) => "[blob]".to_string(),
                };
                row_data.push(s);
            }
            Ok(row_data)
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok((headers, rows))
}

/// Inserts one row into a SQLite table.
/// values must be in the same column order as the table schema.
pub fn insert_row(db_path: &str, table_name: &str, values: &[String]) -> Result<(), String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("cannot open '{}': {}", db_path, e))?;

    let placeholders: Vec<&str> = (0..values.len()).map(|_| "?").collect();
    let sql = format!(
        "INSERT INTO \"{}\" VALUES ({})",
        table_name,
        placeholders.join(", ")
    );

    conn.execute(&sql, rusqlite::params_from_iter(values.iter()))
        .map_err(|e| e.to_string())?;

    Ok(())
}
