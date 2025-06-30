use log::{error, warn};
use rand::distr::{Alphanumeric, SampleString};
use tracing::info;
use kb_xml_rs::document::XmlDocument;
use kb_xml_rs::node::XmlNode;
use crate::external::model::{ChatResponseStream, SendMessageResponseStream};
use crate::cli::chat::error::{ChatError, Result};
use crate::cli::chat::message::LlmMessage;

pub const RESPONSE_TAG_NAME: &str = "response";
pub const CODE_TAG_NAME: &str = "code";
pub const KB_TAG_NAMESPACE: &str = "kb";

#[derive(Debug)]
pub enum ChatResponse {
    Response(String),
    Code(String),
    EndStream { message: LlmMessage },
}

pub struct ChatResponseParser {
    stream: SendMessageResponseStream,
    message_id: String,
    message_buffer: String,
}

impl ChatResponseParser {

    pub fn new(response_stream: SendMessageResponseStream) -> Self {
        let message_id = Alphanumeric.sample_string(&mut rand::rng(), 9);
        Self {
            stream: response_stream,
            message_id,
            message_buffer: String::new(),
        }
    }

    pub async fn next(&mut self) -> Result<Vec<ChatResponse>> {
        // TODO: should loop over stream for future response tags like tool use or thinking
        match self.stream.recv().await {
            Ok(chat_response) => {
                match chat_response {
                    Some(chat_response) => {
                        match chat_response {
                            ChatResponseStream::LlmResponseEvent { content } => {
                                Ok(self.parse_xml_to_chat_response(content)?)
                            }
                        }
                    }
                    None => {
                        let message = std::mem::take(&mut self.message_buffer);
                        let llm_message = LlmMessage::new_response(
                            Some(self.message_id.clone()),
                            message,
                        );
                        Ok(vec![ChatResponse::EndStream{ message: llm_message }])
                    }
                }
            }
            Err(err) => Err(ChatError::from(err))
        }
    }

    fn parse_xml_to_chat_response(&mut self, message: String) -> Result<Vec<ChatResponse>> {
        info!("Entered parse_xml_to_chat_response");
        let doc = XmlDocument::parse(message.as_str())?;
        
        let chat_responses = doc.into_iter()
            .filter_map(|node| {
                match node {
                    XmlNode::Element(mut element) => {
                        info!("Mapping element: {element:?}");
                        debug_assert!(element.children().len() > 0, "No children in element: {element:?}");
                        let content = if let Some(content) = element.take_children().first().cloned() {
                            match content {
                                XmlNode::Text(text) => text,
                                other_node => {
                                    error!("Unhandled xml node: {other_node:?}");
                                    return None 
                                }
                            }
                        } else {
                            error!("Found node element ({element:?}) but it has no content?");
                            return None 
                        };
                        
                        let response = match element.name().as_str() {
                            RESPONSE_TAG_NAME => Some(ChatResponse::Response(content)),
                            CODE_TAG_NAME => Some(ChatResponse::Code(content)),
                            unhandled_tag_name => {
                                warn!("Unhandled xml tag: {unhandled_tag_name}");
                                
                                None 
                            }
                        };
                        
                        response
                    }
                    XmlNode::Text(text) => None,
                    other_node => {
                        warn!("Found unhandled xml tag: {other_node:?}");
                        None
                    },
                }
            })
            .collect::<Vec<ChatResponse>>();

        Ok(chat_responses)
    }
}

#[cfg(test)]
mod test {
    use crate::cli::chat::parser::{ChatResponse, ChatResponseParser};
    use crate::external::model::{ChatResponseStream, SendMessageResponseStream};
    use crate::cli::chat::error::{Result};
    
    #[tokio::test]
    async fn when_parse_simple_response_then_succeed() -> Result<()> {
        let response_stream = SendMessageResponseStream::new_mock(vec![ChatResponseStream::LlmResponseEvent {
            content: r#"
            <kb:response>
                I am a large language model.
            </kb:response>
            "#.to_string()
        }]);
        
        let mut parser = ChatResponseParser::new(response_stream);
    
        let mut chat_responses: Vec<ChatResponse> = vec![];
        let max_iters = 10_000;
        let mut iter = 0;
        loop {
            iter += 1;
            if iter >= max_iters {
                panic!("Breaking out of infinite loop")
            }
            
            let response = parser.next().await?;
            chat_responses.extend(response);
            
            match chat_responses.last() {
                Some(ChatResponse::EndStream { .. }) => {
                    break
                }
                _ => {}
            }
        }
        
        assert_eq!(
            chat_responses.len(),
            2, 
            "Expected 2 response, found {} instead ({:?})",
            chat_responses.len(),
            &chat_responses,
        );
        
        let first_response = chat_responses.get(0).expect("First response is valid");
        match first_response {
            ChatResponse::Response(response) => {
                assert_eq!(response, "I am a large language model.")
            }
            other_response => assert!(false, "Expected ChatResponse::Response, found: {other_response:?}")
        }
        
        let second_response = chat_responses.get(1).expect("Second response is valid");
        match second_response {
            ChatResponse::EndStream { .. } => { 
                /* no-op */
            }
            other_response => assert!(false, "Expected ChatResponse::EndStream, found: {other_response:?}")
        }
        
        Ok(())
    }

}
