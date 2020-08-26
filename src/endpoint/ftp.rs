use ftp::openssl::ssl::{SslContext, SslMethod};
use ftp::types::FileType;
use ftp::{FtpError, FtpStream};
use std::io::{Error, ErrorKind};

pub trait FtpEndpoint {
  fn get_hostname(&self) -> String;
  fn get_port(&self) -> u16;
  fn is_secure(&self) -> bool;
  fn get_username(&self) -> Option<String>;
  fn get_password(&self) -> Option<String>;

  fn get_ftp_stream(&self) -> Result<FtpStream, FtpError> {
    let mut ftp_stream = FtpStream::connect((self.get_hostname().as_str(), self.get_port()))?;
    if self.is_secure() {
      let builder = SslContext::builder(SslMethod::tls()).map_err(|_e| {
        FtpError::ConnectionError(Error::new(ErrorKind::Other, "unable to build SSL context"))
      })?;
      let context = builder.build();
      // Switch to secure mode
      ftp_stream = ftp_stream.into_secure(context)?;
    }

    if let (Some(username), Some(password)) = (self.get_username(), self.get_password()) {
      ftp_stream.login(username.as_str(), password.as_str())?;
    }

    ftp_stream.transfer_type(FileType::Binary)?;
    Ok(ftp_stream)
  }
}
