import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { listProviderDescriptors } from '../../lib/ipc/providers';
import i18n, { i18nReady } from '../../i18n';
import { en } from '../../i18n/en';
import { HotkeySettingsProvider } from '../../state/HotkeySettingsContext';
import {
  AZURE_DEPLOYMENT_PLACEHOLDER,
  AZURE_ENDPOINT_PLACEHOLDER,
  AZURE_ASR_API_VERSION_ACCOUNT,
  azureApiVersionValidationError,
  ChannelCredentialFields,
  LLM_LABELS,
} from './ProvidersSection';
import { ASR_LABELS } from './shared';
import { presetsFor } from './ChannelList';
import {
  filterOrcaRouterModels,
  readCredential,
  setCredential,
} from '../../lib/ipc/asr-credentials';
import type { ProviderDescriptor } from '../../lib/ipc';

const atlascloudPreset = LLM_LABELS.find((p) => p.id === 'atlascloud');
if (LLM_LABELS.find((p) => p.id === 'agent-maestro')?.nameKey !== 'agentMaestro') {
  throw new Error('Agent Maestro LLM label is missing');
}
if (LLM_LABELS.find((p) => p.id === 'opencode')?.nameKey !== 'opencode') {
  throw new Error('OpenCode LLM label is missing');
}
if (LLM_LABELS.find((p) => p.id === 'tencentTokenHub')?.nameKey !== 'tencentTokenHub') {
  throw new Error('Tencent Cloud TokenHub LLM label is missing');
}
if (LLM_LABELS.find((p) => p.id === 'azure-openai')?.nameKey !== 'azureOpenai') {
  throw new Error('Azure OpenAI LLM label is missing');
}
if (ASR_LABELS.find((p) => p.id === 'azure-openai')?.nameKey !== 'azureOpenai') {
  throw new Error('Azure OpenAI ASR label is missing');
}
if (ASR_LABELS.find((p) => p.id === 'tencent-cloud')?.nameKey !== 'asrTencentCloud') {
  throw new Error('Tencent Cloud ASR label is missing');
}
if (
  AZURE_DEPLOYMENT_PLACEHOLDER !== 'deployment-name' ||
  AZURE_ENDPOINT_PLACEHOLDER !== 'https://your-resource.openai.azure.com' ||
  AZURE_ASR_API_VERSION_ACCOUNT !== 'asr.azure_api_version'
) {
  throw new Error(
    'Azure settings constants must use deployment, resource-root, and ASR API version slots',
  );
}

if (!atlascloudPreset) {
  throw new Error('Atlas Cloud LLM preset is missing');
}

const openAiCompatiblePreset = ASR_LABELS.find((p) => p.id === 'openai-compatible');

if (!openAiCompatiblePreset) {
  throw new Error('Custom OpenAI-compatible ASR preset is missing');
}

const zenmuxPreset = ASR_LABELS.find((p) => p.id === 'zenmux');

if (!zenmuxPreset) {
  throw new Error('ZenMux ASR preset is missing');
}

const coreAsr = presetsFor('asr', 'win', true, undefined, [
  {
    kind: 'asr',
    providerType: 'openai-compatible',
    labelKey: 'asrOpenAiCompatible',
    defaultEndpoint: null,
    defaultModel: null,
    authRequirement: 'endpoint_model_optional_api_key',
    validationProbe: 'asr_silence',
    staticModels: [],
    defaultRequestFormat: null,
    supportedRequestFormats: [],
  },
]);

if (coreAsr.length !== 1 || coreAsr[0].authRequirement !== 'endpoint_model_optional_api_key') {
  throw new Error(
    'Core provider descriptor must replace the browser fallback in the channel picker',
  );
}

const protocolCatalog = [
  { id: 'model/chat', supported_endpoint_types: ['openai'] },
  { id: 'model/responses', supported_endpoint_types: ['openai-response'] },
  { id: 'model/messages', supported_endpoint_types: ['anthropic'] },
  { id: 'model/all', supported_endpoint_types: ['openai', 'openai-response', 'anthropic'] },
  { id: 'model/unknown' },
];
for (const [format, expected] of [
  ['chat_completions', ['model/chat', 'model/all']],
  ['responses', ['model/responses', 'model/all']],
  ['messages', ['model/messages', 'model/all']],
] as const) {
  const actual = filterOrcaRouterModels(protocolCatalog, 'llm', format);
  if (actual.join(',') !== expected.join(',')) {
    throw new Error(`unexpected ${format} catalog: ${actual.join(',')}`);
  }
}

const asrCatalog = [
  {
    id: 'google/gemini-audio',
    supported_endpoint_types: ['openai'],
    architecture: { input_modalities: ['text', 'audio'] },
  },
  {
    id: 'google/gemini-text',
    supported_endpoint_types: ['openai'],
    architecture: { input_modalities: ['text'] },
  },
  { id: 'google/gemini-unknown', supported_endpoint_types: ['openai'] },
  {
    id: 'meta/audio',
    supported_endpoint_types: ['openai'],
    architecture: { input_modalities: ['audio'] },
  },
];
if (filterOrcaRouterModels(asrCatalog, 'asr').join(',') !== 'google/gemini-audio') {
  throw new Error('OrcaRouter ASR catalog must require declared Gemini audio input');
}

for (const labels of [LLM_LABELS, ASR_LABELS]) {
  if (!labels.some((label) => label.id === 'orcarouter' && label.nameKey === 'orcarouter')) {
    throw new Error('OrcaRouter provider label is missing');
  }
}

await i18nReady;
i18n.addResourceBundle('en', 'translation', en, true, true);
await i18n.changeLanguage('en');

const azureLlmDescriptor = {
  kind: 'llm',
  providerType: 'azure-openai',
  labelKey: 'azureOpenai',
  defaultEndpoint: null,
  defaultModel: null,
  authRequirement: 'api_key',
  validationProbe: 'llm_text',
  staticModels: [],
  defaultRequestFormat: 'chat_completions',
  supportedRequestFormats: ['chat_completions', 'responses'],
  supportsModelListing: false,
  supportsThinking: false,
} satisfies ProviderDescriptor;

const azureAsrDescriptor = {
  kind: 'asr',
  providerType: 'azure-openai',
  labelKey: 'azureOpenai',
  defaultEndpoint: null,
  defaultModel: null,
  authRequirement: 'api_key',
  validationProbe: 'asr_silence',
  staticModels: [],
  defaultRequestFormat: null,
  supportedRequestFormats: [],
  supportsModelListing: false,
  supportsThinking: false,
} satisfies ProviderDescriptor;

const renderChannelFields = (
  kind: 'llm' | 'asr',
  descriptor: ProviderDescriptor,
  providerType = 'azure-openai',
) =>
  renderToStaticMarkup(
    createElement(
      HotkeySettingsProvider,
      null,
      createElement(ChannelCredentialFields, {
        kind,
        providerType,
        channelId: `test-${kind}-azure`,
        descriptor,
      }),
    ),
  );

const originalConsoleError = console.error;
console.error = (...args: unknown[]) => {
  if (String(args[0]).includes('useLayoutEffect does nothing on the server')) return;
  originalConsoleError(...args);
};

const azureLlmMarkup = renderChannelFields('llm', azureLlmDescriptor);
for (const expected of [
  'Deployment name',
  'Enter your Azure deployment name, which may differ from the model name.',
  'Enter an Azure deployment name manually; model discovery is not available.',
  'Azure uses deployment defaults for model-specific reasoning settings.',
]) {
  if (!azureLlmMarkup.includes(expected)) {
    throw new Error(`Azure LLM settings markup is missing: ${expected}`);
  }
}
for (const forbidden of ['Messages', 'Fetch models', 'Thinking mode', 'Some models can only']) {
  if (azureLlmMarkup.includes(forbidden)) {
    throw new Error(`Azure LLM settings markup must not include: ${forbidden}`);
  }
}

const azureAsrMarkup = renderChannelFields('asr', azureAsrDescriptor);
for (const markup of [azureLlmMarkup, azureAsrMarkup]) {
  if (markup.includes('fetch and select a model from your provider')) {
    throw new Error('Azure deployment guidance must not advertise model discovery');
  }
}
for (const expected of [
  'Deployment name',
  'Enter your Azure deployment name, which may differ from the model name.',
  'Azure API version',
  'Required for transcription. Use an API version supported by your deployment.',
  'Audio is transcribed after recording stops. This is Azure OpenAI, not Azure AI Speech.',
  'Enter an Azure deployment name manually; model discovery is not available.',
]) {
  if (!azureAsrMarkup.includes(expected)) {
    throw new Error(`Azure ASR settings markup is missing: ${expected}`);
  }
}
if (azureAsrMarkup.includes('Fetch models')) {
  throw new Error('Azure ASR settings markup must not offer model discovery');
}
console.error = originalConsoleError;

if (azureApiVersionValidationError('') !== 'azureApiVersionRequired') {
  throw new Error('Azure API version validator must require non-empty values');
}
if (azureApiVersionValidationError('   ') !== 'azureApiVersionRequired') {
  throw new Error('Azure API version validator must trim blank values');
}
if (azureApiVersionValidationError('2024-10-21') !== null) {
  throw new Error('Azure API version validator must accept stable YYYY-MM-DD values');
}
if (azureApiVersionValidationError(' 2024-10-21-preview ') !== null) {
  throw new Error('Azure API version validator must accept trimmed preview values');
}
if (azureApiVersionValidationError('2024-10-preview') !== 'azureApiVersionInvalid') {
  throw new Error('Azure API version validator must reject malformed values');
}

await setCredential(AZURE_ASR_API_VERSION_ACCOUNT, '2024-10-21', 'azure-asr-version-test');
if (
  (await readCredential(AZURE_ASR_API_VERSION_ACCOUNT, 'azure-asr-version-test')) !== '2024-10-21'
) {
  throw new Error('Azure ASR API version must persist in the channel-scoped credential slot');
}
await setCredential(AZURE_ASR_API_VERSION_ACCOUNT, '', 'azure-asr-version-test');
if ((await readCredential(AZURE_ASR_API_VERSION_ACCOUNT, 'azure-asr-version-test')) !== '') {
  throw new Error('Azure ASR API version removal must use the same channel-scoped credential slot');
}

const azureOrcaRouterDescriptor = {
  ...azureLlmDescriptor,
  providerType: 'orcarouter',
  labelKey: 'orcarouter',
  supportsModelListing: true,
  defaultEndpoint: 'https://orcarouter.example/v1',
  defaultModel: 'model-x',
} satisfies ProviderDescriptor;

const orcaRouterMarkup = renderChannelFields('llm', azureOrcaRouterDescriptor, 'orcarouter');
if (orcaRouterMarkup.includes('Fetch models')) {
  throw new Error('OrcaRouter settings markup must not offer fetch models');
}

const llmDescriptors = await listProviderDescriptors('llm');
const agentMaestro = llmDescriptors.find(
  (descriptor) => descriptor.providerType === 'agent-maestro',
);
const lmstudio = llmDescriptors.find((descriptor) => descriptor.providerType === 'lmstudio');
if (!agentMaestro || !lmstudio) throw new Error('Expected LLM descriptors are missing');

function renderChannelCredentialFields(
  providerType: string,
  descriptor: NonNullable<typeof agentMaestro>,
) {
  return renderToStaticMarkup(
    createElement(HotkeySettingsProvider, {
      children: createElement(ChannelCredentialFields, {
        kind: 'llm',
        providerType,
        channelId: 'render-fixture',
        descriptor,
      }),
    }),
  );
}

const maestroMarkup = renderChannelCredentialFields('agent-maestro', agentMaestro);
const lmstudioMarkup = renderChannelCredentialFields('lmstudio', lmstudio);
if (!maestroMarkup.includes(i18n.t('settings.providers.agentMaestroHint'))) {
  throw new Error('Agent Maestro operational help should render before the fields');
}
if (maestroMarkup.includes(i18n.t('settings.providers.thinkingModeLabel'))) {
  throw new Error('Agent Maestro should hide the thinking toggle');
}
if (!lmstudioMarkup.includes(i18n.t('settings.providers.thinkingModeLabel'))) {
  throw new Error('LM Studio should keep the thinking toggle');
}
if (maestroMarkup.includes(i18n.t('settings.providers.requestFormatLabel'))) {
  throw new Error('Agent Maestro should not render the protocol selector');
}
