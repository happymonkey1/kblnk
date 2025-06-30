use crate::error::{ParseError, Result};
use crate::parser::document::LexedXmlDocument;
use crate::parser::node::LexedXmlNode;

#[derive(Debug)]
pub enum Token {
    Space,
    LessThan,
    GreaterThan,
    Slash,
    Colon,
    Char,
    Equal,
    Quote,
    Question,
    Exclamation,
    Hyphen,
}

impl Token {
    pub fn char_to_token(ch: char) -> Token {
        match ch {
            ' ' | '\n' | '\t' | '\r' => Token::Space,
            '<' => Token::LessThan,
            '>' => Token::GreaterThan,
            '/' => Token::Slash,
            ':' => Token::Colon,
            '=' => Token::Equal,
            '\'' | '\"' => Token::Quote,
            '!' => Token::Exclamation,
            '?' => Token::Question,
            '-' => Token::Hyphen,
            _ => Token::Char,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ParserState {
    Data,
    TagBegin,
    TagName,
    TagEnd,

    AttributeNameBegin,
    AttributeName,
    AttributeNameEnd,

    AttributeValueBegin,
    AttributeValue,

    CommentBegin,
    Comment,
    CommentEnd,

    DeclarationBegin,
    Declaration,
    DeclarationEnd,
}

pub struct KbXmlParser {
    current_state: ParserState,
    peek_buffer: Option<char>,
    /// Internal data buffer to store parsed content
    data_buffer: String,
    tag_name: String,
    tag_namespace: String,
    attribute_name: String,
    attribute_value: String,
    is_closing: bool,
    is_self_closing: bool,
    is_comment: bool,
    /// Handles flag for whether we are parsing an XML declaration tag
    is_declaration: bool,
    hyphen_count: usize,
    open_quote: Option<QuoteChar>,
    document: LexedXmlDocument,
}

#[derive(Eq, PartialEq)]
enum QuoteChar {
    Single,
    Double,
}

impl TryFrom<char> for QuoteChar {
    type Error = ParseError;

    fn try_from(value: char) -> std::result::Result<Self, Self::Error> {
        match value {
            '\'' => Ok(QuoteChar::Single),
            '\"' => Ok(QuoteChar::Double),
            _ => Err(ParseError::UnexpectedCharacterError(value))
        }
    }
}

impl KbXmlParser {

    pub fn new() -> Self {
        Self {
            current_state: ParserState::Data,
            peek_buffer: None,
            data_buffer: String::new(),
            tag_name: String::new(),
            tag_namespace: String::new(),
            attribute_name: String::new(),
            attribute_value: String::new(),
            is_closing: false,
            is_self_closing: false,
            is_declaration: false,
            is_comment: false,
            hyphen_count: 0,
            open_quote: None,
            document: LexedXmlDocument::new(),
        }
    }

    pub fn parse(&mut self, data: &str) -> Result<LexedXmlDocument> {
        for (index, ch) in data.chars().enumerate() {
            self.peek_buffer = data.chars().nth(index + 1);
            self.step(ch)?
        }

        let doc = std::mem::take(&mut self.document);
        Ok(doc)
    }

    fn step(&mut self, ch: char) -> Result<()> {
        let tok = Token::char_to_token(ch);
        let next_state: ParserState = match &self.current_state {
            ParserState::Data =>
                match tok {
                    Token::LessThan => {
                        let trimmed_buffer  = self.data_buffer.trim();
                        if !trimmed_buffer.is_empty() {
                            self.push_node(LexedXmlNode::new_content(trimmed_buffer.to_string()));

                            self.data_buffer = String::new();
                        }

                        self.tag_name = String::new();
                        self.is_closing = false;
                        self.is_self_closing = false;
                        self.is_comment = false;
                        self.is_declaration = false;
                        self.hyphen_count = 0;

                        ParserState::TagBegin
                    }
                    _ => {
                        self.data_buffer.push(ch);
                        ParserState::Data
                    }
                }
            ParserState::TagBegin =>
                match tok {
                    Token::Char => {
                        self.tag_name.push(ch);

                        ParserState::TagName
                    }
                    Token::GreaterThan => self.noop_state_transition(),
                    Token::Slash => {
                        debug_assert!(!self.is_self_closing, "TabBegin should not be self closing");
                        self.is_closing = true;

                        ParserState::TagBegin
                    }
                    Token::Exclamation => ParserState::CommentBegin,
                    Token::Question => ParserState::Declaration,
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }
            ParserState::TagName =>
                match tok {
                    Token::Colon => {
                        std::mem::swap(&mut self.tag_name, &mut self.tag_namespace);

                        ParserState::TagBegin
                    }
                    Token::Char => {
                        self.tag_name.push(ch);

                        ParserState::TagName
                    }
                    Token::Space => {
                        if self.is_closing {
                            ParserState::TagEnd
                        } else {
                            let peek_token = self.peek_token();

                            match peek_token {
                                Some(Token::Slash) => {
                                    self.is_self_closing = true;

                                    ParserState::TagEnd
                                }
                                // Consume space token and continue (we may need to consume more spaces)
                                Some(Token::Space) => self.noop_state_transition(),
                                Some(_) => {
                                    let (name, namespace) = self.consume_name_and_namespace();
                                    self.push_node(LexedXmlNode::TagOpen { name, namespace });

                                    ParserState::AttributeNameBegin
                                }
                                None => return Err(ParseError::InvalidStateError(self.current_state, tok)),
                            }
                        }
                    }
                    Token::Slash => {
                        if !self.is_closing {
                            self.is_self_closing = true;

                            ParserState::TagEnd
                        } else {
                            return Err(ParseError::InvalidStateError(self.current_state, tok))
                        }
                    }
                    Token::GreaterThan => {
                        let (name, namespace) = self.consume_name_and_namespace();

                        if self.is_closing {
                            self.push_node(LexedXmlNode::new_tag_close(name, namespace))
                        } else {
                            self.push_node(LexedXmlNode::new_tag_open(name, namespace))
                        }

                        ParserState::Data
                    }
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }
            ParserState::TagEnd =>
                match tok {
                    Token::GreaterThan => {
                        let (name, namespace) = self.consume_name_and_namespace();

                        if self.is_closing {
                            self.push_node(LexedXmlNode::new_tag_close(name, namespace));
                        } else if self.is_self_closing {
                            self.push_node(LexedXmlNode::TagSelfClosing { name, namespace })
                        } else {
                            return Err(ParseError::InvalidStateError(self.current_state, tok))
                        }

                        ParserState::Data
                    }
                    Token::Slash => {
                        if self.is_self_closing {
                            self.noop_state_transition()
                        } else {
                            return Err(ParseError::UnexpectedToken(self.current_state, tok))
                        }
                    }
                    Token::Char => self.noop_state_transition(),
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }

            ParserState::AttributeNameBegin =>
                match tok {
                    Token::Char => {
                        self.attribute_name.push(ch);

                        ParserState::AttributeName
                    }
                    Token::GreaterThan => {
                        /* Consume gt token */

                        self.data_buffer = String::new();
                        ParserState::Data
                    }
                    Token::Space => self.noop_state_transition(),
                    Token::Slash => {
                        self.is_closing = true;
                        ParserState::TagEnd
                    }
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }
            ParserState::AttributeName =>
                match tok {
                    Token::Char => {
                        self.attribute_name.push(ch);
                        ParserState::AttributeName
                    }
                    Token::Space => self.noop_state_transition(),
                    Token::Equal => {
                        /* Consume equal token */
                        ParserState::AttributeValueBegin
                    }
                    Token::GreaterThan => {
                        /* Consume gt token */
                        self.attribute_value = String::new();

                        self.push_attribute_node();

                        ParserState::AttributeNameEnd
                    }
                    Token::Slash => {
                        self.is_closing = true;
                        self.attribute_value = String::new();

                        self.push_attribute_node();

                        ParserState::TagEnd
                    }
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }
            ParserState::AttributeNameEnd =>
                match tok {
                    Token::Char => {
                        self.attribute_value = String::new();
                        self.push_attribute_node();

                        self.attribute_name = String::from(ch);

                        ParserState::AttributeNameBegin
                    }
                    Token::Space => self.noop_state_transition(),
                    Token::Equal => self.noop_state_transition(),
                    Token::GreaterThan => {
                        /* Consume gt token */
                        self.push_attribute_node();
                        self.data_buffer = String::new();
                        ParserState::Data
                    }
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }

            ParserState::AttributeValueBegin =>
                match tok {
                    Token::Char => {
                        // TODO: should we transition to parsing value instead of error?
                        return Err(ParseError::InvalidDocumentError)
                    }
                    Token::Space => self.noop_state_transition(),
                    Token::Quote => {
                        self.attribute_value = String::new();
                        self.open_quote = Some(QuoteChar::try_from(ch)?);

                        ParserState::AttributeValue
                    }
                    Token::GreaterThan => {
                        self.attribute_value = String::new();
                        self.push_attribute_node();
                        self.data_buffer = String::new();

                        ParserState::Data
                    }
                    _ => return Err(ParseError::UnexpectedToken(self.current_state, tok)),
                }
            ParserState::AttributeValue =>
                match tok {
                    Token::Char => {
                        self.attribute_value.push(ch);

                        ParserState::AttributeValue
                    }
                    Token::Colon => {
                        if self.open_quote.is_some() {
                            self.attribute_value.push(ch);

                            ParserState::AttributeValue
                        } else {
                            return Err(ParseError::UnexpectedToken(self.current_state, tok))
                        }
                    }
                    Token::Space => {
                        if self.open_quote.is_some() {
                            self.attribute_value.push(ch);

                            ParserState::AttributeValue
                        } else {
                            self.push_attribute_node();

                            ParserState::AttributeNameBegin
                        }
                    }
                    Token::Quote => {
                        let end_quote = Some(QuoteChar::try_from(ch)?);
                        if end_quote.eq(&self.open_quote) {
                            self.push_attribute_node();

                            ParserState::AttributeNameBegin
                        } else {
                            self.attribute_value.push(ch);

                            ParserState::AttributeValue
                        }
                    }
                    Token::GreaterThan => {
                        if self.open_quote.is_some() {
                            self.attribute_value.push(ch);

                            ParserState::AttributeValue
                        } else {
                            self.push_attribute_node();

                            self.data_buffer = String::new();

                            ParserState::Data
                        }
                    }
                    Token::Slash => {
                        if self.open_quote.is_some() {
                            self.attribute_value.push(ch);

                            ParserState::AttributeValue
                        } else {
                            self.push_attribute_node();

                            self.is_closing = true;

                            ParserState::TagEnd
                        }
                    }
                    tok => return Err(ParseError::UnexpectedToken(self.current_state, tok))
                }

            ParserState::CommentBegin =>
                match tok {
                    Token::Hyphen => {
                        self.hyphen_count += 1;
                        if self.hyphen_count == 2 {
                            self.hyphen_count = 0;

                            ParserState::Comment
                        } else {
                            self.noop_state_transition()
                        }
                    },
                    Token::Space => self.noop_state_transition(),
                    Token::GreaterThan => {
                        if self.is_comment {
                            self.data_buffer.push(ch);
                            self.noop_state_transition()
                        } else {
                            return Err(ParseError::UnexpectedToken(self.current_state, tok))
                        }
                    }
                    unexpected => return Err(ParseError::UnexpectedToken(self.current_state, unexpected))
                }
            ParserState::Comment =>
                match tok {
                    Token::Hyphen => {
                        self.hyphen_count += 1;
                        match self.peek_token() {
                            Some(Token::Hyphen) => ParserState::CommentEnd,
                            Some(_) => {
                                self.data_buffer.push(ch);
                                ParserState::Comment
                            }
                            None => return Err(ParseError::InvalidDocumentError)
                        }
                    }
                    _ => {
                        self.data_buffer.push(ch);
                        self.noop_state_transition()
                    }
                }
            ParserState::CommentEnd =>
                match tok {
                    Token::GreaterThan => {
                        self.push_comment_node();
                        ParserState::Data
                    }
                    Token::Hyphen => {
                        match self.peek_token() {
                            Some(Token::GreaterThan) => {
                                ParserState::CommentEnd
                            }
                            Some(_) => {
                                self.data_buffer.push_str("--");
                                ParserState::Comment
                            }
                            None => return Err(ParseError::InvalidDocumentError)
                        }
                    }
                    _ => {
                        self.hyphen_count = 0;
                        self.data_buffer.push(ch);
                        self.noop_state_transition()
                    }
                }


            // TODO: actual consume and produce a node?
            ParserState::DeclarationBegin =>
                match tok {
                    _ => {
                        self.is_declaration = true;
                        ParserState::Declaration
                    },
                }
            // TODO: actual consume and produce a node?
            ParserState::Declaration =>
                match tok {

                    Token::Question => ParserState::DeclarationEnd,
                    _ => self.noop_state_transition()
                }
            // TODO: actual consume and produce a node?
            ParserState::DeclarationEnd =>
                match tok {
                    Token::GreaterThan => {
                        self.data_buffer = String::new();
                        ParserState::Data
                    },
                    other_tok => return Err(ParseError::UnexpectedToken(self.current_state, other_tok)),
                }
        };

        self.current_state = next_state;
        Ok(())
    }

    fn noop_state_transition(&self) -> ParserState {
        self.current_state
    }

    fn consume_name_and_namespace(&mut self) -> (String, Option<String>) {
        let name = std::mem::take(&mut self.tag_name);
        let namespace = if !self.tag_namespace.is_empty() {
            Some(std::mem::take(&mut self.tag_namespace))
        } else {
            None
        };

        (name, namespace)
    }

    fn consume_attribute_buffers(&mut self) -> (String, Option<String>) {
        let name = std::mem::take(&mut self.attribute_name);
        let value = if !self.attribute_value.is_empty() {
            Some(std::mem::take(&mut self.attribute_value))
        } else {
            None
        };

        (name, value)
    }

    fn push_node(&mut self, xml_node: LexedXmlNode) {
        self.document.push_node(xml_node);
    }

    fn push_comment_node(&mut self) {
        let comment = std::mem::take(&mut self.data_buffer);
        self.push_node(LexedXmlNode::Comment(comment))
    }

    fn push_attribute_node(&mut self) {
        let (name, value) = self.consume_attribute_buffers();
        self.push_node(LexedXmlNode::new_attribute(name, value));
    }

    fn peek_char(&self) -> Option<char> {
        self.peek_buffer
    }

    fn peek_token(&self) -> Option<Token> {
        Some(Token::char_to_token(self.peek_char()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Result;
    use crate::parser::document::LexedXmlDocument;
    use crate::parser::node::LexedXmlNode;
    use crate::parser::parser::KbXmlParser;

    macro_rules! assert_xml_node {
        ($node:expr, $pat:pat => $body:block) => {
            match $node {
                $pat => $body,
                other => panic!("Expected {}, found: {other:?}", stringify!($pat)),
            }
        };
    }

    #[test]
    fn when_parse_single_tag_then_succeed() {
        let data = "<hello>hi</hello>";

        let mut parser = KbXmlParser::new();

        let doc = parser.parse(data);
        assert!(doc.is_ok());

        let doc = doc.unwrap();
        assert_eq!(doc.len(), 3);

        let open_node = doc.get_node_at(0).cloned().unwrap();
        let expected_open_node_name = String::from("hello");
        assert!(matches!(open_node, LexedXmlNode::TagOpen { .. }));
        match open_node {
            LexedXmlNode::TagOpen { name, namespace } => {
                assert_eq!(name, expected_open_node_name);
                assert_eq!(namespace, None);
            }
            other => assert!(false, "Expected XmlNode::TagOpen, found: {other:?}")
        }

        let content_node = doc.get_node_at(1).cloned().unwrap();
        let expected_content = "hi".to_string();
        assert!(matches!(content_node, LexedXmlNode::Content { .. }));
        match content_node {
            LexedXmlNode::Content(data) => {
                assert_eq!(data, expected_content);
            }
            other => assert!(false, "Expected XmlNode::Content, found: {other:?}")
        }

        let close_node = doc.get_node_at(2).cloned().unwrap();
        let expected_close_node_name = String::from("hello");
        assert!(matches!(close_node, LexedXmlNode::TagClose { .. }));
        match close_node {
            LexedXmlNode::TagClose { name, namespace } => {
                assert_eq!(name, expected_close_node_name);
                assert_eq!(namespace, None);
            }
            other => assert!(false, "Expected XmlNode::TagClose, found: {other:?}")
        }
    }

    #[test]
    fn when_parse_node_with_namespace_then_succeed() -> Result<()> {
        let data = "<kb:hello></kb:hello>";

        let mut parser = KbXmlParser::new();
        let doc = parser.parse(data)?;


        let open_node = doc.get_node_at(0).cloned().unwrap();
        let expected_tag_name = "hello".to_string();
        let expected_tag_namespace = "kb".to_string();

        match open_node {
            LexedXmlNode::TagOpen { name, namespace } => {
                assert_eq!(name, expected_tag_name);

                assert!(namespace.is_some(), "Namespace can not be empty");
                assert_eq!(namespace.unwrap(), expected_tag_namespace);
            }
            other => assert!(false, "Expected XmlNode::TagOpen, found: {other:?}")
        }

        let close_node = doc.get_node_at(1).cloned().unwrap();
        match close_node {
            LexedXmlNode::TagClose { name, namespace } => {
                assert_eq!(name, expected_tag_name);

                assert!(namespace.is_some(), "Namespace can not be empty");
                assert_eq!(namespace.unwrap(), expected_tag_namespace);
            }
            other => assert!(false, "Expected XmlNode::TagClose, found: {other:?}")
        }

        Ok(())
    }

    #[test]
    fn when_parse_nested_tags_then_succeed() -> Result<()> {
        let data = "<hello>Hello<world>World</world></hello>";
        let mut parser = KbXmlParser::new();
        let doc = parser.parse(data)?;

        assert_eq!(doc.len(), 6, "Expected 6 nodes, found {} instead. Document={:?}", doc.len(), doc);
        let first_node = doc.get_node_at(0).expect("First node is valid");
        assert_xml_node!(first_node, LexedXmlNode::TagOpen { name, namespace } => {
            assert_eq!(name, &"hello".to_string());
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        let second_node = doc.get_node_at(1).expect("Second node is valid");
        assert_xml_node!(second_node, LexedXmlNode::Content(data) => {
            assert_eq!(data, &"Hello".to_string())
        });

        let third_node = doc.get_node_at(2).expect("Third node is valid");
        assert_xml_node!(third_node, LexedXmlNode::TagOpen { name, namespace } => {
            assert_eq!(name, &"world".to_string());
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        let fourth_node = doc.get_node_at(3).expect("Fourth node is valid");
        assert_xml_node!(fourth_node, LexedXmlNode::Content(data) => {
            assert_eq!(data, &"World".to_string())
        });

        let fifth_node = doc.get_node_at(4).expect("Fifth node is valid");
        assert_xml_node!(fifth_node, LexedXmlNode::TagClose { name, namespace } => {
            assert_eq!(name, &"world".to_string());
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        let sixth_node = doc.get_node_at(5).expect("Sixth node is valid");
        assert_xml_node!(sixth_node, LexedXmlNode::TagClose { name, namespace } => {
            assert_eq!(name, &"hello".to_string());
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        Ok(())
    }

    #[test]
    fn when_parse_self_closing_tag_then_succeed() -> Result<()> {
        let tags = vec![
            "<kablunk/>",
            "<kablunk />",
            "<kablunk                      />",
        ];

        for data in tags {
            let mut parser = KbXmlParser::new();
            let doc = parser.parse(data)?;

            assert_eq!(doc.len(), 1, "Invalid node count parsed from document: {doc:?}");

            let node = doc.get_node_at(0).expect("Node is valid");
                assert_xml_node!(node, LexedXmlNode::TagSelfClosing { name, namespace } => {
                assert_eq!(name, "kablunk");
                assert!(namespace.is_none(), "Namespace should be empty");
            });
        }

        Ok(())
    }

    #[test]
    fn when_parse_attribute_name_and_value_then_succeed() -> Result<()> {
        let data = "<data foo=\"bar\"></data>";
        let mut parser = KbXmlParser::new();
        let doc = parser.parse(data)?;

        assert_eq!(doc.len(), 3);

        let first_node = doc.get_node_at(0).expect("First node valid");
        assert_xml_node!(first_node, LexedXmlNode::TagOpen { name, namespace } => {
            assert_eq!(name, "data");
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        let second_node = doc.get_node_at(1).expect("Second node valid");
        assert_xml_node!(second_node, LexedXmlNode::Attribute { key, value } => {
            assert_eq!(key, "foo");
            assert_eq!(value.clone().unwrap(), "bar");
        });

        let third_node = doc.get_node_at(2).expect("Third node valid");
        assert_xml_node!(third_node, LexedXmlNode::TagClose { name, namespace } => {
            assert_eq!(name, "data");
            assert!(namespace.is_none(), "Namespace should be empty")
        });

        Ok(())
    }

    #[test]
    fn when_parse_multiple_attribute_name_and_value_then_succeed() -> Result<()> {
        let data = "<data foo=\"bar\" baz=\"qux\" />";
        let mut parser = KbXmlParser::new();
        let doc = parser.parse(data)?;

        assert_eq!(doc.len(), 4, "Expected document to have 3 nodes: {doc:?}");

        let expected_doc = LexedXmlDocument::from_nodes(
            vec![
                LexedXmlNode::TagOpen { name: "data".to_string(), namespace: None },
                LexedXmlNode::Attribute { key: "foo".to_string(), value: Some("bar".to_string()) },
                LexedXmlNode::Attribute { key: "baz".to_string(), value: Some("qux".to_string()) },
                LexedXmlNode::TagClose { name: String::new(), namespace: None },
            ]
        );

        assert_eq!(doc, expected_doc);

        Ok(())
    }

    #[test]
    fn when_parse_comment_then_succeed() -> Result<()> {
        let comments = vec![
            "<!---->",
            "<!-- -->",
            "<!--foo-->",
            "<!-- foo-->",
            "<!--foo -->",
            "<!-- foo -->",
            "<!--  -------------- -- ------ -->",
            r#"<!--
                Source: https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms762271(v=vs.85)
                Copyright © Microsoft.
                Used under the Microsoft Docs license (MIT).
            -->
            "#
        ];

        for (index, tag) in comments.iter().enumerate() {
            let mut parser = KbXmlParser::new();
            let doc = parser.parse(tag)?;

            assert_eq!(doc.len(), 1, "Expected to find comment node in document (index={index}): {doc:?}");

            let comment_node = doc.get_node_at(0).expect("Valid node in comment document");
            assert_xml_node!(comment_node, LexedXmlNode::Comment(comment) => {
                let expected_comment = match index {
                    0 => "",
                    1 => " ",
                    2 => "foo",
                    3 => " foo",
                    4 => "foo ",
                    5 => " foo ",
                    6 => "  -------------- -- ------ ",
                    7 => r#"
                Source: https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms762271(v=vs.85)
                Copyright © Microsoft.
                Used under the Microsoft Docs license (MIT).
            "#,
                    _ => panic!("Unexpected comment tag index"),
                };

                assert_eq!(comment, expected_comment)
            });
        }

        Ok(())
    }

    #[test]
    fn when_parse_xml_declaration_then_consume_without_producing_node() -> Result<()> {
        let data = "<?xml version=\"1.0\"?>";
        let mut parser = KbXmlParser::new();
        let doc = parser.parse(data)?;

        assert_eq!(doc.len(), 0);

        Ok(())
    }

}
