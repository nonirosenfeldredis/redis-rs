use std::io;

use crate::{ConnectionLike, RedisError};

/// Implementation of Redis connections for R2D2 connection pool
///
/// Basic example:
///
/// ```rust,no_run
/// # let client = redis::Client::open("redis://127.0.0.1/").unwrap();
/// let pool = r2d2::Pool::builder().max_size(5).build(client).unwrap();
/// let mut con = pool.get().unwrap();
/// let info = redis::cmd("READONLY").query(&mut *con);
///
/// ```
///

macro_rules! impl_manage_connection {
    ($client:ty, $connection:ty) => {
        impl r2d2::ManageConnection for $client {
            type Connection = $connection;
            type Error = RedisError;

            fn connect(&self) -> Result<Self::Connection, Self::Error> {
                self.get_connection()
            }

            fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
                if conn.check_connection() {
                    Ok(())
                } else {
                    Err(RedisError::from(io::Error::from(io::ErrorKind::BrokenPipe)))
                }
            }

            fn has_broken(&self, conn: &mut Self::Connection) -> bool {
                !conn.is_open()
            }
        }
    };
}

impl_manage_connection!(crate::Client, crate::Connection);

#[cfg(feature = "cluster")]
impl_manage_connection!(
    crate::cluster::ClusterClient,
    crate::cluster::ClusterConnection
);
