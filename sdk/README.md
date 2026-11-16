# @ggoldani/agent-passport-sdk

TypeScript SDK for the [Agent Passport](https://github.com/ggoldani/agent-passport-sdk trust registry on Stellar/Soroban. Provides typed read/write access to the smart contract with pluggable transports.

## Install

```bash
npm install @ggoldani/agent-passport-sdk
```

## Quick Start

### On-chain transport (direct RPC)

```typescript
import { AgentPassportClient, SorobanRpcTransport } from "@ggoldani/agent-passport-sdk"

const transport = new SorobanRpcTransport({
  rpcUrl: "https://soroban-rpc.stellar.org",
  networkPassphrase: "Public Global Stellar Network ; September 2022",
  signerSecretKey: process.env.SECRET_KEY!,
})

const client = new AgentPassportClient({
  contractId: "CD2ZS3ADHBJZ6ISEZQ5QA6DMAC264Z6AMYCMP6RF37BVUQK3UGMBGVAA",
  transport,
})

const agent = await client.getAgent("G...")
console.log(agent.name, agent.score)
```

### REST API transport (read-only + analytics)

The REST transport talks to the public API at `https://agent-passport.xyz` (read endpoints are served at the domain root). Public reads are rate-limited per IP (e.g. 60 req/min on `/agents`) — cache results in your application rather than polling.

```typescript
import { AgentPassportClient, TrustApiTransport } from "@ggoldani/agent-passport-sdk"

const transport = new TrustApiTransport({
  apiUrl: "https://agent-passport.xyz",
})

const client = new AgentPassportClient({
  contractId: "CD2ZS3ADHBJZ6ISEZQ5QA6DMAC264Z6AMYCMP6RF37BVUQK3UGMBGVAA",
  transport,
})

const check = await client.trustCheck("G...", { threshold: 70 })
console.log(check.trusted, check.trust_tier)
```

## Transports

| Transport | Reads | Writes | API Calls |
|-----------|-------|--------|-----------|
| `SorobanRpcTransport` | Yes (simulate) | Yes (submit tx) | No |
| `TrustApiTransport` | Yes (REST) | No | Yes (analytics, trust-check, badge) |

For full read/write capability, use `SorobanRpcTransport` for contract operations and `TrustApiTransport` for API analytics.

## API

### AgentPassportClient

| Method | Description |
|--------|-------------|
| `getAgent(address)` | Fetch agent profile |
| `listAgents(from?, limit?)` | Paginated agent list |
| `getConfig()` | Contract configuration |
| `getRelayers()` | Authorized relayers |
| `getRating(txHash)` | Rating for an interaction |
| `listAgentInteractions(address, from?, limit?)` | Interaction history |
| `registerAgent(address, input)` | Register new agent |
| `updateProfile(address, input)` | Update agent profile |
| `deregisterAgent(address)` | Remove agent |
| `registerInteraction(relayer, record)` | Record interaction |
| `submitRating(rating)` | Rate an interaction (1-100) |
| `submitRichRating(input)` | On-chain rating + optional sub-scores |
| `trustCheck(address, options?)` | Trust pass/fail check |
| `getAnalytics(address, options?)` | Agent statistics |
| `getBadgeStats(address)` | Trust badge data |
| `addRelayer(admin, relayer)` | Authorize relayer |
| `removeRelayer(admin, relayer)` | Revoke relayer |
| `transferAdmin(admin, newAdmin)` | Initiate admin transfer |
| `acceptAdmin(newAdmin)` | Accept admin transfer |
| `cancelAdminTransfer(admin)` | Cancel pending transfer |

## Changelog

See [CHANGELOG.md](./CHANGELOG.md). Highlights since 0.1.0: `SorobanRpcTransport` (read + write contract calls with Ed25519 signing), `TrustApiTransport` (instant REST reads from `https://agent-passport.xyz`), `trustCheck()`, `getBadgeStats()`, `submitRichRating()` with sub-scores, pagination + relayer/admin/profile management, and security hardening from the audit rounds.

## License

MIT
