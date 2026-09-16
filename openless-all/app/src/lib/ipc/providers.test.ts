import { listProviderDescriptors, type ProviderKind } from './providers';
import generated from './provider-descriptors.generated.json';

const preview = generated.llm.find((descriptor) => descriptor.providerType === 'lmstudio');
const lmstudio = (await listProviderDescriptors('llm')).find(
  (descriptor) => descriptor.providerType === 'lmstudio',
);
if (
  !preview ||
  !lmstudio ||
  Object.entries(lmstudio).some(
    ([key, value]) =>
      JSON.stringify(value) !== JSON.stringify(preview[key as keyof typeof preview]),
  )
)
  throw new Error('LM Studio preview snapshots must agree');

const azurePreviewLlm = generated.llm.find(
  (descriptor) => descriptor.providerType === 'azure-openai',
);
const azurePreviewAsr = generated.asr.find(
  (descriptor) => descriptor.providerType === 'azure-openai',
);
const azureLlm = (await listProviderDescriptors('llm')).find(
  (descriptor) => descriptor.providerType === 'azure-openai',
);
const azureAsr = (await listProviderDescriptors('asr')).find(
  (descriptor) => descriptor.providerType === 'azure-openai',
);
if (
  !azurePreviewLlm ||
  !azurePreviewAsr ||
  !azureLlm ||
  !azureAsr ||
  azurePreviewLlm.defaultEndpoint !== null ||
  azurePreviewLlm.defaultModel !== null ||
  azurePreviewAsr.defaultEndpoint !== null ||
  azurePreviewAsr.defaultModel !== null ||
  azureLlm.defaultEndpoint !== null ||
  azureLlm.defaultModel !== null ||
  (azureLlm as { supportsModelListing?: boolean }).supportsModelListing !== false ||
  (azureLlm as { supportsThinking?: boolean }).supportsThinking !== false ||
  (azureAsr as { supportsModelListing?: boolean }).supportsModelListing !== false
) {
  throw new Error(
    'Azure preview descriptors must expose no defaults and opt out of discovery/thinking',
  );
}

for (const kind of ['asr', 'llm', 'omni'] as ProviderKind[]) {
  const descriptors = await listProviderDescriptors(kind);
  if (!descriptors.length)
    throw new Error(
      `${kind}: browser preview must expose the Core provider catalog so its editor can load`,
    );
  const ids = new Set<string>();
  for (const descriptor of descriptors) {
    if (
      descriptor.kind !== kind ||
      !descriptor.providerType ||
      !descriptor.labelKey ||
      !descriptor.authRequirement
    ) {
      throw new Error(`${kind}: incomplete provider descriptor`);
    }
    if (ids.has(descriptor.providerType))
      throw new Error(`${kind}: duplicate provider ${descriptor.providerType}`);
    ids.add(descriptor.providerType);
  }
}
console.log('browser provider catalog exposes ASR, LLM and Omni editors');
