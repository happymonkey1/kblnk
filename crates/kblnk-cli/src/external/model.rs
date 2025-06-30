use std::fmt::{Debug, Formatter, Write};

use crate::external::error::Result;

pub enum SendMessageResponseStream{
    GoogleAiStudio(google_ai_rs::genai::ResponseStream),
    /// Mocks will be popped from the list. Use [SendMessageResponseStream::new] to construct from
    /// a (un)reversed list
    #[cfg(test)]
    Mock(Vec<ChatResponseStream>),
}

impl SendMessageResponseStream {
    /// Construct a mock response, reversing the input responses before constructing
    #[cfg(test)]
    pub fn new_mock(mut responses: Vec<ChatResponseStream>) -> Self {
        responses.reverse();
        Self::Mock(responses)
    }
}

/// Custom Debug implementation because [google_ai_rs::genai::ResponseStream] does not implement
/// the Debug trait
impl Debug for SendMessageResponseStream {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let f_string = match self {
            SendMessageResponseStream::GoogleAiStudio(_) =>
                String::from("SendMessageResponseStream::GoogleAiStudio"),
            #[cfg(test)]
            SendMessageResponseStream::Mock(responses) =>
                format!("SendMessageResponseStream::Mock({:?})", responses),
        };
        
        f.write_str(f_string.as_str()) 
    }
}

impl SendMessageResponseStream {

    pub async fn recv(&mut self) -> Result<Option<ChatResponseStream>> {
        match self {
            SendMessageResponseStream::GoogleAiStudio(response_stream) => {
                let chat_response = response_stream.next().await?
                    .map_or(
                        None,
                        |r| Some(ChatResponseStream::from(r))
                    );
                
                Ok(chat_response)
            }
            #[cfg(test)]
            SendMessageResponseStream::Mock(responses) => {
                Ok(responses.pop())
            }
        }
    }

}

#[derive(Debug)]
pub enum ChatResponseStream {
    LlmResponseEvent{ content: String },
}
