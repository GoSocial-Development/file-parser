extern crate ini;
use chrono::NaiveDate;
use colored::Colorize;
use ini::{Ini, Properties};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::fs::{self, DirEntry, File};
use std::io::prelude::*;
use std::io::{self, BufRead};
use std::path::Path;
use std::process;

fn main() {
    let mut errors: Vec<String> = Vec::new();

    let mut number_of_issues: usize = 0;

    let ini_path = "./config.ini";

    if !Path::new(&ini_path).exists() {
        println!("{}", "config.ini file not found".red());
        process::exit(1);
    }

    let ini: Ini = Ini::load_from_file(ini_path).unwrap();
    let config: &Properties = ini.section(Some("CONFIG")).unwrap();
    let cfg_row_types: &Properties = ini.section(Some("ROW_CONFIG")).unwrap();
    let cfg_column_names: &Properties = ini.section(Some("COLUMN_NAMES")).unwrap();
    let cfg_table_names: &Properties = ini.section(Some("TABLE_NAMES")).unwrap();
    let cfg_column_links: &Properties = ini.section(Some("COLUMN_LINKS")).unwrap();

    if !validate_ini(cfg_column_names, cfg_row_types) {
        process::exit(1);
    }

    let config_input_path = config.get("INPUT_FOLDER").unwrap().to_string();
    let mut input_path: String = "./input".to_string();
    if config_input_path.len() > 0 {
        input_path = config_input_path;
    }

    if !Path::new(&input_path).is_dir() {
        println!(
            "{}",
            "Please create a directory named input and add files to it!".red()
        );
        process::exit(1);
    }

    let number_of_files = fs::read_dir(&input_path).unwrap().count();

    if number_of_files == 0 {
        println!("{}", "No files to process!".red());
        process::exit(1);
    }

    let mut files: Vec<DirEntry> = Vec::new();
    for file in fs::read_dir(&input_path).unwrap() {
        let file_name = file
            .as_ref()
            .unwrap()
            .file_name()
            .to_str()
            .unwrap()
            .to_string();
        let file_type = file_name.split(".").last().unwrap();
        if validate_file_type(&file_type) {
            files.push(file.unwrap());
        } else {
            errors.push(format!("{} not suported! File: {}", file_type, file_name));
        }
    }

    println!("Checking {} files", &files.len());

    for (i, file) in files.iter().enumerate() {
        let file_errors = check_file(&file, &cfg_row_types, &config);
        number_of_issues += file_errors.len();
        if file_errors.len() > 0 {
            errors.push(format!("FILE: {}", file.file_name().to_string_lossy()));
            for error in file_errors {
                errors.push(error);
            }
        }
        print!("\x1B[2J\x1B[1;1H");
        println!(
            "Checked file {} of {}",
            (i + 1).to_string().green(),
            files.len().to_string().green()
        );
    }
    match fs::remove_file("errors.txt") {
        Err(_e) => (),
        _ => (),
    }

    if errors.len() > 0 {
        println!("Found {} issues", number_of_issues.to_string().red());
        write_errors(errors);
        println!("All issues can be found in {}", "errors.txt".green());
        process::exit(1);
    } else {
        println!("{}", "Found No Issues".green());
        let mut sqls: Vec<(String, String)> = Vec::<(String, String)>::new();
        for (i, file) in files.iter().enumerate() {
            let mut result: Vec<(String, String)> = process_file(
                &file,
                cfg_table_names,
                cfg_column_names,
                cfg_row_types,
                cfg_column_links,
            );
            sqls.append(&mut result);
            println!(
                "Processed file {} of {}",
                (i + 1).to_string().green(),
                files.len().to_string().green()
            );
        }

        match fs::remove_dir_all("./output") {
            Err(_e) => (),
            _ => (),
        }

        match fs::create_dir_all("./output") {
            Err(_e) => println!("Error Creating Output Dir. Please create it manually"),
            _ => (),
        }

        let mut sql_files: Vec<(String, File)> = Vec::<(String, File)>::new();
        for (_, val) in config
            .get("ROW_TYPES")
            .unwrap()
            .to_string()
            .split(",")
            .collect::<Vec<&str>>()
            .iter()
            .enumerate()
        {
            sql_files.push((
                val.to_string(),
                OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(["./output/", &val.to_string(), ".sql"].join(""))
                    .unwrap(),
            ));
        }

        for (_, value) in sqls.iter().enumerate() {
            for (_, file_value) in sql_files.iter().enumerate() {
                if value.0.to_string() == file_value.0.to_string() {
                    match write!(&file_value.1, "{}", value.1) {
                        Err(e) => println!("{:?}", e),
                        _ => (),
                    }

                    match writeln!(&file_value.1, "\n") {
                        Err(e) => println!("{:?}", e),
                        _ => (),
                    }
                }
            }
        }

        println!("Done");
    }
}

fn validate_ini(cfg_column_names: &Properties, cfg_row_types: &Properties) -> bool {
    let mut valid: bool = true;
    for (key, _value) in cfg_row_types.iter() {
        if !valid {
            continue;
        }
        if cfg_column_names
            .get(key.to_string())
            .unwrap()
            .split("+")
            .collect::<Vec<&str>>()
            .len()
            != cfg_row_types
                .get(key.to_string())
                .unwrap()
                .split("+")
                .collect::<Vec<&str>>()
                .len()
        {
            valid = false;
            println!(
                "Invalid INI Config, please check the Columns Names and Column Types section of {:?}",
                &key
            );
        }
    }

    return valid;
}

fn process_file(
    file: &DirEntry,
    cfg_table_names_conf: &Properties,
    cfg_column_names: &Properties,
    cfg_row_types: &Properties,
    cfg_columns_links: &Properties,
) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::<(String, String)>::new();
    if let Ok(lines) = read_lines(file.path()) {
        let mut previous_value = HashMap::new();

        for (_i, line) in lines.enumerate() {
            if let Ok(ip) = line {
                let row_values: Vec<String> = ip.split("\t").map(|s| s.to_string()).collect();
                let first_letter = row_values[0].chars().nth(0).unwrap();
                let column_names = cfg_column_names
                    .get(&first_letter.to_string())
                    .unwrap()
                    .split("+")
                    .collect::<Vec<&str>>();

                previous_value.insert(first_letter, row_values.clone());

                let mut column_values: Vec<(String, String)> = Vec::<(String, String)>::new();

                for (key, _value) in column_names.iter().enumerate() {
                    column_values
                        .push((column_names[key].to_string(), row_values[key].to_string()));
                }

                let table_name = cfg_table_names_conf.get(&first_letter.to_string()).unwrap();

                let mut sql_statement: String = ["INSERT INTO ", table_name, " ( "].join("");

                for (_key, value) in column_values.iter().enumerate() {
                    sql_statement += &["\"", &(value.0.to_string()), "\"", ","].join("");
                }

                if first_letter == 'H' {
                    sql_statement +=
                        &[",\"", "hdrFileName", "\",", "\"hdrFileQuarter", "\","].join("");
                }

                let mut column_links = Vec::<&str>::new();

                if cfg_columns_links.get(&first_letter.to_string()).is_some() {
                    column_links = cfg_columns_links
                        .get(&first_letter.to_string())
                        .unwrap()
                        .split("+")
                        .collect::<Vec<&str>>();
                }

                if column_links.iter().len() > 0 {
                    for (_key, value) in column_links.iter().enumerate() {
                        sql_statement +=
                            &[",\"", value.split("->").nth(0).unwrap(), "\","].join("");
                    }
                }

                let row_types = cfg_row_types
                    .get(&first_letter.to_string())
                    .unwrap()
                    .split("+")
                    .collect::<Vec<&str>>();

                sql_statement += ") VALUES (";
                for (key, value) in column_values.iter().enumerate() {
                    if !row_types[key].contains("number") {
                        let mut string_value = value.1.to_string().replace("'", "\\'").clone();
                        if &string_value.chars().last() == &Some('"') {
                            string_value.pop();
                        }

                        if &string_value.chars().nth(0) == &Some('"') {
                            string_value.remove(0);
                        }

                        sql_statement += &["'", &string_value, "'"].join("");
                    } else {
                        sql_statement += &value.1.to_string();
                    }

                    sql_statement += ",";
                }

                if first_letter == 'H' {
                    let file_name: String = file.file_name().to_string_lossy().to_string();
                    let file_name_no_ext = file_name.split(".").enumerate().nth(0).unwrap();

                    sql_statement += &[
                        ", '",
                        &file_name,
                        "', '",
                        &file_name_no_ext.1[file_name_no_ext.1.len() - 4..],
                        "',",
                    ]
                    .join("");
                }

                if column_links.iter().len() > 0 {
                    for (_key, value) in column_links.iter().enumerate() {
                        if value.len() > 0 {
                            sql_statement += &[
                                ",'",
                                &previous_value
                                    .get(
                                        &value
                                            .split("->")
                                            .nth(1)
                                            .unwrap()
                                            .split(":")
                                            .nth(0)
                                            .unwrap()
                                            .chars()
                                            .nth(0)
                                            .unwrap(),
                                    )
                                    .unwrap()
                                    .iter()
                                    .nth(
                                        value
                                            .split("->")
                                            .nth(1)
                                            .unwrap()
                                            .split(":")
                                            .nth(1)
                                            .unwrap()
                                            .parse::<usize>()
                                            .unwrap(),
                                    )
                                    .unwrap(),
                                "',",
                            ]
                            .join("");
                        }
                    }
                }

                sql_statement += ")";

                sql_statement = sql_statement.replace(",)", ")").replace(",,", ",");

                result.push((first_letter.to_string(), sql_statement));
            }
        }
    }
    return result;
}

fn validate_file_type(file_type: &str) -> bool {
    match file_type {
        "tab" | "txt" | "ta1" => return true,
        _ => false,
    }
}

fn check_file(file: &DirEntry, row_types: &Properties, config: &Properties) -> Vec<String> {
    let mut file_errors: Vec<String> = Vec::new();
    if let Ok(lines) = read_lines(file.path()) {
        for (i, line) in lines.enumerate() {
            if let Ok(ip) = line {
                let first_letter = ip.chars().nth(0).unwrap();
                let mut allowed_types: Vec<String> = Vec::new();
                for (key, _value) in row_types.iter() {
                    allowed_types.push(key.to_string());
                }

                if first_letter.is_whitespace() {
                    file_errors.push(format!("#{}, Empty Line: {}", i + 1, first_letter));
                    continue;
                }

                if !allowed_types.contains(&first_letter.to_string()) {
                    file_errors.push(format!(
                        "#{}, Unexpected Letter Type: {}",
                        i + 1,
                        first_letter
                    ));
                    continue;
                }

                let row_values = ip.split("\t").collect::<Vec<&str>>();

                for (key, value) in row_types.iter() {
                    if key == &first_letter.to_string() {
                        let fields = value.split("+").collect::<Vec<&str>>();

                        if row_values.len() < fields.len() {
                            file_errors.push(format!(
                                "#{}, Wrong number of fields, Line Type => {}",
                                i + 1,
                                key
                            ));
                            continue;
                        }

                        for (key, value) in fields.iter().enumerate() {
                            if value.contains("number") {
                                if !value.contains("*") {
                                    if row_values[key].to_string().len() == 0 {
                                        file_errors.push(format!(
                                            "#{}, Invalid Number at column #{}",
                                            i + 1,
                                            key
                                        ));
                                    }
                                } else if !is_string_numeric(row_values[key].to_string()) {
                                    file_errors.push(format!(
                                        "#{}, Invalid Number at column #{}",
                                        i + 1,
                                        key
                                    ));
                                }
                            }

                            if value.contains("string")
                                && !value.contains("*")
                                && row_values[key].len() == 0
                            {
                                file_errors.push(format!(
                                    "#{}, Invalid String at column #{}",
                                    i + 1,
                                    key
                                ));
                            }

                            if value.contains("date") {
                                if !value.contains("*")
                                    || value.contains("*") && row_values[key].len() != 0
                                {
                                    let format = value.split("|").collect::<Vec<&str>>()[1];
                                    match NaiveDate::parse_from_str(row_values[key], format) {
                                        Ok(_) => continue,
                                        Err(_) => {
                                            file_errors.push(format!(
                                                "#{}, Invalid Date at column #{}",
                                                i + 1,
                                                key
                                            ));
                                        }
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return file_errors;
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn write_errors(errors: Vec<String>) {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("./errors.txt")
        .unwrap();
    for err in errors {
        if err.contains("FILE") {
            match writeln!(file, "=============================") {
                Err(e) => println!("{:?}", e),
                _ => (),
            }
        }
        match writeln!(file, "{}", err) {
            Err(e) => println!("{:?}", e),
            _ => (),
        }
    }
}

fn is_string_numeric(str: String) -> bool {
    for c in str.chars() {
        if !c.is_numeric() {
            return false;
        }
    }
    return true;
}
