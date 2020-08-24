use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mcai_worker_sdk::job::{Job, JobResult};

use transfer_worker::TransferWorkerParameters;

fn process_job(job: Job) -> () {
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters::<TransferWorkerParameters>().unwrap();
  let _result = transfer_worker::message::process(None, parameters, job_result).unwrap();
}

fn ftp_upload_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/ftp_upload.json").unwrap();
  let job = Job::new(&message).unwrap();
  c.bench_with_input(BenchmarkId::new("FTP", "Upload"), &job, |b, job| {
    b.iter(|| process_job(black_box(job.clone())));
  });
}

fn ftp_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/ftp_download.json").unwrap();
  let job = Job::new(&message).unwrap();
  c.bench_with_input(BenchmarkId::new("FTP", "Download"), &job, |b, job| {
    b.iter(|| process_job(black_box(job.clone())));
  });
}

fn local_copy_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/local_copy.json").unwrap();
  let job = Job::new(&message).unwrap();
  c.bench_with_input(BenchmarkId::new("Local", "Copy"), &job, |b, job| {
    b.iter(|| process_job(black_box(job.clone())));
  });
}

fn s3_download_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/s3_download.json").unwrap();
  let job = Job::new(&message).unwrap();
  c.bench_with_input(BenchmarkId::new("S3", "Download"), &job, |b, job| {
    b.iter(|| process_job(black_box(job.clone())))
  });
}

fn s3_upload_benchmark(c: &mut Criterion) {
  let message = std::fs::read_to_string("./examples/s3_upload.json").unwrap();
  let job = Job::new(&message).unwrap();
  c.bench_with_input(BenchmarkId::new("S3", "Upload"), &job, |b, job| {
    b.iter(|| process_job(black_box(job.clone())));
  });
}

// fn clean_up() {
//   // std::fs::remove_file("share/tmp_*");
//   for entry in WalkDir::new("share") {
//     if let Ok(dir_entry) = entry {
//       let file_name = dir_entry.file_name().to_str().unwrap().to_string();
//       print!("{}", dir_entry.path().display());
//       if file_name.starts_with("tmp_") || file_name.starts_with(".minio") {
//         if dir_entry.file_type().is_dir() {
//           let result = std::fs::remove_dir_all(dir_entry.path());
//           print!(" => REMOVED: {:?}", result);
//         } else {
//           let result = std::fs::remove_file(dir_entry.path());
//           print!(" => REMOVED: {:?}", result);
//         }
//       }
//       println!();
//     }
//   }
// }

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(10);
  targets =
    ftp_upload_benchmark,
    ftp_download_benchmark,
    local_copy_benchmark,
    s3_upload_benchmark,
    s3_download_benchmark
}

criterion_main!(benches);
