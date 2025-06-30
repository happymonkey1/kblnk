use crate::parser::node::LexedXmlNode;

#[derive(Debug, Default)]
#[derive(PartialEq)]
pub struct LexedXmlDocument {
    nodes: Vec<LexedXmlNode>,
}

impl LexedXmlDocument {
    pub fn new() -> Self {
        Self {
            nodes: Vec::<LexedXmlNode>::new(),
        }
    }
    
    #[cfg(test)]
    pub fn from_nodes(nodes: Vec<LexedXmlNode>) -> Self {
        Self { nodes }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, LexedXmlNode> {
        self.nodes.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, LexedXmlNode> {
        self.nodes.iter_mut()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<LexedXmlNode> {
        self.nodes.into_iter()
    }

    pub fn push_node(&mut self, node: LexedXmlNode) {
        self.nodes.push(node)
    }

    pub fn get_nodes(&self) -> &Vec<LexedXmlNode> {
        &self.nodes
    }

    /// Returns the number of nodes in the [LexedXmlDocument]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node_at(&self, index: usize) -> Option<&LexedXmlNode> {
        self.nodes.get(index)
    }

    pub fn get_node_at_mut(&mut self, index: usize) -> Option<&mut LexedXmlNode> {
        self.nodes.get_mut(index)
    }
}