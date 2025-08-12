use async_trait::async_trait;
use futures::{Sink, Stream, StreamExt, SinkExt, TryStreamExt};
use serde_json;
use std::io;
use std::pin::Pin;
use tokio::net::TcpStream;
use tokio::process::Command;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec, LinesCodecError};

use crate::language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError};
use crate::schemas::{messages::Message, StreamData};

#[derive(Clone, Debug)]
pub enum McpTransport {
    Stream(String),
}

#[derive(Clone)]
pub struct McpClient {
    transport: McpTransport,
    options: CallOptions,
}

impl McpClient {
    pub fn new(transport: McpTransport) -> Self {
        Self {
            transport,
            options: CallOptions::default(),
        }
    }

    pub fn with_options(mut self, options: CallOptions) -> Self {
        self.options = options;
        self
    }
}

type McpStream = Pin<Box<dyn Stream<Item = Result<String, io::Error>> + Unpin + Send>>;
type McpSink = Pin<Box<dyn Sink<String, Error = io::Error> + Unpin + Send>>;

fn map_codec_error(e: LinesCodecError) -> io::Error {
    match e {
        LinesCodecError::Io(e) => e,
        e => io::Error::new(io::ErrorKind::Other, e.to_string()),
    }
}

async fn create_mcp_stream_sink(transport: &McpTransport) -> Result<(McpSink, McpStream), LLMError> {
    match transport {
        McpTransport::Stream(addr) => {
            let stream = TcpStream::connect(addr).await?;
            let (reader, writer) = tokio::io::split(stream);
            let sink = FramedWrite::new(writer, LinesCodec::new());
            let stream = FramedRead::new(reader, LinesCodec::new());

            let sink = sink.sink_map_err(map_codec_error);
            let stream = stream.map_err(map_codec_error);

            Ok((
                Box::pin(sink),
                Box::pin(stream),
            ))
        }
    }
}


#[async_trait]
impl LLM for McpClient {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        let (mut sink, mut stream) = create_mcp_stream_sink(&self.transport).await?;

        let message_json = serde_json::to_string(messages)?;
        sink.send(message_json).await?;

        let mut response = String::new();
        while let Some(line) = stream.next().await {
            let line = line?;
            response.push_str(&line);
        }

        Ok(GenerateResult {
            generation: response,
            tokens: None,
        })
    }

    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let (mut sink, mut stream) = create_mcp_stream_sink(&self.transport).await?;

        let message_json = serde_json::to_string(messages)?;
        sink.send(message_json).await?;

        let response_stream = async_stream::try_stream! {
            while let Some(line) = stream.next().await {
                let line = line?;
                let data = serde_json::from_str(&line)?;
                yield StreamData::new(data, None, &line);
            }
        };

        Ok(Box::pin(response_stream))
    }

    fn add_options(&mut self, options: CallOptions) {
        self.options.merge_options(options);
    }
}
