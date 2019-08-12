extern crate dirs;
extern crate rusqlite;
extern crate time;

use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Result};
use time::Timespec;

use std::fs;
use std::env;
use std::path::{Path, PathBuf};

fn dump(s: &String) {
    println!("dumping [{}]", s);
}

fn setup(p: &Path) -> Result<()> {
    let res = fs::create_dir_all(p);
    let _res = match res {
        Ok(_) => dump(&String::from("setup complete!")),
        Err(err) => println!("failed to setup, because [{}]", err),
    };
    Ok(())
}

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    time_created: Timespec,
    data: Option<Vec<u8>>,
}

fn go() -> Result<()> {
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE person (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL,
                  time_created    TEXT NOT NULL,
                  data            BLOB
                  )",
        params![],
    )?;
    let me = Person {
        id: 0,
        name: "Steven".to_string(),
        time_created: time::get_time(),
        data: None,
    };
    conn.execute(
        "INSERT INTO person (name, time_created, data)
                  VALUES (?1, ?2, ?3)",
        params![me.name, me.time_created, me.data],
    )?;

    let mut stmt = conn.prepare("SELECT id, name, time_created, data FROM person")?;
    let person_iter = stmt.query_map(params![], |row| {
        Ok(Person {
            id: row.get(0)?,
            name: row.get(1)?,
            time_created: row.get(2)?,
            data: row.get(3)?,
        })
    })?;

    for person in person_iter {
        println!("Found person {:?}", person.unwrap());
    }
    Ok(())
}

fn print_usage(prog: String) {
    println!("Usage: {} [-h,--help] <key> [<value>]", prog);
    println!("  -h,--help  print this help message");
    println!("  if <value> is a file, the contents of the file will be read and stored");
}

fn main() {

    // process the arguments
    let argv : Vec<String> = env::args().collect();
    let program = argv[0].clone();

    // if someone passes in -h/--help, print the usage
    // if someone leaves out the key, print the usage
    // if someone leaves out the value, lookup the key
    // if someone puts key and value, store the value at the key

    // skip the first element
    let mut args : Vec<String> = Vec::new();
    for (index,arg) in argv.iter().skip(1).enumerate() {

        match arg.as_ref() {
            "-h" | "--help" => {
                println!("got a help request at index {}!", index);
                print_usage(program);
                return;
            }
            _ => {
                args.push(arg.to_string());
            }
        }
    }

    if args.len() < 1 {
        print_usage(program);
        return;
    }

    let mut config : PathBuf = dirs::config_dir().unwrap();

    config.push("mappy");
    println!("the user's config directory is {:?}", config);
    let f = setup(&config);
    let _f = match f {
        Ok(_) => f,
        Err(error) => {
            panic!("There was a problem opening the file: {:?}", error)
        },
    };
}
