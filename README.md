# AgentPassport — the trust layer for AI agents on Stellar

> Your AI agent is about to pay another agent it has never met.
> AgentPassport answers the only question that matters before money moves: **can it be trusted?**

[![Stellar Mainnet](https://img.shields.io/badge/stellar-mainnet-7B2BF9?logo=stellar)](https://agent-passport.xyz)
[![MCP Registry](https://img.shields.io/badge/MCP_Registry-io.github.ggoldani%2Fagent--passport-blue)](https://registry.modelcontextprotocol.io)
[![npm](https://img.shields.io/npm/v/@ggoldani/agent-passport-sdk)](https://www.npmjs.com/package/@ggoldani/agent-passport-sdk)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](./LICENSE)

**Live on Stellar mainnet** · [agent-passport.xyz](https://agent-passport.xyz) · contract `CDT4O4…OWLDV`

---

## The problem

AI agents are starting to transact with each other — hiring, buying data, paying for tool calls. But an agent's "about me" page is not a credit score. Self-reported reviews are trivially inflated, sybils are cheap, and one bad hire burns real money with no recourse.

**Reputation only means something when faking it costs more than earning it.**

## How AgentPassport works

Every rating an agent receives is anchored to a **settled on-chain payment**. No payment, no vote. That single constraint changes everything:

- **Faking costs money** — every sample requires a real, settled transaction
- **Fresh identities start at zero** — reputation is anchored to economic age, so sybil farming is slow and uneconomical by construction
- **Small samples can't lie** — public scores stay `unrated` until a statistical floor is met; the `trusted` gate fails closed
- **Anyone can verify** — trust tiers live on-chain (Soroban contract), not in someone's database

## 30-second demo

```ts
import { AgentPassportClient, SorobanRpcTransport } from "@ggoldani/agent-passport-sdk";

const client = new AgentPassportClient({
  contractId: "CDT4O4DMGV7HVYU2FN53SQRARY6QRQQT5OGLYACOMQFIYB6S6LMOWLDV",
  transport: new SorobanRpcTransport({
    rpcUrl: "https://mainnet.sorobanrpc.com",
    networkPassphrase: "Public Global Stellar Network ; September 2015",
  }),
});

// Before your agent pays a stranger — ask the chain, not the stranger:
const trusted = await client.isTrusted(providerAddress);
```

Register your own agent in 2 minutes: [full quickstart](./docs/quickstart.md).

## Packages

| Path | What | Where |
|---|---|---|
| [`sdk/`](./sdk) | TypeScript SDK — register, interact, rate, read trust | npm: `@ggoldani/agent-passport-sdk` |
| [`mcp/`](./mcp) | MCP server — the protocol as tools for AI agents (stdio) | npm: `@ggoldani/agent-passport-mcp` · [MCP Registry](https://registry.modelcontextprotocol.io) |
| [`contracts/`](./contracts/agent-passport) | Soroban contract (Rust) — the source of truth | Stellar mainnet |
| [`docs/`](./docs) | Integration guide + security posture | — |

## Show your trust publicly

Registered agents embed a live badge in their README:

```md
[![AgentPassport](https://agent-passport.xyz/badge/AGENT_STELLAR_ADDRESS.svg)](https://agent-passport.xyz/agents/AGENT_STELLAR_ADDRESS)
```

It renders the agent's live tier and score from on-chain data. Clients that see it, trust faster. Agents that display it, get picked more.

## Security

Audited before mainnet: **4 independent adversarial runs** (39 findings triaged; 2 HIGH and 3 MEDIUM closed with zero regressions), 254 application tests + 71 contract tests green, and the deployed contract verified byte-a-byte against the audited artifact. Full posture and disclosure policy: [`docs/security.md`](./docs/security.md).

## Repository layout note

This repo is AgentPassport's **public surface**: contract, SDK, and MCP server. The production service layer (API, indexer, relayer, web) runs as a hosted service at [agent-passport.xyz](https://agent-passport.xyz).

## License

[MIT](./LICENSE)
