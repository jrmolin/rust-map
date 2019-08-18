extern crate dirs;
extern crate rusqlite;
extern crate time;
extern crate base64;

use rusqlite::{params, Connection, Result};

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

fn path_is_file(p: &Path) -> bool {
    let meta = fs::metadata(p);

    meta.is_ok() && meta.unwrap().is_file()
}

fn open_or_create_db(p: &Path) -> Result<Connection> {
    let mut create = true;

    if path_is_file(p) {
        create = false;
    }

    let conn = Connection::open(p)?;

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

fn lookup(conn: &Connection, key: &String) -> Result<String> {
    // 

    let mut stmt = try!(conn.prepare("SELECT id, key, value FROM mapping where key = :key"));
    let mapping_iter = stmt.query_map_named(&[(":key", key)], |row| {
        Ok(Mapping {
            id: row.get(0)?,
            key: row.get(1)?,
            value: row.get(2)?,
        })
    })?;

    for map in mapping_iter {
        let m = &map.unwrap();
        let value = match &m.value {
            Some(i) => i,
            None => panic!("no value set!"),
        };
        return Ok(value.to_string());
    };

    panic!("could not find [{:?}]", key)
}

fn insert(conn: &Connection, key: &String, value: &String) -> Result<()> {
    // 

    conn.execute(
        "REPLACE INTO mapping (key, value)
                  VALUES (?1, ?2)",
        params![key, value],
    )?;

    Ok(())
}

fn print_usage(prog: &String) {
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
                print_usage(&program);
                return;
            }
            "-v" | "--verbose" => {
                DEBUG.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                args.push(arg.to_string());
            }
        }
    }

    if args.len() < 1 {
        print_usage(&program);
        return;
    }

    // if someone leaves out the key, print the usage
    // if someone leaves out the value, lookup the key
    // if someone puts key and value, store the value at the key

    let mut config : PathBuf = dirs::config_dir().unwrap();

    config.push("mappy");
    dump!("the user's config directory is {:?}", config);
    let _f = match setup(&config) {
        Ok(_f) => _f,
        Err(error) => {
            panic!("Error: There was a problem opening the file: {:?}", error)
        },
    };

    config.push("maps.db");

    // open a connection to the database file
    dump!("opening {:?}", config);
    let res = open_or_create_db(&config);
    let conn = match res {
        Ok(conn_) => conn_,
        Err(error) => {
            panic!("Error: failed to open {:?} with {:?}", config, error);
        },
    };

    // do the thing now
    if args.len() >= 2 {
        // we only care about the first two
        let key = &args[0];
        let value_orig = args[1].clone();

        // do base64 thing
        let mut value_base64 = base64::encode(&value_orig);

        let value_path = Path::new(&value_orig);

        if path_is_file(&value_path) {
            let value_orig = fs::read_to_string(value_path).unwrap();
            value_base64 = base64::encode(&value_orig);
            println!("value is a file! {:?}", value_orig);
        }

        let result = insert(&conn, &key, &value_base64);
        let _result = match result {
            Ok(res) => res,
            Err(error) => {
                panic!("Error: could not insert {:?} :: {:?}", &key[..], error);
            }
        };
    } else if args.len() == 1 {
        let key = &args[0];

        let result = lookup(&conn, key);
        let result = match result {
            Ok(res) => res,
            Err(error) => {
                panic!("Error: could not find {:?} :: {:?}", &key[..], error);
            }
        };
        let result_orig = base64::decode(&result).unwrap();
        let result_string = str::from_utf8(&result_orig).unwrap();
        println!("{}", result_string);
    }
    let _res = conn.close();
}
