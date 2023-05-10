use color_eyre::Result;
use ignore::Walk;
use sqlx::postgres::PgPoolOptions;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("Shutting down with force");
        std::process::exit(1);
    });

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://mikko:password@localhost:5454/proxy".to_string());
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS files (\
           path_and_hash TEXT PRIMARY KEY,\
           data BYTEA NOT NULL\
        )",
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS revision_files (\
           revision_id TEXT NOT NULL,\
           path_and_hash TEXT NOT NULL,\
           PRIMARY KEY (revision_id, path_and_hash)
        )",
    )
    .execute(&pool)
    .await?;

    println!("All migrations applied");

    for result in Walk::new("./") {
        match result {
            Ok(entry) => {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        let path = path.to_str().unwrap();
                        let contents = tokio::fs::read(path).await.unwrap();
                        let hash = seahash::hash(&contents[..]);
                        let hash = format!("{:08x}", hash);
                        println!("- found {path}#{hash}");
                    }
                }
            }
            Err(err) => println!("ERROR: {}", err),
        }
    }

    let mut cmd = Command::new("deno");
    cmd.arg("run").arg("-A").arg("main.ts");
    cmd.env("PORT", "3001");
    unsafe {
        cmd.pre_exec(|| {
            let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            if ret != 0 {
                panic!("prctl failed");
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().expect("Could not execute command");
    tokio::spawn(async move {
        child.wait().await.unwrap();
    });

    let listener = TcpListener::bind("0.0.0.0:8000").await.unwrap();
    loop {
        let (mut downstream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut upstream = TcpStream::connect("127.0.0.1:3001").await.unwrap();
            tokio::io::copy_bidirectional(&mut downstream, &mut upstream)
                .await
                .unwrap();
        });
    }
}
