#![no_std]

use soroban_sdk::{contract, contractevent, contractimpl, panic_with_error, Address, Env, String, Vec};

use crate::errors::Error;
use crate::storage::{
    add_relayer_to_storage, append_profile_owner, bump_agent_index_ttl, bump_entry_ttl,
    bump_live_entry, bump_registry_ttl, ensure_counterparty_list_contains,
    has_provider_counterparty, is_relayer, mark_provider_counterparty,
    read_config, read_interaction, read_profile, read_profile_owners_count,
    read_profile_owner_by_index, read_provider_counterparty_list,
    read_provider_interaction_count, read_provider_interaction_tx_hash,
    read_provider_rating_count, read_provider_rating_total, read_rating, read_relayers_count,
    read_relayer_by_index, remove_profile, remove_profile_owner, remove_relayer_from_storage,
    write_config, write_interaction, write_profile, write_provider_rating_count,
    write_provider_rating_total, write_rating, StorageKey,
};
use crate::types::{
    AgentProfile, AgentProfileInput, Config, InteractionRecord, RatingInput, RatingRecord,
};

mod errors;
mod storage;
mod types;

const MAX_NAME_LEN: u32 = 128;
const MAX_DESC_LEN: u32 = 512;
const MAX_URL_LEN: u32 = 256;
const MAX_TAGS: u32 = 20;
const MAX_TAG_LEN: u32 = 32;
const MAX_PAGE_SIZE: u32 = 100;
const DEFAULT_PAGE_SIZE: u32 = 20;
const ADMIN_TRANSFER_TIMELOCK_SECS: u64 = 7 * 24 * 60 * 60;
/// Cap por interação em stroops (10^21 = 10^14 XLM — ordens de magnitude acima
/// de qualquer pagamento real; auditoria 24/10 F3: sem cap, i128::MAX brickava
/// o provider ou corrompia o volume em release).
const MAX_INTERACTION_AMOUNT_STROOPS: u128 = 1_000_000_000_000_000_000_000;
/// Revisão rodada 1 (audit run-1): mesmo bound da API para service_label
const MAX_LABEL_LEN: u32 = 256;

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRegistered {
    #[topic]
    pub owner_address: Address,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub service_url: Option<String>,
    pub mcp_server_url: Option<String>,
    pub payment_endpoint: Option<String>,
    pub created_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractionRegistered {
    #[topic]
    pub provider_address: Address,
    #[topic]
    pub consumer_address: Address,
    /// Auditoria 24/10 F4: o relayer vai no topic para atribuição forense
    /// (chave comprometida é rastreável on-chain).
    #[topic]
    pub relayer: Address,
    pub tx_hash: soroban_sdk::BytesN<32>,
    pub amount: i128,
    pub timestamp: u64,
    pub service_label: Option<String>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RatingSubmitted {
    #[topic]
    pub provider_address: Address,
    #[topic]
    pub consumer_address: Address,
    #[topic]
    pub interaction_tx_hash: soroban_sdk::BytesN<32>,
    pub score: u32,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileUpdated {
    #[topic]
    pub owner_address: Address,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub service_url: Option<String>,
    pub mcp_server_url: Option<String>,
    pub payment_endpoint: Option<String>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentDeregistered {
    #[topic]
    pub owner_address: Address,
}

fn validate_profile_input(env: &Env, input: &AgentProfileInput) {
    if input.name.len() == 0 {
        panic_with_error!(env, Error::NameRequired);
    }
    if input.name.len() > MAX_NAME_LEN {
        panic_with_error!(env, Error::NameTooLong);
    }
    if input.description.len() == 0 {
        panic_with_error!(env, Error::DescriptionRequired);
    }
    if input.description.len() > MAX_DESC_LEN {
        panic_with_error!(env, Error::DescriptionTooLong);
    }
    if let Some(url) = &input.service_url {
        if url.len() > MAX_URL_LEN {
            panic_with_error!(env, Error::ServiceUrlTooLong);
        }
    }
    if let Some(url) = &input.mcp_server_url {
        if url.len() > MAX_URL_LEN {
            panic_with_error!(env, Error::McpServerUrlTooLong);
        }
    }
    if let Some(url) = &input.payment_endpoint {
        if url.len() > MAX_URL_LEN {
            panic_with_error!(env, Error::PaymentEndpointTooLong);
        }
    }
    if input.tags.len() > MAX_TAGS {
        panic_with_error!(env, Error::TooManyTags);
    }
    let tags_ref = &input.tags;
    let tag_count = tags_ref.len();
    let mut ti: u32 = 0;
    while ti < tag_count {
        if tags_ref.get_unchecked(ti).len() > MAX_TAG_LEN {
            panic_with_error!(env, Error::TagTooLong);
        }
        ti += 1;
    }
}

#[contract]
pub struct AgentPassport;

#[contractimpl]
impl AgentPassport {
    /// Auditoria 24/10 F2: init virou CONSTRUCTOR (`__constructor` no sdk 25) —
    /// roda no próprio CREATE do contrato com os args do deploy. Deploy +
    /// inicialização são ATÔMICOS: não existe janela para front-running de
    /// admin (era HIGH: deploy e init em transações separadas deixavam um
    /// atacante virar admin da raiz de confiança imutável).
    pub fn __constructor(env: Env, admin: Address, authorized_relayer: Address) {
        let config = Config {
            admin,
            pending_admin: None,
            admin_transfer_at: 0,
        };

        write_config(&env, &config);
        add_relayer_to_storage(&env, &authorized_relayer);
    }

    pub fn get_config(env: Env) -> Config {
        read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized))
    }

    pub fn get_relayers(env: Env) -> Vec<Address> {
        let count = read_relayers_count(&env);
        let mut relayers = Vec::new(&env);
        let mut i = 0;
        while i < count {
            if let Some(r) = read_relayer_by_index(&env, i) {
                relayers.push_back(r);
            }
            i += 1;
        }
        relayers
    }

    pub fn register_agent(env: Env, owner_address: Address, input: AgentProfileInput) {
        owner_address.require_auth();

        if read_profile(&env, &owner_address).is_some() {
            panic_with_error!(&env, Error::OwnershipConflict);
        }

        validate_profile_input(&env, &input);

        let created_at = env.ledger().timestamp();

        // Auditoria 24/10 F1/F6: reputação acompanha o ENDEREÇO, não o perfil.
        // deregister remove o perfil do índice mas preserva agregados em
        // storage próprio (rating count/total, índice de interações, lista de
        // counterparties). O re-register recomputa tudo — laundering de
        // reputação via deregister→re-register deixa de existir.
        let rating_count = read_provider_rating_count(&env, &owner_address);
        let rating_total = read_provider_rating_total(&env, &owner_address);
        let interaction_count = read_provider_interaction_count(&env, &owner_address);
        let counterparties = read_provider_counterparty_list(&env, &owner_address);
        let last_interaction_timestamp = if interaction_count > 0 {
            read_provider_interaction_tx_hash(&env, &owner_address, interaction_count - 1)
                .and_then(|tx| read_interaction(&env, &tx))
                .map(|record| record.timestamp)
                .unwrap_or(0)
        } else {
            0
        };

        let profile = AgentProfile {
            name: input.name,
            description: input.description,
            tags: input.tags,
            owner_address,
            service_url: input.service_url,
            mcp_server_url: input.mcp_server_url,
            payment_endpoint: input.payment_endpoint,
            created_at,
            score: if rating_count > 0 { (rating_total / rating_count) as u32 } else { 0 },
            verified_interactions_count: interaction_count,
            total_economic_volume: 0,
            unique_counterparties_count: counterparties.len() as u64,
            last_interaction_timestamp,
        };

        // total_economic_volume não é recompute-safe sem iterar todas as
        // interações (custo não limitado) — histórico documentado: o volume
        // volta a zerar no re-register; score/contagens NÃO.
        write_profile(&env, &profile);
        append_profile_owner(&env, &profile.owner_address);
        AgentRegistered {
            owner_address: profile.owner_address.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            tags: profile.tags.clone(),
            service_url: profile.service_url.clone(),
            mcp_server_url: profile.mcp_server_url.clone(),
            payment_endpoint: profile.payment_endpoint.clone(),
            created_at: profile.created_at,
        }
        .publish(&env);
    }

    pub fn get_agent(env: Env, owner_address: Address) -> AgentProfile {
        read_profile(&env, &owner_address).unwrap_or_else(|| panic_with_error!(&env, Error::ProfileNotFound))
    }

    pub fn list_agents(env: Env, from: u32, limit: u32) -> Vec<AgentProfile> {
        let capped_limit = if limit == 0 { DEFAULT_PAGE_SIZE } else if limit > MAX_PAGE_SIZE { MAX_PAGE_SIZE } else { limit };
        let count = read_profile_owners_count(&env);
        let start = from as u64;

        let mut profiles = Vec::new(&env);
        let mut i = start;
        let mut taken: u32 = 0;

        while i < count && taken < capped_limit {
            if let Some(owner) = read_profile_owner_by_index(&env, i) {
                if let Some(profile) = read_profile(&env, &owner) {
                    profiles.push_back(profile);
                    taken += 1;
                }
            }
            i += 1;
        }

        profiles
    }

    pub fn register_interaction(env: Env, relayer: Address, interaction: InteractionRecord) {
        relayer.require_auth();

        if !is_relayer(&env, &relayer) {
            panic_with_error!(&env, Error::UnauthorizedRelayer);
        }

        if read_interaction(&env, &interaction.tx_hash).is_some() {
            panic_with_error!(&env, Error::DuplicateTxHash);
        }

        // Auditoria 24/10 F3: amount é input do relayer — sem validação,
        // i128::MAX brickava o provider (panic no += u128) ou corrompia o
        // volume em release (wrap silencioso).
        if interaction.amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }
        let amount_abs = interaction.amount.unsigned_abs();
        if amount_abs > MAX_INTERACTION_AMOUNT_STROOPS {
            panic_with_error!(&env, Error::AmountExceedsCap);
        }

        if interaction.provider_address == interaction.consumer_address {
            panic_with_error!(&env, Error::SelfInteractionNotAllowed);
        }

        // Revisão rodada 1 (audit run-1): bound de service_label também no
        // contrato — a API limita em 256; isso é a última linha de defesa
        if let Some(label) = &interaction.service_label {
            if label.len() > MAX_LABEL_LEN {
                panic_with_error!(&env, Error::LabelTooLong);
            }
        }

        let timestamp = env.ledger().timestamp();

        let mut provider_profile = read_profile(&env, &interaction.provider_address)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MissingAgent));
        provider_profile.verified_interactions_count += 1;
        provider_profile.total_economic_volume = provider_profile
            .total_economic_volume
            .checked_add(amount_abs)
            .unwrap_or_else(|| panic_with_error!(&env, Error::VolumeOverflow));
        if !has_provider_counterparty(
            &env,
            &interaction.provider_address,
            &interaction.consumer_address,
        ) {
            provider_profile.unique_counterparties_count += 1;
            mark_provider_counterparty(
                &env,
                &interaction.provider_address,
                &interaction.consumer_address,
            );
        } else {
            // HIGH-2 (security audit run-3): o mark pode ter sobrevivido à List
            // (ratings bumpam só o mark) — re-anexa antes do bump; bump em chave
            // evictada panica e bricka o par permanentemente
            ensure_counterparty_list_contains(
                &env,
                &interaction.provider_address,
                &interaction.consumer_address,
            );
        }
        if timestamp > provider_profile.last_interaction_timestamp {
            provider_profile.last_interaction_timestamp = timestamp;
        }

        let stored = InteractionRecord {
            provider_address: interaction.provider_address.clone(),
            consumer_address: interaction.consumer_address.clone(),
            amount: interaction.amount,
            tx_hash: interaction.tx_hash.clone(),
            timestamp,
            service_label: interaction.service_label.clone(),
        };
        write_interaction(&env, &stored);
        write_profile(&env, &provider_profile);

        // B4 (security audit run-1): atividade renova o registro — sem bump,
        // o registry inteiro expirava ~1 ano (Config/relayers/índice/counterparty)
        bump_registry_ttl(&env);
        bump_entry_ttl(&env, &StorageKey::Relayer(relayer.clone()));
        // HIGH-2 (run-3): Profile é re-escrito (set = TTL default curto) — bump
        // explícito mantém o perfil vivo pelo mesmo horizonte do índice
        bump_entry_ttl(&env, &StorageKey::Profile(interaction.provider_address.clone()));
        bump_agent_index_ttl(&env, &interaction.provider_address);
        bump_entry_ttl(
            &env,
            &StorageKey::ProviderCounterpartyList(interaction.provider_address.clone()),
        );
        bump_entry_ttl(
            &env,
            &StorageKey::ProviderCounterparty(
                interaction.provider_address.clone(),
                interaction.consumer_address.clone(),
            ),
        );

        InteractionRegistered {
            provider_address: interaction.provider_address.clone(),
            consumer_address: interaction.consumer_address.clone(),
            relayer: relayer.clone(),
            tx_hash: interaction.tx_hash.clone(),
            amount: interaction.amount,
            timestamp,
            service_label: interaction.service_label.clone(),
        }
        .publish(&env);
    }

    pub fn list_agent_interactions(env: Env, provider_address: Address, from_seq: u32, limit: u32) -> Vec<InteractionRecord> {
        let capped_limit = if limit == 0 { DEFAULT_PAGE_SIZE } else if limit > MAX_PAGE_SIZE { MAX_PAGE_SIZE } else { limit };
        let count = read_provider_interaction_count(&env, &provider_address);
        let skip = from_seq as u64;

        let mut interactions = Vec::new(&env);
        let mut seq = if count > skip { count - skip } else { 0 };
        let mut taken: u32 = 0;

        while seq > 0 && taken < capped_limit {
            let index = seq - 1;
            if let Some(tx_hash) = read_provider_interaction_tx_hash(&env, &provider_address, index) {
                if let Some(interaction) = read_interaction(&env, &tx_hash) {
                    interactions.push_back(interaction);
                    taken += 1;
                }
            }
            seq = index;
        }

        interactions
    }

    pub fn get_rating(
        env: Env,
        interaction_tx_hash: soroban_sdk::BytesN<32>,
    ) -> Option<RatingRecord> {
        read_rating(&env, &interaction_tx_hash)
    }

    pub fn submit_rating(env: Env, rating: RatingInput) {
        rating.consumer_address.require_auth();

        if rating.score < 1 || rating.score > 100 {
            panic_with_error!(&env, Error::InvalidScore);
        }

        if rating.provider_address == rating.consumer_address {
            panic_with_error!(&env, Error::SelfRatingNotAllowed);
        }

        let interaction = read_interaction(&env, &rating.interaction_tx_hash)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MissingInteraction));
        if interaction.provider_address != rating.provider_address
            || interaction.consumer_address != rating.consumer_address
        {
            panic_with_error!(&env, Error::MissingInteraction);
        }

        if read_rating(&env, &rating.interaction_tx_hash).is_some() {
            panic_with_error!(&env, Error::DuplicateRating);
        }

        let timestamp = env.ledger().timestamp();
        let record = RatingRecord {
            provider_address: rating.provider_address.clone(),
            consumer_address: rating.consumer_address.clone(),
            interaction_tx_hash: rating.interaction_tx_hash.clone(),
            score: rating.score,
            timestamp,
        };
        write_rating(&env, &record);

        let mut provider_profile = read_profile(&env, &rating.provider_address)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MissingAgent));
        let next_count = read_provider_rating_count(&env, &rating.provider_address) + 1;
        let next_total = read_provider_rating_total(&env, &rating.provider_address)
            + u64::from(rating.score);
        write_provider_rating_count(&env, &rating.provider_address, next_count);
        write_provider_rating_total(&env, &rating.provider_address, next_total);
        provider_profile.score = (next_total / next_count) as u32;
        write_profile(&env, &provider_profile);

        // B4 (security audit run-1): rating também é atividade — renova o TTL
        bump_registry_ttl(&env);
        bump_agent_index_ttl(&env, &rating.provider_address);
        // HIGH-2 (security audit run-3): simetria de bumps — a List renova junto
        // (só se viva; submit_interaction re-cria a List quando o mark sobreviveu)
        bump_live_entry(
            &env,
            &StorageKey::ProviderCounterpartyList(rating.provider_address.clone()),
        );
        // set renova pro TTL default curto (~4096 ledgers) — agregados de score
        // sobrevivem ao gap de atividade só com bump explícito
        bump_entry_ttl(&env, &StorageKey::Profile(rating.provider_address.clone()));
        bump_entry_ttl(&env, &StorageKey::ProviderRatingCount(rating.provider_address.clone()));
        bump_entry_ttl(&env, &StorageKey::ProviderRatingTotal(rating.provider_address.clone()));
        bump_entry_ttl(
            &env,
            &StorageKey::ProviderCounterparty(
                rating.provider_address.clone(),
                rating.consumer_address.clone(),
            ),
        );

        RatingSubmitted {
            provider_address: rating.provider_address.clone(),
            consumer_address: rating.consumer_address.clone(),
            interaction_tx_hash: rating.interaction_tx_hash.clone(),
            score: rating.score,
            timestamp,
        }
        .publish(&env);
    }

    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        admin.require_auth();

        let mut config =
            read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if admin != config.admin {
            panic_with_error!(&env, Error::NotAdmin);
        }

        config.pending_admin = Some(new_admin);
        config.admin_transfer_at = env.ledger().timestamp() + ADMIN_TRANSFER_TIMELOCK_SECS;
        write_config(&env, &config);
    }

    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();

        let mut config =
            read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        let pending = match config.pending_admin {
            Some(a) if a == new_admin => a,
            _ => panic_with_error!(&env, Error::NotPendingAdmin),
        };

        let now = env.ledger().timestamp();
        if now < config.admin_transfer_at {
            panic_with_error!(&env, Error::AdminTransferNotExpired);
        }

        config.admin = pending;
        config.pending_admin = None;
        config.admin_transfer_at = 0;
        write_config(&env, &config);
    }

    pub fn cancel_admin_transfer(env: Env, admin: Address) {
        admin.require_auth();

        let mut config =
            read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if admin != config.admin {
            panic_with_error!(&env, Error::NotAdmin);
        }

        config.pending_admin = None;
        config.admin_transfer_at = 0;
        write_config(&env, &config);
    }

    pub fn add_relayer(env: Env, admin: Address, relayer: Address) {
        admin.require_auth();

        let config =
            read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if admin != config.admin {
            panic_with_error!(&env, Error::NotAdmin);
        }

        if is_relayer(&env, &relayer) {
            panic_with_error!(&env, Error::AlreadyRelayer);
        }

        add_relayer_to_storage(&env, &relayer);
    }

    pub fn remove_relayer(env: Env, admin: Address, relayer: Address) {
        admin.require_auth();

        let config =
            read_config(&env).unwrap_or_else(|| panic_with_error!(&env, Error::NotInitialized));
        if admin != config.admin {
            panic_with_error!(&env, Error::NotAdmin);
        }

        if !is_relayer(&env, &relayer) {
            panic_with_error!(&env, Error::NotRelayer);
        }

        remove_relayer_from_storage(&env, &relayer);
    }

    pub fn update_profile(env: Env, owner_address: Address, input: AgentProfileInput) {
        owner_address.require_auth();

        let mut profile = read_profile(&env, &owner_address)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ProfileNotFound));

        validate_profile_input(&env, &input);

        profile.name = input.name;
        profile.description = input.description;
        profile.tags = input.tags;
        profile.service_url = input.service_url;
        profile.mcp_server_url = input.mcp_server_url;
        profile.payment_endpoint = input.payment_endpoint;

        write_profile(&env, &profile);
        ProfileUpdated {
            owner_address: profile.owner_address,
            name: profile.name,
            description: profile.description,
            tags: profile.tags,
            service_url: profile.service_url,
            mcp_server_url: profile.mcp_server_url,
            payment_endpoint: profile.payment_endpoint,
        }
        .publish(&env);
    }

    pub fn deregister_agent(env: Env, owner_address: Address) {
        owner_address.require_auth();

        if read_profile(&env, &owner_address).is_none() {
            panic_with_error!(&env, Error::ProfileNotFound);
        }

        remove_profile(&env, &owner_address);
        remove_profile_owner(&env, &owner_address);
        // Auditoria 24/10 F1: rating count/total e índices de interação/
        // counterparty PERSISTEM — a reputação pertence ao endereço. O
        // re-register recomputa score/contagens (ver register_agent).
        AgentDeregistered {
            owner_address,
        }
        .publish(&env);
    }
}

#[cfg(test)]
mod test {
    use super::{AgentPassport, AgentPassportClient};

    include!("test.rs");
}
