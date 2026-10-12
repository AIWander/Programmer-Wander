# Privacy Policy: Programmer for Claude Desktop

Effective 2026-10-11. Applies to the Programmer Claude Desktop extension (`programmer-*.mcpb`)
published by AIWander.

Programmer is a local program. It does not collect, sell or share personal data, has no
account, and contains no analytics, telemetry or crash reporting. It runs only while Claude
Desktop runs it, as the signed-in Windows user, and talks only to Claude Desktop over a local
pipe (stdio).

**Data collection.** None by AIWander. Programmer reads files, runs commands and returns
results only when Claude calls one of its tools, and returns those results only to Claude
Desktop. What Claude Desktop then sends to Anthropic as part of your conversation is governed
by Anthropic's privacy policy, not this one.

**Usage and storage.** Programmer stores its own working state on your computer in a
`.programmer` folder inside the extension's install folder: a safety audit log of commands it
checked, saved shell-session settings, live-shell checkpoints, screenshots you asked for, and
WSL job logs. Nothing is stored anywhere else by Programmer itself.

**Third-party sharing.** None. Programmer sends nothing to AIWander or anyone else. It makes a
network connection only when Claude calls `http_request` or `port_check`, and only to the
address named in that call.

**Data retention.** The `.programmer` folder is kept until you update or uninstall the
extension, which deletes it. You can delete it at any time while Claude Desktop is closed.
Changes Claude made to your files with the tools are yours and are not reverted by uninstall.

**Contact.** contact@aiwander.ai, or https://github.com/AIWander/Programmer-Wander/issues.
