const STELLAR_ADDRESS_RE = /^G[A-Z2-7]{55}$/;

const REQUEST_TIMEOUT_MS = 15_000;

export function validateStellarAddress(address: string): void {
  if (!STELLAR_ADDRESS_RE.test(address)) {
    if (!address.startsWith("G")) {
      throw new Error(
        `"${address}" is not a valid Stellar public key. Stellar addresses start with "G" followed by 56 characters (e.g., GBVDQYSFGXEACZD7LG3NI7UAGLOP72D5SDDNJTHHWHD5EYDZHTPLG2IR). Make sure you're passing the agent's public key, not a secret key or contract ID.`
      );
    }
    throw new Error(
      `"${address}" looks like a Stellar address but has the wrong length (got ${address.length}, expected 56). Stellar public keys are 56 characters starting with "G" (e.g., GBVDQYSFGXEACZD7LG3NI7UAGLOP72D5SDDNJTHHWHD5EYDZHTPLG2IR). Double-check the address and try again.`
    );
  }
}

export function getApiUrl(): string {
  const url = process.env.AGENTPASSPORT_API_URL || "http://localhost:3002";
  if (!url.startsWith("http://") && !url.startsWith("https://")) {
    throw new Error(
      `AGENTPASSPORT_API_URL must be a valid URL starting with "http://" or "https://". Got: "${url}".`
    );
  }
  return url.replace(/\/+$/, "");
}

function buildUrl(path: string): string {
  // concat (não new URL(path, base)) para não descartar path prefix da base
  return getApiUrl() + path;
}

function throwForStatus(status: number, bodyText: string, notFoundMessage?: string): never {
  let serverMessage = bodyText.slice(0, 200);
  try {
    const parsed = JSON.parse(bodyText) as { error?: string };
    if (parsed.error) serverMessage = parsed.error;
  } catch {
    // corpo não-JSON: usa o texto bruto
  }
  if (status === 404 && notFoundMessage) {
    throw new Error(notFoundMessage);
  }
  if (status === 429) {
    throw new Error(
      `Too many requests — the AgentPassport API is rate-limiting this client. Wait a moment and try again.`
    );
  }
  if (status >= 500) {
    throw new Error(
      `The AgentPassport API server encountered an internal error (HTTP ${status}). This is not your fault — try again in a few moments. If it persists, check the AgentPassport server logs.`
    );
  }
  throw new Error(serverMessage || `The AgentPassport API returned an error (HTTP ${status})`);
}

async function request<T>(url: string, init: RequestInit, notFoundMessage?: string): Promise<T> {
  try {
    const res = await fetch(url, { ...init, signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
    if (!res.ok) {
      const bodyText = await res.text().catch(() => "unknown error");
      throwForStatus(res.status, bodyText, notFoundMessage);
    }
    return (await res.json()) as T;
  } catch (error) {
    if (error instanceof TypeError) {
      throw new Error(
        `Could not reach the AgentPassport API. Make sure the API server is running and accessible (set AGENTPASSPORT_API_URL to point to a different server).`
      );
    }
    throw error;
  }
}

export async function apiFetch<T>(
  path: string,
  params?: Record<string, string>,
  opts?: { notFoundMessage?: string }
): Promise<T> {
  const url = new URL(buildUrl(path));
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== "") {
        url.searchParams.set(key, value);
      }
    }
  }

  return request<T>(
    url.toString(),
    {},
    opts?.notFoundMessage ??
      `No agent found at this address. The agent may not be registered on AgentPassport yet, or the address might be incorrect. You can search for registered agents using the agent_search tool.`
  );
}

export async function apiPost<T>(path: string, body: unknown): Promise<T> {
  return request<T>(
    buildUrl(path),
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    }
  );
}
