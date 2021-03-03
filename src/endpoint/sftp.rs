use dirs::home_dir;
use mcai_worker_sdk::{debug, info, warn};
use ssh2::Sftp;
use std::net::TcpStream;

pub trait SftpEndpoint {
  fn get_hostname(&self) -> String;
  fn get_port(&self) -> u16;
  fn get_username(&self) -> String;
  fn get_password(&self) -> Option<String>;

  fn get_sftp_stream(&self) -> Result<Sftp, String> {
    let port = self.get_port();
    let username = self.get_username();
    let password = self.get_password();

    debug!(
      "Attempting to connect to {}:{}.",
      &self.get_hostname(),
      port
    );
    let tcp_stream =
      TcpStream::connect((self.get_hostname().as_str(), port)).map_err(|e| e.to_string())?;

    let mut session = ssh2::Session::new().map_err(|e| e.to_string())?;
    session.set_timeout(10000);
    session.set_compress(true);
    session.set_tcp_stream(tcp_stream);
    session.handshake().map_err(|e| e.to_string())?;

    check_remote_host(&mut session, &self.get_hostname(), port)?;
    authenticate(&mut session, &username, password)?;

    info!(
      "Connected to host {}@{}:{}.",
      username,
      self.get_hostname(),
      port
    );

    let sftp = session.sftp().map_err(|e| e.to_string())?;
    Ok(sftp)
  }
}

fn check_remote_host(
  session: &mut ssh2::Session,
  destination: &str,
  port: u16,
) -> Result<(), String> {
  let mut known_hosts = session.known_hosts().map_err(|e| e.to_string())?;
  let known_hosts_path = home_dir()
    .ok_or_else(|| "Unable to find home directory".to_string())?
    .join(".ssh/known_hosts");

  known_hosts
    .read_file(&known_hosts_path, ssh2::KnownHostFileKind::OpenSSH)
    .map_err(|e| e.to_string())?;

  let (key, key_type) = session.host_key().ok_or_else(|| "Host key not found".to_string())?;

  match known_hosts.check_port(destination, port, key) {
    ssh2::CheckResult::Match => {
      debug!(
        "Host key for {}:{} matches entry in {:?}.",
        destination, port, known_hosts_path
      );
      Ok(())
    }
    ssh2::CheckResult::NotFound => {
      let fingerprint = session
        .host_key_hash(ssh2::HashType::Sha256)
        .map(|hash| ("SHA256", hash))
        .or_else(|| {
          session
            .host_key_hash(ssh2::HashType::Sha1)
            .map(|hash| ("SHA128", hash))
        })
        .map(|(hash_type, fingerprint)| format!("{}:{}", hash_type, base64::encode(fingerprint)))
        .ok_or_else(|| "Host fingerprint not found".to_string())?;

      info!(
        "No matching host key for {}:{} was not found in {:?}.",
        destination, port, known_hosts_path
      );

      // TODO Ask before adding fingerprint to known hosts?
      warn!("Add fingerprint to known hosts: {}", fingerprint);
      known_hosts
        .add(destination, key, "", key_type.into())
        .map_err(|e| e.to_string())?;

      known_hosts
        .write_file(&known_hosts_path, ssh2::KnownHostFileKind::OpenSSH)
        .map_err(|e| e.to_string())?;
      Ok(())
    }
    ssh2::CheckResult::Mismatch => {
      warn!("####################################################");
      warn!("# WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! #");
      warn!("####################################################");
      Err(format!(
        "Fingerprint for '{}' host mismatched!",
        destination
      ))
    }
    ssh2::CheckResult::Failure => Err(format!("Host file check failed for '{}'!", destination)),
  }
}

fn authenticate(
  session: &mut ssh2::Session,
  username: &str,
  password: Option<String>,
) -> Result<(), String> {
  if session.authenticated() {
    return Ok(());
  }

  let auth_methods: &str = session.auth_methods(username).map_err(|e| e.to_string())?;

  debug!("Session authentication methods: {}", auth_methods);

  if auth_methods.contains("publickey") {
    debug!("Try authenticating with SSH key...");
    let _result = session.userauth_agent(username).map_err(|e| e.to_string());
  }

  if !session.authenticated() && auth_methods.contains("password") {
    if let Some(password) = password {
      debug!("Try authenticating with password...");
      session
        .userauth_password(username, &password)
        .map_err(|e| e.to_string())?;
    }
  }

  if session.authenticated() {
    Ok(())
  } else {
    Err(format!("Authentication failed for user: {}", username))
  }
}
