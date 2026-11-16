import { z } from "zod";
import { apiFetch, apiPost, validateStellarAddress } from "./client.js";

const EVENT_TYPES = [
  "agent_registered",
  "interaction_registered",
  "rating_submitted",
  "profile_updated",
  "agent_deregistered",
] as const;
// fonte da verdade: src/indexer/events.ts (EventType) no repo raiz — o pacote MCP é
// autocontido no npm, então a lista é duplicada aqui de propósito. Ao adicionar um
// 6º evento no contrato, atualize os dois lugares.

export const inboxListSchema = z.object({
  address: z.string().describe("Stellar address of the agent that owns the inbox (G...)"),
  limit: z.number().int().min(1).max(100).optional().describe("Max messages to return (default 20)"),
  offset: z.number().int().min(0).optional().describe("Pagination offset (default 0)"),
});

export type InboxListParams = z.infer<typeof inboxListSchema>;

export interface InboxListResponse {
  data: {
    message_id: string;
    subscription_id: number;
    event_id: string;
    channel: string;
    status: string;
    payload: string;
    available_at: number;
    expires_at: number;
    attempt_count: number;
  }[];
  total: number;
  has_more: boolean;
  served: number;
}

const UNTRUSTED_CHARS = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F\u200B-\u200F\u2028\u2029\u202A-\u202E\u2060-\u2064\uFEFF]/g
const MAX_PAYLOAD_CHARS = 2000

/** Payload on-chain é dado não-confiável (pode conter injeção de prompt). */
function sanitizeUntrusted(text: string): string {
  return text.replace(UNTRUSTED_CHARS, "\uFFFD").slice(0, MAX_PAYLOAD_CHARS)
}

export async function inboxList(params: InboxListParams): Promise<InboxListResponse> {
  validateStellarAddress(params.address);

  const query: Record<string, string> = {};
  if (params.limit !== undefined) query.limit = String(params.limit);
  if (params.offset !== undefined) query.offset = String(params.offset);

  const response = await apiFetch<InboxListResponse>(`/inbox/${params.address}`, query, {
    notFoundMessage:
      "No inbox endpoint found at the AgentPassport API — possible API version mismatch. Check AGENTPASSPORT_API_URL.",
  })
  // payload on-chain é dado não-confiável: sanitiza antes de virar texto de tool MCP
  return {
    ...response,
    data: response.data.map((m) => ({ ...m, payload: sanitizeUntrusted(m.payload) })),
  }
}

export const inboxAckSchema = z.object({
  address: z.string().describe("Stellar address of the agent that owns the message (G...)"),
  message_id: z.string().describe("Message identifier returned by inbox_list"),
});

export type InboxAckParams = z.infer<typeof inboxAckSchema>;

export interface InboxAckResponse {
  acked: boolean;
  message_id: string;
}

export async function inboxAck(params: InboxAckParams): Promise<InboxAckResponse> {
  validateStellarAddress(params.address);
  return apiPost<InboxAckResponse>("/inbox/ack", {
    message_id: params.message_id,
    agent_address: params.address,
  });
}

export const subscribeSchema = z.object({
  address: z.string().describe("Stellar address of the agent that will receive events (G...)"),
  event_interests: z
    .union([z.literal("*"), z.array(z.enum(EVENT_TYPES)).min(1)])
    .optional()
    .describe('Event types to receive: "*" for all, or a list like ["rating_submitted", "interaction_registered"] (default "*")'),
  channel: z
    .literal("mcp_inbox")
    .optional()
    .describe('Delivery channel — only "mcp_inbox" has delivery semantics today (default "mcp_inbox")'),
});

export type SubscribeParams = z.infer<typeof subscribeSchema>;

export interface SubscribeResponse {
  data: {
    id: number;
    agent_address: string;
    channel: string;
    event_interests: string;
    status: string;
  };
  duplicate?: boolean;
}

export async function subscribe(params: SubscribeParams): Promise<SubscribeResponse> {
  validateStellarAddress(params.address);
  return apiPost<SubscribeResponse>("/subscriptions", {
    agent_address: params.address,
    event_interests: params.event_interests ?? "*",
    channel: params.channel ?? "mcp_inbox",
  });
}
