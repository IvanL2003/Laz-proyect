use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl CsvTable {
    pub fn from_file(path: &Path) -> Result<CsvTable, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cannot read CSV file '{}': {}", path.display(), e))?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<CsvTable, String> {
        let mut lines = content.lines();

        let header_line = lines.next().ok_or("CSV file is empty")?;
        let headers: Vec<String> = parse_csv_line(header_line);

        if headers.is_empty() {
            return Err("CSV file has no columns".to_string());
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
                    "CSV row has {} fields, expected {} (headers: {:?})",
                    fields.len(), headers.len(), headers
                ));
            }
            rows.push(fields);
        }

        Ok(CsvTable { headers, rows })
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

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        let mut content = String::new();
        content.push_str(&self.headers.join(","));
        content.push('\n');
        for row in &self.rows {
            let escaped: Vec<String> = row.iter().map(|f| escape_csv_field(f)).collect();
            content.push_str(&escaped.join(","));
            content.push('\n');
        }
        fs::write(path, &content)
            .map_err(|e| format!("cannot write CSV file '{}': {}", path.display(), e))
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
        let table = CsvTable::parse(csv).unwrap();
        assert_eq!(table.headers, vec!["name", "age", "city"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["Alice", "30", "Madrid"]);
        assert_eq!(table.rows[1], vec!["Bob", "25", "Barcelona"]);
    }

    #[test]
    fn test_parse_quoted_csv() {
        let csv = "name,bio\nAlice,\"likes, commas\"\nBob,\"says \"\"hello\"\"\"";
        let table = CsvTable::parse(csv).unwrap();
        assert_eq!(table.rows[0][1], "likes, commas");
        assert_eq!(table.rows[1][1], "says \"hello\"");
    }

    #[test]
    fn test_column_index() {
        let csv = "name,age,city\nAlice,30,Madrid";
        let table = CsvTable::parse(csv).unwrap();
        assert_eq!(table.column_index("age"), Some(1));
        assert_eq!(table.column_index("missing"), None);
    }

    #[test]
    fn test_append_row() {
        let csv = "name,age\nAlice,30";
        let mut table = CsvTable::parse(csv).unwrap();
        table.append_row(&["Bob".to_string(), "25".to_string()]).unwrap();
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1], vec!["Bob", "25"]);
    }

    #[test]
    fn test_row_as_map() {
        let csv = "name,age\nAlice,30";
        let table = CsvTable::parse(csv).unwrap();
        let map = table.row_as_map(0);
        assert_eq!(map.get("name").unwrap(), "Alice");
        assert_eq!(map.get("age").unwrap(), "30");
    }
}
