use futures::io::ErrorKind;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Method;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Error;
use std::str::FromStr;
use url::Url;

pub fn get_method(method: &str) -> Result<Method, Error> {
  Method::from_str(&method.to_uppercase()).map_err(|_invalid_method| {
    Error::new(
      ErrorKind::InvalidData,
      format!("Invalid '{}' HTTP method.", method),
    )
  })
}

pub fn get_url(endpoint: &str) -> Result<Url, Error> {
  if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
    Url::parse(endpoint).map_err(|parse_error| {
      Error::new(
        ErrorKind::InvalidData,
        format!("Failed to parse '{}' URL: {}", endpoint, parse_error),
      )
    })
  } else {
    Err(Error::new(
      ErrorKind::InvalidData,
      format!(
        "Invalid '{}' URL, expected HTTP URL base ('http' or 'https').",
        endpoint
      ),
    ))
  }
}

pub fn get_headers(json_headers: &str) -> Result<HeaderMap, Error> {
  let mut headers = HeaderMap::new();
  let headers_map: HashMap<String, Value> = serde_json::from_str(json_headers).unwrap();
  for (key, value) in headers_map.iter() {
    let header_name = HeaderName::from_str(key).unwrap();
    let header_value = get_header_value(value)?;
    headers.append(header_name, header_value);
  }
  Ok(headers)
}

fn get_header_value(json_value: &Value) -> Result<HeaderValue, Error> {
  match json_value {
    Value::Bool(boolean) => get_header_value(&Value::String(boolean.to_string())),
    Value::Number(number) => {
      if number.is_u64() {
        Ok(HeaderValue::from(number.as_u64().unwrap()))
      } else if number.is_i64() {
        Ok(HeaderValue::from(number.as_i64().unwrap()))
      } else {
        get_header_value(&Value::String(number.as_f64().unwrap().to_string()))
      }
    }
    Value::String(string) => HeaderValue::from_str(string).map_err(|error| {
      Error::new(
        ErrorKind::Other,
        format!(
          "Cannot parse '{:?}' HTTP header value: {}",
          json_value, error
        ),
      )
    }),
    _ => Err(Error::new(
      ErrorKind::Other,
      format!("Unsupported '{:?}' HTTP header value format.", json_value),
    )),
  }
}

#[test]
pub fn test_get_method() {
  assert_eq!(Method::GET, get_method("GET").unwrap());
  assert_eq!(Method::GET, get_method("get").unwrap());
  assert_eq!(Method::GET, get_method("Get").unwrap());
  assert_eq!(Method::PUT, get_method("PUT").unwrap());
  assert_eq!(Method::PUT, get_method("put").unwrap());
  assert_eq!(Method::PUT, get_method("Put").unwrap());
  assert_eq!(Method::POST, get_method("POST").unwrap());
  assert_eq!(Method::POST, get_method("post").unwrap());
  assert_eq!(Method::POST, get_method("Post").unwrap());
  // etc..

  let error = get_method("get ").unwrap_err();
  let expected = Error::new(
    ErrorKind::InvalidData,
    "Invalid 'get ' HTTP method.".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());
}

#[test]
pub fn test_get_url() {
  assert_eq!(
    "http://www.media-io.com/".to_string(),
    get_url("http://www.media-io.com").unwrap().to_string()
  );
  assert_eq!(
    "https://www.media-io.com/".to_string(),
    get_url("https://www.media-io.com").unwrap().to_string()
  );

  let error = get_url("http://media-io com").unwrap_err();
  let expected = Error::new(
    ErrorKind::InvalidData,
    "Failed to parse 'http://media-io com' URL: invalid domain character".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());

  let error = get_url("ftp://media-io.com").unwrap_err();
  let expected = Error::new(
    ErrorKind::InvalidData,
    "Invalid 'ftp://media-io.com' URL, expected HTTP URL base ('http' or 'https').".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());
}

#[test]
pub fn test_get_headers() {
  let json = "{}";
  let headers = get_headers(json).unwrap();
  assert!(headers.is_empty());

  let json = r#"{
    "content-length": 12345,
    "content-type": "application/json"
  }"#;
  let headers = get_headers(json).unwrap();
  assert_eq!(headers.len(), 2);
  assert_eq!(
    Some(&HeaderValue::from_str("12345").unwrap()),
    headers.get("content-length")
  );
  assert_eq!(
    Some(&HeaderValue::from_str("application/json").unwrap()),
    headers.get("content-type")
  );
}

#[test]
pub fn test_get_header_value() {
  use serde_json::value::{Map, Number};

  let error = get_header_value(&Value::Null).unwrap_err();
  let expected = Error::new(
    ErrorKind::Other,
    "Unsupported 'Null' HTTP header value format.".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());

  let string = "Hello there!".to_string();
  let header_value = get_header_value(&Value::String(string.clone())).unwrap();
  assert_eq!(string, header_value);

  let boolean = true;
  let header_value = get_header_value(&Value::Bool(boolean.clone())).unwrap();
  assert_eq!(boolean.to_string(), header_value);

  let unsigned_int: u32 = 123;
  let header_value = get_header_value(&Value::Number(unsigned_int.into())).unwrap();
  assert_eq!(unsigned_int.to_string(), header_value);

  let signed_int: i16 = -123;
  let header_value = get_header_value(&Value::Number(signed_int.into())).unwrap();
  assert_eq!(signed_int.to_string(), header_value);

  let float: f64 = 1.23;
  let header_value = get_header_value(&Value::Number(Number::from_f64(float).unwrap())).unwrap();
  assert_eq!(float.to_string(), header_value);

  let invalid_string = "\0\0".to_string();
  let error = get_header_value(&Value::String(invalid_string.clone())).unwrap_err();
  let expected = Error::new(
    ErrorKind::Other,
    "Cannot parse 'String(\"\\u{0}\\u{0}\")' HTTP header value: failed to parse header value"
      .to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());

  let array = vec![];
  let error = get_header_value(&Value::Array(array)).unwrap_err();
  let expected = Error::new(
    ErrorKind::Other,
    "Unsupported 'Array([])' HTTP header value format.".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());

  let object = Map::new();
  let error = get_header_value(&Value::Object(object)).unwrap_err();
  let expected = Error::new(
    ErrorKind::Other,
    "Unsupported 'Object({})' HTTP header value format.".to_string(),
  );
  assert_eq!(expected.to_string(), error.to_string());
}
