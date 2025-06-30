use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum XmlNode {
    Element(XmlElement),
    Text(String),
    Comment(String),
    CData(String),
}

#[derive(Clone, Debug)]
pub struct XmlElement {
    name: String,
    namespace: Option<String>,
    attributes: HashMap<String, String>,
    children: Vec<XmlNode>,
}

pub struct XmlAttribute<'a> {
    key: &'a str,
    value: &'a str,
}

impl XmlNode {
    pub fn new_element(
        name: impl Into<String>,
        namespace: Option<String>,
        attributes: HashMap<String, String>,
        children: Vec<XmlNode>,
    ) -> Self {
        Self::Element(XmlElement::new(name.into(), namespace, attributes, children))
    }

    pub fn new_text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn new_comment(comment: impl Into<String>) -> Self {
        Self::Comment(comment.into())
    }

    pub fn new_cdata(data: impl Into<String>) -> Self {
        Self::CData(data.into())
    }
    
    pub fn as_element(&self) -> Option<&XmlElement> {
        match self {
            XmlNode::Element(element) => Some(element),
            _ => None,
        }
    }
    
    pub fn as_element_mut(&mut self) -> Option<&mut XmlElement> {
        match self {
            XmlNode::Element(element) => Some(element),
            _ => None,
        }
    }
}

// Querying API
impl XmlElement {
    pub fn new(
        name: String,
        namespace: Option<String>,
        attributes: HashMap<String, String>,
        children: Vec<XmlNode>,
    ) -> Self {
        Self {
            name,
            namespace,
            attributes,
            children
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn namespace(&self) -> Option<&String> {
        self.namespace.as_ref()
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
    
    pub fn attributes_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.attributes
    }
    
    pub fn attr<'a>(&'a self, key: &'a str) -> Option<XmlAttribute<'a>> {
        if self.attributes.contains_key(key) {
            let attr = self.attributes.get(key).unwrap();
            Some(XmlAttribute {
                key,
                value: attr.as_str(),
            })
        } else {
            None
        }
    }

    pub fn children(&self) -> &[XmlNode] {
        self.children.as_slice()
    }

    pub fn children_mut(&mut self) -> &mut [XmlNode] {
        self.children.as_mut_slice()
    }
    
    pub fn take_children(&mut self) -> Vec<XmlNode> {
        std::mem::take(&mut self.children)
    }

    pub fn elements(&self) -> impl Iterator<Item = &XmlElement> {
        self.children.iter()
            .filter_map(|node| {
                match node {
                    XmlNode::Element(element) => Some(element),
                    _ => None,
                }
            })
    }

    pub fn elements_mut(&mut self) -> impl Iterator<Item = &mut XmlElement> {
        self.children.iter_mut()
            .filter_map(|node| {
                match node {
                    XmlNode::Element(element) => Some(element),
                    _ => None,
                }
            })
    }

    pub fn first_named_child(&self, name: &str) -> Option<&XmlElement> {
        self.children.iter()
            .find_map(|node| {
                match node {
                    XmlNode::Element(element) => {
                        if element.name.as_str() == name {
                            Some(element)
                        } else {
                            None
                        }
                    }
                    _ => None
                }
            })
    }
}

// Mutation API
impl XmlElement {
    pub fn set_attr(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    pub fn add_text(&mut self, text: &str) {
        self.children.push(XmlNode::new_text(text.to_string()))
    }

    pub fn add_child(&mut self, child: XmlNode) {
        self.children.push(child)
    }

    pub fn add_element(&mut self, element: XmlElement) {
        self.children.push(XmlNode::Element(element))
    }
}

impl <'a> XmlAttribute<'a> {
    pub fn key(&self) -> &'a str {
        self.key
    }
    
    pub fn value(&self) -> &'a str {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::node::XmlNode;

    #[test]
    fn when_as_element_with_element_node_then_succeed() {
        let node = XmlNode::new_element("foo".to_string(), None, HashMap::new(), Vec::new());
        let element = node.as_element();
        assert!(element.is_some());
        assert_eq!(element.expect("Node is element").name(), "foo");
    }
    
    #[test]
    fn when_as_element_with_non_element_node_then_returns_none() {
        let node = XmlNode::new_text("foo");
        let element = node.as_element();
        assert!(element.is_none());
    }
    
}