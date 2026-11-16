extern crate std;

use crate::types::AgentProfileInput;
use crate::types::InteractionRecord;
use crate::types::RatingInput;
use crate::storage::StorageKey;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};
use soroban_sdk::{BytesN, String, Vec};

fn setup(env: &Env) -> (Address, AgentPassportClient, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let relayer = Address::generate(env);
    let contract_id = env.register(AgentPassport, (admin.clone(), relayer.clone()));
    let client = AgentPassportClient::new(env, &contract_id);
    (contract_id, client, admin, relayer)
}

fn register_agent(env: &Env, client: &AgentPassportClient, owner: &Address) {
    let input = AgentProfileInput {
        name: String::from_str(env, "test-agent"),
        description: String::from_str(env, "Test agent description"),
        tags: Vec::from_array(env, [String::from_str(env, "test")]),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(owner, &input);
}

fn register_interaction_tx(
    env: &Env,
    client: &AgentPassportClient,
    contract_id: &Address,
    relayer: &Address,
    provider: &Address,
    consumer: &Address,
    amount: i128,
    tx_byte: u8,
) -> BytesN<32> {
    let tx_hash = BytesN::from_array(env, &[tx_byte; 32]);
    let interaction = InteractionRecord {
        provider_address: provider.clone(),
        consumer_address: consumer.clone(),
        amount,
        tx_hash: tx_hash.clone(),
        timestamp: 100,
        service_label: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: relayer,
            invoke: &MockAuthInvoke {
                contract: contract_id,
                fn_name: "register_interaction",
                args: (relayer, &interaction).into_val(env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(relayer, &interaction);
    tx_hash
}

fn submit_rating_tx(
    env: &Env,
    client: &AgentPassportClient,
    contract_id: &Address,
    consumer: &Address,
    provider: &Address,
    interaction_tx_hash: &BytesN<32>,
    score: u32,
) {
    let rating = RatingInput {
        provider_address: provider.clone(),
        consumer_address: consumer.clone(),
        interaction_tx_hash: interaction_tx_hash.clone(),
        score,
    };
    client
        .mock_auths(&[MockAuth {
            address: consumer,
            invoke: &MockAuthInvoke {
                contract: contract_id,
                fn_name: "submit_rating",
                args: (&rating,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating);
}

#[test]
fn init_persists_admin_and_relayer_config() {
    let env = Env::default();
    let (_, client, admin, relayer) = setup(&env);

    let config = client.get_config();
    assert_eq!(config.admin, admin);
    assert!(config.pending_admin.is_none());
    assert_eq!(config.admin_transfer_at, 0);

    let relayers = client.get_relayers();
    assert_eq!(relayers.len(), 1);
    assert_eq!(relayers.get_unchecked(0), relayer);
}

// Auditoria 24/10 F2: o teste `init_rejects_reinitialization` foi removido —
// com __constructor, a inicialização acontece 1x no CREATE (garantia do
// protocolo), não existe mais função de init re-invocável.

#[test]
fn get_relayers_returns_all_relayers() {
    let env = Env::default();
    let (_, client, admin, _relayer) = setup(&env);
    let relayer2 = Address::generate(&env);

    env.mock_all_auths();
    client.add_relayer(&admin, &relayer2);

    let relayers = client.get_relayers();
    assert_eq!(relayers.len(), 2);
}

#[test]
fn register_agent_persists_profile() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let input = AgentProfileInput {
        name: String::from_str(&env, "stellar-intel"),
        description: String::from_str(&env, "Paid Stellar intelligence provider"),
        tags: Vec::from_array(&env, [String::from_str(&env, "stellar")]),
        service_url: Some(String::from_str(&env, "https://stellarintel.test/service")),
        mcp_server_url: Some(String::from_str(&env, "https://stellarintel.test/mcp")),
        payment_endpoint: Some(String::from_str(&env, "https://stellarintel.test/pay")),
    };

    env.mock_all_auths();
    client.register_agent(&owner, &input);

    let profile = client.get_agent(&owner);
    assert_eq!(profile.owner_address, owner);
    assert_eq!(profile.name, input.name);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn register_agent_rejects_duplicate() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let input = AgentProfileInput {
        name: String::from_str(&env, "second"),
        description: String::from_str(&env, "Dup"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #13)")]
fn register_agent_rejects_mcp_server_url_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_url = String::from_str(&env, &"h".repeat(257));
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: Some(long_url),
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #14)")]
fn register_agent_rejects_payment_endpoint_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_url = String::from_str(&env, &"h".repeat(257));
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: Some(long_url),
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn register_agent_rejects_empty_name() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let input = AgentProfileInput {
        name: String::from_str(&env, ""),
        description: String::from_str(&env, "Has description"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn register_agent_rejects_empty_description() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let input = AgentProfileInput {
        name: String::from_str(&env, "has-name"),
        description: String::from_str(&env, ""),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn register_agent_rejects_name_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_name = String::from_str(&env, &"a".repeat(129));
    let input = AgentProfileInput {
        name: long_name,
        description: String::from_str(&env, "desc"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn register_agent_rejects_description_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_desc = String::from_str(&env, &"d".repeat(513));
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: long_desc,
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #12)")]
fn register_agent_rejects_service_url_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_url = String::from_str(&env, &"h".repeat(257));
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags: Vec::new(&env),
        service_url: Some(long_url),
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn register_agent_rejects_too_many_tags() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let mut tags = Vec::new(&env);
    for _ in 0..21 {
        tags.push_back(String::from_str(&env, "t"));
    }
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags,
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn register_agent_rejects_tag_too_long() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let long_tag = String::from_str(&env, &"x".repeat(33));
    let tags = Vec::from_array(&env, [long_tag]);
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags,
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);
}

#[test]
fn register_agent_accepts_max_boundary_values() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    let name = String::from_str(&env, &"a".repeat(128));
    let desc = String::from_str(&env, &"d".repeat(512));
    let url = String::from_str(&env, &"h".repeat(256));
    let mut tags = Vec::new(&env);
    for _ in 0..20 {
        tags.push_back(String::from_str(&env, &"t".repeat(32)));
    }
    let input = AgentProfileInput {
        name,
        description: desc,
        tags,
        service_url: Some(url),
        mcp_server_url: None,
        payment_endpoint: None,
    };
    env.mock_all_auths();
    client.register_agent(&owner, &input);

    let profile = client.get_agent(&owner);
    assert_eq!(profile.name.len(), 128);
}

#[test]
fn emits_lifecycle_events_for_registration_interaction_and_rating() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let owner = Address::generate(&env);
    let consumer = Address::generate(&env);
    let name = String::from_str(&env, "stellar-intel");
    let description = String::from_str(&env, "Paid Stellar intelligence provider");
    let tags = Vec::from_array(&env, [String::from_str(&env, "stellar")]);
    let agent_input = AgentProfileInput {
        name: name.clone(),
        description: description.clone(),
        tags: tags.clone(),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };

    env.mock_all_auths();
    client.register_agent(&owner, &agent_input);

    let events = env.events().all().filter_by_contract(&contract_id);
    assert_eq!(events.events().len(), 1);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &owner, &consumer, 275, 9);

    let events = env.events().all().filter_by_contract(&contract_id);
    assert_eq!(events.events().len(), 1);

    submit_rating_tx(&env, &client, &contract_id, &consumer, &owner, &tx_hash, 95);

    let events = env.events().all().filter_by_contract(&contract_id);
    assert_eq!(events.events().len(), 1);
}

#[test]
fn list_agents_returns_registered_profiles() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    register_agent(&env, &client, &owner1);
    register_agent(&env, &client, &owner2);

    let agents = client.list_agents(&0, &10);
    assert_eq!(agents.len(), 2);
}

#[test]
fn list_agents_with_pagination() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    for _ in 0..5 {
        let owner = Address::generate(&env);
        register_agent(&env, &client, &owner);
    }

    let page1 = client.list_agents(&0, &2);
    assert_eq!(page1.len(), 2);
    let page2 = client.list_agents(&2, &2);
    assert_eq!(page2.len(), 2);
    let page3 = client.list_agents(&4, &2);
    assert_eq!(page3.len(), 1);
    let empty = client.list_agents(&10, &10);
    assert_eq!(empty.len(), 0);
}

#[test]
fn list_agents_defaults_limit_when_zero() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let agents = client.list_agents(&0, &0);
    assert_eq!(agents.len(), 1);
}

#[test]
fn list_agent_interactions_returns_newest_first() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer1 = Address::generate(&env);
    let consumer2 = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let older = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer1, 100, 1);
    let newer = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer2, 250, 2);

    let interactions = client.list_agent_interactions(&provider, &0, &10);
    assert_eq!(interactions.len(), 2);
    assert_eq!(interactions.get_unchecked(0).tx_hash, newer);
    assert_eq!(interactions.get_unchecked(1).tx_hash, older);
}

#[test]
fn list_agent_interactions_with_pagination() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    for i in 0..5u8 {
        register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, i + 10);
    }

    assert_eq!(client.list_agent_interactions(&provider, &0, &2).len(), 2);
    assert_eq!(client.list_agent_interactions(&provider, &2, &2).len(), 2);
    assert_eq!(client.list_agent_interactions(&provider, &4, &2).len(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn only_authorized_relayer_can_register_interaction() {
    let env = Env::default();
    let (contract_id, client, _, _relayer) = setup(&env);
    let attacker = Address::generate(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let interaction = InteractionRecord {
        provider_address: provider,
        consumer_address: consumer,
        amount: 100,
        tx_hash: BytesN::from_array(&env, &[3; 32]),
        timestamp: 300,
        service_label: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&attacker, &interaction).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&attacker, &interaction);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn register_interaction_rejects_duplicate_tx_hash() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let dup_hash = BytesN::from_array(&env, &[4; 32]);
    let first = InteractionRecord {
        provider_address: provider.clone(),
        consumer_address: c1,
        amount: 100,
        tx_hash: dup_hash.clone(),
        timestamp: 300,
        service_label: None,
    };
    let second = InteractionRecord {
        provider_address: provider.clone(),
        consumer_address: c2,
        amount: 200,
        tx_hash: dup_hash,
        timestamp: 400,
        service_label: None,
    };

    client
        .mock_auths(&[MockAuth {
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&relayer, &first).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&relayer, &first);
    client
        .mock_auths(&[MockAuth {
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&relayer, &second).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&relayer, &second);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn register_interaction_rejects_nonexistent_provider() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);

    let interaction = InteractionRecord {
        provider_address: provider,
        consumer_address: consumer,
        amount: 100,
        tx_hash: BytesN::from_array(&env, &[50; 32]),
        timestamp: 300,
        service_label: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&relayer, &interaction).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&relayer, &interaction);
}

#[test]
#[should_panic(expected = "Error(Contract, #25)")]
fn register_interaction_rejects_provider_equals_consumer() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let agent = Address::generate(&env);
    register_agent(&env, &client, &agent);

    let interaction = InteractionRecord {
        provider_address: agent.clone(),
        consumer_address: agent,
        amount: 100,
        tx_hash: BytesN::from_array(&env, &[52; 32]),
        timestamp: 300,
        service_label: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&relayer, &interaction).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&relayer, &interaction);
}

#[test]
fn register_interaction_updates_provider_metrics() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 500, 5);

    let profile = client.get_agent(&provider);
    assert_eq!(profile.verified_interactions_count, 1);
    assert_eq!(profile.total_economic_volume, 500);
    assert_eq!(profile.unique_counterparties_count, 1);
}

#[test]
fn register_interaction_uses_ledger_timestamp() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    env.ledger().set_timestamp(9999);
    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 55);

    let interactions = client.list_agent_interactions(&provider, &0, &10);
    assert_eq!(interactions.get_unchecked(0).timestamp, 9999);
    assert_eq!(interactions.get_unchecked(0).tx_hash, tx_hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn only_verified_counterparty_can_rate() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let verified_consumer = Address::generate(&env);
    let attacker = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let interaction = InteractionRecord {
        provider_address: provider.clone(),
        consumer_address: verified_consumer,
        amount: 100,
        tx_hash: BytesN::from_array(&env, &[6; 32]),
        timestamp: 600,
        service_label: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &relayer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "register_interaction",
                args: (&relayer, &interaction).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .register_interaction(&relayer, &interaction);

    let rating = RatingInput {
        provider_address: provider,
        consumer_address: attacker.clone(),
        interaction_tx_hash: BytesN::from_array(&env, &[6; 32]),
        score: 80,
    };
    client
        .mock_auths(&[MockAuth {
            address: &attacker,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_rating",
                args: (&rating,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn submit_rating_rejects_duplicate() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 7);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx_hash, 80);

    let rating2 = RatingInput {
        provider_address: provider,
        consumer_address: consumer.clone(),
        interaction_tx_hash: tx_hash,
        score: 90,
    };
    client
        .mock_auths(&[MockAuth {
            address: &consumer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_rating",
                args: (&rating2,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating2);
}

#[test]
fn submit_rating_updates_provider_score() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 8);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx_hash, 80);

    assert_eq!(client.get_agent(&provider).score, 80);
}

#[test]
fn submit_rating_averages_multiple_ratings() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx1 = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &c1, 100, 20);
    let tx2 = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &c2, 100, 21);

    submit_rating_tx(&env, &client, &contract_id, &c1, &provider, &tx1, 60);
    submit_rating_tx(&env, &client, &contract_id, &c2, &provider, &tx2, 100);

    assert_eq!(client.get_agent(&provider).score, 80);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn submit_rating_rejects_score_zero() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 30);
    let rating = RatingInput {
        provider_address: provider,
        consumer_address: consumer.clone(),
        interaction_tx_hash: tx_hash,
        score: 0,
    };
    client
        .mock_auths(&[MockAuth {
            address: &consumer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_rating",
                args: (&rating,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn submit_rating_rejects_score_over_100() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 31);
    let rating = RatingInput {
        provider_address: provider,
        consumer_address: consumer.clone(),
        interaction_tx_hash: tx_hash,
        score: 101,
    };
    client
        .mock_auths(&[MockAuth {
            address: &consumer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_rating",
                args: (&rating,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating);
}

#[test]
fn submit_rating_accepts_score_boundary_100() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 33);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx_hash, 100);

    assert_eq!(client.get_agent(&provider).score, 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn submit_rating_rejects_self_rating() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_agent(&env, &client, &consumer);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 32);
    let rating = RatingInput {
        provider_address: provider.clone(),
        consumer_address: provider.clone(),
        interaction_tx_hash: tx_hash,
        score: 50,
    };
    client
        .mock_auths(&[MockAuth {
            address: &provider,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_rating",
                args: (&rating,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .submit_rating(&rating);
}

#[test]
fn get_rating_returns_persisted_record() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 9);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx_hash, 90);

    let record = client.get_rating(&tx_hash).expect("rating should exist");
    assert_eq!(record.score, 90);
}

#[test]
/// Auditoria 24/10 F3: amount negativo ERA aceito (abs virava volume positivo) —
/// agora é erro de contrato #27. Teste antigo codificava o bug como feature.
#[should_panic(expected = "Error(Contract, #27)")]
fn register_interaction_handles_negative_amount() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, -100, 40);
}

/// F3: i128::MIN (o pior caso do antigo saturating_abs) agora é #27, não brick.
#[test]
#[should_panic(expected = "Error(Contract, #27)")]
fn register_interaction_handles_i128_min_amount() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, i128::MIN, 41);
}

#[test]
fn transfer_admin_sets_pending_and_timelock() {
    let env = Env::default();
    let (_, client, admin, _) = setup(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&admin, &new_admin);

    let config = client.get_config();
    assert_eq!(config.pending_admin, Some(new_admin));
    assert!(config.admin_transfer_at > 0);
}

#[test]
fn accept_admin_transfers_after_timelock() {
    let env = Env::default();
    let (contract_id, client, admin, _) = setup(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&admin, &new_admin);

    let config = client.get_config();
    env.ledger().set_timestamp(config.admin_transfer_at);

    client
        .mock_auths(&[MockAuth {
            address: &new_admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "accept_admin",
                args: (&new_admin,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .accept_admin(&new_admin);

    let config = client.get_config();
    assert_eq!(config.admin, new_admin);
    assert!(config.pending_admin.is_none());
    assert_eq!(config.admin_transfer_at, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn accept_admin_rejects_before_timelock() {
    let env = Env::default();
    let (contract_id, client, admin, _) = setup(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&admin, &new_admin);

    client
        .mock_auths(&[MockAuth {
            address: &new_admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "accept_admin",
                args: (&new_admin,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .accept_admin(&new_admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #19)")]
fn accept_admin_rejects_wrong_address() {
    let env = Env::default();
    let (contract_id, client, admin, _) = setup(&env);
    let new_admin = Address::generate(&env);
    let wrong = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&admin, &new_admin);

    let config = client.get_config();
    env.ledger().set_timestamp(config.admin_transfer_at);

    client
        .mock_auths(&[MockAuth {
            address: &wrong,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "accept_admin",
                args: (&wrong,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .accept_admin(&wrong);
}

#[test]
fn cancel_admin_transfer_clears_pending() {
    let env = Env::default();
    let (_, client, admin, _) = setup(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&admin, &new_admin);
    client.cancel_admin_transfer(&admin);

    let config = client.get_config();
    assert!(config.pending_admin.is_none());
    assert_eq!(config.admin_transfer_at, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #21)")]
fn non_admin_cannot_transfer_admin() {
    let env = Env::default();
    let (_, client, _, _) = setup(&env);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);

    env.mock_all_auths();
    client.transfer_admin(&attacker, &new_admin);
}

#[test]
fn add_relayer_works() {
    let env = Env::default();
    let (_, client, admin, _) = setup(&env);
    let new_relayer = Address::generate(&env);

    env.mock_all_auths();
    client.add_relayer(&admin, &new_relayer);

    assert_eq!(client.get_relayers().len(), 2);
}

#[test]
fn remove_relayer_works() {
    let env = Env::default();
    let (_, client, admin, relayer) = setup(&env);

    env.mock_all_auths();
    client.remove_relayer(&admin, &relayer);

    assert_eq!(client.get_relayers().len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn add_duplicate_relayer_rejected() {
    let env = Env::default();
    let (_, client, admin, relayer) = setup(&env);

    env.mock_all_auths();
    client.add_relayer(&admin, &relayer);
}

#[test]
#[should_panic(expected = "Error(Contract, #23)")]
fn remove_nonexistent_relayer_rejected() {
    let env = Env::default();
    let (_, client, admin, _) = setup(&env);
    let stranger = Address::generate(&env);

    env.mock_all_auths();
    client.remove_relayer(&admin, &stranger);
}

#[test]
fn update_profile_works() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let input = AgentProfileInput {
        name: String::from_str(&env, "updated-name"),
        description: String::from_str(&env, "Updated description"),
        tags: Vec::from_array(&env, [String::from_str(&env, "updated")]),
        service_url: Some(String::from_str(&env, "https://updated.test")),
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);

    let profile = client.get_agent(&owner);
    assert_eq!(profile.name, String::from_str(&env, "updated-name"));
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn update_profile_rejects_nonexistent() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);

    let input = AgentProfileInput {
        name: String::from_str(&env, "updated"),
        description: String::from_str(&env, "Up"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #17)")]
fn update_profile_rejects_empty_name() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let input = AgentProfileInput {
        name: String::from_str(&env, ""),
        description: String::from_str(&env, "Has description"),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #18)")]
fn update_profile_rejects_empty_description() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let input = AgentProfileInput {
        name: String::from_str(&env, "has-name"),
        description: String::from_str(&env, ""),
        tags: Vec::new(&env),
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #15)")]
fn update_profile_rejects_too_many_tags() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let mut tags = Vec::new(&env);
    for _ in 0..21 {
        tags.push_back(String::from_str(&env, "t"));
    }
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags,
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);
}

#[test]
#[should_panic(expected = "Error(Contract, #16)")]
fn update_profile_rejects_tag_too_long() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    let long_tag = String::from_str(&env, &"x".repeat(33));
    let tags = Vec::from_array(&env, [long_tag]);
    let input = AgentProfileInput {
        name: String::from_str(&env, "name"),
        description: String::from_str(&env, "desc"),
        tags,
        service_url: None,
        mcp_server_url: None,
        payment_endpoint: None,
    };
    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_profile",
                args: (&owner, &input).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .update_profile(&owner, &input);
}

#[test]
fn deregister_agent_removes_from_listing() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);
    register_agent(&env, &client, &owner);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "deregister_agent",
                args: (&owner,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deregister_agent(&owner);

    assert_eq!(client.list_agents(&0, &100).len(), 0);
}

#[test]
fn deregister_agent_preserves_interactions() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let tx_hash = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 50);

    client
        .mock_auths(&[MockAuth {
            address: &provider,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "deregister_agent",
                args: (&provider,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deregister_agent(&provider);

    let interactions = client.list_agent_interactions(&provider, &0, &10);
    assert_eq!(interactions.len(), 1);
    assert_eq!(interactions.get_unchecked(0).tx_hash, tx_hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn deregister_agent_rejects_nonexistent() {
    let env = Env::default();
    let (contract_id, client, _, _) = setup(&env);
    let owner = Address::generate(&env);

    client
        .mock_auths(&[MockAuth {
            address: &owner,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "deregister_agent",
                args: (&owner,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .deregister_agent(&owner);
}

// ════════════════════════════════════════════════════════════════════════
// Auditoria independente 24/10 — testes dos fixes F1/F3/F4/F9
// ════════════════════════════════════════════════════════════════════════

#[allow(unused_imports)]
use soroban_sdk::FromVal;

fn deregister_agent_helper(env: &Env, client: &AgentPassportClient, contract_id: &Address, owner: &Address) {
    client
        .mock_auths(&[MockAuth {
            address: owner,
            invoke: &MockAuthInvoke {
                contract: contract_id,
                fn_name: "deregister_agent",
                args: (owner,).into_val(env),
                sub_invokes: &[],
            },
        }])
        .deregister_agent(owner);
}

/// F1: reputação acompanha o endereço — deregister→re-register não zera score.
#[test]
fn re_register_preserves_reputation() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);

    register_agent(&env, &client, &provider);
    let tx = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 10_000_000, 31);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx, 30);
    deregister_agent_helper(&env, &client, &contract_id, &provider);
    register_agent(&env, &client, &provider);

    let profile = client.get_agent(&provider);
    assert_eq!(profile.score, 30);
    assert_eq!(profile.verified_interactions_count, 1);
    assert_eq!(profile.unique_counterparties_count, 1);
}

/// F6: nova interação pós re-register incrementa counterparties normalmente.
#[test]
fn new_counterparty_after_re_register_counts() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);

    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 10_000_000, 32);
    deregister_agent_helper(&env, &client, &contract_id, &provider);
    register_agent(&env, &client, &provider);

    let consumer2 = Address::generate(&env);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer2, 10_000_000, 33);

    let profile = client.get_agent(&provider);
    assert_eq!(profile.verified_interactions_count, 2);
    assert_eq!(profile.unique_counterparties_count, 2);
}

/// F1: consumer de interação pré-existente avalia normalmente após re-register.
#[test]
fn rating_after_re_register_updates_score() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);

    register_agent(&env, &client, &provider);
    let tx = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 10_000_000, 34);
    deregister_agent_helper(&env, &client, &contract_id, &provider);
    register_agent(&env, &client, &provider);

    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx, 80);
    assert_eq!(client.get_agent(&provider).score, 80);
}

/// F3: amount <= 0 rejeitado com erro de contrato (não vira volume).
#[test]
#[should_panic(expected = "Error(Contract, #27)")]
fn register_interaction_rejects_zero_amount() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 0, 35);
}

#[test]
#[should_panic(expected = "Error(Contract, #27)")]
fn register_interaction_rejects_negative_amount() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, -5, 36);
}

/// F3: cap por interação — i128::MAX agora falha com erro limpo (antes brickava).
#[test]
#[should_panic(expected = "Error(Contract, #28)")]
fn register_interaction_rejects_amount_over_cap() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000_000_000_000_000_001, 37);
}

// F9 (auditoria 24/10): os guards NotInitialized (#26) permanecem como defesa
// em profundidade, mas o cenário "contrato sem config" é inalcançável desde a
// conversão para __constructor — todo CREATE roda o constructor (register sem
// args falha na própria criação: "2 expected, got 0"). Sem teste: o estado não
// existe mais.

/// F4: evento InteractionRegistered tem 4 topics e topic[3] é o relayer.
#[test]
fn interaction_event_includes_relayer_topic() {
    use soroban_sdk::xdr::ContractEventBody;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 38);

    let events = env.events().all().filter_by_contract(&contract_id);
    let interaction_event = events
        .events()
        .iter()
        .find(|e| match &e.body {
            ContractEventBody::V0(v0) => v0.topics.len() == 4,
            _ => false,
        })
        .expect("evento InteractionRegistered com 4 topics");
    let topic_relayer = match &interaction_event.body {
        ContractEventBody::V0(v0) => Address::from_val(&env, v0.topics.get(3).unwrap()),
        _ => panic!("contract event sem V0 body"),
    };
    assert_eq!(topic_relayer, relayer);
}

/// F3 boundary: exatamente o cap é ACEITO (mudança do cap não quebra happy path).
#[test]
fn register_interaction_accepts_amount_exactly_at_cap() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000_000_000_000_000_000, 42);

    assert_eq!(client.get_agent(&provider).total_economic_volume, 1_000_000_000_000_000_000_000u128);
}

/// B4 (security audit run-1): atividade renova o TTL do registry — sem bump,
/// o registro inteiro (Config, relayers, índice de owners) expirava ~1 ano.
/// Teste comportamental: avança o ledger além do TTL default de write (~4096
/// ledgers); entries SÓ sobrevivem se o bump renovou.
#[test]
fn register_interaction_extends_registry_ttl() {
    use crate::storage::StorageKey;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 42);

    // 10k ledgers adiante: write-only (TTL ~4096) já expirou; bump (~60 dias
    // = ~1_036_800 ledgers) mantém viva
    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 10_000);

    let config_alive = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, crate::types::Config>(&StorageKey::Config)
            .is_some()
    });
    let relayer_alive = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, bool>(&StorageKey::Relayer(relayer.clone()))
            .is_some()
    });
    let index_alive = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, u64>(&StorageKey::ProfileOwnerIndex(provider.clone()))
            .is_some()
    });

    assert!(config_alive, "Config deve sobreviver além do TTL de write (bump por atividade)");
    assert!(relayer_alive, "Relayer deve sobreviver além do TTL de write (bump por atividade)");
    assert!(index_alive, "ProfileOwnerIndex deve sobreviver além do TTL de write (bump por atividade)");
}

/// Revisão rodada 1 (audit run-1): bound de service_label no contrato —
/// defense-in-depth contra blob storage on-chain (a API já limita em 256).
#[test]
#[should_panic(expected = "Error(Contract, #30)")]
fn register_interaction_rejects_label_over_256() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);

    let interaction = InteractionRecord {
        provider_address: provider,
        consumer_address: consumer,
        amount: 100,
        tx_hash: BytesN::from_array(&env, &[9u8; 32]),
        timestamp: 100,
        service_label: Some(String::from_str(&env, &"x".repeat(257))),
    };
    client.mock_auths(&[MockAuth {
        address: &relayer,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "register_interaction",
            args: (relayer.clone(), interaction.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.register_interaction(&relayer, &interaction);
}

/// Revisão rodada 1 (audit run-1): bump de TTL também no submit_rating —
/// rating é atividade e renova o registry (Config etc.).
#[test]
fn submit_rating_extends_registry_ttl() {
    use crate::storage::StorageKey;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 100, 41);

    let rating = RatingInput {
        provider_address: provider.clone(),
        consumer_address: consumer.clone(),
        interaction_tx_hash: BytesN::from_array(&env, &[41u8; 32]),
        score: 90,
    };
    client.mock_auths(&[MockAuth {
        address: &consumer,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "submit_rating",
            args: (rating.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.submit_rating(&rating);

    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 10_000);

    let config_alive = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, crate::types::Config>(&StorageKey::Config)
            .is_some()
    });
    assert!(config_alive, "Config deve sobreviver além do TTL de write (bump por rating)");
}

// ── HIGH-2 (security audit run-3): pair-brick + TTL de agregados ────────────
// O mark (ProviderCounterparty) e o índice (ProviderCounterpartyList) têm vidas
// independentes: ratings bumpam só o mark. Se a List evicta enquanto o mark
// vive, o próximo submit_interaction bumpa uma chave evictada → PANICA
// (StorageMissingValue) → o par fica permanentemente brickado.

fn evict_counterparty_list(env: &Env, contract_id: &Address, provider: &Address) {
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .remove(&StorageKey::ProviderCounterpartyList(provider.clone()));
    });
}

fn counterparty_list_alive(env: &Env, contract_id: &Address, provider: &Address) -> bool {
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, Vec<Address>>(&StorageKey::ProviderCounterpartyList(
                provider.clone(),
            ))
            .is_some()
    })
}

fn entry_alive(env: &Env, contract_id: &Address, key: StorageKey) -> bool {
    env.as_contract(&contract_id, || env.storage().persistent().get::<StorageKey, u64>(&key).is_some())
}

/// Simulação fiel do cenário do run-3: ratings recorrentes mantêm o mark vivo
/// (+60d a cada rating) enquanto a List, bumpada só por interactions, expira.
#[test]
fn interaction_after_list_evict_does_not_panic_and_restores_list() {
    use crate::storage::StorageKey;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 1);

    // List evicta; mark sobrevive (ratings só bumpam o mark)
    evict_counterparty_list(&env, &contract_id, &provider);

    // Próxima interação do MESMO par: sem fix, bump em chave evictada PANICA
    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 2);

    assert!(
        counterparty_list_alive(&env, &contract_id, &provider),
        "List deve ser re-criada quando o mark sobrevive ao evict"
    );
}

/// End-to-end com queima real: ratings espaçados mantêm o mark vivo; a List do
/// interaction original expira; a interação de retorno re-cria a List (sem panic).
#[test]
fn ratings_keep_mark_alive_while_list_expires_then_interaction_recovers() {
    use crate::storage::StorageKey;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    let tx1 = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 3);

    // rating a meio caminho do TTL do interaction: empurra SÓ o mark
    let seq0 = env.ledger().sequence();
    env.ledger().set_sequence_number(seq0 + 500_000);
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx1, 80);

    // total ~1.1M ledgers desde a interaction: List (bumpada só por ela)
    // expirou em seq0+1.036.800; o mark (bumpado pelo rating) vive até
    // seq0+500k+1.036.800 = seq0+1.536.800 — divergência clássica do run-3
    env.ledger().set_sequence_number(seq0 + 1_100_000);

    register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 4);

    assert!(
        counterparty_list_alive(&env, &contract_id, &provider),
        "interaction de retorno deve restaurar a List evictada (pair-brick morto)"
    );
}

#[test]
fn rating_with_evicted_list_does_not_panic() {
    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    let tx = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 5);

    evict_counterparty_list(&env, &contract_id, &provider);

    // rating sobre interação do par com List evictada: bump só-se-viva nunca pânica
    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx, 90);
}

/// Set renova pro TTL default curto (~4096 ledgers) — agregados de score só
/// sobrevivem além disso se o rating os BUMPAR (simetria run-3).
#[test]
fn rating_renews_profile_and_aggregates_ttl() {
    use crate::storage::StorageKey;

    let env = Env::default();
    let (contract_id, client, _, relayer) = setup(&env);
    let provider = Address::generate(&env);
    let consumer = Address::generate(&env);
    register_agent(&env, &client, &provider);
    let tx = register_interaction_tx(&env, &client, &contract_id, &relayer, &provider, &consumer, 1_000_000, 6);

    submit_rating_tx(&env, &client, &contract_id, &consumer, &provider, &tx, 85);

    // 10k ledgers: write-only (TTL ~4096) já expirou; bump (60d) mantém viva
    let seq = env.ledger().sequence();
    env.ledger().set_sequence_number(seq + 10_000);

    let profile_alive = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<StorageKey, crate::types::AgentProfile>(&StorageKey::Profile(provider.clone()))
            .is_some()
    });
    assert!(
        profile_alive,
        "Profile deve ser BUMPADO no rating (set deixa TTL default curto)"
    );
    assert!(
        entry_alive(&env, &contract_id, StorageKey::ProviderRatingCount(provider.clone())),
        "ProviderRatingCount deve ser BUMPADO no rating"
    );
    assert!(
        entry_alive(&env, &contract_id, StorageKey::ProviderRatingTotal(provider.clone())),
        "ProviderRatingTotal deve ser BUMPADO no rating"
    );
}
