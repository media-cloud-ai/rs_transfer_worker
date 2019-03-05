use amqp_worker::*;
use amqp_worker::job::Credential;

use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use ftp::{FtpError, FtpStream};
use ftp::types::FileType;
use ftp::openssl::ssl::{ SslContext, SslMethod };


/// Process incoming job message
pub fn process(message: &str) -> Result<u64, MessageError> {
    let job = job::Job::new(message)?;
    println!("reveived message: {:?}", job);

    match job.check_requirements() {
        Ok(_) => {}
        Err(message) => {
            return Err(message);
        }
    }

    let mut source_path = get_job_string_parameter(&job, "source_path")?;
    let source_prefix = job.get_string_parameter("source_prefix");
    if source_prefix.is_some() {
        source_path = source_prefix.unwrap() + &source_path;
    }

    let mut destination_path = get_job_string_parameter(&job, "destination_path")?;
    let destination_prefix = job.get_string_parameter("destination_prefix");
    if destination_prefix.is_some() {
        destination_path = destination_prefix.unwrap() + &destination_path;
    }

    let source_hostname = job.get_credential_parameter("source_hostname");
    let destination_hostname = job.get_credential_parameter("destination_hostname");
    let ssl_enabled = job.get_boolean_parameter("ssl");
    let is_ssl_enabled = ssl_enabled.unwrap_or(false);

    if source_hostname.is_some() {
        // Download case
        let source_username = get_job_credential_parameter(&job, "source_username")?;
        let source_password = get_job_credential_parameter(&job, "source_password")?;

        // check if destination directory exists
        let destination_directory = Path::new(destination_path.as_str()).parent().unwrap();
        if !destination_directory.exists() {
            // create new path
            fs::create_dir_all(&destination_path)
                .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;
        }

        let hostname = source_hostname.unwrap().request_value(&job)?;
        let user = source_username.request_value(&job)?;
        let password = source_password.request_value(&job)?;
        let _downloaded_size = ftp_download(hostname, user, password, source_path, &destination_path, is_ssl_enabled)
            .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;

    } else if destination_hostname.is_some() {
        // Upload case
        let destination_username = get_job_credential_parameter(&job, "destination_username")?;
        let destination_password = get_job_credential_parameter(&job, "destination_password")?;

        let hostname = destination_hostname.unwrap().request_value(&job)?;
        let user = destination_username.request_value(&job)?;
        let password = destination_password.request_value(&job)?;
        let _uploaded_size = ftp_upload(source_path, hostname, user, password, &destination_path, is_ssl_enabled)
            .map_err(|e| MessageError::ProcessingError(job.job_id, e.to_string()))?;

    } else {
        return Err(MessageError::ProcessingError(job.job_id, "Invalid job message parameters".to_string()));
    }

    Ok(job.job_id)
}


/// Retrieve required credential parameter
fn get_job_credential_parameter(job: &job::Job, parameter: &str) -> Result<Credential, MessageError> {
    let parameter_value = job.get_credential_parameter(parameter);
    if parameter_value.is_none() {
        return Err(MessageError::ProcessingError(job.job_id, format!("missing {} parameter", parameter.replace("_", " "))));
    }
    Ok(parameter_value.unwrap())
}


/// Retrieve required string parameter
fn get_job_string_parameter(job: &job::Job, parameter: &str) -> Result<String, MessageError> {
    let parameter_value = job.get_string_parameter(parameter);
    if parameter_value.is_none() {
        return Err(MessageError::ProcessingError(job.job_id, format!("missing {} parameter", parameter.replace("_", " "))));
    }
    Ok(parameter_value.unwrap())
}


/// Execute FTP download
fn ftp_download(hostname: String, user: String, password: String, source_path: String,
                destination_path: &String, ssl_enabled: bool) -> Result<u64, FtpError> {

    // Connect remote server
    let mut ftp_stream = FtpStream::connect((hostname.as_str(), 21)).unwrap();

    if ssl_enabled {
        let ctx = SslContext::builder(SslMethod::tls()).unwrap().build();
        // Switch to the secure mode
        ftp_stream = ftp_stream.into_secure(ctx).unwrap();
    }

    ftp_stream.login(user.as_str(), password.as_str()).unwrap();
    debug!("current dir: {}", ftp_stream.pwd().unwrap());

    // We need to enable binary transfer type to ensure the final data size is correct
    ftp_stream.transfer_type(FileType::Binary).unwrap();

    // Download the file
    debug!("Download remote file: {:?}", source_path);
    debug!("Remote dir content: {:?}", ftp_stream.list(Some("/")));
    let length = ftp_stream.retr(source_path.as_str(), |stream| {
        let dest_file = File::create(&destination_path).unwrap();
        let mut file_writer: BufWriter<File> = BufWriter::new(dest_file);
        io::copy(stream, &mut file_writer)
            .map_err(|e| FtpError::ConnectionError(e))
    }).unwrap();

    ftp_stream.quit().unwrap();

    debug!("Done: {:?}", length);
    Ok(length)
}


/// Execute FTP upload
fn ftp_upload(source_path: String, hostname: String, user: String, password: String,
              destination_path: &String, ssl_enabled: bool) -> Result<usize, FtpError> {

    // Connect remote server
    let mut ftp_stream = FtpStream::connect((hostname.as_str(), 21)).unwrap();

    if ssl_enabled {
        let ctx = SslContext::builder(SslMethod::tls()).unwrap().build();
        // Switch to the secure mode
        ftp_stream = ftp_stream.into_secure(ctx).unwrap();
    }

    ftp_stream.login(user.as_str(), password.as_str()).unwrap();
    debug!("current dir: {}", ftp_stream.pwd().unwrap());

    // We need to enable binary transfer type to ensure the final data size is correct
    ftp_stream.transfer_type(FileType::Binary).unwrap();

    // Upload a file
    debug!("Upload local file: {:?}", source_path);
    let source_file = File::open(source_path).unwrap();
    let mut reader = BufReader::new(source_file);
    ftp_stream.put(destination_path.as_str(), &mut reader).unwrap();
    debug!("Remote dir content: {:?}", ftp_stream.list(Some("/")));
    let length = ftp_stream.size(destination_path.as_str()).unwrap_or(Some(0)).unwrap();

    ftp_stream.quit().unwrap();

    Ok(length)
}

