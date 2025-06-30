# kb-xml-rs

Simple (non-compliant) XML parser, written in Rust.

# Usage

```rust
let doc = XmlDocument::parse(r#"
    <book xmlns="https://rs.kablunk.com">
        <title>Kablunk</title>
        <author>happymonkey1</author>
    </book>
"#);

let root_node = doc.root()
    .expect("Root node is valid")
    .as_element()
    .expect("Root node is element");

assert_eq!(root_node.name(), "book");
```