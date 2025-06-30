// TODO: support attributes
#[derive(Clone, Debug, PartialEq)]
pub enum LexedXmlNode {
    TagOpen { name: String, namespace: Option<String>, },
    TagClose { name: String, namespace: Option<String> },
    TagSelfClosing { name: String, namespace: Option<String>, },

    Attribute { key: String, value: Option<String> },
    
    Content(String),
    
    Comment(String),
}

impl LexedXmlNode {

    pub fn new_tag_open(name: String, namespace: Option<String>) -> Self {
        Self::TagOpen { name, namespace }
    }

    pub fn new_tag_close(name: String, namespace: Option<String>) -> Self {
        Self::TagClose { name, namespace }
    }

    pub fn new_tag_self_closing(name: String, namespace: Option<String>) -> Self {
        Self::TagSelfClosing { name, namespace }
    }
    
    pub fn new_attribute(key: String, value: Option<String>) -> Self {
        Self::Attribute { key, value }
    }

    pub fn new_content(data: String) -> Self {
        Self::Content(data)
    }

    pub fn get_name(&self) -> Option<&String> {
        match self {
            LexedXmlNode::TagOpen { name, .. } => Some(name),
            LexedXmlNode::TagClose { name, ..} => Some(name),
            LexedXmlNode::TagSelfClosing { name, .. } => Some(name),
            LexedXmlNode::Attribute { ..} => None,
            LexedXmlNode::Content(_) => None,
            LexedXmlNode::Comment(_) => None,
        }
    }

    pub fn has_namespace(&self) -> bool {
        match self {
            LexedXmlNode::TagOpen { namespace, .. } => namespace.is_some(),
            LexedXmlNode::TagClose { namespace, .. } => namespace.is_some(),
            LexedXmlNode::TagSelfClosing { namespace, .. } => namespace.is_some(),
            LexedXmlNode::Attribute { .. } => false,
            LexedXmlNode::Content(_) => false,
            LexedXmlNode::Comment(_) => false,
        }
    }

    pub fn get_namespace(&self) -> Option<&String> {
        match self {
            LexedXmlNode::TagOpen { namespace, .. } => namespace.as_ref(),
            LexedXmlNode::TagClose { namespace, ..} => namespace.as_ref(),
            LexedXmlNode::TagSelfClosing { namespace, .. } => namespace.as_ref(),
            LexedXmlNode::Attribute { .. } => None,
            LexedXmlNode::Content(_) => None,
            LexedXmlNode::Comment(_) => None,
        }
    }
    
    pub fn is_close_tag(&self) -> bool {
        match self {
            LexedXmlNode::TagClose { .. } => true,
            _ => false,
        }
    }

}