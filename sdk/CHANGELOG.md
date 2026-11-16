# Changelog

All notable changes to `@ggoldani/agent-passport-sdk` are documented here. Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/).

## [0.2.0] — 2026-10-25

Everything below landed after the initial 0.1.0 publish (repo path `src/sdk/`).

### Added
- **ScVal conversion layer** — `buildMethodArgs` maps typed SDK arguments to Soroban contract args (`c519264`).
- **`SorobanRpcTransport`** — read (simulate) and write (submit) contract calls directly against Soroban RPC, with Ed25519 signing, transaction preparation and polling (`d75aead`).
- **`submitRichRating()`** plus `RichRatingInput` / `RichRatingRecord` types — on-chain rating with optional quality/speed/reliability/communication sub-scores (`09fd037`, `b0a8a06`).
- **`TrustApiTransport`** — instant REST reads from the public API at `https://agent-passport.xyz` (`126c901`), including `fetchApi()` hook on transports (`14e5709`).
- **`trustCheck()`** — pass/fail trust verification with custom threshold/options (`14e5709`).
- **`getBadgeStats()`** — trust-badge data as JSON (`6668817`).
- **Pagination, relayer set and admin lifecycle** — `from`/`limit` pagination args, `getRelayers()`, two-step admin transfer (`transferAdmin` / `acceptAdmin` / `cancelAdminTransfer`) and profile management (`registerAgent` / `updateProfile` / `deregisterAgent`) (`9452fc4`).

### Fixed
- `TrustApiTransport.getRating` returns `null` only on genuine 404s; resilient indexer sync, auto-created tables, transactional handlers and robust 404 handling (`4790bb6`, `0800627`).
- Indexer pagination arguments and an exhaustive `get_relayers` switch in the API transport (`1eb1bb6`).
- `score` column support in rich ratings and `trust_tier` type alignment across the SDK (`a2a2c83`).
- Security hardening from the code review and security audit rounds (`4a2c599`, `9482e7e`, `39993d2`).

### Changed
- npm publish config and barrel export with NodeNext-compatible `.js` import extensions (`7b5aa1a`, `f5dacec`).
- README: REST-transport quick start, canonical API base `https://agent-passport.xyz`, rate-limit guidance for public reads.

## [0.1.0] — 2026-04-18

Initial npm release — `AgentPassportClient` contract interface and core types.
