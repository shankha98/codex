use async_trait::async_trait;
use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct SaveMemoryHandler;

#[derive(Deserialize)]
struct SaveMemoryArgs {
    content: String,
    category: String,
    reasoning: String,
}

#[async_trait]
impl ToolHandler for SaveMemoryHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn is_mutating(&self, _invocation: &ToolInvocation) -> bool {
        true
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session, payload, ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "save_memory handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: SaveMemoryArgs = parse_arguments(&arguments)?;

        match session
            .services
            .slate_client
            .commit(
                &args.content,
                "Saved Memory",
                &args.category,
                &args.reasoning,
            )
            .await
        {
            Ok(_) => Ok(ToolOutput::Function {
                content: "Memory saved successfully.".to_string(),
                content_items: None,
                success: Some(true),
            }),
            Err(e) => Err(FunctionCallError::RespondToModel(format!(
                "Failed to save memory: {}",
                e
            ))),
        }
    }
}
