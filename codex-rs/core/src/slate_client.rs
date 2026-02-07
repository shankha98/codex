use anyhow::Context;
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

    pub async fn focus(&self, content: &str) -> anyhow::Result<()> {
        let url = format!("{}/v1/context", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(&CreateMemoryRequest {
                project_id: &self.project_id,
                content,
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
}
