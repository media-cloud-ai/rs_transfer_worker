use crate::endpoint::s3::S3Endpoint;
use crate::{message::StreamData, reader::StreamReader};
use async_std::sync::Sender;
use async_trait::async_trait;
use rusoto_s3::{GetObjectRequest, HeadObjectRequest, S3Client, S3};
use std::io::{Error, ErrorKind, Read};

pub struct S3Reader {
  pub hostname: Option<String>,
  pub access_key_id: String,
  pub secret_access_key: String,
  pub region: Option<String>,
  pub bucket: String,
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

    let head = client
      .head_object(head_request)
      .sync()
      .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

    if let Some(file_size) = head.content_length {
      sender.send(StreamData::Size(file_size as u64)).await;
    }
    let object = client.get_object(request).sync();

    let object = object.map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

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
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let cloned_bucket = self.bucket.clone();
    let cloned_path = path.to_string();
    let client = self
      .get_s3_client()
      .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))?;

    let ret = runtime.spawn(async move {
      S3Reader::read_file(client, &cloned_path, &cloned_bucket, sender).await
    });

    ret.await.map_err(|e| Error::new(ErrorKind::Other, e))?
  }
}
