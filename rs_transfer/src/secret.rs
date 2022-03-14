use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Secret {
  Ftp {
    hostname: String,
    port: Option<u16>,
    secure: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    prefix: Option<String>,
  },
  GCS {
    credential: GcsCredential,
    bucket: String,
  },
  Http {
    endpoint: Option<String>,
    method: Option<String>,
    headers: Option<String>,
    body: Option<String>,
  },
  Local,
  S3 {
    hostname: Option<String>,
    access_key_id: String,
    secret_access_key: String,
    region: Option<String>,
    bucket: String,
  },
  Sftp {
    hostname: String,
    port: Option<u16>,
    username: String,
    password: Option<String>,
    prefix: Option<String>,
    known_host: Option<String>,
  },
  Cursor {
    content: Option<Vec<u8>>,
  },
}

impl Default for Secret {
  fn default() -> Self {
    Secret::Local
  }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct GcsCredential{
  #[serde(rename = "type")]
  gcs_type:String,
  project_id: String,
  private_key_id: String,
  private_key: String,
  client_email: String,
  client_id: String,
  auth_uri: String,
  token_uri: String,
  auth_provider_x509_cert_url: String,
  client_x509_cert_url: String,

}

#[test]
pub fn test_secret_default() {
  assert_eq!(Secret::default(), Secret::Local {});
}

#[test]
pub fn test_secret_ftp() {
  let json_str = r#"{
    "type": "ftp",
    "hostname": "ftp://ftp_server_name",
    "username": "Johnny",
    "password": "B_g00d"
  }"#;
  let expected = Secret::Ftp {
    hostname: "ftp://ftp_server_name".to_string(),
    port: None,
    secure: None,
    username: Some("Johnny".to_string()),
    password: Some("B_g00d".to_string()),
    prefix: None,
  };
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}

#[test]
pub fn test_secret_http() {
  let json_str = r#"{
    "type": "http",
    "endpoint": "http://www.hostname.com",
    "method": "POST",
    "headers": "{\"content-type\": \"application/json\"}",
    "body": "{\"Johnny\": \"Ca$h\"}"
  }"#;
  let expected = Secret::Http {
    endpoint: Some("http://www.hostname.com".to_string()),
    method: Some("POST".to_string()),
    headers: Some("{\"content-type\": \"application/json\"}".to_string()),
    body: Some("{\"Johnny\": \"Ca$h\"}".to_string()),
  };
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}

#[test]
pub fn test_secret_local() {
  let json_str = r#"{
    "type": "local"
  }"#;
  let expected = Secret::Local {};
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}

#[test]
pub fn test_secret_cursor() {
  let json_str = r#"{
    "type": "cursor"
  }"#;
  let expected = Secret::Cursor { content: None };
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}

#[test]
pub fn test_secret_s3() {
  let json_str = r#"{
    "type": "s3",
    "hostname": "s3.server.name",
    "access_key_id": "123_ACCESS_KEY",
    "secret_access_key": "456_SECRET_KEY",
    "bucket": "johnny"
  }"#;
  let expected = Secret::S3 {
    hostname: Some("s3.server.name".to_string()),
    access_key_id: "123_ACCESS_KEY".to_string(),
    secret_access_key: "456_SECRET_KEY".to_string(),
    region: None,
    bucket: "johnny".to_string(),
  };
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}

#[test]
pub fn test_secret_sftp() {
  let json_str = r#"{
    "type": "sftp",
    "hostname": "127.0.0.1",
    "username": "Johnny",
    "password": "B_g00d"
  }"#;
  let expected = Secret::Sftp {
    hostname: "127.0.0.1".to_string(),
    port: None,
    username: "Johnny".to_string(),
    password: Some("B_g00d".to_string()),
    prefix: None,
    known_host: None,
  };
  let secret: Secret = serde_json::from_str(json_str).unwrap();
  assert_eq!(secret, expected);
}
