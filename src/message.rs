use amqp_worker::job::Credential;
use amqp_worker::*;
use ftp::openssl::ssl::{SslContext, SslMethod};
use ftp::types::FileType;
use ftp::{FtpError, FtpStream};
use std::fs::{create_dir_all, File};
use std::io::{copy, BufReader, BufWriter, Error, ErrorKind};
use std::path::{Path, PathBuf};

/// Process incoming job message
pub fn process(message: &str) -> Result<u64, MessageError> {
  let job = job::Job::new(message)?;
  info!("reveived message: {:?}", job);
  job.check_requirements()?;

  let mut source_path = get_string_parameter_required(&job, "source_path")?;
  if let Some(source_prefix) = job.get_string_parameter("source_prefix") {
    source_path = source_prefix + &source_path;
  }

  let mut destination_path = get_string_parameter_required(&job, "destination_path")?;
  if let Some(destination_prefix) = job.get_string_parameter("destination_prefix") {
    destination_path = destination_prefix + &destination_path;
  }

  let source_hostname = job.get_credential_parameter("source_hostname");
  let destination_hostname = job.get_credential_parameter("destination_hostname");
  let ssl_enabled = job.get_boolean_parameter("ssl").unwrap_or(false);

  if let Some(source_hostname) = source_hostname {
    // Download case
    let source_username = get_credential_parameter_required(&job, "source_username")?;
    let source_password = get_credential_parameter_required(&job, "source_password")?;
    let source_port = job.get_integer_parameter("source_port").unwrap_or(21) as u16;

    // check if destination directory exists
    let destination_directory = Path::new(&destination_path).parent().unwrap();
    if !destination_directory.exists() {
      // create new path
      create_dir_all(&destination_directory)
        .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
    }

    let hostname = source_hostname.request_value(&job)?;
    let user = source_username.request_value(&job)?;
    let password = source_password.request_value(&job)?;
    let _downloaded_size = execute_ftp_download(
      &hostname,
      source_port,
      &user,
      &password,
      &source_path,
      &destination_path,
      ssl_enabled,
    )
    .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
    Ok(job.job_id)
  } else if let Some(destination_hostname) = destination_hostname {
    // Upload case
    let destination_username = get_credential_parameter_required(&job, "destination_username")?;
    let destination_password = get_credential_parameter_required(&job, "destination_password")?;
    let destination_port = job.get_integer_parameter("destination_port").unwrap_or(21) as u16;

    let hostname = destination_hostname.request_value(&job)?;
    let user = destination_username.request_value(&job)?;
    let password = destination_password.request_value(&job)?;
    let _uploaded_size = execute_ftp_upload(
      &source_path,
      &hostname,
      destination_port,
      &user,
      &password,
      &destination_path,
      ssl_enabled,
    )
    .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
    Ok(job.job_id)
  } else {
    Err(MessageError::ProcessingError(
      job.job_id,
      "Invalid job message parameters".to_string(),
    ))
  }
}

fn get_credential_parameter_required(
  job: &job::Job,
  parameter: &str,
) -> Result<Credential, MessageError> {
  job.get_credential_parameter(parameter).ok_or_else(|| {
    MessageError::ProcessingError(
      job.job_id,
      format!("missing {} parameter", parameter.replace("_", " ")),
    )
  })
}

fn get_string_parameter_required(job: &job::Job, parameter: &str) -> Result<String, MessageError> {
  job.get_string_parameter(parameter).ok_or_else(|| {
    MessageError::ProcessingError(
      job.job_id,
      format!("missing {} parameter", parameter.replace("_", " ")),
    )
  })
}

fn execute_ftp_download(
  hostname: &str,
  port: u16,
  user: &str,
  password: &str,
  source_path: &str,
  destination_path: &str,
  ssl_enabled: bool,
) -> Result<u64, FtpError> {
  let mut ftp_stream = FtpStream::connect((hostname, port))?;

  if ssl_enabled {
    let builder = SslContext::builder(SslMethod::tls()).map_err(|_e| {
      FtpError::ConnectionError(Error::new(ErrorKind::Other, "unable to build SSL context"))
    })?;
    let context = builder.build();
    ftp_stream = ftp_stream.into_secure(context)?;
  }

  ftp_stream.login(user, password)?;
  debug!("current directory: {}", ftp_stream.pwd()?);

  // We need to enable binary transfer type to ensure the final data size is correct
  ftp_stream.transfer_type(FileType::Binary)?;

  debug!("Download remote file: {:?}", source_path);
  debug!("Remote directory content: {:?}", ftp_stream.list(Some("/")));
  let length = ftp_stream.retr(source_path, |stream| {
    let dest_file = File::create(&destination_path).unwrap();
    let mut file_writer: BufWriter<File> = BufWriter::new(dest_file);
    copy(stream, &mut file_writer).map_err(|e| FtpError::ConnectionError(e))
  })?;

  ftp_stream.quit()?;
  Ok(length)
}

fn execute_ftp_upload(
  source_path: &str,
  hostname: &str,
  port: u16,
  user: &str,
  password: &str,
  destination_path: &str,
  ssl_enabled: bool,
) -> Result<usize, FtpError> {
  let mut ftp_stream = FtpStream::connect((hostname, port))?;

  if ssl_enabled {
    let builder = SslContext::builder(SslMethod::tls()).map_err(|_e| {
      FtpError::ConnectionError(Error::new(ErrorKind::Other, "unable to build SSL context"))
    })?;
    let context = builder.build();
    ftp_stream = ftp_stream.into_secure(context)?;
  }

  ftp_stream.login(user, password)?;
  debug!("current directory: {}", ftp_stream.pwd()?);
  ftp_stream.transfer_type(FileType::Binary)?;

  // create destination directories if not exists
  let destination_directory = Path::new(&destination_path).parent().unwrap();
  let mut root_dir = PathBuf::from("/");
  for folder in destination_directory.iter() {
    if folder == "/" {
      continue;
    }

    root_dir = root_dir.join(folder);
    match ftp_stream.mkdir(root_dir.to_str().unwrap()) {
      Ok(()) => {}
      Err(FtpError::InvalidResponse(msg)) => {
        if msg
          != format!(
            "Expected code [257], got response: 550 {}: File exists.\r\n",
            root_dir.to_str().unwrap()
          )
        {
          return Err(FtpError::InvalidResponse(msg));
        }
      }
      Err(msg) => return Err(msg),
    }
  }

  ftp_stream.cwd(root_dir.to_str().unwrap())?;
  let filename = Path::new(&destination_path).file_name().unwrap();

  debug!("Upload local file: {:?}", source_path);
  let source_file = File::open(source_path).map_err(|e| FtpError::ConnectionError(e))?;
  let mut reader = BufReader::new(source_file);
  ftp_stream.put(filename.to_str().unwrap(), &mut reader)?;
  let length = ftp_stream.size(filename.to_str().unwrap())?;

  ftp_stream.quit()?;
  Ok(length.unwrap_or(0))
}
