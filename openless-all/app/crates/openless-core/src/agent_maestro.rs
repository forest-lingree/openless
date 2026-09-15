use serde::Deserialize;

use crate::{BackendError, BackendErrorCode};

pub(crate) const PROVIDER_ID: &str = "agent-maestro";
pub(crate) const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:23333/api/openai/v1";

fn endpoint_error() -> BackendError {
    BackendError::new(BackendErrorCode::InvalidArgument, "agentMaestroEndpointInvalid")
}

pub(crate) fn models_url(endpoint: &str) -> Result<String, BackendError> {
    let mut url = url::Url::parse(endpoint.trim()).map_err(|_| endpoint_error())?;
    if url.host_str().is_none() || !matches!(url.scheme(), "http" | "https") {
        return Err(endpoint_error());
    }
    let path = url.path().trim_end_matches('/');
    let base = path.strip_suffix("/chat/completions").unwrap_or(path);
    let prefix = base.strip_suffix("/api/openai/v1").ok_or_else(endpoint_error)?;
    let path = format!("{prefix}/api/v1/lm/chatModels");
    url.set_path(&path);
    Ok(url.to_string())
}

pub(crate) fn validate_endpoint(endpoint: &str) -> Result<(), BackendError> {
    models_url(endpoint).map(|_| ())
}

#[derive(Deserialize)]
struct Model {
    id: String,
    vendor: String,
}

pub(crate) fn parse_models(body: &[u8]) -> Result<Vec<String>, BackendError> {
    let entries: Vec<Model> = serde_json::from_slice(body)
        .map_err(|_| BackendError::new(BackendErrorCode::Provider, "agentMaestroModelsInvalid"))?;
    let mut models = entries
        .into_iter()
        .filter(|model| model.vendor == "copilot")
        .map(|model| model.id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::BackendErrorCode;

    #[test]
    fn provider_defaults_are_stable_and_valid() {
        assert_eq!(PROVIDER_ID, "agent-maestro");
        assert_eq!(DEFAULT_ENDPOINT, "http://127.0.0.1:23333/api/openai/v1");
        validate_endpoint(DEFAULT_ENDPOINT).unwrap();
    }

    #[test]
    fn models_url_keeps_the_localhost_default_endpoint_shape() {
        let url = models_url("http://localhost:23333/api/openai/v1").unwrap();
        assert_eq!(url, "http://localhost:23333/api/v1/lm/chatModels");
    }

    #[test]
    fn models_url_keeps_custom_ports_and_trailing_slashes() {
        let url = models_url("http://127.0.0.1:24444/api/openai/v1/").unwrap();
        assert_eq!(url, "http://127.0.0.1:24444/api/v1/lm/chatModels");
    }

    #[test]
    fn models_url_keeps_bridge_prefix_and_query_parameters() {
        let url = models_url("https://example.com/bridge/api/openai/v1/chat/completions/?tenant=1")
            .unwrap();
        assert_eq!(url, "https://example.com/bridge/api/v1/lm/chatModels?tenant=1");
    }

    #[test]
    fn models_url_keeps_ipv6_loopback_hosts() {
        let url = models_url("http://[::1]:23333/api/openai/v1").unwrap();
        assert_eq!(url, "http://[::1]:23333/api/v1/lm/chatModels");
    }

    #[test]
    fn models_url_rejects_invalid_endpoints_with_stable_invalid_argument_error() {
        for endpoint in [
            "not-a-url",
            "file:///api/openai/v1",
            "ftp://example.com/api/openai/v1",
            "http://localhost:23333/v1",
            "http://localhost:23333/api/openai/v1/wrong",
        ] {
            let error = models_url(endpoint).unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument);
            assert_eq!(error.message, "agentMaestroEndpointInvalid");
            assert!(error.details.is_none());
        }
    }

    #[test]
    fn parse_models_keeps_only_copilot_models_and_sorts_them() {
        let body = serde_json::json!([
            {"id":"  zulu  ","vendor":"copilot","extra":"ignored"},
            {"id":"alpha","vendor":"other"},
            {"id":"beta","vendor":"copilot"},
            {"id":"beta","vendor":"copilot"},
            {"id":"   ","vendor":"copilot"},
            {"id":" alpha ","vendor":"copilot"}
        ]);

        let models = parse_models(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(models, vec!["alpha".to_string(), "beta".to_string(), "zulu".to_string()]);
    }

    #[test]
    fn parse_models_accepts_empty_and_other_vendor_only_lists() {
        assert!(parse_models(b"[]").unwrap().is_empty());
        assert!(parse_models(br#"[{"id":"x","vendor":"other"}]"#).unwrap().is_empty());
    }

    #[test]
    fn parse_models_rejects_invalid_payloads_with_stable_provider_error() {
        for body in [
            b"not json".as_slice(),
            br#"{"data":[]}"#.as_slice(),
            br#"[{"id":1,"vendor":"copilot"}]"#.as_slice(),
            br#"[{"id":"x"}]"#.as_slice(),
            br#"[null]"#.as_slice(),
        ] {
            let error = parse_models(body).unwrap_err();
            assert_eq!(error.code, BackendErrorCode::Provider);
            assert_eq!(error.message, "agentMaestroModelsInvalid");
            assert!(error.details.is_none());
        }
    }
}
