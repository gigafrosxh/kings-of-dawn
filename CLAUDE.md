# Kings of Dawn – Claude Code

## Task routing (mandatory)

- Use the current issue and release, not the whole project spec. Read INFO.md sections 1–5 for workflow when needed; use docs/ROADMAP.md to resolve the version/feature file.
- Read only the one matching docs/features/NN-FNN.md and relevant dependencies. For bugs, start with reproduction and impacted code; do not load unrelated feature specs.
- Read docs/ARCHITECTURE.md, docs/SIMULATION.md, docs/DESIGN.md, docs/QUALITY.md or docs/HISTORICAL_ATLAS.md only when relevant. M0: docs/FOUNDATION.md. Master prompt is for project-wide architecture only.
- Never read PROJECT_SPEC.html, docs/archive/, image binaries, embedded base64, or generated UE/Rust directories as text. Open only the specific mockup visually if UI work requires it.
- Search before reading; prefer symbols and narrow ranges. Use RTK/Serena/Context Mode if installed; otherwise native rg and targeted output. Exclude Binaries/, Intermediate/, Saved/, DerivedDataCache/, target/, .git/. Avoid redundant tool calls and full build logs.

## Non-negotiable

- UE5 + Rust authoritative simulation; C++ FFI adapters and UMG/Slate presentation; do not duplicate simulation logic.
- Follow INFO.md: one feature per version, .0 then .1–.5 priority patches; new bugs next patch; .6+ until no known open bugs and final tests pass. No next feature early.
- No fabricated tests, releases or historical data. Do not close feature issue before stabilization. Preserve tests, saves and documented contracts; record blockers.
- Keep replies concise; correctness and verification outrank token savings.

Both AGENTS.md and CLAUDE.md express the same rules; do not load both just to repeat context.
