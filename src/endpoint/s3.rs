use rusoto_core::{HttpClient, Region};
use rusoto_credential::StaticProvider;
use rusoto_s3::S3Client;
use std::io::{Error, ErrorKind};
use std::str::FromStr;

pub trait S3Endpoint {
  fn get_hostname(&self) -> Option<String>;
  fn get_access_key(&self) -> String;
  fn get_secret_key(&self) -> String;
  fn get_region_as_string(&self) -> Option<String>;

  fn get_region(&self) -> Region {
    if let Some(hostname) = self.get_hostname() {
      if let Some(region) = self.get_region_as_string() {
        return Region::Custom {
          name: region,
          endpoint: hostname,
        };
      }
    } else if let Some(region) = self.get_region_as_string() {
      return Region::from_str(&region).unwrap_or_default();
    }
    Region::default()
  }

  fn get_s3_client(&self) -> Result<S3Client, Error> {
    let access_key = self.get_access_key();
    let secret_key = self.get_secret_key();
    let region = self.get_region();

    Ok(S3Client::new_with(
      HttpClient::new().map_err(|_| {
        Error::new(
          ErrorKind::ConnectionRefused,
          "Unable to create HTTP client".to_string(),
        )
      })?,
      StaticProvider::new_minimal(access_key, secret_key),
      region,
    ))
  }
}
