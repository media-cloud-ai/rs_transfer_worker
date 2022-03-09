use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion};
use mcai_worker_sdk::job::{Job, JobResult};
use std::time::Duration;
use rs_transfer_worker::{message, TransferWorkerParameters};

struct Order {
  message: String,
}

impl From<&str> for Order {
  fn from(filename: &str) -> Self {
    let message = std::fs::read_to_string(filename).unwrap();
    Order { message }
  }
}

fn bench_job(b: &mut Bencher, order: &Order) {
  let job = Job::new(&order.message).unwrap();
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters::<TransferWorkerParameters>().unwrap();
  b.iter(|| message::process(None, parameters.clone(), job_result.clone()));
}

fn ftp_upload_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/ftp_upload.json");
  c.bench_with_input(BenchmarkId::new("FTP", "Upload"), &order, bench_job);
}

fn ftp_download_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/ftp_download.json");
  c.bench_with_input(BenchmarkId::new("FTP", "Download"), &order, bench_job);
}

fn http_download_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/http_download.json");
  c.bench_with_input(BenchmarkId::new("HTTP", "Download"), &order, bench_job);
}

fn local_copy_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/local_copy.json");
  c.bench_with_input(BenchmarkId::new("Local", "Copy"), &order, bench_job);
}

fn s3_download_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/s3_download.json");
  c.bench_with_input(BenchmarkId::new("S3", "Download"), &order, bench_job);
}

fn s3_upload_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/s3_upload.json");
  c.bench_with_input(BenchmarkId::new("S3", "Upload"), &order, bench_job);
}

fn sftp_upload_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/sftp_upload.json");
  c.bench_with_input(BenchmarkId::new("SFTP", "Upload"), &order, bench_job);
}

fn sftp_download_benchmark(c: &mut Criterion) {
  let order = Order::from("./examples/sftp_download.json");
  c.bench_with_input(BenchmarkId::new("SFTP", "Download"), &order, bench_job);
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
