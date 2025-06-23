use rand::distr::SampleString;
mod command;
mod context;
pub mod prompt;
mod conversation_state;
pub mod message;
pub mod error;
mod util;
mod input;

use std::process::ExitCode;
use std::sync::Arc;
use crossterm::{cursor, execute, queue, style};
use crossterm::style::{Color, Stylize};
use rand_distr::Alphanumeric;
use tokio::signal::ctrl_c;
use tracing::{error, info, warn};
use crate::cli::chat::error::ChatError;
use crate::cli::chat::error::Result;
use crate::cli::chat::command::{ChatCommand, ContextSubcommand};
use crate::cli::chat::conversation_state::ConversationState;
use crate::cli::CLI_NAME;
use crate::cli::util::input_source::InputSource;
use crate::external::auth::auth_credentials::AuthCredentials;
use crate::external::error::StreamingClientError;
use crate::external::LlmServerProvider;
use crate::external::model::SendMessageResponseStream;
use crate::external::streaming_client::{StreamingClientConfig, StreamingClientImpl};
use crate::platform::error::PlatformError;
use crate::platform::PlatformContext;
use crate::util::shared_writer::SharedWriter;

const HELP_TEXT: &str = "Help text should be here...";
const CONTEXT_HELP_TEXT: &str = "Context help text should be here...";
pub const KBLNK_CONTEXT_FILENAME: &str = "KBLNK.md";

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
    info!("Entered init_chat_context");

    let stdin = std::io::stdin();
    let context = PlatformContext::new();

    let output = SharedWriter::stderr();
    let (prompt_request_sender, prompt_request_receiver) = std::sync::mpsc::channel::<Option<String>>();
    let (prompt_response_sender, prompt_response_receiver) = std::sync::mpsc::channel::<Vec<String>>();
    let mut input_source = match InputSource::new(prompt_request_sender, prompt_response_receiver) {
        Ok(input_source) => input_source,
        Err(err) => return Err(ChatError::InitializationError(format!("Failed to initialize InputSource: {err:?}")))
    };

    let conversation_id = Alphanumeric.sample_string(&mut rand::rng(), 9);

    // Load auth credentials from a serialized file
    // When the file is not present, a default auth credentials source is created
    let auth_credentials = AuthCredentials::build_config(Arc::clone(&context)).await?;

    // Try to load config from a serialized file
    // When the file is not present (initial launch), prompt the user to enter requisite initialization information
    let streaming_client_config = match StreamingClientConfig::build_streaming_client_config(Arc::clone(&context)).await {
        Ok(config) => Ok(config),
        Err(err) => match err {
            StreamingClientError::PlatformError(PlatformError::FileNotExistError) => {
                create_streaming_client_config_with_user(Arc::clone(&context), &mut input_source, auth_credentials).await
            }
            err => Err(ChatError::StreamingClientError(err)),
        }
    }?;

    debug_assert!(streaming_client_config.auth_credentials.is_some(), "auth credentials are not valid");

    let client = StreamingClientImpl::new(streaming_client_config).await?;
    info!("Finished constructing streaming client");
    
    let chat_context = ChatContext::new(
        context,
        conversation_id.as_str(),
        output,
        input_source,
        client,
    ).await?;
    info!("Finished construct chat context");
    
    Ok(chat_context)
}

// TODO: refactor this mess
pub async fn create_streaming_client_config_with_user(
    context: Arc<PlatformContext>,
    input_source: &mut InputSource,
    mut auth_credentials: AuthCredentials,
) -> Result<StreamingClientConfig> {
    let model: LlmServerProvider;
    
    
    // TODO: provide user with list of available models

    // Prompt user for the language model provider
    loop {
        let ctrl_c_handler = ctrl_c();
        
        let model_input = tokio::select! {
            Ok(_) = ctrl_c_handler => return Err(ChatError::Interrupted),
            res = input::read_user_input(input_source, Some("Please select your model provider: ")) => res, 
        };
        
        match model_input {
            Ok(model_input) => {
                if let Some(found_model) = LlmServerProvider::try_from_string(model_input.as_str()) {
                    model = found_model;
                    break;
                } 
            }
            Err(ChatError::Interrupted) => return Err(ChatError::Interrupted),
            Err(err) => {
                warn!("Unhandled chat error during user input: {err:?}");
            }
        }
    }

    // Prompt user for credentials based on the found model
    loop {
        match model {
            LlmServerProvider::LlamaCpp => todo!("LlamaCpp model provider is not supported!"),
            LlmServerProvider::GoogleAiStudio { .. } => {
                let api_key = input::read_user_input(input_source, Some("Please input your Google AI Studio API key: ")).await;
                
                if api_key.is_ok() {
                    let api_key = api_key?;
                    let api_key_redo = input::read_user_input(input_source, Some("Please re-enter your Google AI Studio API key: ")).await;
                    
                    if api_key_redo.is_ok() {
                        let api_key_redo = api_key_redo?;
                        if api_key.eq(&api_key_redo) {
                            auth_credentials.google_api_key = Some(api_key);
                            break;
                        }
                    }
                }
            }
        }
    }
    
    auth_credentials.save_config_to_default_location(Arc::clone(&context)).await?;
    
    let streaming_client_config = StreamingClientConfig::builder()
        .with_llm_provider(model)
        .with_auth_credentials(Some(auth_credentials))
        .build()?;
    
    streaming_client_config.save_config_to_default_location(Arc::clone(&context)).await?;
    
    Ok(streaming_client_config)
}

pub struct ChatContext {
    context: Arc<PlatformContext>,
    output: SharedWriter,
    input_source: InputSource,
    conversation_state: ConversationState,
    client: StreamingClientImpl,
}

#[derive(Debug)]
enum ChatState {
    PromptUser,
    UserInput { input: String },
    HandleLlmResponse(SendMessageResponseStream),
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
        client: StreamingClientImpl,
    ) -> Result<Self> {
        let context_clone = Arc::clone(&context);
        let conversation_state = ConversationState::new(context_clone, conversation_id).await;
        
        Ok(Self {
            context: context,
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
                        Ok(_) = ctrl_c_handler => Err(ChatError::Interrupted)
                    }
                }
                ChatState::HandleLlmResponse(response_stream) => {
                    tokio::select! {
                        res = self.handle_response(response_stream) => res,
                        Ok(_) = ctrl_c_handler => Err(ChatError::Interrupted),
                    }
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
        user_input: String,
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

                let client_message = self.conversation_state.as_sendable_conversation_state().await;
                
                ChatState::HandleLlmResponse(self.client.send_message(client_message).await?)
            }
            ChatCommand::Execute { command } => {
                queue!(self.output, style::Print('\n'))?;
                let status = if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", &command]).status()
                } else {
                    std::process::Command::new("bash").args(["-c", &command]).status()
                };
                queue!(self.output, style::Print('\n'))?;

                if status.is_err() {
                    error!("Something went wrong executing command: {:?}", status.err());
                }

                ChatState::PromptUser
            }
            ChatCommand::Help => {
                execute!(self.output, style::Print(HELP_TEXT))?;

                ChatState::PromptUser
            }
            ChatCommand::Clear => {
                warn!("ChatCommand clear is not supported");

                debug_assert!(false, "clear is not implemented");

                ChatState::PromptUser
            }
            ChatCommand::Exit => {
                // TODO: nice llm goodbye message
                execute!(self.output, style::Print("Thanks for chatting!"))?;

                ChatState::Exit
            }
            ChatCommand::Context { subcommand } => {
                match subcommand {
                    ContextSubcommand::Show => {
                        warn!("ContextSubCommand Show is not supported");

                        debug_assert!(false, "context show is not implemented");

                        ChatState::PromptUser
                    }
                    ContextSubcommand::Help => {
                        execute!(self.output, style::Print(CONTEXT_HELP_TEXT))?;

                        ChatState::PromptUser
                    }
                }
            },
            ChatCommand::Usage => {
                let backend_state = self.conversation_state.backend_conversation_state().await;

                if !backend_state.dropped_context_files.is_empty() {
                    execute!(
                        self.output,
                        style::SetForegroundColor(Color::DarkYellow),
                        style::Print("\nSome context files are dropped due to size limit, please run "),
                        style::SetForegroundColor(Color::DarkGreen),
                        style::Print("/context show "),
                        style::SetForegroundColor(Color::DarkYellow),
                        style::Print("to learn more.\n"),
                        style::SetForegroundColor(Color::Reset)
                    )?;
                }

                let context_window_size = self.client.get_context_window_size();
                let usage_data = backend_state.calculate_conversation_size();

                let context_token_count = usage_data.context_messages;
                let llm_token_count = usage_data.assistant_messages;
                let user_token_count = usage_data.user_messages;
                let total_token_used =
                    usage_data.context_messages + usage_data.user_messages + usage_data.assistant_messages;

                let window_width = self.get_terminal_width();
                let progress_bar_width = std::cmp::min(window_width, 80);

                let context_width = ((context_token_count as f64 / context_window_size as f64)
                    * progress_bar_width as f64) as usize;
                let llm_width = ((llm_token_count as f64 / context_window_size as f64)
                    * progress_bar_width as f64) as usize;
                let user_width = ((user_token_count as f64 / context_window_size as f64)
                    * progress_bar_width as f64) as usize;

                let left_over_width = progress_bar_width
                    - std::cmp::min(context_width + llm_width + user_width, progress_bar_width);

                queue!(
                    self.output,
                    style::Print(format!(
                        "\nCurrent context window ({} of {}k tokens used)\n",
                        total_token_used,
                        context_window_size / 1000
                    )),
                    style::SetForegroundColor(Color::DarkCyan),
                    style::Print("|".repeat(if context_width == 0 && context_token_count > 0 {
                        1
                    } else {
                        0
                    })),
                    style::Print("█".repeat(context_width)),
                    style::SetForegroundColor(Color::Blue),
                    style::Print("|".repeat(if llm_width == 0 && llm_token_count > 0 {
                        1
                    } else {
                        0
                    })),
                    style::Print("█".repeat(llm_width)),
                    style::SetForegroundColor(Color::Magenta),
                    style::Print("|".repeat(if user_width == 0 && user_token_count > 0 { 1 } else { 0 })),
                    style::Print("█".repeat(user_width)),
                    style::SetForegroundColor(Color::DarkGrey),
                    style::Print("█".repeat(left_over_width)),
                    style::Print(" "),
                    style::SetForegroundColor(Color::Reset),
                    style::Print(format!(
                        "{:.2}%",
                        (total_token_used as f32 / context_window_size as f32) * 100.0
                    )),
                )?;

                ChatState::PromptUser
            }
        })
    }

    async fn prompt_user(
        &mut self,
    ) -> Result<ChatState> {
        info!("Entered prompt_user");
        let user_input = match self.read_user_input("> ", false) {
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
                (Err(_), _) => {
                    /* no-op */
                },
            }
        }
    }
    
    async fn handle_response(
        &mut self,
        response_stream: SendMessageResponseStream
    ) -> Result<ChatState> {
        todo!("handle response is not implemented!")
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
                    err => return Err(err) 
                }

                Ok(ChatState::PromptUser)
            }
        }
    }

    fn get_terminal_width(&self) -> usize {
        // TODO: dynamically retrieve terminal width
        let terminal_width = 80;
        warn!("Terminal width is defaulting to fixed value: {}", terminal_width);

        terminal_width
    }
}