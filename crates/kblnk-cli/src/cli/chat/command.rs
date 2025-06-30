use std::io::Write;

#[derive(Debug, PartialEq, Eq)]
pub enum ChatCommand {
    Ask {
        prompt: String,
    },
    Clear,
    Context { subcommand: ContextSubcommand },
    Execute { command: String },
    Usage,
    Exit,
    Help,
}

pub const COMPLETION_COMMANDS: &[&str] = &[
    "/clear",
    "/context show",
    "/usage",
    "/exit",
    "/help"
];

#[derive(Debug, PartialEq, Eq)]
pub enum ContextSubcommand {
    Show,
    Help,
}

impl ChatCommand {
    pub fn parse(input: &str, output: &mut impl Write) -> Result<Self, String> {
        let input = input.trim();

        if input.starts_with("\\/") {
            return Ok(Self::Ask{ prompt: input[1..].to_string() })
        }
        
        if let Some(command) = input.strip_prefix("/") {
            let parts = command.split_whitespace().collect::<Vec<&str>>();
            
            if parts.is_empty() {
                return Err("No command found".to_string())
            }
            
            return Ok(match parts[0].to_lowercase().as_str() {
                "clear" | "cls" => Self::Clear,
                "context" => {
                    if parts.len() < 2 {
                        return Ok(Self::Context { subcommand: ContextSubcommand::Help })
                    }
                    
                    match parts[1].to_lowercase().as_str() {
                        "show" => {
                            Self::Context { subcommand: ContextSubcommand::Show }
                        }
                        context_subcommand_unknown => {
                            tracing::warn!("Failed to parse context subcommand: {context_subcommand_unknown}");
                            Self::Context { subcommand: ContextSubcommand::Help }
                        }
                    }
                }
                "usage" | "u" => Self::Usage,
                "exit" | "q" | "quit" => Self::Exit,
                "help" | "h" | "?" => Self::Help,
                unknown => {
                    // Check for aliases, which may not be currently supported, and display corresponding command suggestion(s) 
                    if let Some(suggestion) = Self::check_alias_commands(input) {
                        return Err(suggestion)
                    }

                    return Err(format!(
                        "Unknown command: '/{}'. Type '/help' for a list of available commands and their usage.",
                        unknown
                    ))
                }
            })
        }
        
        if let Some(command) = input.strip_prefix("!") {
            return Ok(Self::Execute {
                command: command.to_string()
            });
        }
        
        Ok(Self::Ask {
            prompt: input.to_string()
        })
    }
    
    fn check_alias_commands(input: &str) -> Option<String> {
        let lowered = input.to_lowercase();
        match lowered.as_str() {
            "quit" | "q" | "exit()" => {
                Some("Did you mean to use the command '/exit' to exit? Type '/exit' to exit.".to_string())
            }
            "clear" | "cls" => {
                Some("Did you mean to use the command '/clear' to clear the conversation? Type '/clear' to clear.".to_string())
            }
            "help" | "?" => {
                Some("Did you mean to use the command '/help' for help? Type '/help' to see available commands and their usage.".to_string())
            }
            _ => None,
        }
    }
}