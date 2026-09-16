# Azure OpenAI setup

OpenLess supports **Azure OpenAI** as separate LLM and speech-to-text providers.
Both use Azure resource API keys. This integration does not use Azure AI Speech,
Microsoft Entra ID, or managed identities.

## Before you start

Create an Azure OpenAI resource and deploy the models you want to use:

- A text model deployment for polishing, such as a deployment named `writing-prod`.
- A Whisper or compatible GPT transcription deployment, such as `dictation-prod`.

Use the **deployment name**, not the underlying model name, in OpenLess. These
names are only examples; enter the names from your Azure resource. The two
channels can use different resources and API keys.

Find your resource endpoint and key in the Azure portal's **Keys and Endpoint**
page. Enter keys only in the application's API Key fields; do not put keys in
URLs, screenshots, source code, or bug reports. OpenLess uses its existing
credential storage.

## LLM channel

In Settings, add an LLM channel and select **Azure OpenAI**.

| Field | Value |
| --- | --- |
| API Key | Your Azure OpenAI resource key |
| Base URL / Endpoint | For example, `https://your-resource.openai.azure.com` |
| Deployment name | Your text deployment name, for example `writing-prod` |
| Request format | **Chat Completions** or **Responses** |

OpenLess uses the v1 API:

```text
https://your-resource.openai.azure.com/openai/v1/chat/completions
https://your-resource.openai.azure.com/openai/v1/responses
```

You can enter a resource root, an explicit `/openai/v1` base, or either complete
v1 operation URL. An explicitly qualified v1 URL can retain a gateway prefix.
Legacy deployment-based LLM URLs and the Anthropic Messages format are not
supported by this provider.

LLM responses stream through the existing polishing pipeline. Deployment names
are treated as opaque: OpenLess does not infer model-specific reasoning options
from the deployment name. Temperature is omitted unless explicitly configured;
leave it blank for deployments that do not accept it.

## Transcription channel

Add a speech-recognition/ASR channel and select **Azure OpenAI**.

| Field | Value |
| --- | --- |
| API Key | Your transcription resource key |
| Base URL / Endpoint | For example, `https://your-resource.openai.azure.com` |
| Deployment name | Your transcription deployment name, for example `dictation-prod` |
| Azure API version | A version supported by that deployment |

**The API version is required; there is no automatic default.** Stable versions
use `YYYY-MM-DD`; preview versions use `YYYY-MM-DD-preview`. A Whisper example
such as `2024-10-21` is not a guarantee of support for a GPT transcription model.
Check the documentation and deployment details for your model.

OpenLess constructs a deployment-based file-transcription request:

```text
https://your-resource.openai.azure.com/openai/deployments/dictation-prod/audio/transcriptions?api-version=2024-10-21
```

You can instead enter a complete transcription URL. Its deployment and any
`api-version` query parameter must match the corresponding fields; conflicts
are reported rather than silently overwritten.

Audio is uploaded **after recording stops**, not as live partial transcription.
OpenLess sends 16 kHz mono PCM WAV audio with `response_format=json`, using
Azure's `api-key` header. The deployment must support this plain-text JSON
transcription interface. Diarization and verbose timestamp metadata are not
requested.

Long recordings are split into uploads of at most 10 minutes each, keeping each
WAV below 25 MB. This is an upload-chunk limit, not a 10-minute recording cap.
Results are joined in order. If an upload fails, OpenLess reports the failure
rather than presenting an incomplete transcript as a successful result.

## Test and use the channels

1. Save both channels and use their connection-test controls.
2. Select the Azure channels for your normal ASR + LLM pipeline.
3. Record a short phrase, stop recording, and confirm transcription and polishing.
4. Reopen Settings to confirm the deployment names and STT API version persisted.

Connection tests use the same provider construction as actual dictation. The
ASR test sends a short silence WAV; the LLM test sends a small text prompt. These
tests contact Azure and may incur usage charges.

Model discovery is intentionally unavailable: a general model catalog is not a
list of deployments in your resource. Enter deployment names manually.

The Linux egui host exposes the same Azure configuration, including the STT API
version and the LLM request-format selector.

## Troubleshooting

- **Not configured:** Check endpoint, key, and deployment name. STT also requires
  an API version.
- **401 / 403:** Check the key, resource, and whether key-based access is enabled.
  Entra-only resources cannot use this integration.
- **404:** Check the deployment name, resource, operation, and API version.
- **400:** Check the deployment's supported API version and request format.
  Remove an explicit LLM temperature if the model does not support it.
- **429:** Check Azure quota, deployment capacity, and rate limits.
- **Endpoint conflict:** A complete transcription URL disagrees with the
  deployment or version fields.
- **Malformed or truncated response:** OpenLess rejects failed transfers and
  invalid transcription JSON rather than treating them as empty speech.

This implementation is covered by local HTTP-fixture and configuration tests.
A successful test against your actual Azure resource is still needed to verify
its deployment, region, permissions, and API-version compatibility.

## Microsoft references

- [Azure OpenAI v1 API](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/api-version-lifecycle?view=foundry-classic)
- [Transcription quickstart](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/whisper-quickstart?view=foundry-classic)
- [Audio REST API reference](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/reference?view=foundry-classic)
