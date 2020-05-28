use crate::{message::StreamData, target_configuration::TargetConfiguration, writer::StreamWriter};
use async_std::{sync::Receiver, task};
use async_trait::async_trait;
use mcai_worker_sdk::{info, job::Job, publish_job_progression, McaiChannel};
use rusoto_s3::CompletedPart;
use std::{
  io::{Error, ErrorKind},
  sync::mpsc,
  thread,
  time::Duration,
};
use threadpool::ThreadPool;

#[derive(Clone, Debug)]
pub struct S3Writer {}

#[async_trait]
impl StreamWriter for S3Writer {
  async fn write_stream(
    &self,
    target: TargetConfiguration,
    receiver: Receiver<StreamData>,
    channel: Option<McaiChannel>,
    job: &Job,
  ) -> Result<(), Error> {
    let upload_identifier = target.start_multi_part_s3_upload().await?;
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
      .unwrap_or_else(|_| 4);

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
          let cloned_target = target.clone();
          let cloned_upload_identifier = upload_identifier.clone();
          let cloned_tx = tx.clone();
          let cloned_part_buffer = part_buffer.clone();

          pool.execute(move || {
            task::block_on(async {
              let part_id = cloned_target
                .upload_s3_part(&cloned_upload_identifier, part_number, cloned_part_buffer)
                .await
                .expect("unable to upload s3 part");
              cloned_tx
                .send(part_id)
                .expect("channel will be there waiting for the pool");
            })
          });

          let mut complete_parts = rx.iter().take(n_jobs).collect::<Vec<CompletedPart>>();
          complete_parts.sort_by(|part1, part2| part1.part_number.cmp(&part2.part_number));

          target
            .complete_s3_upload(&upload_identifier, complete_parts)
            .await?;
          info!(target: &job.job_id.to_string(), "packet size: min = {}, max= {}", min_size, max_size);
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
              publish_job_progression(channel.clone(), job, percent as u8)
                .map_err(|_| Error::new(ErrorKind::Other, "unable to publish job progression"))?;
            }
          }

          part_buffer.append(data);

          if part_buffer.len() > part_size {
            let target = target.clone();
            let upload_identifier = upload_identifier.clone();
            let cloned_tx = tx.clone();
            let cloned_part_buffer = part_buffer.clone();

            while pool.queued_count() > 1 {
              thread::sleep(Duration::from_millis(500));
            }

            pool.execute(move || {
              task::block_on(async {
                let part_id = target
                  .upload_s3_part(&upload_identifier, part_number, cloned_part_buffer)
                  .await
                  .expect("unable to upload s3 part");

                cloned_tx
                  .send(part_id)
                  .expect("channel will be there waiting for the pool");
              })
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
