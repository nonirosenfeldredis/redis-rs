use crate::cluster::ClusterConnection;
#[cfg(feature = "tls")]
use crate::tls::{Certificate, RedisIdentity};

use super::{
    ConnectionAddr, ConnectionInfo, ErrorKind, IntoConnectionInfo, RedisError, RedisResult,
};

/// Used to configure and build a [ClusterClient](ClusterClient).
pub struct ClusterClientBuilder {
    initial_nodes: RedisResult<Vec<ConnectionInfo>>,
    readonly: bool,
    username: Option<String>,
    password: Option<String>,
    #[cfg(feature = "tls")]
    pub(crate) ca_cert: Option<Certificate>,
    #[cfg(feature = "tls")]
    pub(crate) identity: Option<RedisIdentity>,
}

impl ClusterClientBuilder {
    /// Generate the base configuration for new Client.
    pub fn new<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> ClusterClientBuilder {
        ClusterClientBuilder {
            initial_nodes: initial_nodes
                .into_iter()
                .map(|x| x.into_connection_info())
                .collect(),
            readonly: false,
            username: None,
            password: None,
            #[cfg(feature = "tls")]
            ca_cert: None,
            #[cfg(feature = "tls")]
            identity: None
        }
    }

    /// Builds a [ClusterClient](ClusterClient). Despite the name, this does not actually open
    /// a connection to Redis Cluster, but will perform some basic checks of the initial
    /// nodes' URLs and passwords.
    ///
    /// # Errors
    ///
    /// Upon failure to parse initial nodes or if the initial nodes have different passwords,
    /// an error is returned.
    pub fn open(self) -> RedisResult<ClusterClient> {
        ClusterClient::build(self)
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

    /// Set read only mode for new ClusterClient (default is false).
    /// If readonly is true, all queries will go to replica nodes. If there are no replica nodes,
    /// queries will be issued to the primary nodes.

    pub fn readonly(mut self, readonly: bool) -> ClusterClientBuilder {
        self.readonly = readonly;
        self
    }

    /// relevant for secure TLS : Set ca certificate for new ClusterClient.
    #[cfg(feature = "tls")]
    pub fn ca_cert(mut self, ca_cert: Certificate) -> ClusterClientBuilder {
        self.ca_cert = Some(ca_cert);
        self
    }

    /// relevant for secure TLS : Set identity ( certificate & private key) for new ClusterClient.
    #[cfg(feature = "tls")]
    pub fn identity(mut self, identity: RedisIdentity) -> ClusterClientBuilder {
        self.identity = Some(identity);
        self
    }
}

/// This is a Redis cluster client.
pub struct ClusterClient {
    initial_nodes: Vec<ConnectionInfo>,
    readonly: bool,
    username: Option<String>,
    password: Option<String>,
    #[cfg(feature = "tls")]
    ca_cert: Option<Certificate>,
    #[cfg(feature = "tls")]
    identity: Option<RedisIdentity>
}

impl ClusterClient {
    /// Create a [ClusterClient](ClusterClient) with the default configuration. Despite the name,
    /// this does not actually open a connection to Redis Cluster, but only performs some basic
    /// checks of the initial nodes' URLs and passwords.
    ///
    /// # Errors
    ///
    /// Upon failure to parse initial nodes or if the initial nodes have different passwords,
    /// an error is returned.
    pub fn open<T: IntoConnectionInfo>(initial_nodes: Vec<T>) -> RedisResult<ClusterClient> {
        ClusterClientBuilder::new(initial_nodes).open()
    }

    /// Opens connections to Redis Cluster nodes and returns a
    /// [ClusterConnection](ClusterConnection).
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to open connections or to create slots.
    pub fn get_connection(&self) -> RedisResult<ClusterConnection> {
        ClusterConnection::new(
            self.initial_nodes.clone(),
            self.readonly,
            self.username.clone(),
            self.password.clone(),
            #[cfg(feature = "tls")]
                self.ca_cert.clone(),
            #[cfg(feature = "tls")]
                self.identity.clone()
        )
    }

    fn build(builder: ClusterClientBuilder) -> RedisResult<ClusterClient> {
        let initial_nodes = builder.initial_nodes?;
        let mut nodes = Vec::with_capacity(initial_nodes.len());
        let mut connection_info_password = None::<String>;
        let mut connection_info_username = None::<String>;
        #[cfg(feature = "tls")]
            let mut connection_info_ca_cert = None::<Certificate>;
        #[cfg(feature = "tls")]
            let mut connection_info_identity = None::<RedisIdentity>;

        for (index, info) in initial_nodes.into_iter().enumerate() {
            if let ConnectionAddr::Unix(_) = info.addr {
                return Err(RedisError::from((ErrorKind::InvalidClientConfig,
                                             "This library cannot use unix socket because Redis's cluster command returns only cluster's IP and port.")));
            }

            if builder.password.is_none() {
                if index == 0 {
                    connection_info_password = info.redis.password.clone();
                } else if connection_info_password != info.redis.password {
                    return Err(RedisError::from((
                        ErrorKind::InvalidClientConfig,
                        "Cannot use different password among initial nodes.",
                    )));
                }
            }

            if builder.username.is_none() {
                if index == 0 {
                    connection_info_username = info.redis.username.clone();
                } else if connection_info_username != info.redis.username {
                    return Err(RedisError::from((
                        ErrorKind::InvalidClientConfig,
                        "Cannot use different username among initial nodes.",
                    )));
                }
            }

            #[cfg(feature = "tls")]
            if builder.ca_cert.is_none(){
                if let ConnectionAddr::TcpTls { ref ca_cert, .. } = info.addr {
                        if index == 0 {
                            connection_info_ca_cert = (*ca_cert).clone();
                        } else if connection_info_ca_cert != (*ca_cert) {
                            return Err(RedisError::from((
                                ErrorKind::InvalidClientConfig,
                                "Cannot use different ca_cert among initial nodes.",
                            )));
                        }
                }
            }

            #[cfg(feature = "tls")]
            if builder.identity.is_none(){
                    if let ConnectionAddr::TcpTls { ref identity, .. } = info.addr {
                        if index == 0 {
                            connection_info_identity = (*identity).clone();
                        } else if connection_info_identity != (*identity) {
                            return Err(RedisError::from((
                                ErrorKind::InvalidClientConfig,
                                "Cannot use different identity among initial nodes.",
                            )));
                        }
                    }
            }

            nodes.push(info);
        }

        Ok(ClusterClient {
            initial_nodes: nodes,
            readonly: builder.readonly,
            username: builder.username.or(connection_info_username),
            password: builder.password.or(connection_info_password),
            #[cfg(feature = "tls")]
            ca_cert: builder.ca_cert.or(connection_info_ca_cert),
            #[cfg(feature = "tls")]
            identity: builder.identity.or(connection_info_identity),
        })
    }
}

impl Clone for ClusterClient {
    fn clone(&self) -> ClusterClient {
        ClusterClient::open(self.initial_nodes.clone()).unwrap()
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
        let client = ClusterClient::open(get_connection_data()).unwrap();
        assert_eq!(client.password, None);
    }

    #[test]
    fn give_password_by_initial_nodes() {
        let client = ClusterClient::open(get_connection_data_with_password()).unwrap();
        assert_eq!(client.password, Some("password".to_string()));
    }

    #[test]
    fn give_username_and_password_by_initial_nodes() {
        let client = ClusterClient::open(get_connection_data_with_username_and_password()).unwrap();
        assert_eq!(client.password, Some("password".to_string()));
        assert_eq!(client.username, Some("user1".to_string()));
    }

    #[test]
    fn give_different_password_by_initial_nodes() {
        let result = ClusterClient::open(vec![
            "redis://:password1@127.0.0.1:6379",
            "redis://:password2@127.0.0.1:6378",
            "redis://:password3@127.0.0.1:6377",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn give_different_username_by_initial_nodes() {
        let result = ClusterClient::open(vec![
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
            .open()
            .unwrap();
        assert_eq!(client.password, Some("pass".to_string()));
        assert_eq!(client.username, Some("user1".to_string()));
    }
}
