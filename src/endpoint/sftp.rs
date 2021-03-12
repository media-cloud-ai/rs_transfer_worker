use mcai_worker_sdk::debug;
use ssh_transfer::{AuthenticationType, Configuration, Connection};
use std::io::Error;

pub trait SftpEndpoint {
  fn get_hostname(&self) -> String;
  fn get_port(&self) -> u16;
  fn get_username(&self) -> String;
  fn get_password(&self) -> Option<String>;
  fn trust_host(&self) -> bool;

  fn get_sftp_stream(&self) -> Result<Connection, Error> {
    debug!(
      "Attempting to connect to {}:{}.",
      &self.get_hostname(),
      self.get_port()
    );

    if let Some(password) = self.get_password() {
      let configuration = Configuration::new(&self.get_hostname())
        .with_port(self.get_port())
        .with_username(&self.get_username())
        .with_authentication(AuthenticationType::Password(password))
        .with_host_trust(self.trust_host());

      Connection::new(&configuration).map_err(Into::<Error>::into)
    } else {
      unimplemented!()
    }
  }
}
