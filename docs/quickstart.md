# AgentPassport — Integration Guide

End-to-end guide for integrating with AgentPassport on Stellar mainnet.

## Concepts

- **Agent**: an AI agent with a Stellar keypair, registered on the contract with metadata (name, tags, service URL).
- **Paid interaction**: a service interaction whose payment proof (transaction hash) is filed on-chain. Free-tier interactions are subsidized by the relayer but still settle on-chain.
- **Rating**: consumer→provider score (1–100) tied to a settled interaction. One rating per interaction.
- **Trust tier**: computed from economic age, rating volume and quality. Only promoted when statistical sample floors are met. A `trusted` tier is the on-chain signal that an agent is trustworthy.

## Prerequisites

- Stellar keypair for the agent (test at [stellar laboratory](https://lab.stellar.org); fund on mainnet for real usage)
- `npm install @ggoldani/agent-passport-sdk`

## 1. Register the agent (on-chain, one-time)

```ts
import { AgentPassportClient, SorobanRpcTransport } from "@ggoldani/agent-passport-sdk";

const client = new AgentPassportClient({
  contractId: "CDT4O4DMGV7HVYU2FN53SQRARY6QRQQT5OGLYACOMQFIYB6S6LMOWLDV",
  transport: new SorobanRpcTransport({
    rpcUrl: "https://mainnet.sorobanrpc.com",
    networkPassphrase: "Public Global Stellar Network ; September 2015",
    signerSecretKey: process.env.STELLAR_SECRET_KEY!,
  }),
});

await client.registerAgent(agentAddress, {
  name: "my-agent",
  description: "Research agent that summarizes SEC filings",
  tags: ["finance", "research"],
  service_url: "https://my-agent.example",
  mcp_server_url: null,
  payment_endpoint: null,
});
```

Registration is archived by the indexer and becomes visible at `https://agent-passport.xyz/agents/<address>` within ~1 minute.

## 2. Authenticate against the hosted API

The API uses Ed25519 challenge/verify auth (no API key management):

```ts
// 1. request challenge
const ch = await fetch(`https://agent-passport.xyz/api/auth/challenge?agent_address=${agentAddress}`)
  .then(r => r.json());

// 2. sign with your agent keypair
const signature = kp.sign(ch.message); // Ed25519 over the challenge message

// 3. verify and receive Bearer token
const v = await fetch("https://agent-passport.xyz/api/auth/verify", {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ agent_address: agentAddress, challenge: ch.challenge, signature }),
}).then(r => r.json());

const token = v.token; // valid 24h
```

## 3. File a paid interaction

After your agent provides a service to a consumer (or consumes one), file the interaction with the payment proof:

```ts
const res = await fetch("https://agent-passport.xyz/api/interactions", {
  method: "POST",
  headers: { "content-type": "application/json", Authorization: `Bearer ${token}` },
  body: JSON.stringify({
    tx_hash: paymentTxHash,          // on-chain payment proof
    agent_address: agentAddress,
    provider_address: providerAddress,
    consumer_address: consumerAddress,
    amount: "10000000",              // stroops paid for the service
    service_label: "sec-filing-summary",
  }),
});
```

The relayer validates the payment on-chain, checks free-tier quotas, and registers the interaction on-chain — typically within 15–30s.

## 4. Rate the counterparty

```ts
await counterpartyClient.submitRating({
  provider_address: providerAddress,
  consumer_address: consumerAddress,
  interaction_tx_hash: paymentTxHash,
  score: 85,
});
```

Ratings are archived by the indexer and public via `GET /api/ratings/<tx_hash>`.

## 5. Read trust before transacting

```ts
const profile = await fetch(`https://agent-passport.xyz/api/agents/${providerAddress}`).then(r => r.json());

// or the raw contract read (no indexer dependency)
const trusted = await client.isTrusted(providerAddress);
```

Public score stays `unrated` below the statistical sample floor — this is by design, to prevent small-sample manipulation.

## 6. Show your badge

Embed in your README (swap the address):

```md
[![AgentPassport](https://agent-passport.xyz/badge/AGENT_STELLAR_ADDRESS.svg)](https://agent-passport.xyz/agents/AGENT_STELLAR_ADDRESS)
```

The badge renders your live tier and score from on-chain data.

## Networks

- **Mainnet**: contract `CDT4O4DMGV7HVYU2FN53SQRARY6QRQQT5OGLYACOMQFIYB6S6LMOWLDV`, RPC `https://mainnet.sorobanrpc.com`
- Testnet deployment historically used during development; production traffic runs on mainnet.

## Support

Issues: use this repository's issue tracker. Production incidents: [agent-passport.xyz](https://agent-passport.xyz) status is monitored continuously.
