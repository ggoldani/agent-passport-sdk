import { describe, expect, it } from "vitest"
import { Keypair } from "@stellar/stellar-sdk"
import { signingDecision, NEVER_AUTO_SIGN_METHODS } from "../generated/src/lib/signingDecision.js"

const OPERATOR_SECRET = Keypair.random().secret()
const OPERATOR_PUBLIC = Keypair.fromSecret(OPERATOR_SECRET).publicKey()
const CONTRACT = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4"

function config(overrides: Partial<Parameters<typeof signingDecision>[0]> = {}) {
  return {
    network: "testnet" as const,
    requestTimeoutMs: 30_000,
    contractId: CONTRACT,
    secretKey: OPERATOR_SECRET,
    autoSign: true,
    autoSignLimit: 0,
    networkPassphrase: "Test SDF Network ; September 2015",
    ...overrides,
  }
}

describe("B1 — MCP nunca age como oráculo de assinatura", () => {
  it("recusa contractId diferente do configurado", () => {
    const other = "CBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABC"
    const d = signingDecision(config(), { contractId: other, sourceAccount: OPERATOR_PUBLIC, method: "get_agent" })
    expect(d.sign).toBe(false)
    expect(d.reason).toContain("contractId")
  })

  it("recusa sourceAccount de terceiros (assinaria tx alheia)", () => {
    const attacker = Keypair.random().publicKey()
    const d = signingDecision(config(), { contractId: CONTRACT, sourceAccount: attacker, method: "get_agent" })
    expect(d.sign).toBe(false)
    expect(d.reason).toContain("sourceAccount")
  })

  it("recusa métodos admin/bypass de fee em qualquer modo", () => {
    for (const method of ["register_interaction", "transfer_admin", "add_relayer", "remove_relayer", "accept_admin", "cancel_admin_transfer", "init"]) {
      const d = signingDecision(config(), { contractId: CONTRACT, sourceAccount: OPERATOR_PUBLIC, method })
      expect(d.sign, method).toBe(false)
    }
    expect(NEVER_AUTO_SIGN_METHODS.has("register_interaction")).toBe(true)
  })

  it("recusa sem secret key", () => {
    const d = signingDecision(config({ secretKey: undefined }), { contractId: CONTRACT, sourceAccount: OPERATOR_PUBLIC, method: "get_agent" })
    expect(d.sign).toBe(false)
  })

  it("assinatura permitida SOMENTE com contractId, source e método na política", () => {
    const d = signingDecision(config(), { contractId: CONTRACT, sourceAccount: OPERATOR_PUBLIC, method: "get_agent" })
    expect(d).toEqual({ sign: true, reason: "ok" })
  })
})
