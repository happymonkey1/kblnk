use std::fmt::{Debug, Formatter, Write};

use crate::external::error::Result;

pub enum SendMessageResponseStream{
    GoogleAiStudio(google_ai_rs::genai::ResponseStream),
    Mock(Vec<ChatResponseStream>),
}

/// Custom Debug implementation because [google_ai_rs::genai::ResponseStream] does not implement
/// the Debug trait
impl Debug for SendMessageResponseStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let f_string = match self {
            SendMessageResponseStream::GoogleAiStudio(_) =>
                String::from("SendMessageResponseStream::GoogleAiStudio"),
            SendMessageResponseStream::Mock(responses) =>
                format!("SendMessageResponseStream::Mock({:?})", responses),
        };
        
        f.write_str(f_string.as_str()) 
    }
}

impl SendMessageResponseStream {

    pub async fn recv(&mut self) -> Result<ChatResponseStream> {
        match self {
            SendMessageResponseStream::GoogleAiStudio(response_stream) => {
                let chat_response = response_stream.next().await?
                    .map_or(
                        Ok(ChatResponseStream::EndStream { content: "".to_string() }),
                        |r| ChatResponseStream::try_from(r)
                    )?;
                
                Ok(chat_response)
            }
            SendMessageResponseStream::Mock(_) => todo!("recv not implement for mock SendMessageResponseStream")
        }
    }

}

#[derive(Debug)]
pub enum ChatResponseStream {
    LlmResponseEvent{ content: String },
    EndStream { content: String },
    InvalidLlmResponse,
}
