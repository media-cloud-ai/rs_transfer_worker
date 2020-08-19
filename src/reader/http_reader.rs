use crate::{message::StreamData, reader::StreamReader};
use async_std::sync::Sender;
use async_trait::async_trait;
use reqwest::StatusCode;
use std::io::{Error, ErrorKind};
use tokio::runtime::Runtime;

pub struct HttpReader {
  pub endpoint: String,
  pub method: String,
  pub headers: String,
  pub body: String,
}

#[async_trait]
impl StreamReader for HttpReader {
  async fn read_stream(&self, path: &str, sender: Sender<StreamData>) -> Result<(), Error> {
    Runtime::new()
      .expect("Failed to create Tokio runtime")
      .block_on(async {
        let client = reqwest::Client::builder().build().unwrap();

        let response = client.get(path).send().await.unwrap();

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
