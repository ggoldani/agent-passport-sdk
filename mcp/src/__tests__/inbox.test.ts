import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { apiPost } from "../api-bridge/client.js"
import { inboxAck, inboxList, subscribe, subscribeSchema } from "../api-bridge/inbox.js"

const ADDR = "GBVDQYSFGXEACZD7LG3NI7UAGLOP72D5SDDNJTHHWHD5EYDZHTPLG2IR"

beforeEach(() => {
  vi.stubEnv("AGENTPASSPORT_API_URL", "")
})

afterEach(() => {
  vi.unstubAllEnvs()
  vi.unstubAllGlobals()
})

function stubFetch(status: number, body: unknown) {
  const fetchMock = vi.fn(async () =>
    new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } }),
  )
  vi.stubGlobal("fetch", fetchMock)
  return fetchMock
}

describe("apiPost", () => {
  it("POSTa JSON e faz parse da resposta", async () => {
    const fetchMock = stubFetch(200, { ok: true })
    const result = await apiPost<{ ok: boolean }>("/inbox/ack", { message_id: "m1", agent_address: ADDR })
    expect(result).toEqual({ ok: true })
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toBe("http://localhost:3002/inbox/ack")
    expect(init.method).toBe("POST")
    expect(init.body).toBe(JSON.stringify({ message_id: "m1", agent_address: ADDR }))
  })

  it("propaga a mensagem do servidor em 404", async () => {
    stubFetch(404, { error: "message not found for this agent" })
    await expect(apiPost("/inbox/ack", {})).rejects.toThrow(/message not found/)
  })

  it("400 real → mensagem do servidor", async () => {
    stubFetch(400, { error: "invalid interests" })
    await expect(apiPost("/subscriptions", {})).rejects.toThrow(/invalid interests/)
  })

  it("429 → mensagem amigável de rate limit", async () => {
    stubFetch(429, {})
    await expect(apiPost("/inbox/ack", {})).rejects.toThrow(/Too many requests/)
  })

  it("5xx → mensagem amigável de erro interno", async () => {
    stubFetch(503, "gateway noise")
    await expect(apiPost("/inbox/ack", {})).rejects.toThrow(/internal error/)
  })

  it("corpo de erro não-JSON → usa o texto bruto", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("plain gateway text", { status: 400 })),
    )
    await expect(apiPost("/inbox/ack", {})).rejects.toThrow(/plain gateway text/)
  })

  it("dá mensagem amigável quando a API está fora do ar", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => { throw new TypeError("fetch failed") }))
    await expect(apiPost("/inbox/ack", {})).rejects.toThrow(/Could not reach the AgentPassport API/)
  })
})

describe("subscribe schema", () => {
  it("rejeita channel sem semântica de entrega", () => {
    expect(subscribeSchema.safeParse({ address: ADDR, event_interests: "*", channel: "webhook" }).success).toBe(false)
    expect(subscribeSchema.safeParse({ address: ADDR, event_interests: "*", channel: "mcp_inbox" }).success).toBe(true)
  })
})

describe("inboxList", () => {
  it("chama GET /inbox/:address com paginação e devolve o payload", async () => {
    const apiBody = { data: [{ message_id: "m1", event_id: "e1", channel: "mcp_inbox", status: "available", payload: "{}", available_at: 1, expires_at: 2, attempt_count: 1 }], total: 1, has_more: false, served: 1 }
    const fetchMock = stubFetch(200, apiBody)
    const result = await inboxList({ address: ADDR, limit: 5, offset: 2 })
    expect(result).toEqual(apiBody)
    const [url] = fetchMock.mock.calls[0] as [string]
    expect(url).toBe("http://localhost:3002/inbox/" + ADDR + "?limit=5&offset=2")
  })

  it("valida endereço Stellar antes de chamar a API", async () => {
    await expect(inboxList({ address: "not-stellar" })).rejects.toThrow(/not a valid Stellar public key/)
  })
})

describe("inboxAck", () => {
  it("POSTa /inbox/ack com message_id e agent_address", async () => {
    const fetchMock = stubFetch(200, { acked: true, message_id: "m1" })
    const result = await inboxAck({ address: ADDR, message_id: "m1" })
    expect(result).toEqual({ acked: true, message_id: "m1" })
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toBe("http://localhost:3002/inbox/ack")
    expect(JSON.parse(init.body as string)).toEqual({ message_id: "m1", agent_address: ADDR })
  })

  it("superfície de erro 404 amigável pro agente", async () => {
    stubFetch(404, { error: "message not found for this agent" })
    await expect(inboxAck({ address: ADDR, message_id: "ghost" })).rejects.toThrow(/message not found/)
  })
})

describe("subscribe", () => {
  it("POSTa /subscriptions com wildcard", async () => {
    const fetchMock = stubFetch(201, { data: { id: 1, channel: "mcp_inbox", status: "active" } })
    const result = await subscribe({ address: ADDR, event_interests: "*" })
    expect(result.data).toMatchObject({ id: 1 })
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(url).toBe("http://localhost:3002/subscriptions")
    expect(JSON.parse(init.body as string)).toEqual({ agent_address: ADDR, event_interests: "*", channel: "mcp_inbox" })
  })

  it("POSTa array de interesses", async () => {
    const fetchMock = stubFetch(201, { data: { id: 2 } })
    await subscribe({ address: ADDR, event_interests: ["rating_submitted", "interaction_registered"] })
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit]
    expect(JSON.parse(init.body as string).event_interests).toEqual(["rating_submitted", "interaction_registered"])
  })

  it("propaga flag de duplicata da API", async () => {
    stubFetch(200, { data: { id: 1 }, duplicate: true })
    const result = await subscribe({ address: ADDR, event_interests: "*" })
    expect(result.duplicate).toBe(true)
  })
})
