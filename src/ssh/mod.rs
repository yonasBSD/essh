use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use russh::client::{self, Handle, Msg};
use russh::keys::ssh_key;
use russh::keys::PrivateKeyWithHashAlg;
use russh::{Channel, Disconnect};
use tokio::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum SshError {
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Host key error: {0}")]
    HostKey(String),
    #[error("Channel error: {0}")]
    Channel(String),
    #[error(transparent)]
    Russh(#[from] russh::Error),
    #[error("Key error: {0}")]
    Key(russh::keys::Error),
}

impl From<russh::keys::Error> for SshError {
    fn from(e: russh::keys::Error) -> Self {
        SshError::Key(e)
    }
}

#[derive(Clone, Debug)]
pub struct ConnectConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
}

impl ConnectConfig {
    pub fn new(hostname: String, username: String, auth: AuthMethod) -> Self {
        Self {
            hostname,
            port: 22,
            username,
            auth,
        }
    }
}

#[derive(Clone, Debug)]
pub enum AuthMethod {
    Password(String),
    KeyFile(PathBuf),
    Agent,
}

pub struct ClientHandler {
    host_key_fingerprint: Arc<Mutex<Option<String>>>,
    server_banner: Arc<Mutex<Option<String>>>,
}

impl client::Handler for ClientHandler {
    type Error = SshError;

    fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send {
        let fingerprint = server_public_key
            .fingerprint(ssh_key::HashAlg::Sha256)
            .to_string();
        let fp_store = self.host_key_fingerprint.clone();
        async move {
            *fp_store.lock().await = Some(fingerprint);
            Ok(true)
        }
    }

    fn auth_banner(
        &mut self,
        banner: &str,
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let banner = banner.to_string();
        let banner_store = self.server_banner.clone();
        async move {
            *banner_store.lock().await = Some(banner);
            Ok(())
        }
    }

    fn server_channel_open_forwarded_tcpip(
        &mut self,
        _channel: Channel<Msg>,
        _connected_address: &str,
        _connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async { Ok(()) }
    }
}

pub struct SshSession {
    pub handle: Handle<ClientHandler>,
    session_id: String,
}

pub struct SshClient;

impl SshClient {
    pub async fn connect(
        config: &ConnectConfig,
    ) -> Result<(SshSession, String, Option<String>), SshError> {
        let ssh_config = Arc::new(client::Config::default());

        let host_key_fingerprint = Arc::new(Mutex::new(None));
        let server_banner = Arc::new(Mutex::new(None));

        let handler = ClientHandler {
            host_key_fingerprint: host_key_fingerprint.clone(),
            server_banner: server_banner.clone(),
        };

        let mut handle = client::connect(
            ssh_config,
            (config.hostname.as_str(), config.port),
            handler,
        )
        .await?;

        let auth_result = match &config.auth {
            AuthMethod::Password(pw) => handle
                .authenticate_password(&config.username, pw)
                .await
                .map_err(SshError::Russh)?,
            AuthMethod::KeyFile(path) => {
                let key = russh::keys::load_secret_key(path, None)?;
                let key = PrivateKeyWithHashAlg::new(Arc::new(key), None);
                handle
                    .authenticate_publickey(&config.username, key)
                    .await
                    .map_err(SshError::Russh)?
            }
            AuthMethod::Agent => {
                return Err(SshError::Auth("Agent auth not yet implemented".into()));
            }
        };

        if !auth_result.success() {
            return Err(SshError::Auth("Authentication failed".into()));
        }

        let fingerprint = host_key_fingerprint
            .lock()
            .await
            .take()
            .ok_or_else(|| SshError::HostKey("No host key fingerprint captured".into()))?;

        let banner = server_banner.lock().await.take();

        let session_id = uuid::Uuid::new_v4().to_string();

        Ok((
            SshSession {
                handle,
                session_id,
            },
            fingerprint,
            banner,
        ))
    }
}

impl SshSession {
    pub async fn open_shell(
        &mut self,
        term: &str,
        cols: u32,
        rows: u32,
    ) -> Result<Channel<Msg>, SshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(SshError::Russh)?;

        channel
            .request_pty(false, term, cols, rows, 0, 0, &[])
            .await
            .map_err(SshError::Russh)?;

        channel
            .request_shell(false)
            .await
            .map_err(SshError::Russh)?;

        Ok(channel)
    }

    pub async fn close(self) -> Result<(), SshError> {
        self.handle
            .disconnect(Disconnect::ByApplication, "goodbye", "en")
            .await
            .map_err(SshError::Russh)?;
        Ok(())
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}
