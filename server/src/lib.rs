use std::path::Path;

use anyhow::Result;
use coordinator::scheduler::service::SchedulerServiceFactory;
use rusqlite::OpenFlags;
use tokio::net::ToSocketAddrs;

use crate::coordinator::Coordinator;
use crate::postgres::service::PgConnectionFactory;
use crate::server::Server;

mod coordinator;
mod job;
mod postgres;
mod server;

pub async fn run_server(db_path: &Path, addr: impl ToSocketAddrs) -> Result<()> {
    let (pool, pool_sender) = Coordinator::new(0, move || {
        rusqlite::Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .unwrap()
    })?;
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    let service = SchedulerServiceFactory::new(sender);
    let factory = PgConnectionFactory::new(service);
    let server = Server::bind(addr).await?;
    let scheduler = coordinator::scheduler::Scheduler::new(pool_sender, receiver)?;
    let shandle = tokio::spawn(scheduler.start());

    server.serve(factory).await;
    shandle.await?;
    pool.join().await;

    Ok(())
}