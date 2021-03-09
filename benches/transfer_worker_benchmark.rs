use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use mcai_worker_sdk::job::{Job, JobResult};

use std::time::Duration;
use transfer_worker::TransferWorkerParameters;

fn bench_job(b: &mut Bencher, message: &String) {
  let job = Job::new(&message).unwrap();
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters::<TransferWorkerParameters>().unwrap();
  b.iter(|| transfer_worker::message::process(None, parameters.clone(), job_result.clone()));
}

fn ftp_upload_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/ftp_upload.json").unwrap();
  c.bench_with_input(BenchmarkId::new("FTP", "Upload"), &message, bench_job);
}

fn ftp_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/ftp_download.json").unwrap();
  c.bench_with_input(BenchmarkId::new("FTP", "Download"), &message, bench_job);
}

fn http_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/http_download.json").unwrap();
  c.bench_with_input(BenchmarkId::new("HTTP", "Download"), &message, bench_job);
}

fn local_copy_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/local_copy.json").unwrap();
  c.bench_with_input(BenchmarkId::new("Local", "Copy"), &message, bench_job);
}

fn s3_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/s3_download.json").unwrap();
  c.bench_with_input(BenchmarkId::new("S3", "Download"), &message, bench_job);
}

fn s3_upload_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/s3_upload.json").unwrap();
  c.bench_with_input(BenchmarkId::new("S3", "Upload"), &message, bench_job);
}

fn sftp_upload_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/sftp_upload.json").unwrap();
  c.bench_with_input(BenchmarkId::new("SFTP", "Upload"), &message, bench_job);
}

fn sftp_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/sftp_download.json").unwrap();
  c.bench_with_input(BenchmarkId::new("SFTP", "Download"), &message, bench_job);
}


criterion_group! {
  name = benches;
  config =
    Criterion::default()
      .warm_up_time(Duration::from_millis(500))
      .measurement_time(Duration::new(1, 0))
      .sample_size(10);
  targets =
    ftp_upload_benchmark,
    ftp_download_benchmark,
    http_download_benchmark,
    local_copy_benchmark,
    s3_upload_benchmark,
    s3_download_benchmark,
    sftp_upload_benchmark,
    sftp_download_benchmark
}

criterion_main!(benches);
