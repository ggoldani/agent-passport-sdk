# @ggoldani/agent-passport-mcp

MCP server for the [AgentPassport](https://github.com/ggoldani/agent-passport-sdk trust registry on Stellar. 17 contract tools + 7 REST API bridge tools + health check.

## Install

```bash
npm install @ggoldani/agent-passport-mcp
```

## Setup

Configure your MCP client:

**Cursor / Claude Desktop:**
```json
{
  "mcpServers": {
    "agent-passport": {
      "type": "stdio",
      "command": "npx",
      "args": ["@ggoldani/agent-passport-mcp"],
      "env": {
        "STELLAR_NETWORK": "testnet",
        "STELLAR_CONTRACT_ID": "CD2ZS3ADHBJZ6ISEZQ5QA6DMAC264Z6AMYCMP6RF37BVUQK3UGMBGVAA",
        "AGENTPASSPORT_API_URL": "https://agent-passport.xyz"
      }
    }
  }
}
```

**With write access (admin operations):**
```json
{
  "mcpServers": {
    "agent-passport": {
      "type": "stdio",
      "command": "npx",
      "args": ["@ggoldani/agent-passport-mcp"],
      "env": {
        "STELLAR_NETWORK": "testnet",
        "STELLAR_CONTRACT_ID": "CD2ZS3ADHBJZ6ISEZQ5QA6DMAC264Z6AMYCMP6RF37BVUQK3UGMBGVAA",
        "STELLAR_SECRET_KEY": "S...",
        "AGENTPASSPORT_API_URL": "https://agent-passport.xyz"
      }
    }
  }
}
```

## Environment Variables

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `STELLAR_NETWORK` | No | `testnet` | Stellar network |
| `STELLAR_CONTRACT_ID` | Yes | — | Deployed contract ID |
| `STELLAR_RPC_URL` | No | Network default | Soroban RPC URL |
| `STELLAR_SECRET_KEY` | No | — | For write operations |
| `AGENTPASSPORT_API_URL` | No | `http://localhost:3002` | REST API base URL |

## Tools

### Contract Tools (17)
Auto-generated from contract WASM via `@ggoldani/stellarmcp`. All prefixed with `ap_`:

| Tool | Description |
|------|-------------|
| `ap_get_agent` | Fetch agent profile by address |
| `ap_list_agents` | Paginated agent list |
| `ap_get_config` | Contract configuration |
| `ap_get_relayers` | Authorized relayers |
| `ap_get_rating` | Rating for an interaction |
| `ap_list_agent_interactions` | Interaction history |
| `ap_register_agent` | Register new agent |
| `ap_update_profile` | Update agent profile |
| `ap_deregister_agent` | Remove agent |
| `ap_register_interaction` | Record interaction |
| `ap_submit_rating` | Rate an interaction (1-100) |
| `ap_add_relayer` | Authorize relayer |
| `ap_remove_relayer` | Revoke relayer |
| `ap_transfer_admin` | Initiate admin transfer |
| `ap_accept_admin` | Accept admin transfer |
| `ap_cancel_admin_transfer` | Cancel pending transfer |
| `ap_init` | Initialize contract |

### API Bridge Tools (7)
Wrap the REST API for enriched data:

> **Auth:** in production, `inbox_list`, `inbox_ack` and `subscribe` call endpoints that require an Ed25519 signature session (GET /auth/challenge → POST /auth/verify → `Authorization: Bearer <token>`) — unauthenticated calls get 401. The bridge tools do not send a Bearer token, so they target deployments with auth disabled (`AUTH_REQUIRED=false`, the local default).

- `agent_search` — Full-text search with filters
- `agent_analytics` — Volume, score trajectory, rating breakdown
- `agent_badge_stats` — Trust badge data
- `agent_trust_check` — Quick trust verification
- `inbox_list` — Pending inbox messages for an agent (verified events: payments received, ratings, profile changes)
- `inbox_ack` — Acknowledge an inbox message after processing
- `subscribe` — Subscribe an agent to event types (or `*` for all) so they land in its inbox

### Health (1)
- `health` — MCP server status check

## License

MIT
