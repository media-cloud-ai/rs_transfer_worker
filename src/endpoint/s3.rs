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

#[test]
pub fn test_endpoint_s3_get_region() {
  struct TestS3 {}
  impl S3Endpoint for TestS3 {
    fn get_hostname(&self) -> Option<String> {
      Some("s3.server.name".to_string())
    }

    fn get_access_key(&self) -> String {
      "s3_access_key".to_string()
    }

    fn get_secret_key(&self) -> String {
      "s3_secret_key".to_string()
    }

    fn get_region_as_string(&self) -> Option<String> {
      Some("s3-region".to_string())
    }
  }

  let test_s3 = TestS3 {};
  let expected = Region::Custom {
    name: "s3-region".to_string(),
    endpoint: "s3.server.name".to_string(),
  };
  let region = test_s3.get_region();
  assert_eq!(region, expected);
}

#[test]
pub fn test_endpoint_s3_get_region_no_hostname() {
  struct TestS3 {}
  impl S3Endpoint for TestS3 {
    fn get_hostname(&self) -> Option<String> {
      None
    }

    fn get_access_key(&self) -> String {
      "s3_access_key".to_string()
    }

    fn get_secret_key(&self) -> String {
      "s3_secret_key".to_string()
    }

    fn get_region_as_string(&self) -> Option<String> {
      Some("eu-west-3".to_string())
    }
  }

  let test_s3 = TestS3 {};
  let expected = Region::EuWest3;
  let region = test_s3.get_region();
  assert_eq!(region, expected);
}

#[test]
pub fn test_endpoint_s3_get_region_no_region() {
  struct TestS3 {}
  impl S3Endpoint for TestS3 {
    fn get_hostname(&self) -> Option<String> {
      Some("s3.server.name".to_string())
    }

    fn get_access_key(&self) -> String {
      "s3_access_key".to_string()
    }

    fn get_secret_key(&self) -> String {
      "s3_secret_key".to_string()
    }

    fn get_region_as_string(&self) -> Option<String> {
      None
    }
  }

  let test_s3 = TestS3 {};
  let expected = Region::default();
  let region = test_s3.get_region();
  assert_eq!(region, expected);
}

#[test]
pub fn test_endpoint_s3_get_region_no_hostname_nor_region() {
  struct TestS3 {}
  impl S3Endpoint for TestS3 {
    fn get_hostname(&self) -> Option<String> {
      None
    }

    fn get_access_key(&self) -> String {
      "s3_access_key".to_string()
    }

    fn get_secret_key(&self) -> String {
      "s3_secret_key".to_string()
    }

    fn get_region_as_string(&self) -> Option<String> {
      None
    }
  }

  let test_s3 = TestS3 {};
  let expected = Region::default();
  let region = test_s3.get_region();
  assert_eq!(region, expected);
}
