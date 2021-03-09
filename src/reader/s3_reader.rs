use crate::endpoint::s3::S3Endpoint;
use crate::{message::StreamData, reader::StreamReader};
use async_std::sync::Sender;
use async_trait::async_trait;
use rusoto_s3::{GetObjectRequest, HeadObjectRequest, S3Client, S3};
use std::io::{Error, ErrorKind, Read};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub struct S3Reader {
  pub hostname: Option<String>,
  pub access_key_id: String,
  pub secret_access_key: String,
  pub region: Option<String>,
  pub bucket: String,
  pub runtime: Arc<Mutex<Runtime>>,
}

impl S3Endpoint for S3Reader {
  fn get_hostname(&self) -> Option<String> {
    self.hostname.clone()
  }
  fn get_access_key(&self) -> String {
    self.access_key_id.clone()
  }
  fn get_secret_key(&self) -> String {
    self.secret_access_key.clone()
  }
  fn get_region_as_string(&self) -> Option<String> {
    self.region.clone()
  }
}

impl S3Reader {
  async fn read_file(
    &self,
    client: S3Client,
    path: &str,
    bucket: &str,
    sender: Sender<StreamData>,
  ) -> Result<(), Error> {
    let head_request = HeadObjectRequest {
      bucket: bucket.to_string(),
      key: path.to_string(),
      ..Default::default()
    };

    let request = GetObjectRequest {
      bucket: bucket.to_string(),
      key: path.to_string(),
      ..Default::default()
    };

    let cloned_client = client.clone();
    let handler = self.runtime.clone().lock().unwrap().spawn(async move {
      cloned_client
        .head_object(head_request)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))
    });
    let head = handler.await??;

    if let Some(file_size) = head.content_length {
      sender.send(StreamData::Size(file_size as u64)).await;
    }
    let handler = self.runtime.clone().lock().unwrap().spawn(async move {
      client
        .get_object(request)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))
    });

    let object = handler.await??;

    let s3_byte_stream = object
      .body
      .ok_or_else(|| Error::new(ErrorKind::Other, "No retrieved object data to access."))?;
    let mut reader = s3_byte_stream.into_blocking_read();

    let buffer_size = if let Ok(buffer_size) = std::env::var("S3_READER_BUFFER_SIZE") {
      buffer_size.parse::<u32>().map_err(|_| {
        Error::new(
          ErrorKind::Other,
          "Unable to parse S3_READER_BUFFER_SIZE variable",
        )
      })? as usize
    } else {
      1024 * 1024
    };

    loop {
      let mut buffer: Vec<u8> = vec![0; buffer_size];
      let size = reader.read(&mut buffer)?;
      if size == 0 {
        break;
      }

      sender
        .send(StreamData::Data(buffer[0..size].to_vec()))
        .await;
    }
    sender.send(StreamData::Eof).await;
    Ok(())
  }
}

#[async_trait]
impl StreamReader for S3Reader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error> {
    let cloned_bucket = self.bucket.clone();
    let cloned_path = path.to_string();
    let client = self
      .get_s3_client()
      .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

    self
      .read_file(client, &cloned_path, &cloned_bucket, sender)
      .await
      .map_err(|e| Error::new(ErrorKind::Other, e))
  }
}

#[test]
pub fn test_s3_reader_getters() {
  let hostname = Some("s3.server.name".to_string());
  let access_key_id = "S3_ACCESS_KEY".to_string();
  let secret_access_key = "S3_SECRET_KEY".to_string();
  let region = Some("s3-region".to_string());
  let bucket = "s3-bucket".to_string();

  let runtime = Arc::new(Mutex::new(Runtime::new().unwrap()));

  let s3_reader = S3Reader {
    hostname: hostname.clone(),
    access_key_id: access_key_id.clone(),
    secret_access_key: secret_access_key.clone(),
    region: region.clone(),
    bucket,
    runtime,
  };

  assert_eq!(s3_reader.get_hostname(), hostname);
  assert_eq!(s3_reader.get_access_key(), access_key_id);
  assert_eq!(s3_reader.get_secret_key(), secret_access_key);
  assert_eq!(s3_reader.get_region_as_string(), region);
}
