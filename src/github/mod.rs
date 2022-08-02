use error_stack::{IntoReport, Result, ResultExt};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT},
    Client,
};
use serde::Deserialize;
use std::error::Error;
use std::fmt::{write, Display};
static BASE_URL: &str = "https://api.github.com";

#[derive(Debug)]
pub enum GHAPIError {
    ClientCreationFailed,
    RequestFailed,
    ResponseUnsuccessful(String),
    FailedToDeserialize,
}

impl Display for GHAPIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientCreationFailed => write(f, format_args!("Creating reqwest client failed")),
            Self::RequestFailed => write(f, format_args!("Sending request failed")),
            Self::ResponseUnsuccessful(msg) => {
                write(f, format_args!("Request unsuccessful - {}", msg))
            }
            Self::FailedToDeserialize => write(f, format_args!("Failed to deserialize")),
        }
    }
}

impl Error for GHAPIError {}

pub struct GithubAPI {
    base_url: String,
    client: Client,
}

fn default_headers(api_key: String) -> HeaderMap {
    let mut hm = HeaderMap::new();
    let val = format!("token {}", api_key);
    hm.insert(AUTHORIZATION, HeaderValue::from_str(&val).unwrap());
    hm.insert(USER_AGENT, HeaderValue::from_static("gh-api-service"));
    hm.insert(ACCEPT, HeaderValue::from_static("application/json"));
    hm
}

impl GithubAPI {
    pub fn new(api_key: String, base_url: Option<String>) -> Result<GithubAPI, GHAPIError> {
        let client = reqwest::Client::builder()
            .default_headers(default_headers(api_key))
            .build()
            .report()
            .change_context(GHAPIError::ClientCreationFailed)?;
        let url = base_url.unwrap_or_else(|| BASE_URL.into());
        Ok(GithubAPI {
            client,
            base_url: url,
        })
    }

    pub async fn get_repository_details(&self, path: String) -> Result<Repository, GHAPIError> {
        let resp = self
            .client
            .get(format!("{}/repos/{}", self.base_url, path))
            .send()
            .await
            .report()
            .change_context(GHAPIError::RequestFailed)?;
        if !resp.status().is_success() {
            let body = resp
                .text()
                .await
                .report()
                .change_context(GHAPIError::FailedToDeserialize)?;
            return Err(error_stack::Report::new(GHAPIError::ResponseUnsuccessful(
                body,
            )));
        }
        resp.json::<Repository>()
            .await
            .report()
            .change_context(GHAPIError::FailedToDeserialize)
    }
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
}

#[cfg(test)]
mod tests {
    use super::GithubAPI;
    use std::sync::Once;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };
    static INIT: Once = Once::new();
    static MOCK_BODY: &str = include_str!("mock_repo_details_body.json");
    fn setup() {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });
    }
    #[tokio::test]
    pub async fn test_get_repository_details() {
        setup();
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/tarkalabs/ssh-signer"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(MOCK_BODY)
                    .insert_header("Content-Type", "application/json"),
            )
            .mount(&server)
            .await;
        let key = std::env::var("GITHUB_TOKEN").unwrap();
        let gapi = GithubAPI::new(key, Some(server.uri())).unwrap();
        let resp = gapi
            .get_repository_details("tarkalabs/ssh-signer".into())
            .await
            .unwrap();
        assert_eq!("tarkalabs/ssh-signer", resp.full_name);
    }
}
