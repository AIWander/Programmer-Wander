#!/usr/bin/env node
// Minimal MCP stdio probe: spawn a server, send initialize, notifications/initialized and
// tools/list as newline-delimited JSON-RPC, wait up to --timeout ms for the tools/list reply,
// then kill the child and print one JSON object to stdout:
//   { ok, serverInfo, protocolVersion, toolCount, annotatedCount, titledCount,
//     tools: [{name, description, hasAnnotations, hasTitle}], stderrTail }
// Usage: node mcp-handshake.mjs [--timeout 5000] [--env KEY=VALUE ...] -- <command> [args...]
import { spawn } from 'node:child_process';

const argv = process.argv.slice(2);
let timeoutMs = 5000;
const extraEnv = {};
let i = 0;
for (; i < argv.length; i += 1) {
  const a = argv[i];
  if (a === '--') { i += 1; break; }
  if (a === '--timeout') { timeoutMs = Number(argv[++i]); continue; }
  if (a === '--env') {
    const kv = argv[++i];
    const eq = kv.indexOf('=');
    extraEnv[kv.slice(0, eq)] = kv.slice(eq + 1);
    continue;
  }
  break;
}
const [command, ...args] = argv.slice(i);
if (!command) {
  process.stderr.write('usage: node mcp-handshake.mjs [--timeout ms] [--env K=V] -- <command> [args...]\n');
  process.exit(2);
}

const child = spawn(command, args, {
  env: { ...process.env, ...extraEnv },
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true
});

let buffer = '';
let stderr = '';
let init = null;
let done = false;

const send = (msg) => child.stdin.write(JSON.stringify(msg) + '\n');

function finish(result) {
  if (done) return;
  done = true;
  clearTimeout(timer);
  try { child.kill(); } catch { /* already gone */ }
  result.stderrTail = stderr.slice(-800);
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
  process.exitCode = result.ok ? 0 : 1;
}

const timer = setTimeout(() => finish({ ok: false, error: `no tools/list reply within ${timeoutMs} ms`, serverInfo: init?.serverInfo ?? null }), timeoutMs);

child.on('error', (err) => finish({ ok: false, error: `spawn failed: ${err.message}` }));
child.on('exit', (code) => { if (!done) finish({ ok: false, error: `server exited early with code ${code}`, serverInfo: init?.serverInfo ?? null }); });
child.stderr.on('data', (d) => { stderr += d.toString(); });
child.stdout.on('data', (d) => {
  buffer += d.toString();
  let nl;
  while ((nl = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, nl).trim();
    buffer = buffer.slice(nl + 1);
    if (!line) continue;
    let msg;
    try { msg = JSON.parse(line); } catch { continue; }
    if (msg.id === 1) {
      if (msg.error) return finish({ ok: false, error: `initialize failed: ${JSON.stringify(msg.error)}` });
      init = msg.result;
      send({ jsonrpc: '2.0', method: 'notifications/initialized' });
      send({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} });
    } else if (msg.id === 2) {
      if (msg.error) return finish({ ok: false, error: `tools/list failed: ${JSON.stringify(msg.error)}`, serverInfo: init?.serverInfo ?? null });
      const tools = (msg.result?.tools ?? []).map((t) => ({
        name: t.name, description: t.description ?? '',
        hasAnnotations: Boolean(t.annotations), hasTitle: Boolean(t.title || t.annotations?.title)
      }));
      finish({
        ok: true,
        serverInfo: init?.serverInfo ?? null,
        protocolVersion: init?.protocolVersion ?? null,
        toolCount: tools.length,
        annotatedCount: tools.filter((t) => t.hasAnnotations).length,
        titledCount: tools.filter((t) => t.hasTitle).length,
        tools
      });
    }
  }
});

send({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'mcpb-pack-handshake', version: '1.0.0' }
  }
});
