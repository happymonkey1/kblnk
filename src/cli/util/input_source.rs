use rustyline::error::ReadlineError;
use crate::cli::chat::prompt::rustyline_editor;

#[derive(Debug)]
pub struct InputSource(inner::Inner);

mod inner {
    use rustyline::Editor;
    use rustyline::history::FileHistory;
    use crate::cli::chat::prompt::ChatHelper;

    #[derive(Debug)]
    pub enum Inner {
        Readline(Editor<ChatHelper, FileHistory>)
    }
}

impl InputSource {
    pub fn new(
        sender: std::sync::mpsc::Sender<Option<String>>,
        receiver: std::sync::mpsc::Receiver<Vec<String>>,
    ) -> anyhow::Result<Self> {
        Ok(Self(inner::Inner::Readline(rustyline_editor(sender, receiver)?) ))
    }
    
    pub fn readline(&mut self, prompt: Option<&str>) -> Result<Option<String>, ReadlineError> {
        match &mut self.0 {
            inner::Inner::Readline(rl) => {
                let prompt = prompt.unwrap_or_default();
                let curr_line = rl.readline(prompt);
                match curr_line {
                    Ok(line) => {
                        let _ = rl.add_history_entry(line.as_str());
                        Ok(Some(line))
                    }
                    Err(ReadlineError::Interrupted | ReadlineError::Eof) => Ok(None),
                    Err(err) => Err(err)
                }
            }
        }
    }
}