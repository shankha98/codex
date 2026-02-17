use anyhow::Context;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct SlateClient {
    client: Client,
    base_url: String,
    project_id: String,
}

#[derive(Serialize)]
struct CreateMemoryRequest<'a> {
    project_id: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    relevance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decay: Option<f32>,
}

#[derive(Deserialize)]
struct Memory {
    content: String,
}

impl SlateClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            project_id: "00000000-0000-0000-0000-000000000000".to_string(),
        }
    }

    pub async fn focus(
        &self,
        content: &str,
        relevance: Option<f32>,
        decay: Option<f32>,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/context", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(&CreateMemoryRequest {
                project_id: &self.project_id,
                content,
                relevance,
                decay,
            })
            .send()
            .await?;

        if let Err(e) = res.error_for_status_ref() {
            eprintln!("Slate focus error status: {}", e);
            return Err(e.into());
        }
        eprintln!("Slate focused on content length: {}", content.len());
        Ok(())
    }

    pub async fn focus_transient(&self, content: &str) -> anyhow::Result<()> {
        self.focus(content, None, Some(0.5)).await
    }

    pub async fn focus_important(&self, content: &str) -> anyhow::Result<()> {
        self.focus(content, Some(1.5), Some(0.01)).await
    }

    pub async fn trigger_skill(
        &self,
        skill_name: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/skills/{}/run", self.base_url, skill_name);
        #[derive(Serialize)]
        struct RunSkillRequest<'a> {
            project_id: &'a str,
            input: serde_json::Value,
        }

        let res = self
            .client
            .post(&url)
            .json(&RunSkillRequest {
                project_id: &self.project_id,
                input: args,
            })
            .send()
            .await?;

        if let Err(e) = res.error_for_status_ref() {
            eprintln!("Slate trigger_skill error status: {}", e);
            return Err(e.into());
        }
        eprintln!("Slate triggered skill: {}", skill_name);
        Ok(())
    }

    pub async fn reminisce(&self, query: &str) -> anyhow::Result<Vec<String>> {
        let url = format!("{}/v1/history", self.base_url);
        let res = self
            .client
            .get(&url)
            .query(&[
                ("project_id", &self.project_id),
                ("query", &query.to_string()),
            ])
            .send()
            .await?;

        if let Err(e) = res.error_for_status_ref() {
            eprintln!("Slate reminisce error status: {}", e);
            return Err(e.into());
        }
        let memories: Vec<Memory> = res
            .json()
            .await
            .context("failed to parse reminisce response")?;

        let results: Vec<String> = memories.into_iter().map(|m| m.content).collect();
        eprintln!(
            "Slate reminisce found {} memories for query '{}'",
            results.len(),
            query
        );
        Ok(results)
    }

    pub async fn commit(
        &self,
        input: &str,
        outcome: &str,
        action: &str,
        reasoning: &str,
    ) -> anyhow::Result<()> {
        let url = format!("{}/v1/history", self.base_url);
        let content = format!(
            "Input: {}\nAction: {}\nReasoning: {}\nOutcome: {}",
            input, action, reasoning, outcome
        );
        let res = self
            .client
            .post(&url)
            .json(&CreateMemoryRequest {
                project_id: &self.project_id,
                content: &content,
                relevance: None,
                decay: None,
            })
            .send()
            .await?;

        if let Err(e) = res.error_for_status_ref() {
            eprintln!("Slate commit error status: {}", e);
            return Err(e.into());
        }
        eprintln!("Slate committed: {} -> {}", input, outcome);
        Ok(())
    }

    pub async fn fetch_skills(&self) -> anyhow::Result<Vec<DynamicToolSpec>> {
        let url = format!("{}/v1/skills", self.base_url);

        #[derive(Deserialize)]
        struct SlateSkill {
            name: String,
            description: String,
            input_schema: serde_json::Value,
        }

        let res = self
            .client
            .get(&url)
            .query(&[("project_id", &self.project_id)])
            .send()
            .await?;

        if let Err(e) = res.error_for_status_ref() {
            // 404: Endpoint not found (older server?)
            // 405: Method not allowed (server only supports POST for creation, not listing)
            if e.status() == Some(reqwest::StatusCode::NOT_FOUND) 
                || e.status() == Some(reqwest::StatusCode::METHOD_NOT_ALLOWED) 
            {
                return Ok(vec![]);
            }
            eprintln!("Slate fetch_skills error status: {}", e);
            return Err(e.into());
        }

        let skills: Vec<SlateSkill> = res.json().await?;

        let tools = skills
            .into_iter()
            .map(|skill| DynamicToolSpec {
                name: skill.name,
                description: skill.description,
                input_schema: skill.input_schema,
            })
            .collect();

        Ok(tools)
    }
}
