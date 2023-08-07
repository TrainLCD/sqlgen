use csv::{ReaderBuilder, StringRecord};
use std::path::Path;
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let in_directory = Path::new(&args[1]);
    let out_path = Path::new(&args[2]);

    let entries = fs::read_dir(in_directory)?;
    let mut file_list: Vec<_> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension()? == "csv" {
                Some(path.file_name()?.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    file_list.sort();

    let mut sql_lines = Vec::new();

    for file_name in &file_list {
        if file_name
            .split('!')
            .nth(1)
            .unwrap_or_default()
            .split('.')
            .nth(1)
            .unwrap_or_default()
            != "csv"
        {
            continue;
        }

        let mut rdr = ReaderBuilder::new().from_path(in_directory.join(file_name))?;
        let csv_data: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();

        let mut sql_lines_inner = Vec::new();
        sql_lines_inner.push(format!(
            "INSERT INTO `{}` VALUES ",
            file_name
                .split('!')
                .nth(1)
                .unwrap_or_default()
                .split('.')
                .next()
                .unwrap_or_default()
        ));

        for (idx, data) in csv_data.iter().enumerate().skip(1) {
            let lat_index = csv_data[0]
                .iter()
                .position(|col| col == "lat")
                .unwrap_or(usize::MAX);
            let lon_index = csv_data[0]
                .iter()
                .position(|col| col == "lon")
                .unwrap_or(usize::MAX);
            let geom_text = if lat_index != usize::MAX && lon_index != usize::MAX {
                Some(format!(
                    "ST_GeomFromText('POINT({} {})', 4326)",
                    data.get(lon_index).unwrap_or("0"),
                    data.get(lat_index).unwrap_or("0")
                ))
            } else {
                None
            };

            let cols: Vec<_> = data
                .iter()
                .enumerate()
                .filter_map(|(idx, col)| {
                    if let Some(column_name) = csv_data[0].get(idx) {
                        if column_name.starts_with('#') {
                            return None;
                        }
                    }
                    if col.is_empty() {
                        Some("NULL".to_string())
                    } else {
                        Some(format!("'{}'", col.replace('\'', "\\'")))
                    }
                })
                .collect();

            sql_lines_inner.push(if geom_text.is_some() {
                if idx == csv_data.len() - 1 {
                    format!("({},{})", cols.join(","), geom_text.unwrap())
                } else {
                    format!("({},{}),", cols.join(","), geom_text.unwrap())
                }
            } else if idx == csv_data.len() - 1 {
                format!("({})", cols.join(","))
            } else {
                format!("({}),", cols.join(","))
            });
        }

        sql_lines.push(sql_lines_inner.concat());
    }

    fs::write(out_path, sql_lines.join(";\n"))?;

    Ok(())
}