use std::io::{Error, ErrorKind};

use async_std::sync::Sender;
use async_trait::async_trait;
use reqwest::{Method, StatusCode};
use tokio::runtime::Runtime;

use crate::endpoint::http::{get_headers, get_method, get_url};
use crate::{message::StreamData, reader::StreamReader};

pub struct HttpReader {
  pub endpoint: String,
  pub method: Option<String>,
  pub headers: Option<String>,
  pub body: Option<String>,
}

#[async_trait]
impl StreamReader for HttpReader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error> {
    Runtime::new()
      .expect("Failed to create Tokio runtime")
      .block_on(async {
        let client = reqwest::Client::builder().build().unwrap();

        let method = if let Some(method) = &self.method {
          get_method(method)?
        } else {
          Method::GET
        };
        let mut url = get_url(&self.endpoint)?;
        url.set_path(path);

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
        sender
          .send(StreamData::Data(bytes.await.unwrap().to_vec()))
          .await;

        sender.send(StreamData::Eof).await;
        Ok(())
      })
  }
}
