import { Keypair } from "@stellar/stellar-sdk";

import type { AppConfig } from "../config.js";

/**
 * B1 (security audit run-1): o MCP NÃO é um oráculo de assinatura.
 * Só assina quando TODAS as condições fecham — senão devolve XDR unsigned
 * com o motivo. Fail-closed por design.
 */

/** Métodos admin/fee-bypass: nunca auto-assinados, em nenhum modo. */
export const NEVER_AUTO_SIGN_METHODS: ReadonlySet<string> = new Set([
  // bypassa o fee gate da API — só o relayer paga e submete
  "register_interaction",
  // admin two-step
  "transfer_admin",
  "accept_admin",
  "cancel_admin_transfer",
  "add_relayer",
  "remove_relayer",
  // legacy do contrato antigo (superfície morta)
  "init",
]);

export interface SigningDecision {
  sign: boolean;
  reason: string;
}

export function signingDecision(
  config: AppConfig,
  params: { contractId: string; sourceAccount: string; method: string }
): SigningDecision {
  if (params.contractId !== config.contractId) {
    return {
      sign: false,
      reason:
        "contractId difere do configurado no servidor — assinatura recusada (anti-oráculo: o servidor só fala com o próprio contrato)"
    };
  }

  if (NEVER_AUTO_SIGN_METHODS.has(params.method)) {
    return {
      sign: false,
      reason: `método '${params.method}' é admin/bypass de fee e nunca é auto-assinado — assine o XDR offline com a chave apropriada`
    };
  }

  if (!config.secretKey) {
    return { sign: false, reason: "sem STELLAR_SECRET_KEY configurada" };
  }

  const operatorPublicKey = Keypair.fromSecret(config.secretKey).publicKey();
  if (params.sourceAccount !== operatorPublicKey) {
    return {
      sign: false,
      reason:
        "sourceAccount difere da chave do operador — o servidor não assina transações de terceiros"
    };
  }

  return { sign: true, reason: "ok" };
}
