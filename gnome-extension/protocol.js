export const PROTOCOL_VERSION = 1;
export const MAX_STOVES = 128;
export const STATES = new Set([
  'starting',
  'planning',
  'cooking',
  'needsHuman',
  'cooked',
  'failed',
  'disconnected',
]);

function isShortText(value, maxLength = 128) {
  return typeof value === 'string' && value.length > 0 && value.length <= maxLength && !/[\u0000-\u001f\u007f]/.test(value);
}

function hasOnlyKeys(value, allowedKeys) {
  return Object.keys(value).every(key => allowedKeys.has(key));
}

function progress(value) {
  if (value === null || value === undefined)
    return null;
  if (!value || typeof value !== 'object' || Array.isArray(value) || !hasOnlyKeys(value, new Set(['completed', 'total'])))
    return null;
  if (!Number.isInteger(value.completed) || !Number.isInteger(value.total))
    return null;
  if (value.completed < 0 || value.total < 1 || value.completed > value.total)
    return null;
  return {completed: value.completed, total: value.total};
}

// This is intentionally a whitelist. Session IDs, paths, prompts, actions,
// host identifiers, locators, notification configuration, and credentials are
// rejected rather than merely ignored.
export function validatePayload(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    return null;
  if (!hasOnlyKeys(value, new Set(['version', 'revision', 'stoves'])))
    return null;
  if (value.version !== PROTOCOL_VERSION || !Number.isInteger(value.revision))
    return null;
  if (!Array.isArray(value.stoves) || value.stoves.length > MAX_STOVES)
    return null;

  const stoves = [];
  for (const stove of value.stoves) {
    if (!stove || typeof stove !== 'object' || Array.isArray(stove))
      return null;
    if (!hasOnlyKeys(stove, new Set(['harness', 'project', 'state', 'progress', 'retainedCompletion'])))
      return null;
    if (!isShortText(stove.harness) || !isShortText(stove.project) || !STATES.has(stove.state))
      return null;
    const validatedProgress = progress(stove.progress);
    if (stove.progress !== null && stove.progress !== undefined && !validatedProgress)
      return null;
    if (typeof stove.retainedCompletion !== 'boolean')
      return null;
    stoves.push({
      harness: stove.harness,
      project: stove.project,
      state: stove.state,
      progress: validatedProgress,
      retainedCompletion: stove.retainedCompletion,
    });
  }

  return {version: PROTOCOL_VERSION, revision: value.revision, stoves};
}

export function stateLabel(state) {
  return {
    starting: 'Starting',
    planning: 'Planning',
    cooking: 'Cooking',
    needsHuman: 'Needs attention',
    cooked: 'Cooked',
    failed: 'Failed',
    disconnected: 'Disconnected',
  }[state] || 'Unknown';
}
