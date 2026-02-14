use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum DataFormat {
    Csv,
    Json,
}

impl DataFormat {
    pub fn from_extension(path: &Path) -> Result<DataFormat, String> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("csv") => Ok(DataFormat::Csv),
            Some("json") => Ok(DataFormat::Json),
            Some(ext) => Err(format!("unsupported file format: .{}", ext)),
            None => Err("file has no extension".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub format: DataFormat,
}

impl DataTable {
    /// Load a data file, detecting format by extension
    pub fn from_file(path: &Path) -> Result<DataTable, String> {
        let format = DataFormat::from_extension(path)?;
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cannot read file '{}': {}", path.display(), e))?;

        match format {
            DataFormat::Csv => Self::parse_csv(&content),
            DataFormat::Json => Err("JSON support not yet implemented".to_string()),
        }
    }

    /// Parse CSV content into a DataTable
    pub fn parse_csv(content: &str) -> Result<DataTable, String> {
        let mut lines = content.lines();

        let header_line = lines.next().ok_or("file is empty")?;
        let headers: Vec<String> = parse_csv_line(header_line);

        if headers.is_empty() {
            return Err("file has no columns".to_string());
        }

        let mut rows = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let fields = parse_csv_line(trimmed);
            if fields.len() != headers.len() {
                return Err(format!(
                    "row has {} fields, expected {} (headers: {:?})",
                    fields.len(), headers.len(), headers
                ));
            }
            rows.push(fields);
        }

        Ok(DataTable { headers, rows, format: DataFormat::Csv })
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|h| h == name)
    }

    pub fn row_as_map(&self, row_idx: usize) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if row_idx < self.rows.len() {
            for (i, header) in self.headers.iter().enumerate() {
                map.insert(header.clone(), self.rows[row_idx][i].clone());
            }
        }
        map
    }

    pub fn append_row(&mut self, values: &[String]) -> Result<(), String> {
        if values.len() != self.headers.len() {
            return Err(format!(
                "INSERT expects {} values, got {}",
                self.headers.len(), values.len()
            ));
        }
        self.rows.push(values.to_vec());
        Ok(())
    }

    /// Save to file, using the format detected at load time
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        match self.format {
            DataFormat::Csv => self.save_as_csv(path),
            DataFormat::Json => Err("JSON write not yet implemented".to_string()),
        }
    }

    fn save_as_csv(&self, path: &Path) -> Result<(), String> {
        let mut content = String::new();
        content.push_str(&self.headers.join(","));
        content.push('\n');
        for row in &self.rows {
            let escaped: Vec<String> = row.iter().map(|f| escape_csv_field(f)).collect();
            content.push_str(&escaped.join(","));
            content.push('\n');
        }
        fs::write(path, &content)
            .map_err(|e| format!("cannot write file '{}': {}", path.display(), e))
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next(); // escaped quote
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else {
            match ch {
                ',' => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                '"' => {
                    in_quotes = true;
                }
                _ => {
                    current.push(ch);
                }
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_csv() {
        let csv = "name,age,city\nAlice,30,Madrid\nBob,25,Barcelona";
        let table = DataTable::parse_csv(csv).unwrap();
        assert_eq!(table.headers, vec!["name", "age", "city"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["Alice", "30", "Madrid"]);
        assert_eq!(table.rows[1], vec!["Bob", "25", "Barcelona"]);
        assert_eq!(table.format, DataFormat::Csv);
    }

    #[test]
    fn test_parse_quoted_csv() {
        let csv = "name,bio\nAlice,\"likes, commas\"\nBob,\"says \"\"hello\"\"\"";
        let table = DataTable::parse_csv(csv).unwrap();
        assert_eq!(table.rows[0][1], "likes, commas");
        assert_eq!(table.rows[1][1], "says \"hello\"");
    }

    #[test]
    fn test_column_index() {
        let csv = "name,age,city\nAlice,30,Madrid";
        let table = DataTable::parse_csv(csv).unwrap();
        assert_eq!(table.column_index("age"), Some(1));
        assert_eq!(table.column_index("missing"), None);
    }

    #[test]
    fn test_append_row() {
        let csv = "name,age\nAlice,30";
        let mut table = DataTable::parse_csv(csv).unwrap();
        table.append_row(&["Bob".to_string(), "25".to_string()]).unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1], vec!["Bob", "25"]);
    }

    #[test]
    fn test_row_as_map() {
        let csv = "name,age\nAlice,30";
        let table = DataTable::parse_csv(csv).unwrap();
        let map = table.row_as_map(0);
        assert_eq!(map.get("name").unwrap(), "Alice");
        assert_eq!(map.get("age").unwrap(), "30");
    }

    #[test]
    fn test_format_detection() {
        assert_eq!(DataFormat::from_extension(Path::new("data.csv")).unwrap(), DataFormat::Csv);
        assert_eq!(DataFormat::from_extension(Path::new("data.json")).unwrap(), DataFormat::Json);
        assert!(DataFormat::from_extension(Path::new("data.xml")).is_err());
        assert!(DataFormat::from_extension(Path::new("noext")).is_err());
    }
}
