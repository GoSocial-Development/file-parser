extern crate ini;
use chrono::NaiveDate;
use colored::Colorize;
use ini::{Ini, Properties};
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
    let row_types: &Properties = ini.section(Some("ROW_CONFIG")).unwrap();

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

    println!("Processing {} files", &files.len());

    for (i, file) in files.iter().enumerate() {
        let file_errors = process_file(&file, &row_types, &config);
        number_of_issues += file_errors.len();
        if file_errors.len() > 0 {
            errors.push(format!("FILE: {}", file.file_name().to_string_lossy()));
            for error in file_errors {
                errors.push(error);
            }
        }
        print!("\x1B[2J\x1B[1;1H");
        println!(
            "Processed file {} of {}",
            (i + 1).to_string().green(),
            files.len().to_string().green()
        );
    }

    if errors.len() > 0 {
        println!("Found {} issues", number_of_issues.to_string().red());
        write_errors(errors);
        println!("All issues can be found in {}", "errors.txt".green());
    } else {
        println!("{}", "Found No Issues".green());
    }
}

fn validate_file_type(file_type: &str) -> bool {
    match file_type {
        "tab" | "txt" | "ta1" => return true,
        _ => false,
    }
}

fn process_file(file: &DirEntry, row_types: &Properties, config: &Properties) -> Vec<String> {
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
                    if config.get("IGNORE_EMPTY_LINES").unwrap() == "1" {
                        continue;
                    }
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

                        if row_values.len() < fields.len() + 1 {
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
                                    if row_values[key + 1].to_string().len() == 0 {
                                        file_errors.push(format!(
                                            "#{}, Invalid Number at column #{}",
                                            i + 1,
                                            key + 1
                                        ));
                                    }
                                } else if !is_string_numeric(row_values[key + 1].to_string()) {
                                    file_errors.push(format!(
                                        "#{}, Invalid Number at column #{}",
                                        i + 1,
                                        key + 1
                                    ));
                                }
                            }

                            if value.contains("string")
                                && !value.contains("*")
                                && row_values[key + 1].len() == 0
                            {
                                file_errors.push(format!(
                                    "#{}, Invalid String at column #{}",
                                    i + 1,
                                    key + 1
                                ));
                            }
                            if value.contains("date") && !value.contains("*") {
                                let format = value.split("|").collect::<Vec<&str>>()[1];

                                match NaiveDate::parse_from_str(row_values[key + 1], format) {
                                    Ok(_) => continue,
                                    Err(_) => {
                                        println!("{}:{}", row_values[key + 1], format);
                                        file_errors.push(format!(
                                            "#{}, Invalid Date at column #{}",
                                            i + 1,
                                            key + 1
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
