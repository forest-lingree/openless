use openless_core::llm_protocol::LlmRequestFormat;
use openless_core::{BackendError, BackendErrorCode, ChannelKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AzureRequestFormatState {
    pub selected: Option<LlmRequestFormat>,
    pub invalid_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelListingAction {
    Hyperlink(String),
    FetchButton,
    ManualGuidance(String),
    Hidden,
}

pub fn request_format_label(format: LlmRequestFormat) -> &'static str {
    match format {
        LlmRequestFormat::ChatCompletions => "Chat Completions",
        LlmRequestFormat::Responses => "Responses",
        LlmRequestFormat::Messages => "Messages",
    }
}

pub fn request_format_account_value(format: LlmRequestFormat) -> &'static str {
    match format {
        LlmRequestFormat::ChatCompletions => "chat_completions",
        LlmRequestFormat::Responses => "responses",
        LlmRequestFormat::Messages => "messages",
    }
}

pub fn load_azure_request_format_state(
    raw: Option<String>,
    default: Option<LlmRequestFormat>,
    supported: &[LlmRequestFormat],
) -> AzureRequestFormatState {
    let default = default.filter(|format| supported.contains(format));
    match raw
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        None => AzureRequestFormatState {
            selected: default,
            invalid_value: None,
        },
        Some(value) => match LlmRequestFormat::parse(&value) {
            Ok(format) if supported.contains(&format) => AzureRequestFormatState {
                selected: Some(format),
                invalid_value: None,
            },
            _ => AzureRequestFormatState {
                selected: None,
                invalid_value: Some(value),
            },
        },
    }
}

pub fn azure_api_version_validation_error(value: &str) -> Option<&'static str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Some("Azure OpenAI API version is required")
    } else if openless_core::azure_openai::validate_api_version(trimmed).is_ok() {
        None
    } else {
        Some("Azure OpenAI API version must use YYYY-MM-DD or YYYY-MM-DD-preview")
    }
}

pub fn model_listing_action(
    supports_model_listing: bool,
    models_url: Option<&str>,
    _provider_type: &str,
    _kind: ChannelKind,
) -> ModelListingAction {
    if let Some(url) = models_url {
        ModelListingAction::Hyperlink(url.to_string())
    } else if supports_model_listing {
        ModelListingAction::FetchButton
    } else if _provider_type == "azure-openai" {
        ModelListingAction::ManualGuidance(
            "Azure OpenAI 不支持列出模型；请手动填写 Deployment name。".to_string(),
        )
    } else {
        ModelListingAction::Hidden
    }
}

pub fn validate_azure_editor_state(
    kind: ChannelKind,
    provider_type: &str,
    azure_api_version: &str,
    request_format: &AzureRequestFormatState,
) -> Result<(), BackendError> {
    if provider_type != "azure-openai" {
        return Ok(());
    }
    match kind {
        ChannelKind::Asr => {
            if let Some(message) = azure_api_version_validation_error(azure_api_version) {
                Err(BackendError::new(BackendErrorCode::InvalidArgument, message))
            } else {
                Ok(())
            }
        }
        ChannelKind::Llm => {
            if request_format.invalid_value.is_some() || request_format.selected.is_none() {
                Err(BackendError::new(
                    BackendErrorCode::InvalidArgument,
                    "Azure OpenAI request format is invalid; choose Chat Completions or Responses",
                ))
            } else {
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openless_core::BackendErrorCode;

    #[test]
    fn missing_azure_request_format_uses_supported_default() {
        let state = load_azure_request_format_state(
            None,
            Some(LlmRequestFormat::ChatCompletions),
            &[LlmRequestFormat::ChatCompletions, LlmRequestFormat::Responses],
        );
        assert_eq!(state.selected, Some(LlmRequestFormat::ChatCompletions));
        assert_eq!(state.invalid_value, None);
    }

    #[test]
    fn valid_azure_request_format_preserves_saved_selection() {
        let state = load_azure_request_format_state(
            Some("responses".to_string()),
            Some(LlmRequestFormat::ChatCompletions),
            &[LlmRequestFormat::ChatCompletions, LlmRequestFormat::Responses],
        );
        assert_eq!(state.selected, Some(LlmRequestFormat::Responses));
        assert_eq!(state.invalid_value, None);
    }

    #[test]
    fn invalid_azure_request_format_is_preserved_for_repair() {
        let state = load_azure_request_format_state(
            Some("messages".to_string()),
            Some(LlmRequestFormat::ChatCompletions),
            &[LlmRequestFormat::ChatCompletions, LlmRequestFormat::Responses],
        );
        assert_eq!(state.selected, None);
        assert_eq!(state.invalid_value.as_deref(), Some("messages"));
    }

    #[test]
    fn azure_api_version_validation_requires_stable_or_preview_dates() {
        assert_eq!(
            azure_api_version_validation_error(""),
            Some("Azure OpenAI API version is required"),
        );
        assert_eq!(
            azure_api_version_validation_error("2024-10-preview"),
            Some("Azure OpenAI API version must use YYYY-MM-DD or YYYY-MM-DD-preview"),
        );
        assert_eq!(azure_api_version_validation_error("2024-10-21"), None);
        assert_eq!(azure_api_version_validation_error(" 2024-10-21-preview "), None);
    }

    #[test]
    fn azure_model_listing_uses_manual_guidance_when_disabled() {
        let action = model_listing_action(false, None, "azure-openai", ChannelKind::Llm);
        match action {
            ModelListingAction::ManualGuidance(text) => {
                assert!(text.contains("Deployment name"), "{text}");
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn azure_llm_save_validation_rejects_invalid_request_format() {
        let error = validate_azure_editor_state(
            ChannelKind::Llm,
            "azure-openai",
            "",
            &AzureRequestFormatState {
                selected: None,
                invalid_value: Some("messages".to_string()),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, BackendErrorCode::InvalidArgument);
        assert!(error.message.contains("Chat Completions"), "{}", error.message);
    }

    #[test]
    fn azure_asr_save_validation_requires_api_version() {
        let error = validate_azure_editor_state(
            ChannelKind::Asr,
            "azure-openai",
            "   ",
            &AzureRequestFormatState::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, BackendErrorCode::InvalidArgument);
        assert!(error.message.contains("API version"), "{}", error.message);
    }
}
