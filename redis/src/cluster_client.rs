use crate::cluster::ClusterConnection;

use super::{
    ConnectionAddr, ConnectionInfo, ErrorKind, IntoConnectionInfo, RedisError, RedisResult,
};

/// Used to configure and build a [`ClusterClient`].
pub struct ClusterClientBuilder {
    initial_nodes: RedisResult<Vec<ConnectionInfo>>,
    read_from_replicas: bool,
    username: Option<String>,
    password: Option<String>,
}

impl ClusterClientBuilder {
    /// Creates a new `ClusterClientBuilder` with the the provided initial_nodes.
    ///
    /// This is the same as `ClusterClient::builder(initial_nodes)`.
    pub fn new<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> ClusterClientBuilder {
        ClusterClientBuilder {
            initial_nodes: initial_nodes
                .into_iter()
                .map(|x| x.into_connection_info())
                .collect(),
            read_from_replicas: false,
            username: None,
            password: None,
        }
    }

    /// Creates a new [`ClusterClient`] with the parameters.
    ///
    /// This does not create connections to the Redis Cluster, but only performs some basic checks
    /// on the initial nodes' URLs and passwords/usernames.
    ///
    /// # Errors
    ///
    /// Upon failure to parse initial nodes or if the initial nodes have different passwords or
    /// usernames, an error is returned.
    pub fn build(self) -> RedisResult<ClusterClient> {
        let initial_nodes = self.initial_nodes?;

        let mut nodes = Vec::with_capacity(initial_nodes.len());
        let mut connection_info_password = None::<String>;
        let mut connection_info_username = None::<String>;

        for (index, info) in initial_nodes.into_iter().enumerate() {
            if let ConnectionAddr::Unix(_) = info.addr {
                return Err(RedisError::from((ErrorKind::InvalidClientConfig,
                                         "This library cannot use unix socket because Redis's cluster command returns only cluster's IP and port.")));
            }

            if self.password.is_none() {
                if index == 0 {
                    connection_info_password = info.redis.password.clone();
                } else if connection_info_password != info.redis.password {
                    return Err(RedisError::from((
                        ErrorKind::InvalidClientConfig,
                        "Cannot use different password among initial nodes.",
                    )));
                }
            }

            if self.username.is_none() {
                if index == 0 {
                    connection_info_username = info.redis.username.clone();
                } else if connection_info_username != info.redis.username {
                    return Err(RedisError::from((
                        ErrorKind::InvalidClientConfig,
                        "Cannot use different username among initial nodes.",
                    )));
                }
            }

            nodes.push(info);
        }

        Ok(ClusterClient {
            initial_nodes: nodes,
            read_from_replicas: self.read_from_replicas,
            username: self.username.or(connection_info_username),
            password: self.password.or(connection_info_password),
        })
    }

    /// Set password for new ClusterClient.
    pub fn password(mut self, password: String) -> ClusterClientBuilder {
        self.password = Some(password);
        self
    }

    /// Set username for new ClusterClient.
    pub fn username(mut self, username: String) -> ClusterClientBuilder {
        self.username = Some(username);
        self
    }

    /// Enable read from replicas for new ClusterClient (default is false).
    ///
    /// If True, then read queries will go to the replica nodes & write queries will go to the
    /// primary nodes. If there are no replica nodes, then all queries will go to the primary nodes.
    pub fn read_from_replicas(mut self) -> ClusterClientBuilder {
        self.read_from_replicas = true;
        self
    }

    /// Use `build()`.
    #[deprecated(since = "0.22.0", note = "Use build()")]
    pub fn open(self) -> RedisResult<ClusterClient> {
        self.build()
    }

    /// Use `read_from_replicas()`.
    #[deprecated(since = "0.22.0", note = "Use read_from_replicas()")]
    pub fn readonly(mut self, read_from_replicas: bool) -> ClusterClientBuilder {
        self.read_from_replicas = read_from_replicas;
        self
    }
}

/// This is a Redis cluster client.
pub struct ClusterClient {
    initial_nodes: Vec<ConnectionInfo>,
    read_from_replicas: bool,
    username: Option<String>,
    password: Option<String>,
}

impl ClusterClient {
    /// Creates a `ClusterClient` with the default parameters.
    ///
    /// This does not create connections to the Redis Cluster, but only performs some basic checks
    /// on the initial nodes' URLs and passwords/usernames.
    ///
    /// # Errors
    ///
    /// Upon failure to parse initial nodes or if the initial nodes have different passwords or
    /// usernames, an error is returned.
    pub fn new<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> RedisResult<ClusterClient> {
        ClusterClientBuilder::new(initial_nodes).build()
    }

    /// Creates a [`ClusterClientBuilder`] with the the provided initial_nodes.
    pub fn builder<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> ClusterClientBuilder {
        ClusterClientBuilder::new(initial_nodes)
    }

    /// Opens connections to Redis Cluster nodes and returns a
    /// [`ClusterConnection`].
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to open connections or to create slots.
    pub fn get_connection(&self) -> RedisResult<ClusterConnection> {
        ClusterConnection::new(
            self.initial_nodes.clone(),
            self.read_from_replicas,
            self.username.clone(),
            self.password.clone(),
        )
    }

    /// Use `new()`.
    #[deprecated(since = "0.22.0", note = "Use new()")]
    pub fn open<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> RedisResult<ClusterClient> {
        ClusterClient::new(initial_nodes)
    }
}

impl Clone for ClusterClient {
    fn clone(&self) -> ClusterClient {
        ClusterClient::new(self.initial_nodes.clone()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::{ClusterClient, ClusterClientBuilder};
    use super::{ConnectionInfo, IntoConnectionInfo};

    fn get_connection_data() -> Vec<ConnectionInfo> {
        vec![
            "redis://127.0.0.1:6379".into_connection_info().unwrap(),
            "redis://127.0.0.1:6378".into_connection_info().unwrap(),
            "redis://127.0.0.1:6377".into_connection_info().unwrap(),
        ]
    }

    fn get_connection_data_with_password() -> Vec<ConnectionInfo> {
        vec![
            "redis://:password@127.0.0.1:6379"
                .into_connection_info()
                .unwrap(),
            "redis://:password@127.0.0.1:6378"
                .into_connection_info()
                .unwrap(),
            "redis://:password@127.0.0.1:6377"
                .into_connection_info()
                .unwrap(),
        ]
    }

    fn get_connection_data_with_username_and_password() -> Vec<ConnectionInfo> {
        vec![
            "redis://user1:password@127.0.0.1:6379"
                .into_connection_info()
                .unwrap(),
            "redis://user1:password@127.0.0.1:6378"
                .into_connection_info()
                .unwrap(),
            "redis://user1:password@127.0.0.1:6377"
                .into_connection_info()
                .unwrap(),
        ]
    }

    #[test]
    fn give_no_password() {
        let client = ClusterClient::new(get_connection_data()).unwrap();
        assert_eq!(client.password, None);
    }

    #[test]
    fn give_password_by_initial_nodes() {
        let client = ClusterClient::new(get_connection_data_with_password()).unwrap();
        assert_eq!(client.password, Some("password".to_string()));
    }

    #[test]
    fn give_username_and_password_by_initial_nodes() {
        let client = ClusterClient::new(get_connection_data_with_username_and_password()).unwrap();
        assert_eq!(client.password, Some("password".to_string()));
        assert_eq!(client.username, Some("user1".to_string()));
    }

    #[test]
    fn give_different_password_by_initial_nodes() {
        let result = ClusterClient::new(vec![
            "redis://:password1@127.0.0.1:6379",
            "redis://:password2@127.0.0.1:6378",
            "redis://:password3@127.0.0.1:6377",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn give_different_username_by_initial_nodes() {
        let result = ClusterClient::new(vec![
            "redis://user1:password@127.0.0.1:6379",
            "redis://user2:password@127.0.0.1:6378",
            "redis://user1:password@127.0.0.1:6377",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn give_username_password_by_method() {
        let client = ClusterClientBuilder::new(get_connection_data_with_password())
            .password("pass".to_string())
            .username("user1".to_string())
            .build()
            .unwrap();
        assert_eq!(client.password, Some("pass".to_string()));
        assert_eq!(client.username, Some("user1".to_string()));
    }
}
