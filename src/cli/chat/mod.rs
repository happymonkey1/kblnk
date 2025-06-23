use rand::distr::SampleString;
mod command;
mod context;
pub mod prompt;
mod conversation_state;
pub mod message;

use std::process::ExitCode;
use std::sync::Arc;
use crossterm::{cursor, execute, queue, style};
use crossterm::style::{style, Color, Stylize};
use rand_distr::Alphanumeric;
use thiserror::Error;
use tokio::signal::ctrl_c;
use tracing::{error, info, warn};
use uuid::uuid;
use crate::cli::chat::ChatError::Interrupted;
use crate::cli::chat::command::{ChatCommand, ContextSubcommand};
use crate::cli::chat::conversation_state::ConversationState;
use crate::cli::CLI_NAME;
use crate::cli::util::input_source::InputSource;
use crate::external::streaming_client::StreamingClient;
use crate::platform::PlatformContext;
use crate::util::shared_writer::SharedWriter;

const HELP_TEXT: &str = "Help text should be here...";
const CONTEXT_HELP_TEXT: &str = "Context help text should be here...";

type Result<T> = std::result::Result<T, ChatError>;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("{0}")]
    InitializationError(String),
    #[error("interrupted")]
    Interrupted,
    #[error("{0}")]
    Readline(#[from] rustyline::error::ReadlineError),
    #[error("{0}")]
    StdError(#[from] std::io::Error)
}

pub enum Role {
    User,
    LlmAssistant,
}

pub async fn start_chat_session() -> Result<ExitCode> {
    info!("Starting {CLI_NAME} chat session");

    let mut chat_context = init_chat_context().await?;
    info!("Chat context initialized");
    
    info!("Starting chat loop");
    chat_context.chat_loop().await?;
    info!("Chat loop finished");
    
    Ok(ExitCode::SUCCESS)
}

async fn init_chat_context() -> Result<ChatContext> {

    let stdin = std::io::stdin();
    let context = PlatformContext::new();

    let output = SharedWriter::stderr();
    let (prompt_request_sender, prompt_request_receiver) = std::sync::mpsc::channel::<Option<String>>();
    let (prompt_response_sender, prompt_response_receiver) = std::sync::mpsc::channel::<Vec<String>>();
    let input_source = match InputSource::new(prompt_request_sender, prompt_response_receiver) {
        Ok(input_source) => input_source,
        Err(err) => return Err(ChatError::InitializationError(format!("Failed to initialize InputSource: {err:?}")))
    };

    let conversation_id = Alphanumeric.sample_string(&mut rand::rng(), 9);
    
    let client = StreamingClient::new();
    
    let chat_context = ChatContext::new(
        context,
        conversation_id.as_str(),
        output,
        input_source,
        client,
    ).await?;
    
    Ok(chat_context)
}

pub struct ChatContext {
    ctx: Arc<PlatformContext>,
    output: SharedWriter,
    input_source: InputSource,
    conversation_state: ConversationState,
    client: StreamingClient,
}

#[derive(Debug)]
enum ChatState {
    PromptUser,
    UserInput { input: String },
    HandleLlmResponse,
    Exit,
}

impl Default for ChatState {
    fn default() -> Self {
        Self::PromptUser
    }
}

impl ChatContext {
    pub async fn new(
        context: Arc<PlatformContext>,
        conversation_id: &str,
        output: SharedWriter,
        input_source: InputSource,
        client: StreamingClient,
    ) -> Result<Self> {
        let context_clone = Arc::clone(&context);
        let conversation_state = ConversationState::new(context_clone, conversation_id).await;
        
        Ok(Self {
            ctx: context,
            output,
            input_source,
            conversation_state,
            client,
        })
    }

    async fn chat_loop(&mut self) -> Result<()> {
        info!("Starting chat loop");
        let mut next_state = Some(ChatState::PromptUser);

        loop {
            let chat_state = next_state.take().unwrap_or_default();
            let ctrl_c_handler = ctrl_c();

            info!("Chat state: {chat_state:?}");

            let result: Result<ChatState> = match chat_state {
                ChatState::PromptUser => {
                    self.prompt_user().await
                }
                ChatState::UserInput{ input } => {
                    tokio::select! {
                        res = self.handle_input(input) => res,
                        Ok(_) = ctrl_c_handler => Err(Interrupted)
                    }
                }
                ChatState::HandleLlmResponse => {
                    todo!()
                }
                ChatState::Exit => {
                    info!("Exiting chat loop");
                    return Ok(())
                }
            };

            next_state = Some(self.handle_state_execution_result(result).await?);
        }
    }

    async fn handle_input(
        &mut self,
        mut user_input: String,
    ) -> Result<ChatState> {
        let command_result = ChatCommand::parse(&user_input, &mut self.output);

        if let Err(error_msg) = &command_result {
            execute!(
                self.output,
                style::SetForegroundColor(Color::Red),
                style::Print(format!("\nError: {}\n\n", error_msg)),
                style::SetForegroundColor(Color::Reset)
            )?;

            return Ok(ChatState::PromptUser)
        }

        let command = command_result.unwrap();
        Ok(match command {
            ChatCommand::Ask { prompt } => {
                queue!(self.output, style::SetForegroundColor(Color::Magenta))?;
                queue!(self.output, style::SetForegroundColor(Color::Reset))?;
                queue!(self.output, cursor::Hide)?;
                execute!(self.output, style::Print("\n"))?;
                execute!(self.output, style::Print("Thinking..."))?;

                ChatState::HandleLlmResponse(self.client.send_message(conversation_state).await?)
            }
            ChatCommand::Execute { command } => {
                queue!(self.output, style::Print('\n'))?;
                let status = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", &command]).status()
                } else {
                    std::process::Command::new("bash").args(["-c", &command]).status()
                };
                queue!(self.output, style::Print('\n'))?;

                ChatState::PromptUser
            }
            ChatCommand::Help => {
                execute!(self.output, style::Print(HELP_TEXT));

                ChatState::PromptUser
            }
            ChatCommand::Clear => {
                warn!("ChatCommand warn is not supported");

                ChatState::PromptUser
            }
            ChatCommand::Exit => {
                // TODO: nice llm goodbye message
                execute!(self.output, style::Print("Thanks for chatting!"));

                ChatState::Exit
            }
            ChatCommand::Context { subcommand } => {
                match subcommand {
                    ContextSubcommand::Show => {

                        ChatState::PromptUser
                    }
                    ContextSubcommand::Help => {
                        execute!(self.output, style::Print(CONTEXT_HELP_TEXT));

                        ChatState::PromptUser
                    }
                }
            }
        })
    }

    async fn prompt_user(
        &mut self,
    ) -> Result<ChatState> {
        info!("Entered prompt_user");
        let user_input = match self.read_user_input("", false) {
            Some(user_input) => user_input,
            None => return Ok(ChatState::Exit)
        };

        Ok(ChatState::UserInput {
            input: user_input,
        })
    }

    fn read_user_input(&mut self, prompt: &str, exit_on_single_ctrl_c: bool) -> Option<String> {
        info!("Entered read_user_input");
        let mut ctrl_c = false;
        loop {
            match (self.input_source.readline(Some(prompt)), ctrl_c) {
                (Ok(Some(line)), _) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    return Some(line)
                }
                (Ok(None), false) => {
                    if exit_on_single_ctrl_c {
                        return None
                    }

                    execute!(
                        self.output,
                        style::Print(format!(
                            "\nIn order to exit the cli, pres Ctrl+C or Ctrl+D again or type '{}'",
                            "/exit".green()
                        ))
                    ).unwrap_or_default();

                    ctrl_c = true;
                },
                (Ok(None), true) => return None,
                (Err(_), _) => return None,
            }
        }
    }

    async fn handle_state_execution_result(
        &mut self,
        result: Result<ChatState>
    ) -> Result<ChatState> {
        info!("Entered handle_state_execution_result");
        match result {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("Something went wrong while processing the internal chat state: {err:?}");

                match err {
                    ChatError::Interrupted => {
                        execute!(self.output, style::Print("\n\n"))?;
                        ()
                    }
                    ChatError::InitializationError(_) => {
                        return Err(err)
                    },
                    ChatError::Readline(_) => {
                        return Err(err)
                    }
                    ChatError::StdError(_) => {
                        return Err(err)
                    }
                }

                Ok(ChatState::PromptUser)
            }
        }
    }
}