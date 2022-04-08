# Rust Transfer worker

Based on the [rs_transfer](https://gitlab.com/media-cloud-ai/libraries/rs_transfer) library, this worker aims to stream files data from any supported[^1] source
point to any destination point.

[^1]: See the `rs_transfer` library documentation.

## Examples

Some examples message files can be found in the `./examples` directory


## Dev and benchmarks environment

To setup an environment to test or check performances of the worker, execute the following command from the project directory:
```
docker-compose up
```
This will start a FTP server, a HTTP endpoint and a S3 bucket, based on the `./tests` tree files.

To stop the Docker containers, run:
```
docker-compose down
```

__Note:__ Google Cloud Storage cannot be emulated locally in a Docker container yet.

## Benchmarks

Once the environment is set, benchmarks can be executed running the following command:
```
cargo bench
```

This benches are based on the `./examples` message files.

__Note:__ Google Cloud Storage endpoint benchmarks are skipped as long as it cannot be emulated locally in a Docker container.

## Probe feature

The `media_probe_and_upload` feature allow the worker to analyse the input file with [FFmpeg](https://github.com/nomalab/stainless-ffmpeg) and
transfer the resulting metadata as a file onto a destination storage.

This functionality is triggered when at least one of the transfer endpoints is a local file, and when the job message contains the
following parameters:

- `probe_secret`: Describing the type and access to the remote storage (as a typical `destination_secret` parameter),
- `probe_path` (optional): The path of the directory into which the metadata will be transferred. By default, this path is `job/probe/`.

The name of the resulting metadata file is based on the ID of the job: `<job_id>.json`
