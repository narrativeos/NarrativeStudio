//! Measure DuckDB open (incl. WAL replay) + migration time for the real DB.
use std::time::Instant;

use duckdb::Connection;
use studio_storage::run_migrations;

fn main() {
    let home = std::env::var("HOME").unwrap();
    let src = format!(
        "{home}/Library/Application Support/com.narrativeos.studio/narrative_studio.duckdb"
    );
    let tmp = "/tmp/narrative_time_test.duckdb";
    // Copy DB + WAL to a temp path so we don't fight the running app's lock.
    std::fs::copy(&src, tmp).expect("copy db");
    let wal = format!("{src}.wal");
    if std::path::Path::new(&wal).exists() {
        std::fs::copy(&wal, format!("{tmp}.wal")).expect("copy wal");
    }

    let t0 = Instant::now();
    let conn = Connection::open(tmp).expect("open");
    let t1 = Instant::now();
    run_migrations(&conn).expect("migrations");
    let t2 = Instant::now();
    println!("open (incl WAL replay): {:.3}s", (t1 - t0).as_secs_f64());
    println!("migrations:             {:.3}s", (t2 - t1).as_secs_f64());
    println!("total:                  {:.3}s", (t2 - t0).as_secs_f64());
    let _ = std::fs::remove_file(tmp);
}
