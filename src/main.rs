extern crate dirs;
extern crate rusqlite;
extern crate time;
extern crate base64;

use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Result};
use time::Timespec;

use std::fs;
use std::env;
use std::str;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static DEBUG : AtomicUsize = AtomicUsize::new(0);

macro_rules! dump {
    ($( $args:expr ),*) => { if DEBUG.load(Ordering::Relaxed) > 0 { println!( $( $args ),* ); } }
}

fn setup(p: &Path) -> Result<()> {
    let res = fs::create_dir_all(p);
    let _res = match res {
        Ok(_) => dump!("setup complete!"),
        Err(err) => println!("failed to setup, because [{}]", err),
    };

    Ok(())
}

fn open_or_create_db(p: &Path) -> Result<Connection> {
    let mut create = false;

    if !p.exists() {
        create = true;
    }

    let mut conn = Connection::open(p)?;

    // if file already exists, trust that it's correct
    // CREATE TRIGGER update_appInfo_updatetime  BEFORE update ON appInfo 
    // begin
    // update appinfo set updatedatetime = strftime('%Y-%m-%d %H:%M:%S:%s','now', 'localtime') where bundle_id = old.bundle_id;
    // end
    //
    // CREATE TABLE "appInfo" (bundle_id INTEGER PRIMARY KEY,title text, updatedatetime text DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s','now', 'localtime')))

    if create {
        conn.execute(
            "CREATE TABLE mapping (
                      id              INTEGER PRIMARY KEY,
                      key             TEXT NOT NULL,
                      value           BLOB
                      )",
            params![],
        )?;
        conn.execute(
            "CREATE UNIQUE INDEX idx_mapping_key ON mapping (key);",
            params![],
        )?;

    }
    Ok(conn)
}

#[derive(Debug)]
struct Mapping {
    id: i32,
    key: String,
    value: Option<String>,
}

fn lookup(conn: &Connection, key: String) -> Result<String> {
    // 

    let mut stmt = conn.prepare("SELECT id, key, value FROM mapping")?;
    let mapping_iter = stmt.query_map(params![], |row| {
        Ok(Mapping {
            id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
        })
    })?;

    for map in mapping_iter {
        let m = &map.unwrap();
        if key == m.key {
            let value = match &m.value {
                Some(i) => i,
                None => panic!("no value set!"),
            };
            return Ok(value.to_string());
        };
    };

    panic!("could not find [{:?}]", key)
}

fn insert(conn: &Connection, key: String, value: String) -> Result<()> {
    // 

    let me = Mapping {
        id: 0,
        key: key,
        value: Some(value),
    };
    conn.execute(
        "REPLACE INTO mapping (key, value)
                  VALUES (?1, ?2)",
        params![me.key, me.value],
    )?;

    Ok(())
}

fn print_usage(prog: String) {
    println!("Usage: {} [-h,--help] <key> [<value>]", prog);
    println!("  -h,--help     print this help message");
    println!("  -v,--verbose  print verbose information");
    println!("  if <value> is a file, the contents of the file will be read and stored");
}

fn main() {

    // process the arguments
    let argv : Vec<String> = env::args().collect();
    let program = argv[0].clone();

    // skip the first element
    let mut args : Vec<String> = Vec::new();
    for (index,arg) in argv.iter().skip(1).enumerate() {

        // if someone passes in -h/--help, print the usage
        match arg.as_ref() {
            "-h" | "--help" => {
                dump!("got a help request at index {}!", index);
                print_usage(program);
                return;
            }
            "-v" | "--verbose" => {
                unsafe {
                    DEBUG.fetch_add(1, Ordering::Relaxed);
                }
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

    // if someone leaves out the key, print the usage
    // if someone leaves out the value, lookup the key
    // if someone puts key and value, store the value at the key

    let mut config : PathBuf = dirs::config_dir().unwrap();

    config.push("mappy");
    dump!("the user's config directory is {:?}", config);
    let f = setup(&config);
    let _f = match f {
        Ok(_) => f,
        Err(error) => {
            panic!("There was a problem opening the file: {:?}", error)
        },
    };

    config.push("maps.db");

    // if there is a file at maps.db, try to open it
    // otherwise, create the database
    dump!("opening {:?}", config);
    let res = open_or_create_db(&config);
    let conn = match res {
        Ok(conn_) => conn_,
        Err(error) => {
            panic!("failed to open {:?} with {:?}", config,  error);
        },
    };

    // do the thing now
    if args.len() >= 2 {
        // we only care about the first two
        let key = args[0].clone();
        let value_orig = args[1].clone();

        // do base64 thing
        let value_base64 = base64::encode(&value_orig);
        insert(&conn, key, value_base64);
    } else if args.len() == 1 {
        let key = args[0].clone();

        let result = lookup(&conn, key.to_string());
        let result = match result {
            Ok(res) => res,
            Err(error) => {
                panic!("could not find {:?}", key.to_string());
            }
        };
        let result_orig = base64::decode(&result).unwrap();
        let result_string = str::from_utf8(&result_orig).unwrap();
        println!("{}", result_string);
    }
    conn.close();
}
