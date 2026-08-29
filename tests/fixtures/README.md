# Sanitized Test Fixtures

Fixtures are synthetic, deterministic metadata that prove parser and adapter
behavior without storing user data. They may contain invented event kinds,
opaque IDs, timestamps, and path shapes needed by a test.

Never commit real user session files, prompts, assistant output, source code,
commands, terminal output, tokens, credentials, API keys, or any native
harness session export. Do not hand-edit a fixture from a local session.

Fixture generators must replace all content-bearing fields with stable
placeholders, preserve only the structural fields required by the test, bound
record sizes, and document their input assumptions. Generators are development
tools; their input and any generated raw output must remain outside the repo.
