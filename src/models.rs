use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub text: Option<String>,
    pub page: Option<u32>,
    pub include_description: Option<bool>,
}

impl SearchQuery {
    pub fn search_text(&self) -> Option<&str> {
        self.query.as_deref().or(self.text.as_deref())
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct JobListing {
    pub title: String,
    pub url: String,
    pub employer: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplyRequest {
    pub vacancy_url: Option<String>,
    pub url: Option<String>,
    pub cover_letter: String,
    pub resume_id: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ApplyRequest {
    pub fn vacancy_url(&self) -> Option<&str> {
        self.vacancy_url.as_deref().or(self.url.as_deref())
    }

    pub fn into_applied_response(self) -> Value {
        let mut body = self.extra;

        if let Some(row_number) = body.remove("row_number") {
            body.insert("row_number".to_owned(), row_number);
        }
        if let Some(title) = body.remove("title") {
            body.insert("title".to_owned(), title);
        }
        if let Some(vacancy_url) = self.vacancy_url {
            body.insert("vacancy_url".to_owned(), Value::String(vacancy_url));
        }
        if let Some(url) = self.url {
            body.insert("url".to_owned(), Value::String(url));
        }
        if let Some(employer) = body.remove("employer") {
            body.insert("employer".to_owned(), employer);
        }
        if let Some(description) = body.remove("description") {
            body.insert("description".to_owned(), description);
        }

        body.insert("cover_letter".to_owned(), Value::String(self.cover_letter));

        if let Some(resume_id) = self.resume_id {
            body.insert("resume_id".to_owned(), Value::String(resume_id));
        }

        body.insert("status".to_owned(), Value::String("applied".to_owned()));
        Value::Object(body)
    }
}

#[derive(Debug, Serialize)]
pub struct ApplyResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vacancy_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied: Option<bool>,
}

impl ApplyResponse {
    pub fn failed(detail: impl Into<String>) -> Self {
        Self {
            status: "failed".to_owned(),
            message: None,
            detail: Some(detail.into()),
            vacancy_url: None,
            applied: Some(false),
        }
    }
}
