use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use async_trait::async_trait;
use serde_json::Value;

pub struct SlateHandler;

#[async_trait]
impl ToolHandler for SlateHandler {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn is_mutating(&self, _invocation: &ToolInvocation) -> bool {
        // Slate skills might be mutating, we don't know. Safer to say true.
        true
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let ToolInvocation {
            session,
            tool_name,
            payload,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "slate handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: Value = parse_arguments(&arguments)?;

        match session
            .services
            .slate_client
            .trigger_skill(&tool_name, args)
            .await
        {
            Ok(_) => Ok(ToolOutput::Function {
                content: format!("Skill '{}' triggered successfully", tool_name),
                content_items: None,
                success: Some(true),
            }),
            Err(e) => Err(FunctionCallError::RespondToModel(format!(
                "failed to trigger skill: {}",
                e
            ))),
        }
    }
}
