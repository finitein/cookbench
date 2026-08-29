import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import test from 'node:test';

import {stateLabel, validatePayload} from '../protocol.js';

const fixture = JSON.parse(await readFile(new URL('./fixtures/presentation-v1.json', import.meta.url)));

test('accepts a versioned presentation fixture and preserves every stove', () => {
  const payload = validatePayload(fixture);
  assert.ok(payload);
  assert.equal(payload.stoves.length, 3);
  assert.deepEqual(payload.stoves.map(stove => stove.harness), ['Codex', 'Claude Code', 'Pi']);
  assert.equal(stateLabel(payload.stoves[1].state), 'Needs attention');
});

test('rejects sessions, credentials, notification settings, and unknown fields', () => {
  for (const field of ['sessionId', 'nativeSessionPath', 'prompt', 'credential', 'notificationSettings']) {
    const payload = structuredClone(fixture);
    payload.stoves[0][field] = 'must-not-cross-the-presentation-bridge';
    assert.equal(validatePayload(payload), null, field);
  }
});

test('rejects unsupported protocol versions and malformed progress', () => {
  const wrongVersion = structuredClone(fixture);
  wrongVersion.version = 2;
  assert.equal(validatePayload(wrongVersion), null);

  const malformedProgress = structuredClone(fixture);
  malformedProgress.stoves[0].progress = {completed: 4, total: 3};
  assert.equal(validatePayload(malformedProgress), null);
});
