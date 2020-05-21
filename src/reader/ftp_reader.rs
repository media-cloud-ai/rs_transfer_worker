use crate::{message::StreamData, reader::StreamReader, target_configuration::TargetConfiguration};
use async_std::{sync::Sender, task};
use async_trait::async_trait;
use ftp::FtpError;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub struct FtpReader {}

#[async_trait]
impl StreamReader for FtpReader {
  async fn read_stream(
    &self,
    target: TargetConfiguration,
    sender: Sender<StreamData>,
  ) -> Result<(), Error> {
    let prefix = target.prefix.clone().unwrap_or_else(|| "/".to_string());
    let absolute_path = prefix + &target.path;

    let path = Path::new(&absolute_path);
    let directory = path.parent().unwrap().to_str().unwrap();
    let filename = path.file_name().unwrap().to_str().unwrap();

    let mut ftp_stream = target
      .get_ftp_stream()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    ftp_stream
      .cwd(&directory)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    if let Some(file_size) = ftp_stream
      .size(&filename)
      .map_err(|e| Error::new(ErrorKind::Other, e))?
    {
      sender.send(StreamData::Size(file_size as u64)).await;
    };

    ftp_stream
      .retr(&filename, |stream| {
        let mut buffer = Vec::new();
        stream
          .read_to_end(&mut buffer)
          .map(|_| {
            task::block_on(async {
              sender.send(StreamData::Data(buffer)).await;
            })
          })
          .map_err(FtpError::ConnectionError)
      })
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    sender.send(StreamData::Eof).await;
    Ok(())
  }
}
