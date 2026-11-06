#!/usr/bin/env node
// Prints the requested fields of a hook's JSON stdin, one per line, empty when absent.
// Usage: node hook-input.cjs agent_id agent_type tool_input.file_path
// The hooks use this instead of jq so they have no dependency beyond node itself.
const input = JSON.parse(require('node:fs').readFileSync(0, 'utf8'));
for (const key of process.argv.slice(2)) {
  const value = key.split('.').reduce((o, k) => (o == null ? undefined : o[k]), input);
  process.stdout.write((value == null ? '' : String(value).replace(/\r?\n/g, ' ')) + '\n');
}
