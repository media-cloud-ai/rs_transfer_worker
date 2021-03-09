use crate::{message::StreamData, writer::StreamWriter};
use async_std::{sync::Receiver, task};
use async_trait::async_trait;
use mcai_worker_sdk::job::JobResult;
use mcai_worker_sdk::{info, publish_job_progression, McaiChannel};
use rusoto_s3::{
  CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
  CreateMultipartUploadRequest, UploadPartRequest, S3,
};
use std::{
  io::{Error, ErrorKind},
  sync::mpsc,
  thread,
  time::Duration,
};
use threadpool::ThreadPool;

use crate::endpoint::s3::S3Endpoint;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

#[derive(Clone, Debug)]
pub struct S3Writer {
  pub hostname: Option<String>,
  pub access_key_id: String,
  pub secret_access_key: String,
  pub region: Option<String>,
  pub bucket: String,
  pub runtime: Arc<Mutex<Runtime>>,
}

impl S3Endpoint for S3Writer {
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

impl S3Writer {
  pub async fn start_multi_part_s3_upload(&self, path: &str) -> Result<String, Error> {
    let request = CreateMultipartUploadRequest {
      bucket: self.bucket.clone(),
      key: path.to_string(),
      ..Default::default()
    };

    let client = self.get_s3_client()?;

    let handler = self.runtime.clone().lock().unwrap().spawn(async move {
      client
        .create_multipart_upload(request)
        .await
        .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))
    });

    let object = handler
      .await
      .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))??;

    object
      .upload_id
      .ok_or_else(|| Error::new(ErrorKind::ConnectionRefused, "error"))
  }

  pub async fn upload_s3_part(
    &self,
    path: &str,
    upload_id: &str,
    part_number: i64,
    data: Vec<u8>,
  ) -> Result<CompletedPart, Error> {
    let request = UploadPartRequest {
      body: Some(rusoto_core::ByteStream::from(data)),
      bucket: self.bucket.clone(),
      key: path.to_string(),
      upload_id: upload_id.to_string(),
      part_number,
      ..Default::default()
    };

    let client = self.get_s3_client()?;

    let handler = self.runtime.clone().lock().unwrap().spawn(async move {
      client
        .upload_part(request)
        .await
        .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))
    });

    let object = handler
      .await
      .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))??;

    Ok(CompletedPart {
      e_tag: object.e_tag,
      part_number: Some(part_number),
    })
  }

  pub async fn complete_s3_upload(
    &self,
    path: &str,
    upload_id: &str,
    parts: Vec<CompletedPart>,
  ) -> Result<(), Error> {
    let request = CompleteMultipartUploadRequest {
      bucket: self.bucket.clone(),
      key: path.to_string(),
      upload_id: upload_id.to_string(),
      multipart_upload: Some(CompletedMultipartUpload { parts: Some(parts) }),
      ..Default::default()
    };

    let client = self.get_s3_client()?;

    let handler = self.runtime.clone().lock().unwrap().spawn(async move {
      client
        .complete_multipart_upload(request)
        .await
        .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))
    });

    handler
      .await
      .map_err(|e| Error::new(ErrorKind::ConnectionRefused, e))??;

    Ok(())
  }
}

#[async_trait]
impl StreamWriter for S3Writer {
  async fn write_stream(
    &self,
    path: &str,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job_result: JobResult,
  ) -> Result<(), Error> {
    let upload_identifier = self.start_multi_part_s3_upload(path).await?;

    let mut part_number = 1;

    // limited to 10000 parts
    let part_size = std::env::var("S3_WRITER_PART_SIZE")
      .map(|buffer_size| buffer_size.parse::<usize>())
      .unwrap_or_else(|_| Ok(10 * 1024 * 1024))
      .unwrap_or_else(|_| 10 * 1024 * 1024);

    let mut part_buffer: Vec<u8> = Vec::with_capacity(part_size);

    let n_workers = std::env::var("S3_WRITER_WORKERS")
      .map(|buffer_size| buffer_size.parse::<usize>())
      .unwrap_or_else(|_| Ok(4))
      .unwrap_or(4);

    let mut n_jobs = 0;
    let pool = ThreadPool::new(n_workers);

    let mut file_size = None;
    let mut received_bytes = 0;
    let mut prev_percent = 0;
    let mut min_size = std::usize::MAX;
    let mut max_size = 0;

    let (tx, rx) = mpsc::channel();

    loop {
      let mut stream_data = receiver.recv().await;
      match stream_data {
        Ok(StreamData::Size(size)) => file_size = Some(size),
        Ok(StreamData::Eof) => {
          n_jobs += 1;
          let cloned_writer = self.clone();
          let cloned_upload_identifier = upload_identifier.clone();
          let cloned_tx = tx.clone();
          let cloned_part_buffer = part_buffer.clone();
          let cloned_path = path.to_string();

          task::block_on(async {
            let part_id = cloned_writer
              .upload_s3_part(
                &cloned_path,
                &cloned_upload_identifier,
                part_number,
                cloned_part_buffer,
              )
              .await
              .expect("unable to upload s3 part");
            cloned_tx
              .send(part_id)
              .expect("channel will be there waiting for the pool");
          });

          let mut complete_parts = rx.iter().take(n_jobs).collect::<Vec<CompletedPart>>();
          complete_parts.sort_by(|part1, part2| part1.part_number.cmp(&part2.part_number));

          self
            .complete_s3_upload(path, &upload_identifier, complete_parts)
            .await?;

          info!(target: &job_result.get_str_job_id(), "packet size: min = {}, max= {}", min_size, max_size);
          return Ok(());
        }
        Ok(StreamData::Data(ref mut data)) => {
          min_size = std::cmp::min(data.len(), min_size);
          max_size = std::cmp::max(data.len(), max_size);

          received_bytes += data.len();
          if let Some(file_size) = file_size {
            let percent = received_bytes as f32 / file_size as f32 * 100.0;

            if percent as u8 > prev_percent {
              prev_percent = percent as u8;
              publish_job_progression(channel.clone(), job_result.get_job_id(), percent as u8)
                .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))?;
            }
          }

          part_buffer.append(data);

          if part_buffer.len() > part_size {
            let cloned_writer = self.clone();
            let upload_identifier = upload_identifier.clone();
            let cloned_tx = tx.clone();
            let cloned_part_buffer = part_buffer.clone();
            let cloned_path = path.to_string();

            while pool.queued_count() > 1 {
              thread::sleep(Duration::from_millis(500));
            }

            task::block_on(async {
              let part_id = cloned_writer
                .upload_s3_part(
                  &cloned_path,
                  &upload_identifier,
                  part_number,
                  cloned_part_buffer,
                )
                .await
                .expect("unable to upload s3 part");

              cloned_tx
                .send(part_id)
                .expect("channel will be there waiting for the pool");
            });

            n_jobs += 1;

            part_number += 1;
            part_buffer.clear();
          }
        }
        _ => {}
      }
    }
  }
}

#[test]
pub fn test_s3_writer_getters() {
  let hostname = Some("s3.server.name".to_string());
  let access_key_id = "S3_ACCESS_KEY".to_string();
  let secret_access_key = "S3_SECRET_KEY".to_string();
  let region = Some("s3-region".to_string());
  let bucket = "s3-bucket".to_string();

  let runtime = Arc::new(Mutex::new(Runtime::new().unwrap()));

  let s3_writer = S3Writer {
    hostname: hostname.clone(),
    access_key_id: access_key_id.clone(),
    secret_access_key: secret_access_key.clone(),
    region: region.clone(),
    bucket: bucket.clone(),
    runtime,
  };

  assert_eq!(s3_writer.get_hostname(), hostname);
  assert_eq!(s3_writer.get_access_key(), access_key_id);
  assert_eq!(s3_writer.get_secret_key(), secret_access_key);
  assert_eq!(s3_writer.get_region_as_string(), region);
}
