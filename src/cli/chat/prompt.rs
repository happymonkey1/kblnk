use std::borrow::Cow;
use rustyline::completion::{extract_word, Completer, FilenameCompleter};
use rustyline::{Completer, CompletionType, Config, Context, EditMode, Editor, Helper, Hinter};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::history::DefaultHistory;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use crate::cli::chat::command::{ChatCommand, COMPLETION_COMMANDS};
use crate::cli::chat::Role;

pub struct Prompt {
    role: Role,
    pub(crate) content: String,
}

pub struct ChatCompleter {
    path_completer: PathCompleter,
    prompt_completer: PromptCompleter,
}

impl ChatCompleter {
    fn new(sender: std::sync::mpsc::Sender<Option<String>>, receiver: std::sync::mpsc::Receiver<Vec<String>>) -> Self {
        Self {
            path_completer: PathCompleter::new(),
            prompt_completer: PromptCompleter::new(sender, receiver),
        }
    }
}

impl Completer for ChatCompleter {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let (start, word) = extract_word(line, pos, None, |c| c.is_whitespace());
        
        if word.starts_with('/') {
            return Ok(complete_command(word, start))
        }

        if line.starts_with('@') {
            let search_word = line.strip_prefix('@').unwrap_or("");
            if let Ok(completions) = self.prompt_completer.complete_prompt(search_word) {
                if !completions.is_empty() {
                    return Ok((0, completions));
                }
            }
        }

        if let Ok((pos, completions)) = self.path_completer.complete_path(line, pos, ctx) {
            if !completions.is_empty() {
                return Ok((pos, completions));
            }
        }
        
        Ok((start, Vec::new()))
    }
}

pub struct MultiLineValidator;

impl Validator for MultiLineValidator {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        let input = ctx.input();

        if input.starts_with("```") && !input.ends_with("```") {
            return Ok(ValidationResult::Incomplete)
        }

        if input.ends_with("\\") {
            return Ok(ValidationResult::Incomplete)
        }

        Ok(ValidationResult::Valid(None))
    }
}

struct PathCompleter {
    filename_completer: FilenameCompleter,
}

impl PathCompleter {
    /// Creates a new PathCompleter instance
    pub fn new() -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
        }
    }

    /// Attempts to complete a file path at the given position in the line
    pub fn complete_path(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<String>), ReadlineError> {
        // Use the filename completer to get path completions
        match self.filename_completer.complete(line, pos, ctx) {
            Ok((pos, completions)) => {
                // Convert the filename completer's pairs to strings
                let file_completions: Vec<String> = completions.iter().map(|pair| pair.replacement.clone()).collect();

                // Return the completions if we have any
                Ok((pos, file_completions))
            },
            Err(err) => Err(err),
        }
    }
}

struct PromptCompleter {
    sender: std::sync::mpsc::Sender<Option<String>>,
    receiver: std::sync::mpsc::Receiver<Vec<String>>,
}

impl PromptCompleter {
    fn new(
        sender: std::sync::mpsc::Sender<Option<String>>,
        receiver: std::sync::mpsc::Receiver<Vec<String>>,
    ) -> Self {
        Self {
            sender,
            receiver,
        }
    }
    
    fn complete_prompt(&self, word: &str) -> Result<Vec<String>, ReadlineError> {
        let sender = &self.sender;
        let receiver = &self.receiver;
        
        sender.send(if !word.is_empty() { Some(word.to_string()) } else { None })
            .map_err(|e| ReadlineError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        
        let prompt_info = receiver
            .recv()
            .map_err(|e| ReadlineError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
            .iter()
            .map(|n| format!("@{n}"))
            .collect::<Vec<_>>();
        
        Ok(prompt_info)
    }
}

#[derive(Completer, Helper, Hinter)]
pub struct ChatHelper {
    #[rustyline(Completer)]
    completer: ChatCompleter,
    #[rustyline(Hinter)]
    hinter: (),
    validator: MultiLineValidator
}

impl Validator for ChatHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        self.validator.validate(ctx)
    }
}

impl Highlighter for ChatHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Borrowed(line)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[1m{hint}\x1b[m"))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        false
    }
}

fn complete_command(word: &str, start: usize) -> (usize, Vec<String>) {
    (
        start,
        COMPLETION_COMMANDS.iter()
            .filter(|p| p.starts_with(word))
            .map(|s| (*s).to_owned())
            .collect()
    )
}

pub fn rustyline_editor(
    sender: std::sync::mpsc::Sender<Option<String>>,
    receiver: std::sync::mpsc::Receiver<Vec<String>>,
) -> rustyline::Result<Editor<ChatHelper, DefaultHistory>> {
    let edit_mode = EditMode::Vi;
    
    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(edit_mode)
        .build();
    
    let chat_helper = ChatHelper{
        completer: ChatCompleter::new(sender, receiver),
        hinter: (),
        validator: MultiLineValidator,
    };
    
    let mut rustyline_editor = Editor::with_config(config)?;
    rustyline_editor.set_helper(Some(chat_helper));
    
    Ok(rustyline_editor)
}
