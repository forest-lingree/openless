use url::{form_urlencoded, Url};

use crate::{llm_protocol::LlmRequestFormat, BackendError, BackendErrorCode};

pub const PROVIDER_ID: &str = "azure-openai";
pub const MAX_CHUNK_DURATION_MS: u64 = 600_000;

pub fn validate_api_version(version: &str) -> Result<(), BackendError> {
    let version = version.trim();
    if version.is_empty() {
        return Err(invalid_argument("azureApiVersionRequired"));
    }
    if is_valid_api_version(version) {
        Ok(())
    } else {
        Err(invalid_argument("azureApiVersionInvalid"))
    }
}

pub fn llm_endpoint(
    endpoint: &str,
    format: crate::llm_protocol::LlmRequestFormat,
) -> Result<String, BackendError> {
    let operation = llm_operation_segments(format)?;
    let mut url = parse_endpoint(endpoint)?;
    let segments = decoded_path_segments(&url);
    let mut next_segments = match llm_prefix(&segments) {
        Some(prefix) => prefix,
        None => return Err(invalid_argument("azureUnsupportedProtocol")),
    };
    next_segments.extend(["openai", "v1"]);
    next_segments.extend(operation);
    replace_path_segments(&mut url, &next_segments)?;
    Ok(url.to_string())
}

pub fn transcription_endpoint(
    endpoint: &str,
    deployment: &str,
    version: &str,
) -> Result<String, BackendError> {
    let deployment = validate_deployment(deployment)?;
    let version = version.trim();
    validate_api_version(version)?;

    let mut url = parse_endpoint(endpoint)?;
    let segments = decoded_path_segments(&url);
    let mut next_segments = transcription_prefix(&segments, deployment)?;

    ensure_api_version_query(&mut url, version)?;

    next_segments.extend(["openai", "deployments"]);
    next_segments.push(deployment);
    next_segments.extend(["audio", "transcriptions"]);
    replace_path_segments(&mut url, &next_segments)?;
    Ok(url.to_string())
}

fn invalid_argument(message: &str) -> BackendError {
    BackendError::new(BackendErrorCode::InvalidArgument, message)
}

fn parse_endpoint(endpoint: &str) -> Result<Url, BackendError> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err(invalid_argument("azureEndpointInvalid"));
    }
    let url = Url::parse(endpoint).map_err(|_| invalid_argument("azureEndpointInvalid"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(invalid_argument("azureUnsupportedProtocol"));
    }
    crate::endpoint_security::validate_http_endpoint(endpoint)
        .map_err(|_| invalid_argument("azureEndpointInvalid"))?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(invalid_argument("azureEndpointInvalid"));
    }
    Ok(url)
}

fn is_valid_api_version(version: &str) -> bool {
    if let Some(stable) = version.strip_suffix("-preview") {
        return is_stable_api_version(stable);
    }
    is_stable_api_version(version)
}

fn is_stable_api_version(version: &str) -> bool {
    let bytes = version.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn llm_operation_segments(
    format: LlmRequestFormat,
) -> Result<&'static [&'static str], BackendError> {
    match format {
        LlmRequestFormat::ChatCompletions => Ok(&["chat", "completions"]),
        LlmRequestFormat::Responses => Ok(&["responses"]),
        LlmRequestFormat::Messages => Err(invalid_argument("azureUnsupportedProtocol")),
    }
}

fn llm_prefix(segments: &[String]) -> Option<Vec<&str>> {
    match segments {
        [] => Some(Vec::new()),
        [prefix @ .., openai, v1] if openai == "openai" && v1 == "v1" => {
            Some(prefix.iter().map(String::as_str).collect())
        }
        [prefix @ .., openai, v1, chat, completions]
            if openai == "openai"
                && v1 == "v1"
                && chat == "chat"
                && completions == "completions" =>
        {
            Some(prefix.iter().map(String::as_str).collect())
        }
        [prefix @ .., openai, v1, responses]
            if openai == "openai" && v1 == "v1" && responses == "responses" =>
        {
            Some(prefix.iter().map(String::as_str).collect())
        }
        _ => None,
    }
}

fn validate_deployment(deployment: &str) -> Result<&str, BackendError> {
    let deployment = deployment.trim();
    if deployment.is_empty() || matches!(deployment, "." | "..") {
        return Err(invalid_argument("azureDeploymentRequired"));
    }
    Ok(deployment)
}

fn transcription_prefix<'a>(
    segments: &'a [String],
    deployment: &str,
) -> Result<Vec<&'a str>, BackendError> {
    match segments {
        [] => Ok(Vec::new()),
        [prefix @ .., openai, deployments, current, audio, transcriptions]
            if openai == "openai"
                && deployments == "deployments"
                && audio == "audio"
                && transcriptions == "transcriptions" =>
        {
            if current == deployment {
                Ok(prefix.iter().map(String::as_str).collect())
            } else {
                Err(invalid_argument("azureEndpointConflict"))
            }
        }
        _ => Err(invalid_argument("azureUnsupportedProtocol")),
    }
}

fn ensure_api_version_query(url: &mut Url, version: &str) -> Result<(), BackendError> {
    let versions = url
        .query_pairs()
        .filter(|(name, _)| name == "api-version")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    match versions.as_slice() {
        [] => {
            url.query_pairs_mut().append_pair("api-version", version);
            Ok(())
        }
        [current] if current == version => Ok(()),
        [_] | [_, ..] => Err(invalid_argument("azureEndpointConflict")),
    }
}

fn decoded_path_segments(url: &Url) -> Vec<String> {
    let mut segments: Vec<String> = url
        .path_segments()
        .map(|segments| segments.map(decode_segment).collect())
        .unwrap_or_default();
    while matches!(segments.last(), Some(segment) if segment.is_empty()) {
        segments.pop();
    }
    segments
}

fn decode_segment(segment: &str) -> String {
    let escaped = segment
        .replace('+', "%2B")
        .replace('&', "%26")
        .replace('=', "%3D");
    form_urlencoded::parse(format!("value={escaped}").as_bytes())
        .next()
        .map(|(_, value)| value.into_owned())
        .unwrap_or_default()
}

fn replace_path_segments(url: &mut Url, segments: &[&str]) -> Result<(), BackendError> {
    let mut path = url
        .path_segments_mut()
        .map_err(|_| invalid_argument("azureEndpointInvalid"))?;
    path.clear();
    for segment in segments {
        path.push(segment);
    }
    drop(path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::BackendErrorCode;

    #[test]
    fn azure_constants_match_contract() {
        assert_eq!(PROVIDER_ID, "azure-openai");
        assert_eq!(MAX_CHUNK_DURATION_MS, 600_000);
    }

    #[test]
    fn azure_validate_api_version_requires_value() {
        let error = validate_api_version(" \t\r\n").unwrap_err();
        assert_eq!(error.code, BackendErrorCode::InvalidArgument);
        assert_eq!(error.message, "azureApiVersionRequired");
        assert!(!error.retryable);
    }

    #[test]
    fn azure_validate_api_version_rejects_invalid_shapes() {
        for version in ["2024-10-2", "20241021", "2024-10-21-preview2", "preview"] {
            let error = validate_api_version(version).unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{version}");
            assert_eq!(error.message, "azureApiVersionInvalid", "{version}");
        }
    }

    #[test]
    fn azure_validate_api_version_accepts_stable_and_preview_versions() {
        for version in ["2024-10-21", "2024-10-21-preview"] {
            validate_api_version(version).unwrap();
        }
    }

    #[test]
    fn azure_llm_endpoint_builds_chat_completions_from_resource_root() {
        assert_eq!(
            llm_endpoint(
                "https://example.openai.azure.com",
                LlmRequestFormat::ChatCompletions
            )
            .unwrap(),
            "https://example.openai.azure.com/openai/v1/chat/completions"
        );
    }

    #[test]
    fn azure_llm_endpoint_switches_operations_without_duplicate_suffixes() {
        assert_eq!(
            llm_endpoint(
                "https://example.openai.azure.com/openai/v1/chat/completions/",
                LlmRequestFormat::Responses
            )
            .unwrap(),
            "https://example.openai.azure.com/openai/v1/responses"
        );
        assert_eq!(
            llm_endpoint(
                "https://example.openai.azure.com/openai/v1/responses?api=1",
                LlmRequestFormat::ChatCompletions
            )
            .unwrap(),
            "https://example.openai.azure.com/openai/v1/chat/completions?api=1"
        );
    }

    #[test]
    fn azure_llm_endpoint_preserves_gateway_prefixes_and_queries() {
        assert_eq!(
            llm_endpoint(
                "https://example.com/proxy/tenant/openai/v1/?tenant=1&sig=2",
                LlmRequestFormat::Responses
            )
            .unwrap(),
            "https://example.com/proxy/tenant/openai/v1/responses?tenant=1&sig=2"
        );
    }

    #[test]
    fn azure_llm_endpoint_allows_http_fixture_roots() {
        assert_eq!(
            llm_endpoint("http://127.0.0.1:8080", LlmRequestFormat::ChatCompletions).unwrap(),
            "http://127.0.0.1:8080/openai/v1/chat/completions"
        );
    }

    #[test]
    fn azure_llm_endpoint_rejects_messages_and_incompatible_paths() {
        let unsupported_format = llm_endpoint(
            "https://example.openai.azure.com",
            LlmRequestFormat::Messages,
        )
        .unwrap_err();
        assert_eq!(unsupported_format.code, BackendErrorCode::InvalidArgument);
        assert_eq!(unsupported_format.message, "azureUnsupportedProtocol");

        for endpoint in [
            "https://example.openai.azure.com/openai/v1/messages",
            "https://example.openai.azure.com/openai/deployments/model/chat/completions?api-version=2024-10-21",
            "https://example.openai.azure.com/v1/chat/completions",
        ] {
            let error = llm_endpoint(endpoint, LlmRequestFormat::ChatCompletions).unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{endpoint}");
            assert_eq!(error.message, "azureUnsupportedProtocol", "{endpoint}");
        }
    }

    #[test]
    fn azure_llm_endpoint_rejects_invalid_urls_without_echoing_input() {
        for endpoint in [
            "",
            "not a url",
            "ftp://example.openai.azure.com/openai/v1",
            "https://user:pass@example.openai.azure.com/openai/v1",
            "https://example.openai.azure.com/openai/v1#fragment",
        ] {
            let error = llm_endpoint(endpoint, LlmRequestFormat::ChatCompletions).unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{endpoint}");
            assert!(
                matches!(
                    error.message.as_str(),
                    "azureEndpointInvalid" | "azureUnsupportedProtocol"
                ),
                "{endpoint}: {}",
                error.message
            );
            if !endpoint.is_empty() {
                assert!(!error.message.contains(endpoint));
            }
        }
    }

    #[test]
    fn azure_transcription_endpoint_builds_from_root_and_encodes_deployment() {
        assert_eq!(
            transcription_endpoint(
                "https://example.openai.azure.com",
                "dictation/prod",
                "2024-10-21"
            )
            .unwrap(),
            "https://example.openai.azure.com/openai/deployments/dictation%2Fprod/audio/transcriptions?api-version=2024-10-21"
        );
    }

    #[test]
    fn azure_transcription_endpoint_preserves_queries_on_full_urls() {
        assert_eq!(
            transcription_endpoint(
                "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions?tenant=1",
                "dictation-prod",
                "2024-10-21"
            )
            .unwrap(),
            "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions?tenant=1&api-version=2024-10-21"
        );
        assert_eq!(
            transcription_endpoint(
                "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions/?tenant=1&api-version=2024-10-21",
                "dictation-prod",
                "2024-10-21"
            )
            .unwrap(),
            "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions?tenant=1&api-version=2024-10-21"
        );
    }

    #[test]
    fn azure_transcription_endpoint_trims_padded_api_versions() {
        assert_eq!(
            transcription_endpoint(
                "https://example.openai.azure.com",
                "dictation-prod",
                " 2024-10-21 "
            )
            .unwrap(),
            "https://example.openai.azure.com/openai/deployments/dictation-prod/audio/transcriptions?api-version=2024-10-21"
        );
        assert_eq!(
            transcription_endpoint(
                "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions?tenant=1",
                "dictation-prod",
                " 2024-10-21 "
            )
            .unwrap(),
            "https://example.com/proxy/openai/deployments/dictation-prod/audio/transcriptions?tenant=1&api-version=2024-10-21"
        );
    }

    #[test]
    fn azure_llm_endpoint_preserves_internal_empty_gateway_segments() {
        assert_eq!(
            llm_endpoint(
                "https://example.com/proxy//tenant/openai/v1/",
                LlmRequestFormat::Responses
            )
            .unwrap(),
            "https://example.com/proxy//tenant/openai/v1/responses"
        );
    }

    #[test]
    fn azure_transcription_endpoint_preserves_internal_empty_gateway_segments() {
        assert_eq!(
            transcription_endpoint(
                "https://example.com/proxy//tenant/openai/deployments/dictation-prod/audio/transcriptions/",
                "dictation-prod",
                "2024-10-21"
            )
            .unwrap(),
            "https://example.com/proxy//tenant/openai/deployments/dictation-prod/audio/transcriptions?api-version=2024-10-21"
        );
    }

    #[test]
    fn azure_transcription_endpoint_rejects_missing_or_reserved_deployments() {
        for deployment in ["", " \t\r\n", ".", ".."] {
            let error = transcription_endpoint(
                "https://example.openai.azure.com",
                deployment,
                "2024-10-21",
            )
            .unwrap_err();
            assert_eq!(
                error.code,
                BackendErrorCode::InvalidArgument,
                "{deployment:?}"
            );
            assert_eq!(error.message, "azureDeploymentRequired", "{deployment:?}");
        }
    }

    #[test]
    fn azure_transcription_endpoint_rejects_invalid_or_missing_versions() {
        for version in ["", " \t\r\n", "20241021"] {
            let error = transcription_endpoint(
                "https://example.openai.azure.com",
                "dictation-prod",
                version,
            )
            .unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{version:?}");
            assert!(
                matches!(
                    error.message.as_str(),
                    "azureApiVersionRequired" | "azureApiVersionInvalid"
                ),
                "{version:?}: {}",
                error.message
            );
        }
    }

    #[test]
    fn azure_transcription_endpoint_rejects_duplicate_or_conflicting_api_versions() {
        for endpoint in [
            "https://example.openai.azure.com/openai/deployments/dictation-prod/audio/transcriptions?api-version=2024-10-21&api-version=2024-10-21",
            "https://example.openai.azure.com/openai/deployments/dictation-prod/audio/transcriptions?api-version=2025-01-01",
        ] {
            let error =
                transcription_endpoint(endpoint, "dictation-prod", "2024-10-21").unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{endpoint}");
            assert_eq!(error.message, "azureEndpointConflict", "{endpoint}");
        }
    }

    #[test]
    fn azure_transcription_endpoint_rejects_conflicting_deployments_and_paths() {
        let conflicting = transcription_endpoint(
            "https://example.openai.azure.com/openai/deployments/other/audio/transcriptions",
            "dictation-prod",
            "2024-10-21",
        )
        .unwrap_err();
        assert_eq!(conflicting.code, BackendErrorCode::InvalidArgument);
        assert_eq!(conflicting.message, "azureEndpointConflict");

        for endpoint in [
            "https://example.openai.azure.com/openai/v1/chat/completions",
            "https://example.openai.azure.com/openai/deployments/dictation-prod/audio",
        ] {
            let error =
                transcription_endpoint(endpoint, "dictation-prod", "2024-10-21").unwrap_err();
            assert_eq!(error.code, BackendErrorCode::InvalidArgument, "{endpoint}");
            assert_eq!(error.message, "azureUnsupportedProtocol", "{endpoint}");
        }
    }

    #[test]
    fn azure_chunk_duration_stays_within_upload_limit() {
        let wav_bytes = MAX_CHUNK_DURATION_MS * 16_000 * 2 / 1_000 + 44;
        assert!(wav_bytes < 25_000_000, "{wav_bytes}");
    }
}
