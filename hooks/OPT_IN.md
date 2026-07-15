# Optional host policy guidance

Programmer-Wander does not install a hook automatically. Hook schemas, trust stores, and enforcement behavior differ by AI host, and a package must not write another host's config without explicit authorization.

If the host owner adds policy hooks, keep one owner for each decision and audit boundary:

1. Inspect the exact tool name and arguments before a command, destructive file operation, process termination, Git history rewrite, push, or deployment.
2. Bind any approval to that exact call rather than accepting a model-supplied boolean as consent.
3. Record metadata and redacted outcomes after calls; keep secrets and command bodies out of general logs.
4. Fail closed when a blocking pre-call parser or policy check fails.
5. Prove the host can actually block the event before describing the hook as enforcement.

Do not combine this guidance with an equivalent active guard unless one is explicitly designated as advisory. Instructions and skills influence behavior; they do not replace native permission controls.
