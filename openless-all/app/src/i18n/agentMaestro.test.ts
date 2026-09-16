import { de } from './de';
import { en } from './en';
import { es } from './es';
import { fr } from './fr';
import { ja } from './ja';
import { ko } from './ko';
import { zhCN } from './zh-CN';
import { zhTW } from './zh-TW';

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

for (const [localeId, locale] of [
  ['zh-CN', zhCN],
  ['zh-TW', zhTW],
  ['en', en],
  ['ja', ja],
  ['ko', ko],
  ['es', es],
  ['fr', fr],
  ['de', de],
] as const) {
  const providers = locale.settings.providers;
  assert(
    providers.presets.agentMaestro === 'Agent Maestro',
    `${localeId}: Agent Maestro preset label is missing`,
  );
  for (const key of [
    'agentMaestroHint',
    'agentMaestroEndpointInvalid',
    'agentMaestroModelsInvalid',
  ] as const) {
    assert(
      typeof providers[key] === 'string' && providers[key].trim().length > 0,
      `${localeId}: ${key} must be a non-empty string`,
    );
  }
}
