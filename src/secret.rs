use mcai_worker_sdk::JsonSchema;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Secret {
  #[serde(rename = "ftp")]
  Ftp {
    hostname: String,
    port: Option<u16>,
    secure: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    prefix: Option<String>,
  },
  #[serde(rename = "http")]
  Http {
    endpoint: String,
    method: String,
    headers: String,
    body: String,
  },
  #[serde(rename = "local")]
  Local,
  #[serde(rename = "s3")]
  S3 {
    hostname: Option<String>,
    access_key_id: String,
    secret_access_key: String,
    region: Option<String>,
    bucket: String,
  },
}

impl Default for Secret {
  fn default() -> Self {
    Secret::Local
  }
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
    endpoint: "http://www.hostname.com".to_string(),
    method: "POST".to_string(),
    headers: "{\"content-type\": \"application/json\"}".to_string(),
    body: "{\"Johnny\": \"Ca$h\"}".to_string(),
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
