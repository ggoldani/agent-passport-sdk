# Security Posture

## Audit program

AgentPassport went through **4 independent audit runs** before mainnet cutover, each performed by a separate adversarial reviewer with a fresh context:

| Run | Scope | Outcome |
|---|---|---|
| Run 1 | Contract + protocol design (Sybil, farming, trust integrity) | 8 findings, all fixed |
| Run 2 | API + indexer surface, rate limiting, auth | HIGH/MED findings, all fixed |
| Run 3 | Full project, adversarial (DoS cost, farming economics, pair-brick) | 39 findings; 2 HIGH + 3 MED prioritized |
| Run 4 | Confirmation run (delta + regression) | **GO for mainnet**: 2 HIGH confirmed dead, 3 MED confirmed dead, zero regressions across all 15+ historical findings |

Total test coverage at cutover: **254 application tests + 71 contract tests**, all green; TypeScript strict compile (two tsconfig targets); contract byte-a-byte verified on-chain against the audited artifact.

## Anti-Sybil / anti-farm design

- Ratings require **settled paid interactions** — every sample costs real fees
- **Economic age anchor**: reputation regresses toward zero on identity re-registration; farming a fresh identity means re-building age with real payments
- **Per-pair caps**: a single consumer/provider pair contributes bounded score regardless of interaction volume
- **/24 IP bucketing**: IP rotation cannot multiply rate limits or quotas
- **Statistical sample floor**: public score stays `unrated` below sample threshold; `trusted` gate requires on-chain evidence with statistical significance (fail-closed — unknown providers are never trusted by default)
- **Relayer key custody**: production signing keys held in a secret vault; deploy keys used cold, never persisted on the server

## Residual risk disclosure

- The relayer is a **trusted infrastructure component** (it files free-tier subsidized interactions). Its queue is auditable on-chain: every filed interaction carries the relayer signature.
- Discovery metadata (names, descriptions, tags) is user-supplied and not curated — the trust signal is the on-chain rating/tier history, not the metadata.
- Testnet artifacts (keys, agents) used during development carry no value and are purged from production indexes.

## Disclosure policy

Found an issue? Open a private security advisory on this repository (do **not** open a public issue with exploit details). We acknowledge within 72h.
