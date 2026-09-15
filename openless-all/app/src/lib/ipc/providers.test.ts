import { listProviderDescriptors, type ProviderKind } from './providers';
import generated from './provider-descriptors.generated.json';

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

function assertDescriptorMatches(providerType: string) {
  const expected = generated.llm.find((descriptor) => descriptor.providerType === providerType);
  const actual = llmDescriptors.find((descriptor) => descriptor.providerType === providerType);
  assert(expected, `${providerType}: grouped preview snapshot is missing`);
  assert(actual, `${providerType}: runtime descriptor is missing`);
  const keys = new Set([...Object.keys(expected), ...Object.keys(actual)]);
  for (const key of keys) {
    assert(
      JSON.stringify(actual[key as keyof typeof actual]) ===
        JSON.stringify(expected[key as keyof typeof expected]),
      `${providerType}: descriptor mismatch for ${key}`,
    );
  }
}

const llmDescriptors = await listProviderDescriptors('llm');
assertDescriptorMatches('lmstudio');
assertDescriptorMatches('agent-maestro');
assert(
  !(await listProviderDescriptors('asr')).some(
    (descriptor) => descriptor.providerType === 'agent-maestro',
  ),
  'Agent Maestro must not appear in ASR descriptors',
);
assert(
  !(await listProviderDescriptors('omni')).some(
    (descriptor) => descriptor.providerType === 'agent-maestro',
  ),
  'Agent Maestro must not appear in Omni descriptors',
);

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
