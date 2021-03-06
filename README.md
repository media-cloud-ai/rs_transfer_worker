# Rust Transfer worker

### List of supported remote access to Read/Download data

- [x] Local file
- [x] Download FTP file
- [x] Download SFTP file
- [x] Download HTTP file
- [x] Download S3 file

### List of supported remote access to Write/Upload data

- [x] Write local file
- [x] Upload FTP file
- [x] Upload SFTP file
- [x] Upload S3 file


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


## Benchmarks

Once the environment is set, benchmarks can be executed running the following command:
```
cargo bench
```

This benches are based on the `./examples` message files.
