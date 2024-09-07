use sqlx::{sqlite::SqlitePool, Row};
use tokio;
use std::env;
use std::io::{self};
use std::path::Path;
use std::error::Error;

const VERSION: &str = "0.1.0";

#[derive(Debug)]
struct Config {
    db_path: Option<String>,
    table: Option<String>,
    column: Option<String>,
    search_text: Option<String>,
    replace_text: Option<String>,
    prompt: bool
}

impl Config {
    fn new(args: &[String]) -> Result<Config, &'static str> {
        let mut db_path = None;
        let mut table = None;
        let mut column = None;
        let mut search_text = None;
        let mut replace_text = None;
        let mut prompt = true;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--column" => {
                    if i + 1 < args.len() {
                        column = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        print_help();
                        return Err("Column argument missing");
                    }
                },
                "--table" => {
                    if i + 1 < args.len() {
                        table = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        print_help();
                        return Err("Table argument missing");
                    }
                },
                "--search" => {
                    if i + 1 < args.len() {
                        search_text = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        print_help();
                        return Err("Search Text argument missing");
                    }
                },
                "--replace" => {
                    if i + 1 < args.len() {
                        replace_text = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        print_help();
                        return Err("Replacement Text argument missing");
                    }
                },
                "--no-prompt" => prompt = false,
                "--help" => {
                    print_help();
                    return Err("Help requested");
                }
                _ => {
                    let input = Some(args[i].clone());
                    if search_text.is_none() {
                        search_text = input;
                    } else if replace_text.is_none() {
                        replace_text = input;
                    } else if db_path.is_none() {
                        db_path = check_path(args[i].as_str());
                    } else {
                        print_help();
                        return Err("Multiple primary databases specified");
                    }
                }
            }
            i += 1;
        }

        if db_path.is_none() {
            return Err("No Database Specified");
        }

        if table.is_none() {
            table = Some("justinmetadata".to_string());
        }
        if column.is_none() {
            column = Some("FilePath".to_string());
        }

        Ok(Config {
            db_path,
            table,
            column,
            search_text,
            replace_text,
            prompt
        })
    }
}

fn check_path(path: &str) -> Option<String> {
    if Path::new(path).exists() {
        Some(path.to_string())
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("SMDReplace v{}", VERSION);

    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args)?;

    let db_path = config.db_path.unwrap();
    let table = config.table.unwrap();
    let column = config.column.unwrap();
    let find = config.search_text.unwrap();
    let replace = config.replace_text.unwrap();

    // Create a connection pool with sqlx
    let pool = SqlitePool::connect(&db_path).await?;

    let search_query = format!("SELECT COUNT(rowid) FROM {} WHERE {} LIKE ?", table, column);
    let result: (i64,) = sqlx::query_as(&search_query)
        .bind(format!("%{}%", find))
        .fetch_one(&pool)
        .await?;

    println!("Found {} records matching '{}' in {} of SM database: {}", result.0, find, column, db_path);
    
    if config.prompt {
        println!("Replace with '{}'?  Type 'yes' to confirm", replace);
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        if user_input.trim().to_lowercase() != "yes" {
            println!("Replace aborted.");
            return Ok(());
        }
    }

    println!("Replacing '{}' with '{}' in {} of SM database: {}", find, replace, column, db_path);

    let replace_query = format!(
        "UPDATE {} SET {} = REPLACE({}, '{}', '{}') WHERE {} LIKE '%{}%'",
        table, column, column, find, replace, column, find
    );
    
    sqlx::query(&replace_query).execute(&pool).await?;

    Ok(())
}

fn print_help() {
    let help_message = "
    Usage: SMReplace \"search_text\" \"replacement_text\" <database> [options]
    
    Options:
    -c, --column <column>          Select Column to Search and Replace if you want to modify something other than FilePath
    -h, --help                     Display this help message
    -r, --replace <text>           If you want to manually specify the replacement text in a different order
    -s, --search <text>            If you want to manually specify the search text in a different order
    -t, --table                    Lets you Specify a different table.  Useful for a non SM Database
    -y, --no-prompt                Auto Answer YES to all prompts

    ";
    println!("{}", help_message);
}
