
pub struct TokenCounter;

impl TokenCounter {
    pub const TOKEN_TO_CHAR_RATIO: usize = 3;
    
    pub fn count_tokens(content: &str) -> usize {
        let chars = content.len();

        (chars / Self::TOKEN_TO_CHAR_RATIO + 5) / 10 * 10
    }
}