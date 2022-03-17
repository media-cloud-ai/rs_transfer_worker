use crate::{
  endpoint::http::{get_headers, get_method, get_url},
  reader::{ReaderNotification, StreamReader},
  StreamData,
};
use async_std::channel::Sender;
use async_trait::async_trait;
use reqwest::{Method, StatusCode};
use std::io::{Error, ErrorKind};
use tokio::runtime::Runtime;

pub struct HttpReader {
  pub endpoint: Option<String>,
  pub method: Option<String>,
  pub headers: Option<String>,
  pub body: Option<String>,
}

#[async_trait]
impl StreamReader for HttpReader {
  async fn read_stream(
    &self,
    path: &str,
    sender: Sender<StreamData>,
    channel: &dyn ReaderNotification,
  ) -> Result<u64, Error> {
    let mut total_read_bytes: u64 = 0;
    Runtime::new()
      .expect("Failed to create Tokio runtime")
      .block_on(async {
        let client = reqwest::Client::builder().build().unwrap();

        let method = if let Some(method) = &self.method {
          get_method(method)?
        } else {
          Method::GET
        };

        let url = if let Some(endpoint) = &self.endpoint {
          let mut url = get_url(endpoint)?;
          url.set_path(path);
          url
        } else {
          get_url(path)?
        };

        let mut request_builder = client.request(method, url);
        if let Some(json_headers) = &self.headers {
          request_builder = request_builder.headers(get_headers(json_headers)?);
        }

        if let Some(body) = &self.body {
          request_builder = request_builder.body(body.to_string());
        }

        let request = request_builder.build().map_err(|error| {
          Error::new(
            ErrorKind::Other,
            format!("Failed to build request: {}", error),
          )
        })?;

        let response = client.execute(request).await.unwrap();

        if response.status() != StatusCode::OK {
          return Err(Error::new(
            ErrorKind::Other,
            format!("bad request response: {}", response.status()),
          ));
        }

        let bytes = response.bytes();
        let data_bytes = bytes.await.unwrap();
        total_read_bytes += data_bytes.len() as u64;
        if let Err(error) = sender.send(StreamData::Data(data_bytes.to_vec())).await {
          if channel.is_stopped() && sender.is_closed() {
            log::warn!(
              "Data channel closed: could not send {} read bytes.",
              data_bytes.len()
            );
            return Ok(());
          }

          return Err(Error::new(
            ErrorKind::Other,
            format!("Could not send read data through channel: {}", error),
          ));
        }

        sender.send(StreamData::Eof).await.unwrap();
        Ok(())
      })?;
    Ok(total_read_bytes)
  }
}
