//! Code-owned, fail-closed gate for the complete 364-entry Technical Analysis Master List.
//!
//! This Rust module is the authority for creating strict assessment work plans and permitting
//! a downstream research/paper-trading handoff. Agent-authored claims never promote a catalog
//! concept. Live execution is deliberately outside this contract and always disabled.

use crate::{application::backtest::dataset_sha256, domain::models::Candle};
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
    sync::{Mutex, OnceLock},
    time::Duration,
};
use uuid::Uuid;

pub const MASTER_LIST_VERSION: &str = "1.0.0";
pub const WORK_PLAN_VERSION: &str = "strict-ta-work-plan.v1";
pub const ASSESSMENT_CONTRACT_VERSION: &str = "strict-ta-assessment.v1";
pub const DECISION_GATE_CONTRACT_VERSION: &str = "strict-ta-decision-gate.v4";
pub const REQUIRED_CONCEPT_COUNT: usize = 364;
pub const CATALOG_JSON: &str =
    include_str!("../../../scripts/fixtures/technical-analysis-master-list.v1.json");

static CATALOG: OnceLock<Result<StrictCatalogManifest, String>> = OnceLock::new();
type IssuedWorkPlanRegistry = Mutex<HashMap<String, (i64, StrictAnalysisWorkPlan, bool)>>;

static ISSUED_WORK_PLANS: OnceLock<IssuedWorkPlanRegistry> = OnceLock::new();
const WORK_PLAN_TTL_MS: i64 = 15 * 60 * 1_000;
const MAX_ISSUED_WORK_PLANS: usize = 256;
const MAX_EVIDENCE_REFS_PER_ASSESSMENT: usize = 32;
const MAX_EVIDENCE_REF_LENGTH: usize = 256;
const STRICT_STORE_SCHEMA_VERSION: i64 = 3;
const AUDIT_CHAIN_GENESIS: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCatalogEntry {
    pub id: String,
    pub category: String,
    pub group: String,
    pub name: String,
    pub availability: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCatalogManifest {
    pub schema_version: u32,
    pub master_list_version: String,
    pub expected_entry_count: usize,
    pub categories: Vec<String>,
    pub category_counts: BTreeMap<String, usize>,
    pub note: String,
    pub entries: Vec<StrictCatalogEntry>,
}

pub fn trusted_catalog() -> Result<&'static StrictCatalogManifest, String> {
    match CATALOG.get_or_init(|| {
        let manifest: StrictCatalogManifest = serde_json::from_str(CATALOG_JSON)
            .map_err(|error| format!("Embedded strict catalog manifest is invalid: {error}"))?;
        validate_manifest_integrity(&manifest)?;
        Ok(manifest)
    }) {
        Ok(manifest) => Ok(manifest),
        Err(error) => Err(error.clone()),
    }
}

fn validate_manifest_integrity(manifest: &StrictCatalogManifest) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.master_list_version != MASTER_LIST_VERSION {
        return Err(
            "Embedded master-list schema/version does not match the Rust gate contract.".into(),
        );
    }
    if manifest.expected_entry_count != REQUIRED_CONCEPT_COUNT
        || manifest.entries.len() != REQUIRED_CONCEPT_COUNT
    {
        return Err(format!(
            "Embedded master list must contain exactly {REQUIRED_CONCEPT_COUNT} entries."
        ));
    }
    let mut ids = BTreeSet::new();
    let mut category_counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in &manifest.entries {
        if entry.id.trim().is_empty()
            || entry.category.trim().is_empty()
            || entry.group.trim().is_empty()
            || entry.name.trim().is_empty()
            || entry.availability.trim().is_empty()
            || entry.kind.trim().is_empty()
        {
            return Err("Embedded catalog contains a blank identity or status field.".into());
        }
        let expected_id = format!("{}::{}::{}", entry.category, entry.group, entry.name);
        if entry.id != expected_id {
            return Err(format!(
                "Embedded catalog identity is not canonical for {}.",
                entry.id
            ));
        }
        if !ids.insert(entry.id.as_str()) {
            return Err(format!(
                "Embedded catalog contains duplicate ID {}.",
                entry.id
            ));
        }
        *category_counts.entry(entry.category.clone()).or_default() += 1;
    }
    if manifest.categories.len() != 9
        || category_counts.len() != 9
        || category_counts != manifest.category_counts
    {
        return Err("Embedded catalog category inventory/counts are inconsistent.".into());
    }
    let manifest_categories = manifest.categories.iter().cloned().collect::<BTreeSet<_>>();
    let counted_categories = category_counts.keys().cloned().collect::<BTreeSet<_>>();
    if manifest_categories != counted_categories {
        return Err(
            "Embedded catalog category names differ from the counted category inventory.".into(),
        );
    }
    Ok(())
}

/// Exact implementation versions accepted by the native gate. Agent-supplied arbitrary
/// version strings cannot substitute for a code-owned reviewed implementation entry.
pub fn reviewed_implementation_version(catalog_id: &str) -> Option<&'static str> {
    match catalog_id {
        "Indikator Teknikal::Trend::SMA (Simple Moving Average)" => {
            Some("technical-analysis-calculators.v1")
        }
        "Indikator Teknikal::Trend::EMA (Exponential Moving Average)" => {
            Some("technical-analysis-calculators.v1")
        }
        "Indikator Teknikal::Momentum::RSI (Relative Strength Index)" => {
            Some("technical-analysis-calculators.v1")
        }
        "Indikator Teknikal::Momentum::MACD (Moving Average Convergence Divergence)" => {
            Some("technical-analysis-calculators.v1")
        }
        "Indikator Teknikal::Volatility::ATR (Average True Range)" => {
            Some("technical-analysis-calculators.v1")
        }
        "Indikator Teknikal::Volatility::Bollinger Bands" => {
            Some("technical-analysis-calculators.v1")
        }
        "Risk Management & Evaluation::Strategy Validation & Journal::Backtesting" => {
            Some("rust-backtest-engine.v1")
        }
        _ => None,
    }
}

pub fn specialist_for_category(category: &str) -> Option<&'static str> {
    match category {
        "SMC & ICT" => Some("agent-01-smc-ict"),
        "Price Action" => Some("agent-02-price-action"),
        "Support, Resistance & Trend" => Some("agent-03-support-resistance-trend"),
        "Indikator Teknikal" => Some("agent-04-technical-indicators"),
        "Order Flow & Market Profile" => Some("agent-05-order-flow-market-profile"),
        "Divergence & Intermarket Analysis" => Some("agent-06-divergence-intermarket"),
        "Metode Analisis Lain" => Some("agent-07-other-methods"),
        "Fundamental & Makro" => Some("agent-08-fundamental-macro"),
        "Risk Management & Evaluation" => Some("agent-09-risk-management-evaluation"),
        _ => None,
    }
}

fn knowledge_pack_for_category(category: &str) -> Option<&'static str> {
    match category {
        "SMC & ICT" => Some("docs/agents/01-smc-ict-agent.md"),
        "Price Action" => Some("docs/agents/02-price-action-agent.md"),
        "Support, Resistance & Trend" => Some("docs/agents/03-support-resistance-trend-agent.md"),
        "Indikator Teknikal" => Some("docs/agents/04-technical-indicators-agent.md"),
        "Order Flow & Market Profile" => Some("docs/agents/05-order-flow-market-profile-agent.md"),
        "Divergence & Intermarket Analysis" => {
            Some("docs/agents/06-divergence-intermarket-agent.md")
        }
        "Metode Analisis Lain" => Some("docs/agents/07-other-methods-agent.md"),
        "Fundamental & Makro" => Some("docs/agents/08-fundamental-macro-agent.md"),
        "Risk Management & Evaluation" => {
            Some("docs/agents/09-risk-management-evaluation-agent.md")
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictAnalysisTask {
    pub agent_id: String,
    pub category: String,
    pub knowledge_pack_path: String,
    pub required_assessment_count: usize,
    pub catalog_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictAnalysisWorkPlan {
    pub work_plan_id: String,
    pub schema_version: String,
    pub decision_gate_contract_version: String,
    pub assessment_contract_version: String,
    pub master_list_version: String,
    pub symbol: String,
    pub timeframe: String,
    pub as_of: String,
    pub dataset_sha256: String,
    pub expected_assessment_count: usize,
    pub inventory_identity: String,
    pub dispatch_status: String,
    pub live_execution_authorized: bool,
    pub entries: Vec<StrictCatalogEntry>,
    pub tasks: Vec<StrictAnalysisTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkPlanRequest {
    pub symbol: String,
    pub timeframe: String,
    pub as_of: String,
    pub dataset_sha256: String,
}

fn issued_work_plans() -> &'static IssuedWorkPlanRegistry {
    ISSUED_WORK_PLANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn issue_work_plan(plan: StrictAnalysisWorkPlan) -> Result<StrictAnalysisWorkPlan, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut registry = issued_work_plans()
        .lock()
        .map_err(|_| "Rust-issued work-plan registry lock poisoned".to_string())?;
    registry.retain(|_, (issued_at, _, _)| now.saturating_sub(*issued_at) <= WORK_PLAN_TTL_MS);
    if registry.len() >= MAX_ISSUED_WORK_PLANS {
        return Err("Strict analysis work-plan capacity reached; expire or complete pending plans before creating another.".into());
    }
    registry.insert(plan.work_plan_id.clone(), (now, plan.clone(), false));
    Ok(plan)
}

#[cfg(test)]
fn work_plan_is_unmodified_and_unexpired(plan: &StrictAnalysisWorkPlan) -> Result<bool, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut registry = issued_work_plans()
        .lock()
        .map_err(|_| "Rust-issued work-plan registry lock poisoned".to_string())?;
    registry.retain(|_, (issued_at, _, _)| now.saturating_sub(*issued_at) <= WORK_PLAN_TTL_MS);
    let Some((_, issued, consumed)) = registry.get(&plan.work_plan_id) else {
        return Ok(false);
    };
    if *consumed {
        return Ok(false);
    }
    let supplied_json = serde_json::to_value(plan)
        .map_err(|error| format!("Cannot validate supplied work plan: {error}"))?;
    let issued_json = serde_json::to_value(issued)
        .map_err(|error| format!("Cannot validate Rust-issued work plan: {error}"))?;
    Ok(supplied_json == issued_json)
}

/// Atomically consumes a Rust-issued work plan. A plan may authorize at most one completion
/// attempt, including a blocked attempt; callers must request a fresh plan to retry. This
/// prevents concurrent duplicate submissions from both passing the plan-identity check.
fn consume_work_plan_once(plan: &StrictAnalysisWorkPlan) -> Result<bool, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut registry = issued_work_plans()
        .lock()
        .map_err(|_| "Rust-issued work-plan registry lock poisoned".to_string())?;
    registry.retain(|_, (issued_at, _, _)| now.saturating_sub(*issued_at) <= WORK_PLAN_TTL_MS);
    let Some((_, issued, consumed)) = registry.get_mut(&plan.work_plan_id) else {
        return Ok(false);
    };
    if *consumed {
        return Ok(false);
    }
    let supplied_json = serde_json::to_value(plan)
        .map_err(|error| format!("Cannot validate supplied work plan: {error}"))?;
    let issued_json = serde_json::to_value(issued)
        .map_err(|error| format!("Cannot validate Rust-issued work plan: {error}"))?;
    if supplied_json != issued_json {
        return Ok(false);
    }
    *consumed = true;
    Ok(true)
}

fn build_work_plan(request: CreateWorkPlanRequest) -> Result<StrictAnalysisWorkPlan, String> {
    if request.symbol.trim().is_empty() {
        return Err("symbol is required to create a strict analysis work plan.".into());
    }
    if request.timeframe.trim().is_empty() {
        return Err("timeframe is required to create a strict analysis work plan.".into());
    }
    if timeframe_duration_ms(&request.timeframe).is_none() {
        return Err(
            "timeframe must be one of the supported canonical intervals: 1m, 5m, 15m, 1h, 4h, 1d."
                .into(),
        );
    }
    if !valid_timestamp(&request.as_of) {
        return Err(
            "asOf must be a valid ISO-8601/RFC3339 timestamp with an explicit timezone.".into(),
        );
    }
    if !valid_sha256(&request.dataset_sha256) {
        return Err("datasetSha256 must be a 64-character SHA-256 hex digest.".into());
    }

    let manifest = trusted_catalog()?;
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in &manifest.entries {
        groups
            .entry(entry.category.clone())
            .or_default()
            .push(entry.id.clone());
    }
    let mut tasks = Vec::with_capacity(9);
    // Use manifest category order for deterministic task order, not BTreeMap lexical order.
    for category in &manifest.categories {
        let ids = groups
            .get(category)
            .ok_or_else(|| format!("No catalog entries for specialist category {category}."))?;
        let agent_id = specialist_for_category(category)
            .ok_or_else(|| format!("No specialist routing for category {category}."))?;
        let knowledge_pack_path = knowledge_pack_for_category(category)
            .ok_or_else(|| format!("No knowledge pack for category {category}."))?;
        tasks.push(StrictAnalysisTask {
            agent_id: agent_id.into(),
            category: category.clone(),
            knowledge_pack_path: knowledge_pack_path.into(),
            required_assessment_count: ids.len(),
            catalog_ids: ids.clone(),
        });
    }
    Ok(StrictAnalysisWorkPlan {
        work_plan_id: Uuid::new_v4().to_string(),
        schema_version: WORK_PLAN_VERSION.into(),
        decision_gate_contract_version: DECISION_GATE_CONTRACT_VERSION.into(),
        assessment_contract_version: ASSESSMENT_CONTRACT_VERSION.into(),
        master_list_version: MASTER_LIST_VERSION.into(),
        symbol: request.symbol.trim().to_string(),
        timeframe: request.timeframe.trim().to_string(),
        as_of: request.as_of,
        dataset_sha256: request.dataset_sha256.to_ascii_lowercase(),
        expected_assessment_count: REQUIRED_CONCEPT_COUNT,
        inventory_identity: "rust-embedded-code-owned-master-list".into(),
        dispatch_status: "work_plan_created_not_dispatched".into(),
        live_execution_authorized: false,
        entries: manifest.entries.clone(),
        tasks,
    })
}

/// In-memory issuance remains available for isolated unit tests and parity harnesses only.
/// The Tauri production command uses `create_work_plan_persisted`.
pub fn create_work_plan(request: CreateWorkPlanRequest) -> Result<StrictAnalysisWorkPlan, String> {
    issue_work_plan(build_work_plan(request)?)
}

#[derive(Debug, Clone)]
struct StrictAuditChainEvent {
    seq: i64,
    event_type: String,
    work_plan_id: Option<String>,
    occurred_at_ms: i64,
    payload_sha256: String,
    previous_hash: String,
    event_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrictAuditChainEventMaterial<'a> {
    seq: i64,
    event_type: &'a str,
    work_plan_id: Option<&'a str>,
    occurred_at_ms: i64,
    payload_sha256: &'a str,
    previous_hash: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrictFinalizationDigestMaterial<'a> {
    work_plan_id: &'a str,
    status: &'a str,
    decision: &'a str,
    receipt_sha256: &'a str,
    assessment_bundle_sha256: String,
    result_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyOrphanAttemptDigestMaterial {
    work_plan_id: String,
    plan_sha256: String,
    started_at_ms: i64,
    completed_at_ms: Option<i64>,
    status: String,
    decision: Option<String>,
    receipt_sha256: Option<String>,
    assessment_bundle_sha256: Option<String>,
    result_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictAuditChainStatus {
    pub valid: bool,
    pub event_count: usize,
    pub last_sequence: i64,
    pub head_sha256: String,
}

fn finalization_event_digest(
    work_plan_id: &str,
    status: &str,
    decision: &str,
    receipt_sha256: &str,
    assessment_bundle_json: &str,
    result_json: &str,
) -> Result<String, String> {
    sha256_serializable(&StrictFinalizationDigestMaterial {
        work_plan_id,
        status,
        decision,
        receipt_sha256,
        assessment_bundle_sha256: sha256_bytes(assessment_bundle_json.as_bytes()),
        result_sha256: sha256_bytes(result_json.as_bytes()),
    })
}

fn legacy_orphan_attempt_digest(
    material: &LegacyOrphanAttemptDigestMaterial,
) -> Result<String, String> {
    sha256_serializable(material)
}

fn read_chain_events(
    connection: &Connection,
) -> Result<(Vec<StrictAuditChainEvent>, i64, String), String> {
    let events = {
        let mut statement = connection.prepare(
            "SELECT seq, event_type, work_plan_id, occurred_at_ms, payload_sha256, previous_hash, event_sha256 FROM strict_audit_chain_events ORDER BY seq ASC"
        ).map_err(|error| format!("Cannot read strict audit chain: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(StrictAuditChainEvent {
                    seq: row.get(0)?,
                    event_type: row.get(1)?,
                    work_plan_id: row.get(2)?,
                    occurred_at_ms: row.get(3)?,
                    payload_sha256: row.get(4)?,
                    previous_hash: row.get(5)?,
                    event_sha256: row.get(6)?,
                })
            })
            .map_err(|error| format!("Cannot scan strict audit chain: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot decode strict audit event: {error}"))?;
        rows
    };
    let (last_sequence, head_sha256): (i64, String) = connection
        .query_row(
            "SELECT last_seq, head_sha256 FROM strict_audit_chain_head WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("Cannot read strict audit-chain head: {error}"))?;
    let mut previous = AUDIT_CHAIN_GENESIS.to_string();
    for (index, event) in events.iter().enumerate() {
        if event.seq != index as i64 + 1 {
            return Err(format!(
                "Strict audit-chain sequence gap or reordering at sequence {}.",
                index + 1
            ));
        }
        let event_type_is_known = matches!(
            event.event_type.as_str(),
            "store_initialized"
                | "schema_migrated"
                | "schema_migrated_v2_to_v3"
                | "work_plan_issued"
                | "work_plan_consumed"
                | "assessment_attempt_started"
                | "assessment_attempt_finalized"
                | "legacy_orphan_attempt_imported"
                | "specialist_receipt_recorded"
        );
        if !event_type_is_known
            || event.event_type.trim().is_empty()
            || !valid_sha256(&event.payload_sha256)
            || !valid_sha256(&event.previous_hash)
            || !valid_sha256(&event.event_sha256)
        {
            return Err(format!(
                "Malformed or unknown strict audit-chain event at sequence {}.",
                event.seq
            ));
        }
        if event.previous_hash != previous {
            return Err(format!(
                "Strict audit-chain previous hash mismatch at sequence {}.",
                event.seq
            ));
        }
        let material = StrictAuditChainEventMaterial {
            seq: event.seq,
            event_type: &event.event_type,
            work_plan_id: event.work_plan_id.as_deref(),
            occurred_at_ms: event.occurred_at_ms,
            payload_sha256: &event.payload_sha256,
            previous_hash: &event.previous_hash,
        };
        let computed = sha256_serializable(&material)?;
        if computed != event.event_sha256 {
            return Err(format!("Strict audit-chain event hash mismatch at sequence {}; possible tampering detected.", event.seq));
        }
        previous = event.event_sha256.clone();
    }
    if last_sequence != events.len() as i64 || head_sha256 != previous {
        return Err(
            "Strict audit-chain head mismatch; tail truncation or head tampering is suspected."
                .into(),
        );
    }
    Ok((events, last_sequence, head_sha256))
}

fn append_audit_event_raw(
    tx: &rusqlite::Transaction<'_>,
    event_type: &str,
    work_plan_id: Option<&str>,
    at_ms: i64,
    payload_sha256: &str,
) -> Result<(), String> {
    if event_type.trim().is_empty() || !valid_sha256(payload_sha256) {
        return Err("Refusing malformed audit event.".into());
    }
    let (last_seq, previous_hash): (i64, String) = tx
        .query_row(
            "SELECT last_seq, head_sha256 FROM strict_audit_chain_head WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("Cannot read audit head before append: {error}"))?;
    let seq = last_seq
        .checked_add(1)
        .ok_or_else(|| "Strict audit sequence exhausted.".to_string())?;
    let material = StrictAuditChainEventMaterial {
        seq,
        event_type,
        work_plan_id,
        occurred_at_ms: at_ms,
        payload_sha256,
        previous_hash: &previous_hash,
    };
    let event_sha256 = sha256_serializable(&material)?;
    tx.execute(
        "INSERT INTO strict_audit_chain_events(seq,event_type,work_plan_id,occurred_at_ms,payload_sha256,previous_hash,event_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![seq, event_type, work_plan_id, at_ms, payload_sha256, previous_hash, event_sha256],
    ).map_err(|error| format!("Cannot append audit event: {error}"))?;
    let changed = tx.execute(
        "UPDATE strict_audit_chain_head SET last_seq=?1,head_sha256=?2 WHERE singleton=1 AND last_seq=?3 AND head_sha256=?4",
        params![seq, event_sha256, last_seq, previous_hash],
    ).map_err(|error| format!("Cannot advance audit head: {error}"))?;
    if changed != 1 {
        return Err("Audit head changed concurrently; transaction must abort.".into());
    }
    Ok(())
}

fn chain_event_exists(
    events: &[StrictAuditChainEvent],
    kind: &str,
    plan_id: Option<&str>,
    at_ms: i64,
    digest: &str,
) -> bool {
    events.iter().any(|event| {
        event.event_type == kind
            && event.work_plan_id.as_deref() == plan_id
            && event.occurred_at_ms == at_ms
            && event.payload_sha256 == digest
    })
}

fn verify_audit_chain_connection(connection: &Connection) -> Result<(usize, i64, String), String> {
    let (events, last_sequence, head_sha256) = read_chain_events(connection)?;
    let plans = {
        let mut statement = connection.prepare("SELECT work_plan_id,issued_at_ms,plan_json,plan_sha256,consumed_at_ms FROM strict_work_plans")
            .map_err(|error| format!("Cannot prepare work-plan audit cross-check: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(|error| format!("Cannot scan work-plan audit cross-check: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot decode work-plan audit row: {error}"))?;
        rows
    };
    for (id, issued_at, plan_json, plan_sha, consumed_at) in &plans {
        if sha256_bytes(plan_json.as_bytes()) != *plan_sha {
            return Err(format!(
                "Stored work-plan digest mismatch for {id}; refusing decisions."
            ));
        }
        if !chain_event_exists(&events, "work_plan_issued", Some(id), *issued_at, plan_sha) {
            return Err(format!(
                "Work plan {id} has no matching immutable issuance event."
            ));
        }
        match consumed_at {
            Some(at)
                if !chain_event_exists(&events, "work_plan_consumed", Some(id), *at, plan_sha) =>
            {
                return Err(format!(
                    "Work plan {id} consumed marker differs from audit chain."
                ))
            }
            None if events.iter().any(|event| {
                event.event_type == "work_plan_consumed"
                    && event.work_plan_id.as_deref() == Some(id)
            }) =>
            {
                return Err(format!(
                    "Work plan {id} has a consume event but is unconsumed."
                ))
            }
            _ => {}
        }
    }
    let attempts = {
        let mut statement = connection.prepare("SELECT work_plan_id,plan_sha256,started_at_ms,completed_at_ms,status,decision,receipt_sha256,assessment_bundle_json,result_json FROM strict_analysis_attempts")
            .map_err(|error| format!("Cannot prepare attempt audit cross-check: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|error| format!("Cannot scan attempt audit cross-check: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot decode attempt audit row: {error}"))?;
        rows
    };
    for (id, plan_sha, started, completed, status, decision, receipt, bundle, result) in &attempts {
        let plan_row = plans.iter().find(|(plan_id, _, _, _, _)| plan_id == id);
        match plan_row {
            Some((_, _, _, digest, Some(_))) if digest == plan_sha => {},
            None => {
                let imported_at = completed.unwrap_or(*started);
                let digest = legacy_orphan_attempt_digest(&LegacyOrphanAttemptDigestMaterial {
                    work_plan_id: id.clone(),
                    plan_sha256: plan_sha.clone(),
                    started_at_ms: *started,
                    completed_at_ms: *completed,
                    status: status.clone(),
                    decision: decision.clone(),
                    receipt_sha256: receipt.clone(),
                    assessment_bundle_sha256: bundle
                        .as_deref()
                        .map(|value| sha256_bytes(value.as_bytes())),
                    result_sha256: result
                        .as_deref()
                        .map(|value| sha256_bytes(value.as_bytes())),
                })?;
                if !chain_event_exists(&events, "legacy_orphan_attempt_imported", Some(id), imported_at, &digest) {
                    return Err(format!("Historical attempt {id} lost its plan before audit-chain migration and lacks a matching immutable legacy snapshot."));
                }
                if completed.is_none() {
                    if status != "in_progress" || decision.is_some() || receipt.is_some() || bundle.is_some() || result.is_some() {
                        return Err(format!("Legacy orphan attempt {id} has inconsistent in-progress fields."));
                    }
                } else if !matches!(status.as_str(), "blocked" | "allow_research" | "allow_to_next_layer")
                    || decision.is_none() || receipt.as_deref().is_none_or(|value| !valid_sha256(value)) || bundle.is_none() || result.is_none() {
                    return Err(format!("Legacy orphan attempt {id} has inconsistent finalized fields."));
                }
                continue;
            }
            Some(_) => return Err(format!("Attempt {id} references a work plan that was never consumed or has a mismatched digest.")),
        }
        if !chain_event_exists(
            &events,
            "assessment_attempt_started",
            Some(id),
            *started,
            plan_sha,
        ) {
            return Err(format!(
                "Attempt {id} has no matching immutable start event."
            ));
        }
        match completed {
            None if status == "in_progress"
                && decision.is_none()
                && receipt.is_none()
                && bundle.is_none()
                && result.is_none() => {}
            None => return Err(format!("Attempt {id} has inconsistent in-progress state.")),
            Some(at) => {
                let (Some(decision), Some(receipt), Some(bundle), Some(result)) = (
                    decision.as_deref(),
                    receipt.as_deref(),
                    bundle.as_deref(),
                    result.as_deref(),
                ) else {
                    return Err(format!("Finalized attempt {id} is missing audit fields."));
                };
                let expected_status = match decision {
                    "BLOCK" => "blocked",
                    "ALLOW_RESEARCH" => "allow_research",
                    "ALLOW_TO_NEXT_LAYER" => "allow_to_next_layer",
                    _ => return Err(format!("Attempt {id} has unknown decision.")),
                };
                if status != expected_status || !valid_sha256(receipt) {
                    return Err(format!("Attempt {id} status or receipt digest is invalid."));
                }
                let digest =
                    finalization_event_digest(id, status, decision, receipt, bundle, result)?;
                if !chain_event_exists(
                    &events,
                    "assessment_attempt_finalized",
                    Some(id),
                    *at,
                    &digest,
                ) {
                    return Err(format!(
                        "Finalized attempt {id} differs from audit-chain event."
                    ));
                }
            }
        }
    }
    let dispatch_receipts = {
        let mut statement = connection.prepare(
            "SELECT work_plan_id,agent_id,category,task_input_sha256,output_assessment_sha256,covered_concept_count,completed_at_ms,receipt_sha256 FROM strict_specialist_dispatch_receipts ORDER BY work_plan_id,agent_id"
        ).map_err(|error| format!("Cannot inspect durable specialist receipts: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(|error| format!("Cannot enumerate durable specialist receipts: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Cannot decode durable specialist receipt: {error}"))?;
        rows
    };
    let trusted_manifest = trusted_catalog()?;
    for (plan_id, agent_id, category, input_sha, output_sha, count, completed_at, receipt_sha) in
        &dispatch_receipts
    {
        let plan_row = plans
            .iter()
            .find(|(id, _, _, _, _)| id == plan_id)
            .ok_or_else(|| {
                format!("Specialist receipt {agent_id} refers to a missing work plan.")
            })?;
        let plan_sha = plan_row.3.as_str();
        let stored_plan: StrictAnalysisWorkPlan =
            serde_json::from_str(&plan_row.2).map_err(|error| {
                format!("Cannot decode work plan bound to specialist receipt {agent_id}: {error}")
            })?;
        if stored_plan.work_plan_id != *plan_id {
            return Err(format!(
                "Specialist receipt {agent_id}/{plan_id} is bound to a different plan identity."
            ));
        }
        let (expected_input_sha, expected_count) =
            specialist_task_input_sha256(&stored_plan, category, trusted_manifest)?;
        if input_sha != &expected_input_sha || *count != expected_count as i64 {
            return Err(format!("Specialist receipt {agent_id}/{plan_id} does not match the exact code-owned dispatch input and category inventory."));
        }
        let material = SpecialistDispatchReceiptMaterial {
            work_plan_id: plan_id.clone(),
            agent_id: agent_id.clone(),
            category: category.clone(),
            task_input_sha256: input_sha.clone(),
            output_assessment_sha256: output_sha.clone(),
            covered_concept_count: *count as usize,
            completed_at_ms: *completed_at,
        };
        if !valid_sha256(input_sha)
            || !valid_sha256(output_sha)
            || !valid_sha256(receipt_sha)
            || sha256_serializable(&material)? != *receipt_sha
            || !chain_event_exists(
                &events,
                "specialist_receipt_recorded",
                Some(plan_id),
                *completed_at,
                receipt_sha,
            )
        {
            return Err(format!("Specialist receipt {agent_id}/{plan_id} differs from its immutable audit-chain event."));
        }
        if specialist_for_category(category) != Some(agent_id.as_str())
            || *count <= 0
            || plan_sha.is_empty()
        {
            return Err(format!(
                "Specialist receipt {agent_id}/{plan_id} has invalid route or coverage."
            ));
        }
        // Once a final attempt exists, bind the previously recorded specialist output to the
        // exact assessment subset that was ultimately submitted to the decision gate.
        if let Some(attempt) = attempts.iter().find(|attempt| {
            attempt.0.as_str() == plan_id.as_str() && attempt.3.is_some() && attempt.7.is_some()
        }) {
            let bundle_json = attempt
                .7
                .as_deref()
                .ok_or_else(|| "Finalized attempt lost its assessment bundle.".to_string())?;
            let bundle: Vec<StrictAssessment> = serde_json::from_str(bundle_json)
                .map_err(|error| format!("Cannot decode finalized assessment bundle for specialist cross-check: {error}"))?;
            let category_assessments = bundle
                .iter()
                .filter(|assessment| {
                    trusted_manifest.entries.iter().any(|entry| {
                        entry.id == assessment.catalog_id && entry.category == *category
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let (final_output_sha, final_count) = category_assessment_output_sha256(
                category,
                &category_assessments,
                trusted_manifest,
            )?;
            if final_output_sha != *output_sha || final_count as i64 != *count {
                return Err(format!("Specialist output receipt {agent_id}/{plan_id} does not match the finalized assessment bundle."));
            }
        }
    }
    for (kind, count) in [
        ("work_plan_issued", plans.len()),
        (
            "work_plan_consumed",
            plans
                .iter()
                .filter(|(_, _, _, _, used)| used.is_some())
                .count(),
        ),
        (
            "assessment_attempt_started",
            attempts
                .iter()
                .filter(|(id, _, _, _, _, _, _, _, _)| {
                    plans
                        .iter()
                        .any(|(plan_id, _, _, _, consumed)| plan_id == id && consumed.is_some())
                })
                .count(),
        ),
        (
            "assessment_attempt_finalized",
            attempts
                .iter()
                .filter(|(id, _, _, at, _, _, _, _, _)| {
                    at.is_some()
                        && plans
                            .iter()
                            .any(|(plan_id, _, _, _, consumed)| plan_id == id && consumed.is_some())
                })
                .count(),
        ),
        (
            "legacy_orphan_attempt_imported",
            attempts
                .iter()
                .filter(|(id, _, _, _, _, _, _, _, _)| {
                    !plans.iter().any(|(plan_id, _, _, _, _)| plan_id == id)
                })
                .count(),
        ),
        ("specialist_receipt_recorded", dispatch_receipts.len()),
    ] {
        let recorded = events
            .iter()
            .filter(|event| event.event_type == kind)
            .count();
        if recorded != count {
            return Err(format!(
                "Audit event count for {kind} is {recorded}; durable record count is {count}."
            ));
        }
    }
    Ok((events.len(), last_sequence, head_sha256))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpecialistDispatchReceiptMaterial {
    work_plan_id: String,
    agent_id: String,
    category: String,
    task_input_sha256: String,
    output_assessment_sha256: String,
    covered_concept_count: usize,
    completed_at_ms: i64,
}

fn knowledge_pack_content(category: &str) -> Option<&'static str> {
    match category {
        "SMC & ICT" => Some(include_str!("../../../docs/agents/01-smc-ict-agent.md")),
        "Price Action" => Some(include_str!(
            "../../../docs/agents/02-price-action-agent.md"
        )),
        "Support, Resistance & Trend" => Some(include_str!(
            "../../../docs/agents/03-support-resistance-trend-agent.md"
        )),
        "Indikator Teknikal" => Some(include_str!(
            "../../../docs/agents/04-technical-indicators-agent.md"
        )),
        "Order Flow & Market Profile" => Some(include_str!(
            "../../../docs/agents/05-order-flow-market-profile-agent.md"
        )),
        "Divergence & Intermarket Analysis" => Some(include_str!(
            "../../../docs/agents/06-divergence-intermarket-agent.md"
        )),
        "Metode Analisis Lain" => Some(include_str!(
            "../../../docs/agents/07-other-methods-agent.md"
        )),
        "Fundamental & Makro" => Some(include_str!(
            "../../../docs/agents/08-fundamental-macro-agent.md"
        )),
        "Risk Management & Evaluation" => Some(include_str!(
            "../../../docs/agents/09-risk-management-evaluation-agent.md"
        )),
        _ => None,
    }
}

fn specialist_task_input_sha256(
    plan: &StrictAnalysisWorkPlan,
    category: &str,
    manifest: &StrictCatalogManifest,
) -> Result<(String, usize), String> {
    let agent_id = specialist_for_category(category)
        .ok_or_else(|| format!("No trusted specialist for dispatch category {category}."))?;
    let pack = knowledge_pack_content(category)
        .ok_or_else(|| format!("No embedded knowledge pack for dispatch category {category}."))?;
    let catalog_ids = manifest
        .entries
        .iter()
        .filter(|entry| entry.category == category)
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    if catalog_ids.is_empty() {
        return Err(format!(
            "Dispatch category {category} contains no trusted concepts."
        ));
    }
    let material = serde_json::json!({
        "contractVersion": "strict-agent-dispatch-input.v1",
        "workPlanId": plan.work_plan_id,
        "agentId": agent_id,
        "category": category,
        "masterListVersion": MASTER_LIST_VERSION,
        "assessmentContractVersion": ASSESSMENT_CONTRACT_VERSION,
        "decisionGateContractVersion": DECISION_GATE_CONTRACT_VERSION,
        "symbol": plan.symbol,
        "timeframe": plan.timeframe,
        "asOf": plan.as_of,
        "datasetSha256": plan.dataset_sha256,
        "knowledgePackSha256": sha256_bytes(pack.as_bytes()),
        "catalogIds": catalog_ids
    });
    let count = manifest
        .entries
        .iter()
        .filter(|entry| entry.category == category)
        .count();
    Ok((sha256_serializable(&material)?, count))
}

fn category_assessment_output_sha256(
    category: &str,
    assessments: &[StrictAssessment],
    manifest: &StrictCatalogManifest,
) -> Result<(String, usize), String> {
    let expected_ids = manifest
        .entries
        .iter()
        .filter(|entry| entry.category == category)
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut actual_ids = BTreeSet::<&str>::new();
    for assessment in assessments {
        let trusted = manifest
            .entries
            .iter()
            .find(|entry| entry.id == assessment.catalog_id)
            .ok_or_else(|| {
                format!(
                    "Specialist output contains unknown catalog ID {}.",
                    assessment.catalog_id
                )
            })?;
        let expected_agent = specialist_for_category(category)
            .ok_or_else(|| format!("No trusted specialist for dispatch category {category}."))?;
        if trusted.category != category
            || assessment.category != category
            || assessment.group != trusted.group
            || assessment.name != trusted.name
            || assessment.agent_id != expected_agent
            || assessment.contract_version != ASSESSMENT_CONTRACT_VERSION
            || assessment.master_list_version != MASTER_LIST_VERSION
            || ![
                "evaluated",
                "insufficient_data",
                "not_applicable",
                "unsupported",
            ]
            .contains(&assessment.status.as_str())
        {
            return Err(format!(
                "Specialist assessment identity/status mismatch for {}.",
                assessment.catalog_id
            ));
        }
        if !actual_ids.insert(assessment.catalog_id.as_str()) {
            return Err(format!(
                "Specialist output repeats catalog ID {}.",
                assessment.catalog_id
            ));
        }
    }
    if actual_ids != expected_ids {
        return Err(format!(
            "Specialist output for {category} does not cover exactly {} trusted catalog IDs.",
            expected_ids.len()
        ));
    }
    let mut ordered = assessments.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.catalog_id.cmp(&right.catalog_id));
    Ok((sha256_serializable(&ordered)?, ordered.len()))
}

fn open_strict_store(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!("Cannot create strict-analysis persistence directory: {error}")
        })?;
    }
    let mut connection = Connection::open(path)
        .map_err(|error| format!("Cannot open strict-analysis durable store: {error}"))?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| {
            format!("Cannot configure strict-analysis SQLite busy timeout: {error}")
        })?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
        .map_err(|error| {
            format!("Cannot enable durable strict-analysis SQLite settings: {error}")
        })?;
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| format!("Cannot read strict-analysis store schema version: {error}"))?;
    if version > STRICT_STORE_SCHEMA_VERSION {
        return Err(format!("Strict-analysis store schema {version} is newer than this application supports; refusing to mutate it."));
    }
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS strict_work_plans (
            work_plan_id TEXT PRIMARY KEY NOT NULL, issued_at_ms INTEGER NOT NULL, plan_json TEXT NOT NULL,
            plan_sha256 TEXT NOT NULL CHECK(length(plan_sha256)=64), consumed_at_ms INTEGER NULL
         );
         CREATE INDEX IF NOT EXISTS idx_strict_work_plans_issued ON strict_work_plans(issued_at_ms);
         CREATE INDEX IF NOT EXISTS idx_strict_work_plans_pending ON strict_work_plans(consumed_at_ms,issued_at_ms);
         CREATE TABLE IF NOT EXISTS strict_analysis_attempts (
            work_plan_id TEXT PRIMARY KEY NOT NULL, plan_sha256 TEXT NOT NULL CHECK(length(plan_sha256)=64),
            started_at_ms INTEGER NOT NULL, completed_at_ms INTEGER NULL,
            status TEXT NOT NULL CHECK(status IN ('in_progress','blocked','allow_research','allow_to_next_layer')),
            decision TEXT NULL, receipt_sha256 TEXT NULL, assessment_bundle_json TEXT NULL, result_json TEXT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_strict_analysis_attempts_status ON strict_analysis_attempts(status,started_at_ms);
         CREATE TABLE IF NOT EXISTS strict_audit_chain_events (
            seq INTEGER PRIMARY KEY NOT NULL CHECK(seq>0), event_type TEXT NOT NULL, work_plan_id TEXT NULL,
            occurred_at_ms INTEGER NOT NULL, payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256)=64),
            previous_hash TEXT NOT NULL CHECK(length(previous_hash)=64), event_sha256 TEXT NOT NULL CHECK(length(event_sha256)=64)
         );
         CREATE INDEX IF NOT EXISTS idx_strict_audit_chain_plan ON strict_audit_chain_events(work_plan_id,seq);
         CREATE TABLE IF NOT EXISTS strict_audit_chain_head (
            singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton=1), last_seq INTEGER NOT NULL CHECK(last_seq>=0),
            head_sha256 TEXT NOT NULL CHECK(length(head_sha256)=64)
         );
         CREATE TABLE IF NOT EXISTS strict_specialist_dispatch_receipts (
            work_plan_id TEXT NOT NULL, agent_id TEXT NOT NULL, category TEXT NOT NULL,
            task_input_sha256 TEXT NOT NULL CHECK(length(task_input_sha256)=64),
            output_assessment_sha256 TEXT NOT NULL CHECK(length(output_assessment_sha256)=64),
            covered_concept_count INTEGER NOT NULL CHECK(covered_concept_count > 0),
            completed_at_ms INTEGER NOT NULL,
            receipt_sha256 TEXT NOT NULL CHECK(length(receipt_sha256)=64),
            PRIMARY KEY(work_plan_id,agent_id), UNIQUE(work_plan_id,category)
         );
         CREATE INDEX IF NOT EXISTS idx_strict_dispatch_receipts_plan ON strict_specialist_dispatch_receipts(work_plan_id,category);
         CREATE TRIGGER IF NOT EXISTS strict_audit_chain_no_update BEFORE UPDATE ON strict_audit_chain_events
            BEGIN SELECT RAISE(ABORT,'strict audit-chain events are append-only'); END;
         CREATE TRIGGER IF NOT EXISTS strict_audit_chain_no_delete BEFORE DELETE ON strict_audit_chain_events
            BEGIN SELECT RAISE(ABORT,'strict audit-chain events are append-only'); END;
         CREATE TRIGGER IF NOT EXISTS strict_dispatch_receipts_no_update BEFORE UPDATE ON strict_specialist_dispatch_receipts
            BEGIN SELECT RAISE(ABORT,'strict specialist receipts are append-only'); END;
         CREATE TRIGGER IF NOT EXISTS strict_dispatch_receipts_no_delete BEFORE DELETE ON strict_specialist_dispatch_receipts
            BEGIN SELECT RAISE(ABORT,'strict specialist receipts are append-only'); END;
         INSERT OR IGNORE INTO strict_audit_chain_head(singleton,last_seq,head_sha256) VALUES(1,0,'0000000000000000000000000000000000000000000000000000000000000000');"
    ).map_err(|error| format!("Cannot initialize strict-analysis durable schema: {error}"))?;

    if version < 2 {
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Cannot begin strict-store audit migration: {error}"))?;
        let plans = {
            let mut statement = tx.prepare("SELECT work_plan_id,issued_at_ms,plan_json,plan_sha256,consumed_at_ms FROM strict_work_plans ORDER BY issued_at_ms,work_plan_id")
                .map_err(|error| format!("Cannot prepare legacy work-plan migration: {error}"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                    ))
                })
                .map_err(|error| format!("Cannot scan legacy work plans: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Cannot decode legacy work plan: {error}"))?;
            rows
        };
        let attempts = {
            let mut statement = tx.prepare("SELECT work_plan_id,plan_sha256,started_at_ms,completed_at_ms,status,decision,receipt_sha256,assessment_bundle_json,result_json FROM strict_analysis_attempts ORDER BY started_at_ms,work_plan_id")
                .map_err(|error| format!("Cannot prepare legacy attempt migration: {error}"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                })
                .map_err(|error| format!("Cannot scan legacy attempts: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Cannot decode legacy attempt: {error}"))?;
            rows
        };
        let mut imported = 0usize;
        for (id, issued_at, plan_json, plan_sha, _) in &plans {
            if sha256_bytes(plan_json.as_bytes()) != *plan_sha {
                return Err(format!(
                    "Legacy plan {id} digest mismatch; refusing schema migration."
                ));
            }
            append_audit_event_raw(&tx, "work_plan_issued", Some(id), *issued_at, plan_sha)?;
            imported += 1;
        }
        for (id, _, _, plan_sha, consumed_at) in &plans {
            if let Some(at) = consumed_at {
                append_audit_event_raw(&tx, "work_plan_consumed", Some(id), *at, plan_sha)?;
                imported += 1;
            }
        }
        for (id, plan_sha, started, completed, status, decision, receipt, bundle, result) in
            &attempts
        {
            let matching_plan = plans.iter().find(|(plan_id, _, _, stored_sha, consumed)| {
                plan_id == id && stored_sha == plan_sha && consumed.is_some()
            });
            if matching_plan.is_none() {
                if plans.iter().any(|(plan_id, _, _, _, _)| plan_id == id) {
                    return Err(format!("Legacy attempt {id} references a present but unconsumed/mismatched plan; refusing schema migration."));
                }
                let digest = legacy_orphan_attempt_digest(&LegacyOrphanAttemptDigestMaterial {
                    work_plan_id: id.clone(),
                    plan_sha256: plan_sha.clone(),
                    started_at_ms: *started,
                    completed_at_ms: *completed,
                    status: status.clone(),
                    decision: decision.clone(),
                    receipt_sha256: receipt.clone(),
                    assessment_bundle_sha256: bundle
                        .as_deref()
                        .map(|value| sha256_bytes(value.as_bytes())),
                    result_sha256: result
                        .as_deref()
                        .map(|value| sha256_bytes(value.as_bytes())),
                })?;
                append_audit_event_raw(
                    &tx,
                    "legacy_orphan_attempt_imported",
                    Some(id),
                    completed.unwrap_or(*started),
                    &digest,
                )?;
                imported += 1;
                continue;
            }
            append_audit_event_raw(
                &tx,
                "assessment_attempt_started",
                Some(id),
                *started,
                plan_sha,
            )?;
            imported += 1;
            if let Some(at) = completed {
                let (Some(decision), Some(receipt), Some(bundle), Some(result)) = (
                    decision.as_deref(),
                    receipt.as_deref(),
                    bundle.as_deref(),
                    result.as_deref(),
                ) else {
                    return Err(format!(
                        "Legacy finalized attempt {id} is incomplete; refusing schema migration."
                    ));
                };
                let digest =
                    finalization_event_digest(id, status, decision, receipt, bundle, result)?;
                append_audit_event_raw(
                    &tx,
                    "assessment_attempt_finalized",
                    Some(id),
                    *at,
                    &digest,
                )?;
                imported += 1;
            } else if status != "in_progress"
                || decision.is_some()
                || receipt.is_some()
                || bundle.is_some()
                || result.is_some()
            {
                return Err(format!("Legacy in-progress attempt {id} has inconsistent fields; refusing schema migration."));
            }
        }
        if imported == 0 {
            append_audit_event_raw(
                &tx,
                "store_initialized",
                None,
                chrono::Utc::now().timestamp_millis(),
                &sha256_bytes(b"strict-store-schema-v2"),
            )?;
        } else {
            let migration_digest = sha256_bytes(
                format!(
                    "source_schema={version};plans={};attempts={};events={imported}",
                    plans.len(),
                    attempts.len()
                )
                .as_bytes(),
            );
            append_audit_event_raw(
                &tx,
                "schema_migrated",
                None,
                chrono::Utc::now().timestamp_millis(),
                &migration_digest,
            )?;
        }
        tx.pragma_update(None, "user_version", 2_i64)
            .map_err(|error| format!("Cannot set strict-store schema version 2: {error}"))?;
        tx.commit()
            .map_err(|error| format!("Cannot commit strict-store migration: {error}"))?;
    }
    if version < 3 {
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                format!("Cannot begin specialist-receipt schema migration: {error}")
            })?;
        verify_audit_chain_connection(&tx)?;
        let migration_digest =
            sha256_bytes(b"strict-store-schema-v2-to-v3-specialist-dispatch-receipts");
        append_audit_event_raw(
            &tx,
            "schema_migrated_v2_to_v3",
            None,
            chrono::Utc::now().timestamp_millis(),
            &migration_digest,
        )?;
        tx.pragma_update(None, "user_version", 3_i64)
            .map_err(|error| format!("Cannot set strict-store schema version 3: {error}"))?;
        verify_audit_chain_connection(&tx)?;
        tx.commit().map_err(|error| {
            format!("Cannot commit specialist-receipt schema migration: {error}")
        })?;
    }
    verify_audit_chain_connection(&connection)?;
    Ok(connection)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// Verify the tamper-evident durable chain and its cross-links to all persisted plan/attempt rows.
pub fn verify_persisted_audit_chain(store_path: &Path) -> Result<StrictAuditChainStatus, String> {
    let connection = open_strict_store(store_path)?;
    let (event_count, last_sequence, head_sha256) = verify_audit_chain_connection(&connection)?;
    Ok(StrictAuditChainStatus {
        valid: true,
        event_count,
        last_sequence,
        head_sha256,
    })
}

/// Issue a work plan into a durable SQLite registry. Issuance, expiry cleanup and capacity
/// enforcement occur within one immediate transaction, so concurrent callers cannot overbook
/// the pending-plan limit or create plans whose replay state was never committed.
pub fn create_work_plan_persisted(
    request: CreateWorkPlanRequest,
    store_path: &Path,
) -> Result<StrictAnalysisWorkPlan, String> {
    let plan = build_work_plan(request)?;
    let plan_json = serde_json::to_string(&plan).map_err(|error| {
        format!("Cannot serialize strict work plan for durable issuance: {error}")
    })?;
    let plan_digest = sha256_bytes(plan_json.as_bytes());
    let now = chrono::Utc::now().timestamp_millis();
    let expires_before = now.saturating_sub(WORK_PLAN_TTL_MS);
    let mut connection = open_strict_store(store_path)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot begin atomic strict work-plan issuance: {error}"))?;
    verify_audit_chain_connection(&tx)?;
    let pending: i64 = tx.query_row(
        "SELECT COUNT(*) FROM strict_work_plans WHERE consumed_at_ms IS NULL AND issued_at_ms >= ?1",
        params![expires_before], |row| row.get(0),
    ).map_err(|error| format!("Cannot count pending strict work plans: {error}"))?;
    if pending >= MAX_ISSUED_WORK_PLANS as i64 {
        return Err("Strict analysis work-plan capacity reached; expire or complete pending plans before creating another.".into());
    }
    tx.execute(
        "INSERT INTO strict_work_plans(work_plan_id, issued_at_ms, plan_json, plan_sha256, consumed_at_ms) VALUES(?1, ?2, ?3, ?4, NULL)",
        params![plan.work_plan_id, now, plan_json, plan_digest],
    ).map_err(|error| format!("Cannot durably register strict work plan: {error}"))?;
    append_audit_event_raw(
        &tx,
        "work_plan_issued",
        Some(&plan.work_plan_id),
        now,
        &plan_digest,
    )?;
    verify_audit_chain_connection(&tx)?;
    tx.commit()
        .map_err(|error| format!("Cannot commit strict work-plan issuance: {error}"))?;
    Ok(plan)
}

/// Persist a receipt only inside native code after a real specialist call and strict output validation.
/// It is not exposed as a Tauri command; the current build has no configured provider dispatcher.
#[allow(dead_code)]
pub(crate) fn persist_specialist_dispatch_receipt(
    plan: &StrictAnalysisWorkPlan,
    category: &str,
    assessments: &[StrictAssessment],
    store_path: &Path,
) -> Result<String, String> {
    let manifest = trusted_catalog()?;
    let agent_id = specialist_for_category(category)
        .ok_or_else(|| format!("No trusted specialist for dispatch category {category}."))?;
    let (output_assessment_sha256, covered_count) =
        category_assessment_output_sha256(category, assessments, manifest)?;
    let (task_input_sha256, expected_count) =
        specialist_task_input_sha256(plan, category, manifest)?;
    if covered_count != expected_count {
        return Err(format!("Specialist output covers {covered_count} items but the trusted category requires {expected_count}."));
    }
    let now = chrono::Utc::now().timestamp_millis();
    let mut connection = open_strict_store(store_path)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot begin specialist receipt transaction: {error}"))?;
    verify_audit_chain_connection(&tx)?;
    let row = tx.query_row(
        "SELECT issued_at_ms,plan_json,plan_sha256,consumed_at_ms FROM strict_work_plans WHERE work_plan_id=?1",
        params![plan.work_plan_id],
        |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,Option<i64>>(3)?)),
    ).optional().map_err(|error| format!("Cannot validate issued work plan for specialist receipt: {error}"))?;
    let Some((issued_at, stored_plan_json, stored_plan_sha, consumed_at)) = row else {
        return Err("Cannot persist specialist receipt for a non-issued work plan.".into());
    };
    let supplied_plan_json = serde_json::to_string(plan)
        .map_err(|error| format!("Cannot serialize supplied work plan: {error}"))?;
    if stored_plan_json != supplied_plan_json
        || sha256_bytes(stored_plan_json.as_bytes()) != stored_plan_sha
        || consumed_at.is_some()
        || now <= issued_at
        || now.saturating_sub(issued_at) > WORK_PLAN_TTL_MS
    {
        return Err(
            "Specialist receipt plan is modified, expired, consumed, or has invalid timing.".into(),
        );
    }
    let material = SpecialistDispatchReceiptMaterial {
        work_plan_id: plan.work_plan_id.clone(),
        agent_id: agent_id.into(),
        category: category.into(),
        task_input_sha256,
        output_assessment_sha256,
        covered_concept_count: covered_count,
        completed_at_ms: now,
    };
    let receipt_sha256 = sha256_serializable(&material)?;
    tx.execute(
        "INSERT INTO strict_specialist_dispatch_receipts(work_plan_id,agent_id,category,task_input_sha256,output_assessment_sha256,covered_concept_count,completed_at_ms,receipt_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![material.work_plan_id, material.agent_id, material.category, material.task_input_sha256, material.output_assessment_sha256, material.covered_concept_count as i64, material.completed_at_ms, receipt_sha256],
    ).map_err(|error| format!("Cannot persist specialist execution receipt (duplicate or store constraint): {error}"))?;
    append_audit_event_raw(
        &tx,
        "specialist_receipt_recorded",
        Some(&plan.work_plan_id),
        now,
        &receipt_sha256,
    )?;
    verify_audit_chain_connection(&tx)?;
    tx.commit()
        .map_err(|error| format!("Cannot commit specialist execution receipt: {error}"))?;
    Ok(receipt_sha256)
}

fn specialist_dispatch_receipt_gaps(
    plan: &StrictAnalysisWorkPlan,
    assessments: &[StrictAssessment],
    store_path: &Path,
) -> Result<Vec<String>, String> {
    let manifest = trusted_catalog()?;
    let connection = open_strict_store(store_path)?;
    let issued_at = connection
        .query_row(
            "SELECT issued_at_ms FROM strict_work_plans WHERE work_plan_id=?1",
            params![plan.work_plan_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| {
            format!("Cannot inspect plan issuance before dispatch validation: {error}")
        })?
        .ok_or_else(|| {
            "Specialist receipt validation requires a durable issued work plan.".to_string()
        })?;
    let now = chrono::Utc::now().timestamp_millis();
    let mut stored = HashMap::<String, (SpecialistDispatchReceiptMaterial, String)>::new();
    {
        let mut statement = connection.prepare(
            "SELECT work_plan_id,agent_id,category,task_input_sha256,output_assessment_sha256,covered_concept_count,completed_at_ms,receipt_sha256 FROM strict_specialist_dispatch_receipts WHERE work_plan_id=?1"
        ).map_err(|error| format!("Cannot prepare specialist receipt validation query: {error}"))?;
        let rows = statement
            .query_map(params![plan.work_plan_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(|error| format!("Cannot query specialist receipts: {error}"))?;
        for row in rows {
            let (
                plan_id,
                agent_id,
                category,
                task_input_sha256,
                output_assessment_sha256,
                count,
                completed_at_ms,
                receipt_sha256,
            ) = row.map_err(|error| format!("Cannot read specialist receipt: {error}"))?;
            if stored.contains_key(&agent_id) {
                return Err(format!(
                    "Duplicate durable specialist receipt for {agent_id}."
                ));
            }
            let material = SpecialistDispatchReceiptMaterial {
                work_plan_id: plan_id,
                agent_id: agent_id.clone(),
                category,
                task_input_sha256,
                output_assessment_sha256,
                covered_concept_count: count as usize,
                completed_at_ms,
            };
            if sha256_serializable(&material)? != receipt_sha256 {
                return Err(format!(
                    "Specialist receipt digest mismatch for {agent_id}."
                ));
            }
            stored.insert(agent_id, (material, receipt_sha256));
        }
    }
    let mut gaps = Vec::new();
    for category in &manifest.categories {
        let agent_id = specialist_for_category(category)
            .ok_or_else(|| format!("No specialist configured for category {category}."))?;
        let (expected_input_sha, expected_count) =
            specialist_task_input_sha256(plan, category, manifest)?;
        let category_assessments = assessments
            .iter()
            .filter(|assessment| {
                manifest
                    .entries
                    .iter()
                    .any(|entry| entry.id == assessment.catalog_id && entry.category == *category)
            })
            .cloned()
            .collect::<Vec<_>>();
        let output_result =
            category_assessment_output_sha256(category, &category_assessments, manifest);
        let expected_output_sha = output_result
            .as_ref()
            .map(|(digest, _)| digest.as_str())
            .unwrap_or("");
        match stored.get(agent_id) {
            None => gaps.push(format!(
                "Missing durable execution receipt for {agent_id} ({category})."
            )),
            Some((receipt, stored_receipt_sha)) => {
                let expected_receipt_sha = sha256_serializable(receipt)?;
                let timing_valid = receipt.completed_at_ms > issued_at
                    && receipt.completed_at_ms <= now
                    && now.saturating_sub(issued_at) <= WORK_PLAN_TTL_MS;
                if receipt.work_plan_id != plan.work_plan_id
                    || receipt.category != *category
                    || receipt.task_input_sha256 != expected_input_sha
                    || receipt.output_assessment_sha256 != expected_output_sha
                    || receipt.covered_concept_count != expected_count
                    || *stored_receipt_sha != expected_receipt_sha
                    || !timing_valid
                {
                    gaps.push(format!("Durable execution receipt mismatch for {agent_id} ({category}); receipt, input, output, coverage, or completion time does not match."));
                }
            }
        }
    }
    Ok(gaps)
}

#[cfg(test)]
fn persisted_work_plan_is_unmodified_and_unexpired(
    plan: &StrictAnalysisWorkPlan,
    store_path: &Path,
) -> Result<bool, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let connection = open_strict_store(store_path)?;
    let row = connection.query_row(
        "SELECT issued_at_ms, plan_json, plan_sha256, consumed_at_ms FROM strict_work_plans WHERE work_plan_id=?1",
        params![plan.work_plan_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<i64>>(3)?)),
    ).optional().map_err(|error| format!("Cannot read durable strict work plan: {error}"))?;
    let Some((issued_at, stored_json, stored_digest, consumed_at)) = row else {
        return Ok(false);
    };
    if now.saturating_sub(issued_at) > WORK_PLAN_TTL_MS || consumed_at.is_some() {
        return Ok(false);
    }
    let supplied = serde_json::to_string(plan)
        .map_err(|error| format!("Cannot serialize supplied strict work plan: {error}"))?;
    Ok(sha256_bytes(stored_json.as_bytes()) == stored_digest
        && supplied == stored_json
        && sha256_bytes(supplied.as_bytes()) == stored_digest)
}

/// Durable single-use consume. BEGIN IMMEDIATE serializes competing writers; the conditional
/// UPDATE plus affected-row check ensures at most one completion attempt can consume a plan,
/// including blocked attempts. The consumed marker survives process restarts.
fn consume_work_plan_once_persisted(
    plan: &StrictAnalysisWorkPlan,
    store_path: &Path,
) -> Result<bool, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let supplied_json = serde_json::to_string(plan)
        .map_err(|error| format!("Cannot serialize supplied strict work plan: {error}"))?;
    let supplied_digest = sha256_bytes(supplied_json.as_bytes());
    let mut connection = open_strict_store(store_path)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot begin atomic strict work-plan consume: {error}"))?;
    verify_audit_chain_connection(&tx)?;
    let row = tx.query_row(
        "SELECT issued_at_ms, plan_json, plan_sha256, consumed_at_ms FROM strict_work_plans WHERE work_plan_id=?1",
        params![plan.work_plan_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<i64>>(3)?)),
    ).optional().map_err(|error| format!("Cannot verify durable strict work plan before consume: {error}"))?;
    let Some((issued_at, stored_json, stored_digest, consumed_at)) = row else {
        tx.commit().map_err(|error| {
            format!("Cannot close strict work-plan consume transaction: {error}")
        })?;
        return Ok(false);
    };
    if now.saturating_sub(issued_at) > WORK_PLAN_TTL_MS
        || consumed_at.is_some()
        || sha256_bytes(stored_json.as_bytes()) != stored_digest
        || stored_json != supplied_json
        || stored_digest != supplied_digest
    {
        tx.commit().map_err(|error| {
            format!("Cannot close rejected strict work-plan consume transaction: {error}")
        })?;
        return Ok(false);
    }
    let changed = tx.execute(
        "UPDATE strict_work_plans SET consumed_at_ms=?1 WHERE work_plan_id=?2 AND consumed_at_ms IS NULL AND issued_at_ms=?3 AND plan_sha256=?4",
        params![now, plan.work_plan_id, issued_at, stored_digest],
    ).map_err(|error| format!("Cannot atomically mark strict work plan consumed: {error}"))?;
    if changed != 1 {
        tx.commit().map_err(|error| {
            format!("Cannot close conflicting strict work-plan consume transaction: {error}")
        })?;
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO strict_analysis_attempts(work_plan_id, plan_sha256, started_at_ms, completed_at_ms, status, decision, receipt_sha256, assessment_bundle_json, result_json) VALUES(?1, ?2, ?3, NULL, 'in_progress', NULL, NULL, NULL, NULL)",
        params![plan.work_plan_id, stored_digest, now],
    ).map_err(|error| format!("Cannot record strict-analysis attempt before evaluation: {error}"))?;
    append_audit_event_raw(
        &tx,
        "work_plan_consumed",
        Some(&plan.work_plan_id),
        now,
        &stored_digest,
    )?;
    append_audit_event_raw(
        &tx,
        "assessment_attempt_started",
        Some(&plan.work_plan_id),
        now,
        &stored_digest,
    )?;
    verify_audit_chain_connection(&tx)?;
    tx.commit()
        .map_err(|error| format!("Cannot commit strict work-plan consumption: {error}"))?;
    Ok(true)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StrictAssessment {
    pub contract_version: String,
    pub master_list_version: String,
    pub catalog_id: String,
    pub category: String,
    pub group: String,
    pub name: String,
    pub agent_id: String,
    pub status: String,
    pub as_of: String,
    pub evidence_refs: Vec<String>,
    pub result: Option<Value>,
    pub reason: String,
    pub look_ahead_safe: bool,
    pub implementation_version: String,
    pub data_snapshot_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StrictMarketDataContext {
    pub validated: bool,
    pub source: String,
    pub as_of: String,
    pub dataset_sha256: String,
    pub symbol: String,
    pub timeframe: String,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StrictRiskApproval {
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCompletionRequest {
    pub work_plan: StrictAnalysisWorkPlan,
    pub assessments: Vec<StrictAssessment>,
    pub market_data: StrictMarketDataContext,
    pub risk: StrictRiskApproval,
    pub intent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCategoryCoverage {
    pub required: usize,
    pub received: usize,
    pub evaluated: usize,
    pub unsupported: usize,
    pub insufficient_data: usize,
    pub not_applicable: usize,
    pub missing: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCoverage {
    pub required: usize,
    pub received: usize,
    pub assessed: usize,
    pub missing: usize,
    pub duplicate: usize,
    pub malformed: usize,
    pub unsupported: usize,
    pub insufficient_data: usize,
    pub not_applicable: usize,
    pub evaluated: usize,
    pub catalog_only: usize,
    pub by_category: BTreeMap<String, StrictCategoryCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictNextLayerPayload {
    pub contract_version: String,
    pub assessment_contract_version: String,
    pub master_list_version: String,
    pub assessment_coverage: String,
    pub symbol: String,
    pub timeframe: String,
    pub market_data_source: String,
    pub market_data_as_of: String,
    pub dataset_sha256: String,
    pub risk_approved: bool,
    pub intent: String,
    pub live_execution_authorized: bool,
}

/// Reproducibility receipt for blocked and successful analysis attempts. This is a
/// deterministic integrity fingerprint, not a digital signature or proof of agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictAuditReceipt {
    pub schema_version: String,
    pub work_plan_id: String,
    pub master_list_version: String,
    pub symbol: String,
    pub timeframe: String,
    pub as_of: String,
    pub dataset_sha256: String,
    pub assessment_bundle_sha256: String,
    pub category_assessment_sha256: BTreeMap<String, String>,
    pub coverage_required: usize,
    pub coverage_received: usize,
    pub coverage_assessed: usize,
    pub coverage_missing: usize,
    pub coverage_duplicate: usize,
    pub coverage_malformed: usize,
    pub coverage_unsupported: usize,
    pub coverage_insufficient_data: usize,
    pub coverage_not_applicable: usize,
    pub coverage_evaluated: usize,
    pub coverage_catalog_only: usize,
    pub decision: String,
    pub reasons_sha256: String,
    pub generated_at: String,
    /// SHA-256 over this receipt serialized with this field set to an empty string.
    pub receipt_sha256: String,
    pub live_execution_authorized: bool,
}

/// Validate the self-integrity and structural invariants of a receipt received from another
/// process or persisted store. This detects accidental/tampered field changes but does not
/// authenticate the issuer; SHA-256 alone is not a digital signature.
pub fn verify_audit_receipt(receipt: &StrictAuditReceipt) -> Result<(), String> {
    if receipt.schema_version != "strict-analysis-audit.v1" {
        return Err("Unsupported strict-analysis audit receipt schema.".into());
    }
    if receipt.work_plan_id.trim().is_empty()
        || receipt.symbol.trim().is_empty()
        || receipt.timeframe.trim().is_empty()
    {
        return Err("Audit receipt is missing required identity fields.".into());
    }
    if receipt.master_list_version != MASTER_LIST_VERSION
        || receipt.coverage_required != REQUIRED_CONCEPT_COUNT
    {
        return Err(
            "Audit receipt catalog version or required coverage does not match this build.".into(),
        );
    }
    if !valid_timestamp(&receipt.as_of) || !valid_timestamp(&receipt.generated_at) {
        return Err("Audit receipt contains an invalid timestamp.".into());
    }
    for (field, value) in [
        ("datasetSha256", receipt.dataset_sha256.as_str()),
        (
            "assessmentBundleSha256",
            receipt.assessment_bundle_sha256.as_str(),
        ),
        ("reasonsSha256", receipt.reasons_sha256.as_str()),
        ("receiptSha256", receipt.receipt_sha256.as_str()),
    ] {
        if !valid_sha256(value) {
            return Err(format!(
                "Audit receipt {field} is not a SHA-256 hex digest."
            ));
        }
    }
    if receipt.category_assessment_sha256.len() != 9
        || receipt
            .category_assessment_sha256
            .values()
            .any(|hash| !valid_sha256(hash))
    {
        return Err(
            "Audit receipt must contain valid digests for all nine specialist categories.".into(),
        );
    }
    if receipt.live_execution_authorized {
        return Err("Audit receipt cannot authorize live execution.".into());
    }
    if receipt.coverage_assessed > receipt.coverage_required
        || receipt.coverage_missing > receipt.coverage_required
        || receipt.coverage_evaluated > receipt.coverage_assessed
        || receipt.coverage_catalog_only > receipt.coverage_assessed
    {
        return Err("Audit receipt coverage counters violate structural invariants.".into());
    }
    let mut material = receipt.clone();
    let expected = material.receipt_sha256.clone();
    material.receipt_sha256.clear();
    let actual = sha256_serializable(&material)?;
    if actual != expected.to_ascii_lowercase() {
        return Err("Audit receipt self-hash mismatch; receipt may have been modified.".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrictCompletionResult {
    pub allowed: bool,
    pub decision: String,
    pub reasons: Vec<String>,
    pub coverage: StrictCoverage,
    pub audit_receipt: StrictAuditReceipt,
    pub next_layer_payload: Option<StrictNextLayerPayload>,
    pub live_execution_authorized: bool,
}

fn valid_timestamp(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value).is_ok()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn timeframe_duration_ms(value: &str) -> Option<i64> {
    match value.trim() {
        "1m" => Some(60_000),
        "5m" => Some(300_000),
        "15m" => Some(900_000),
        "1h" => Some(3_600_000),
        "4h" => Some(14_400_000),
        "1d" => Some(86_400_000),
        _ => None,
    }
}

fn meaningful_reason(value: &str) -> bool {
    value.trim().chars().count() >= 12
}
fn canonical_evidence_ref_matches_snapshot(reference: &str, snapshot_sha256: &str) -> bool {
    let Some(rest) = reference.strip_prefix("snapshot:") else {
        return false;
    };
    let Some((bound_snapshot, source_and_locator)) = rest.split_once(':') else {
        return false;
    };
    if !valid_sha256(bound_snapshot)
        || !bound_snapshot.eq_ignore_ascii_case(snapshot_sha256)
    {
        return false;
    }
    let Some((source, locator)) = source_and_locator.split_once(':') else {
        return false;
    };
    let valid_token = |value: &str| {
        !value.is_empty()
            && value.split('/').all(|segment| !segment.is_empty() && !segment.starts_with('.'))
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b'~')
            })
    };
    matches!(
        source,
        "binance" | "mt5" | "rust-indicator" | "rust-structure" | "backtest"
    ) && valid_token(locator)
        && !locator.contains("..")
}

fn has_auditable_refs(refs: &[String], snapshot_sha256: &str) -> bool {
    !refs.is_empty()
        && refs.len() <= MAX_EVIDENCE_REFS_PER_ASSESSMENT
        && refs.iter().all(|item| {
            item.len() <= MAX_EVIDENCE_REF_LENGTH
                && item.trim() == item
                && canonical_evidence_ref_matches_snapshot(item, snapshot_sha256)
        })
        && refs.iter().collect::<BTreeSet<_>>().len() == refs.len()
}

fn evidence_ref_resolves_to_market_snapshot(
    reference: &str,
    market: &StrictMarketDataContext,
) -> bool {
    let Some(rest) = reference.strip_prefix("snapshot:") else {
        return false;
    };
    let Some((snapshot_digest, source_and_locator)) = rest.split_once(':') else {
        return false;
    };
    let Some((source, locator)) = source_and_locator.split_once(':') else {
        return false;
    };
    if source != "binance"
        || !valid_sha256(snapshot_digest)
        || !snapshot_digest.eq_ignore_ascii_case(&market.dataset_sha256)
        || !market.source.starts_with("Binance public REST")
    {
        return false;
    }
    let mut segments = locator.split('/');
    let (Some(symbol), Some(timeframe), Some(timestamp), None) =
        (segments.next(), segments.next(), segments.next(), segments.next())
    else {
        return false;
    };
    if !symbol.eq_ignore_ascii_case(&market.symbol) || timeframe != market.timeframe {
        return false;
    }
    let Ok(timestamp) = timestamp.parse::<i64>() else {
        return false;
    };
    market.candles.iter().any(|candle| candle.time == timestamp)
}

fn has_registered_evidence_refs(refs: &[String], market: &StrictMarketDataContext) -> bool {
    !refs.is_empty()
        && refs.len() <= MAX_EVIDENCE_REFS_PER_ASSESSMENT
        && refs.iter().all(|item| {
            item.len() <= MAX_EVIDENCE_REF_LENGTH
                && item.trim() == item
                && canonical_evidence_ref_matches_snapshot(item, &market.dataset_sha256)
                && evidence_ref_resolves_to_market_snapshot(item, market)
        })
        && refs.iter().collect::<BTreeSet<_>>().len() == refs.len()
}

fn sha256_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("Cannot serialize strict audit material: {error}"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

fn assessment_bundle_sha256(assessments: &[StrictAssessment]) -> Result<String, String> {
    let mut ordered = assessments.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.catalog_id.cmp(&right.catalog_id));
    sha256_serializable(&ordered)
}

fn category_assessment_sha256(
    categories: &[String],
    assessments: &[StrictAssessment],
    trusted_by_id: &HashMap<&str, &StrictCatalogEntry>,
) -> Result<BTreeMap<String, String>, String> {
    let mut digests = BTreeMap::new();
    for category in categories {
        let mut ordered = assessments
            .iter()
            .filter(|assessment| {
                trusted_by_id
                    .get(assessment.catalog_id.as_str())
                    .is_some_and(|entry| entry.category.as_str() == category.as_str())
            })
            .collect::<Vec<_>>();
        ordered.sort_by(|left, right| left.catalog_id.cmp(&right.catalog_id));
        digests.insert(category.clone(), sha256_serializable(&ordered)?);
    }
    Ok(digests)
}

fn validate_work_plan(
    plan: &StrictAnalysisWorkPlan,
    manifest: &StrictCatalogManifest,
    reasons: &mut Vec<String>,
) {
    if plan.work_plan_id.trim().is_empty() {
        reasons.push("Rust-issued work-plan ID is missing.".into());
    }
    if plan.schema_version != WORK_PLAN_VERSION {
        reasons.push("Missing or unsupported strict work-plan version.".into());
    }
    if plan.decision_gate_contract_version != DECISION_GATE_CONTRACT_VERSION {
        reasons.push("Work-plan decision-gate contract version mismatch.".into());
    }
    if plan.assessment_contract_version != ASSESSMENT_CONTRACT_VERSION {
        reasons.push("Work-plan assessment contract version mismatch.".into());
    }
    if plan.master_list_version != MASTER_LIST_VERSION {
        reasons.push("Work-plan master-list version mismatch.".into());
    }
    if plan.symbol.trim().is_empty() || plan.timeframe.trim().is_empty() {
        reasons.push("Work-plan symbol/timeframe scope is missing.".into());
    }
    if timeframe_duration_ms(&plan.timeframe).is_none() {
        reasons.push("Work-plan timeframe is not a supported canonical interval.".into());
    }
    if !valid_timestamp(&plan.as_of) {
        reasons.push("Work-plan timestamp is missing or invalid.".into());
    }
    if !valid_sha256(&plan.dataset_sha256) {
        reasons.push("Work-plan dataset SHA-256 is missing or invalid.".into());
    }
    if plan.expected_assessment_count != REQUIRED_CONCEPT_COUNT {
        reasons.push("Work plan must require exactly 364 assessments.".into());
    }
    if plan.inventory_identity != "rust-embedded-code-owned-master-list" {
        reasons.push("Work-plan inventory identity is not the trusted Rust registry.".into());
    }
    if plan.live_execution_authorized {
        reasons.push("Strict analysis cannot authorize live execution.".into());
    }
    if plan.entries != manifest.entries {
        reasons.push(
            "Work-plan inventory identity/status differs from the Rust-owned master list.".into(),
        );
    }

    let mut expected_tasks = Vec::new();
    for category in &manifest.categories {
        let ids = manifest
            .entries
            .iter()
            .filter(|entry| &entry.category == category)
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        if let (Some(agent_id), Some(knowledge_pack_path)) = (
            specialist_for_category(category),
            knowledge_pack_for_category(category),
        ) {
            expected_tasks.push(StrictAnalysisTask {
                agent_id: agent_id.into(),
                category: category.clone(),
                knowledge_pack_path: knowledge_pack_path.into(),
                required_assessment_count: ids.len(),
                catalog_ids: ids,
            });
        }
    }
    let actual = serde_json::to_value(&plan.tasks).ok();
    let expected = serde_json::to_value(&expected_tasks).ok();
    if actual != expected {
        reasons.push(
            "Work-plan specialist routing differs from the code-owned category assignment.".into(),
        );
    }
    if plan.dispatch_status != "work_plan_created_not_dispatched" {
        reasons.push("Work-plan dispatch status is invalid; this service does not pretend to dispatch agents.".into());
    }
}

fn validate_candle_snapshot(
    candles: &[Candle],
    as_of: &str,
    timeframe: &str,
    expected_sha256: &str,
) -> Result<(), String> {
    if candles.len() < 2 {
        return Err("Strict market-data snapshot must contain at least two candles.".into());
    }
    if candles.len() > 2_000_000 {
        return Err(
            "Strict market-data snapshot exceeds the 2,000,000-candle safety limit.".into(),
        );
    }
    let interval_ms = timeframe_duration_ms(timeframe)
        .ok_or_else(|| "Strict market-data timeframe is unsupported.".to_string())?;
    let as_of_ms = DateTime::parse_from_rfc3339(as_of)
        .map_err(|_| "Strict market-data asOf must be a valid RFC3339 timestamp.".to_string())?
        .timestamp_millis();
    let mut previous_time = i64::MIN;
    for (index, candle) in candles.iter().enumerate() {
        if candle.time <= 0 || candle.time <= previous_time {
            return Err(format!(
                "Strict market-data timestamps must be positive and strictly increasing (invalid row {index})."
            ));
        }
        if candle.time > as_of_ms {
            return Err(format!("Strict market-data candle at row {index} occurs after the asOf cutoff; look-ahead is forbidden."));
        }
        if candle.time.saturating_add(interval_ms) > as_of_ms {
            return Err(format!("Strict market-data candle at row {index} was not fully closed at the asOf cutoff; incomplete-bar look-ahead is forbidden."));
        }
        if !candle.open.is_finite()
            || !candle.high.is_finite()
            || !candle.low.is_finite()
            || !candle.close.is_finite()
            || !candle.volume.is_finite()
            || candle.open <= 0.0
            || candle.high <= 0.0
            || candle.low <= 0.0
            || candle.close <= 0.0
            || candle.volume < 0.0
            || candle.high < candle.open.max(candle.close).max(candle.low)
            || candle.low > candle.open.min(candle.close).min(candle.high)
        {
            return Err(format!(
                "Strict market-data OHLCV invariants failed at row {index}."
            ));
        }
        previous_time = candle.time;
    }
    let computed = dataset_sha256(candles);
    if !valid_sha256(expected_sha256) || computed != expected_sha256.to_ascii_lowercase() {
        return Err(
            "Strict market-data fingerprint does not match the supplied OHLCV snapshot.".into(),
        );
    }
    Ok(())
}

pub fn evaluate_completion(
    request: StrictCompletionRequest,
) -> Result<StrictCompletionResult, String> {
    evaluate_completion_with_consumer(request, consume_work_plan_once, &[])
}

/// Production completion path: replay protection is persisted in SQLite and survives process
/// restart. If the durable store cannot be opened or committed, the gate fails closed.
pub fn evaluate_completion_persisted(
    request: StrictCompletionRequest,
    store_path: &Path,
) -> Result<StrictCompletionResult, String> {
    let submitted_assessments_json =
        serde_json::to_string(&request.assessments).map_err(|error| {
            format!(
                "Cannot serialize submitted strict assessment bundle for durable audit: {error}"
            )
        })?;
    let dispatch_gaps =
        specialist_dispatch_receipt_gaps(&request.work_plan, &request.assessments, store_path)?;
    let result = evaluate_completion_with_consumer(
        request,
        |plan| consume_work_plan_once_persisted(plan, store_path),
        &dispatch_gaps,
    )?;
    if result.reasons.iter().any(|reason| {
        reason.contains("Work plan is unknown, modified, expired, or already consumed")
    }) {
        return Err("Strict work plan is unknown, modified, expired, or already consumed; issue a fresh work plan before retrying.".into());
    }
    // Returning a completion response requires the durable attempt/audit record to be
    // committed. If persistence fails, this command returns an error and no handoff payload.
    persist_completion_result(&result, &submitted_assessments_json, store_path)?;
    Ok(result)
}

fn persist_completion_result(
    result: &StrictCompletionResult,
    assessments_json: &str,
    store_path: &Path,
) -> Result<(), String> {
    verify_audit_receipt(&result.audit_receipt)?;
    if result.live_execution_authorized || result.audit_receipt.live_execution_authorized {
        return Err(
            "Strict audit persistence rejected an attempt that claims live-execution authority."
                .into(),
        );
    }
    let result_json = serde_json::to_string(result).map_err(|error| {
        format!("Cannot serialize strict completion result for durable audit: {error}")
    })?;
    let receipt_digest = result.audit_receipt.receipt_sha256.clone();
    let work_plan_id = result.audit_receipt.work_plan_id.clone();
    let status = match result.decision.as_str() {
        "BLOCK" => "blocked",
        "ALLOW_RESEARCH" => "allow_research",
        "ALLOW_TO_NEXT_LAYER" => "allow_to_next_layer",
        _ => return Err("Strict audit persistence rejected an unknown decision state.".into()),
    };
    let now = chrono::Utc::now().timestamp_millis();
    let mut connection = open_strict_store(store_path)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Cannot begin strict completion audit transaction: {error}"))?;
    verify_audit_chain_connection(&tx)?;
    let changed = tx.execute(
        "UPDATE strict_analysis_attempts SET completed_at_ms=?1, status=?2, decision=?3, receipt_sha256=?4, assessment_bundle_json=?5, result_json=?6 WHERE work_plan_id=?7 AND status='in_progress' AND result_json IS NULL",
        params![now, status, result.decision, receipt_digest, assessments_json, result_json, work_plan_id],
    ).map_err(|error| format!("Cannot persist strict completion audit result: {error}"))?;
    if changed != 1 {
        return Err("No unique in-progress strict-analysis attempt exists for durable finalization; handoff is denied.".into());
    }
    let finalization_digest = finalization_event_digest(
        &work_plan_id,
        status,
        &result.decision,
        &receipt_digest,
        assessments_json,
        &result_json,
    )?;
    append_audit_event_raw(
        &tx,
        "assessment_attempt_finalized",
        Some(&work_plan_id),
        now,
        &finalization_digest,
    )?;
    verify_audit_chain_connection(&tx)?;
    tx.commit()
        .map_err(|error| format!("Cannot commit strict completion audit record: {error}"))?;
    Ok(())
}

fn evaluate_completion_with_consumer<F>(
    request: StrictCompletionRequest,
    consume: F,
    additional_reasons: &[String],
) -> Result<StrictCompletionResult, String>
where
    F: FnOnce(&StrictAnalysisWorkPlan) -> Result<bool, String>,
{
    let manifest = trusted_catalog()?;
    let mut reasons = additional_reasons.to_vec();
    validate_work_plan(&request.work_plan, manifest, &mut reasons);
    match consume(&request.work_plan) {
        Ok(true) => {},
        Ok(false) => reasons.push("Work plan is unknown, modified, expired, or already consumed; each plan permits exactly one completion attempt.".into()),
        Err(error) => reasons.push(error),
    }
    let plan = &request.work_plan;
    let market = &request.market_data;
    if let Err(error) = validate_candle_snapshot(
        &market.candles,
        &market.as_of,
        &market.timeframe,
        &market.dataset_sha256,
    ) {
        reasons.push(error);
    }
    if market.symbol != plan.symbol {
        reasons.push("Market-data symbol does not match the work-plan scope.".into());
    }
    if market.timeframe != plan.timeframe {
        reasons.push("Market-data timeframe does not match the work-plan scope.".into());
    }
    if !market.dataset_sha256.eq_ignore_ascii_case(&plan.dataset_sha256) {
        reasons.push("Market-data fingerprint does not match the work-plan snapshot.".into());
    }
    if market.as_of != plan.as_of {
        reasons
            .push("Market-data timestamp does not match the work-plan snapshot timestamp.".into());
    }

    let trusted_by_id = manifest
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<HashMap<_, _>>();
    let mut by_id: HashMap<&str, &StrictAssessment> = HashMap::new();
    let mut duplicate = 0usize;
    let mut malformed = 0usize;
    let mut coverage_by_category = BTreeMap::<String, StrictCategoryCoverage>::new();
    for entry in &manifest.entries {
        let row = coverage_by_category
            .entry(entry.category.clone())
            .or_default();
        row.required += 1;
    }
    for assessment in &request.assessments {
        let Some(trusted) = trusted_by_id.get(assessment.catalog_id.as_str()) else {
            malformed += 1;
            reasons.push("Assessment contains an unknown or malformed catalogId.".into());
            continue;
        };
        coverage_by_category
            .entry(trusted.category.clone())
            .or_default()
            .received += 1;
        if by_id
            .insert(assessment.catalog_id.as_str(), assessment)
            .is_some()
        {
            duplicate += 1;
            reasons.push(format!(
                "Duplicate assessment for {}.",
                assessment.catalog_id
            ));
        }
    }
    if request.assessments.len() != REQUIRED_CONCEPT_COUNT {
        reasons.push("Assessment count must equal exactly 364.".into());
    }

    let mut unsupported = 0usize;
    let mut insufficient_data = 0usize;
    let mut not_applicable = 0usize;
    let mut evaluated = 0usize;
    let mut catalog_only = 0usize;
    for trusted in &manifest.entries {
        let Some(assessment) = by_id.get(trusted.id.as_str()).copied() else {
            coverage_by_category
                .entry(trusted.category.clone())
                .or_default()
                .missing += 1;
            reasons.push(format!("Missing assessment: {}.", trusted.id));
            continue;
        };
        let expected_agent = specialist_for_category(&trusted.category).unwrap_or("");
        if assessment.contract_version != ASSESSMENT_CONTRACT_VERSION
            || assessment.master_list_version != MASTER_LIST_VERSION
            || assessment.catalog_id != trusted.id
            || assessment.category != trusted.category
            || assessment.group != trusted.group
            || assessment.name != trusted.name
            || assessment.agent_id != expected_agent
        {
            reasons.push(format!(
                "Assessment identity/contract/agent routing mismatch: {}.",
                trusted.id
            ));
        }
        if !valid_timestamp(&assessment.as_of) {
            reasons.push(format!(
                "Missing or invalid assessment timestamp for {}.",
                trusted.id
            ));
        }
        if assessment.as_of != plan.as_of {
            reasons.push(format!(
                "Assessment asOf does not match the exact work-plan snapshot cutoff: {}.",
                trusted.id
            ));
        }
        if ![
            "evaluated",
            "insufficient_data",
            "not_applicable",
            "unsupported",
        ]
        .contains(&assessment.status.as_str())
        {
            reasons.push(format!("Invalid assessment status for {}.", trusted.id));
            continue;
        }
        if trusted.availability == "Catalog only" {
            catalog_only += 1;
            if assessment.status != "unsupported" {
                reasons.push(format!(
                    "Catalog-only concept must remain unsupported until reviewed promotion: {}.",
                    trusted.id
                ));
            }
        }
        match assessment.status.as_str() {
            "evaluated" => {
                evaluated += 1;
                coverage_by_category
                    .entry(trusted.category.clone())
                    .or_default()
                    .evaluated += 1;
                if !has_registered_evidence_refs(&assessment.evidence_refs, &market) {
                    reasons.push(format!(
                        "Evaluated concept lacks evidence references: {}.",
                        trusted.id
                    ));
                }
                if assessment.result.is_none()
                    || assessment.result.as_ref().is_some_and(Value::is_null)
                {
                    reasons.push(format!(
                        "Evaluated concept lacks an explicit non-null result: {}.",
                        trusted.id
                    ));
                }
                if !assessment.look_ahead_safe {
                    reasons.push(format!(
                        "Evaluated concept lacks explicit look-ahead safety confirmation: {}.",
                        trusted.id
                    ));
                }
                match reviewed_implementation_version(&trusted.id) {
                    Some(expected_version) if assessment.implementation_version == expected_version => {},
                    Some(expected_version) => reasons.push(format!("Implementation version for {} must match the Rust-owned reviewed version {}.", trusted.id, expected_version)),
                    None => reasons.push(format!("No Rust-owned reviewed implementation is registered for evaluated concept {}.", trusted.id)),
                }
                if !valid_sha256(&assessment.data_snapshot_id) {
                    reasons.push(format!(
                        "Evaluated concept lacks valid SHA-256 data snapshot ID: {}.",
                        trusted.id
                    ));
                }
                if !assessment.data_snapshot_id.eq_ignore_ascii_case(&plan.dataset_sha256) {
                    reasons.push(format!(
                        "Assessment data snapshot does not match the work-plan snapshot: {}.",
                        trusted.id
                    ));
                }
            }
            "not_applicable" => {
                not_applicable += 1;
                coverage_by_category
                    .entry(trusted.category.clone())
                    .or_default()
                    .not_applicable += 1;
                if !meaningful_reason(&assessment.reason) {
                    reasons.push(format!(
                        "Not-applicable status requires a meaningful rationale: {}.",
                        trusted.id
                    ));
                }
                if !has_auditable_refs(&assessment.evidence_refs, &market.dataset_sha256) {
                    reasons.push(format!(
                        "Not-applicable status requires auditable contextual evidence: {}.",
                        trusted.id
                    ));
                }
            }
            "insufficient_data" => {
                insufficient_data += 1;
                coverage_by_category
                    .entry(trusted.category.clone())
                    .or_default()
                    .insufficient_data += 1;
                if !meaningful_reason(&assessment.reason) {
                    reasons.push(format!(
                        "Insufficient-data status requires a reason: {}.",
                        trusted.id
                    ));
                }
                reasons.push(format!(
                    "Insufficient data blocks downstream decisions: {}.",
                    trusted.id
                ));
            }
            "unsupported" => {
                unsupported += 1;
                coverage_by_category
                    .entry(trusted.category.clone())
                    .or_default()
                    .unsupported += 1;
                if !meaningful_reason(&assessment.reason) {
                    reasons.push(format!(
                        "Unsupported status requires a reason: {}.",
                        trusted.id
                    ));
                }
                reasons.push(format!(
                    "Unsupported concept blocks downstream decisions: {}.",
                    trusted.id
                ));
            }
            _ => unreachable!("status was validated above"),
        }
    }

    if !market.validated {
        reasons.push("Market data has not passed the validated-data contract.".into());
    }
    if !market.source.starts_with("Binance public REST") {
        reasons.push("Strict completion requires trusted Rust-owned Binance public REST provenance.".into());
    }
    if !valid_timestamp(&market.as_of) {
        reasons.push("Market-data timestamp is missing or invalid.".into());
    }
    if !valid_sha256(&market.dataset_sha256) {
        reasons.push("Market-data SHA-256 snapshot fingerprint is missing or invalid.".into());
    }
    if market.symbol.trim().is_empty() {
        reasons.push("Market-data symbol/instrument scope is missing.".into());
    }
    if market.timeframe.trim().is_empty() {
        reasons.push("Market-data timeframe scope is missing.".into());
    }
    if !request.risk.passed {
        reasons.push(if request.risk.reason.trim().is_empty() {
            "Risk gateway did not approve the handoff.".into()
        } else {
            format!(
                "Risk gateway did not approve the handoff: {}",
                request.risk.reason
            )
        });
    }
    if request.intent != "research" && request.intent != "paper_trade" {
        reasons.push("Intent must be explicitly research or paper_trade.".into());
    }

    reasons.sort();
    reasons.dedup();
    let assessed = by_id.len();
    let missing = REQUIRED_CONCEPT_COUNT.saturating_sub(assessed);
    let allowed = reasons.is_empty()
        && assessed == REQUIRED_CONCEPT_COUNT
        && catalog_only == 0
        && unsupported == 0
        && insufficient_data == 0;
    let decision = if !allowed {
        "BLOCK"
    } else if request.intent == "research" {
        "ALLOW_RESEARCH"
    } else {
        "ALLOW_TO_NEXT_LAYER"
    };
    let next_layer_payload = if allowed {
        Some(StrictNextLayerPayload {
            contract_version: DECISION_GATE_CONTRACT_VERSION.into(),
            assessment_contract_version: ASSESSMENT_CONTRACT_VERSION.into(),
            master_list_version: MASTER_LIST_VERSION.into(),
            assessment_coverage: "364/364".into(),
            symbol: plan.symbol.clone(),
            timeframe: plan.timeframe.clone(),
            market_data_source: market.source.clone(),
            market_data_as_of: market.as_of.clone(),
            dataset_sha256: market.dataset_sha256.to_ascii_lowercase(),
            risk_approved: true,
            intent: request.intent,
            live_execution_authorized: false,
        })
    } else {
        None
    };
    let coverage = StrictCoverage {
        required: REQUIRED_CONCEPT_COUNT,
        received: request.assessments.len(),
        assessed,
        missing,
        duplicate,
        malformed,
        unsupported,
        insufficient_data,
        not_applicable,
        evaluated,
        catalog_only,
        by_category: coverage_by_category,
    };
    let category_digests =
        category_assessment_sha256(&manifest.categories, &request.assessments, &trusted_by_id)?;
    let mut audit_receipt = StrictAuditReceipt {
        schema_version: "strict-analysis-audit.v1".into(),
        work_plan_id: plan.work_plan_id.clone(),
        master_list_version: MASTER_LIST_VERSION.into(),
        symbol: plan.symbol.clone(),
        timeframe: plan.timeframe.clone(),
        as_of: plan.as_of.clone(),
        dataset_sha256: plan.dataset_sha256.clone(),
        assessment_bundle_sha256: assessment_bundle_sha256(&request.assessments)?,
        category_assessment_sha256: category_digests,
        coverage_required: coverage.required,
        coverage_received: coverage.received,
        coverage_assessed: coverage.assessed,
        coverage_missing: coverage.missing,
        coverage_duplicate: coverage.duplicate,
        coverage_malformed: coverage.malformed,
        coverage_unsupported: coverage.unsupported,
        coverage_insufficient_data: coverage.insufficient_data,
        coverage_not_applicable: coverage.not_applicable,
        coverage_evaluated: coverage.evaluated,
        coverage_catalog_only: coverage.catalog_only,
        decision: decision.into(),
        reasons_sha256: sha256_serializable(&reasons)?,
        generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        receipt_sha256: String::new(),
        live_execution_authorized: false,
    };
    audit_receipt.receipt_sha256 = sha256_serializable(&audit_receipt)?;
    verify_audit_receipt(&audit_receipt)?;
    Ok(StrictCompletionResult {
        allowed,
        decision: decision.into(),
        reasons,
        coverage,
        audit_receipt,
        next_layer_payload,
        live_execution_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AS_OF: &str = "2026-10-27T08:00:00.000Z";
    fn sample_candles() -> Vec<Candle> {
        vec![
            Candle {
                time: 1_700_000_000_000,
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 10.0,
            },
            Candle {
                time: 1_700_003_600_000,
                open: 101.0,
                high: 104.0,
                low: 100.0,
                close: 103.0,
                volume: 12.0,
            },
            Candle {
                time: 1_700_007_200_000,
                open: 103.0,
                high: 105.0,
                low: 102.0,
                close: 104.0,
                volume: 14.0,
            },
        ]
    }

    fn sample_sha() -> String {
        dataset_sha256(&sample_candles())
    }

    fn sample_plan() -> StrictAnalysisWorkPlan {
        create_work_plan(CreateWorkPlanRequest {
            symbol: "BTC-USD".into(),
            timeframe: "1h".into(),
            as_of: AS_OF.into(),
            dataset_sha256: sample_sha(),
        })
        .unwrap()
    }

    fn assessments_for(plan: &StrictAnalysisWorkPlan) -> Vec<StrictAssessment> {
        plan.entries
            .iter()
            .map(|entry| StrictAssessment {
                contract_version: ASSESSMENT_CONTRACT_VERSION.into(),
                master_list_version: MASTER_LIST_VERSION.into(),
                catalog_id: entry.id.clone(),
                category: entry.category.clone(),
                group: entry.group.clone(),
                name: entry.name.clone(),
                agent_id: specialist_for_category(&entry.category).unwrap().into(),
                status: if entry.availability == "Catalog only" {
                    "unsupported".into()
                } else {
                    "evaluated".into()
                },
                as_of: AS_OF.into(),
                evidence_refs: if entry.availability == "Catalog only" {
                    Vec::new()
                } else {
                    vec![format!("snapshot:{}:binance:{}/{}/{}", plan.dataset_sha256, plan.symbol, plan.timeframe, 1_700_000_000_000_i64)]
                },
                result: if entry.availability == "Catalog only" {
                    None
                } else {
                    Some(serde_json::json!({"fixtureOnly": true}))
                },
                reason: if entry.availability == "Catalog only" {
                    "No reviewed implementation exists for this concept.".into()
                } else {
                    String::new()
                },
                look_ahead_safe: entry.availability != "Catalog only",
                implementation_version: if entry.availability == "Catalog only" {
                    String::new()
                } else {
                    reviewed_implementation_version(&entry.id).unwrap().into()
                },
                data_snapshot_id: if entry.availability == "Catalog only" {
                    String::new()
                } else {
                    sample_sha()
                },
            })
            .collect()
    }

    fn request(
        plan: StrictAnalysisWorkPlan,
        assessments: Vec<StrictAssessment>,
    ) -> StrictCompletionRequest {
        StrictCompletionRequest {
            market_data: StrictMarketDataContext {
                validated: true,
                source: "Binance public REST".into(),
                as_of: plan.as_of.clone(),
                dataset_sha256: plan.dataset_sha256.clone(),
                symbol: plan.symbol.clone(),
                timeframe: plan.timeframe.clone(),
                candles: sample_candles(),
            },
            risk: StrictRiskApproval {
                passed: true,
                reason: String::new(),
            },
            intent: "paper_trade".into(),
            work_plan: plan,
            assessments,
        }
    }

    #[test]
    fn embedded_master_list_is_exact_364_with_nine_categories_and_expected_status_counts() {
        let manifest = trusted_catalog().unwrap();
        assert_eq!(manifest.entries.len(), 364);
        assert_eq!(manifest.categories.len(), 9);
        let mut status = BTreeMap::<&str, usize>::new();
        for entry in &manifest.entries {
            *status.entry(entry.availability.as_str()).or_default() += 1;
        }
        assert_eq!(status.get("Catalog only").copied(), Some(357));
        assert_eq!(status.get("Calculated now").copied(), Some(6));
        assert_eq!(status.get("Workflow available").copied(), Some(1));
    }

    #[test]
    fn work_plan_routes_every_catalog_id_once_to_the_nine_correct_specialists() {
        let plan = sample_plan();
        assert_eq!(plan.entries.len(), 364);
        assert_eq!(plan.tasks.len(), 9);
        assert!(!plan.live_execution_authorized);
        assert_eq!(plan.dispatch_status, "work_plan_created_not_dispatched");
        let ids = plan
            .tasks
            .iter()
            .flat_map(|task| task.catalog_ids.iter().cloned())
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 364);
        assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), 364);
        for task in &plan.tasks {
            assert_eq!(
                task.agent_id,
                specialist_for_category(&task.category).unwrap()
            );
            assert_eq!(task.required_assessment_count, task.catalog_ids.len());
        }
    }

    #[test]
    fn invalid_scope_time_or_fingerprint_rejects_work_plan() {
        assert!(create_work_plan(CreateWorkPlanRequest {
            symbol: "".into(),
            timeframe: "1h".into(),
            as_of: AS_OF.into(),
            dataset_sha256: sample_sha()
        })
        .is_err());
        assert!(create_work_plan(CreateWorkPlanRequest {
            symbol: "BTC".into(),
            timeframe: "1h".into(),
            as_of: "2026-10-27T08:00:00".into(),
            dataset_sha256: sample_sha()
        })
        .is_err());
        assert!(create_work_plan(CreateWorkPlanRequest {
            symbol: "BTC".into(),
            timeframe: "1h".into(),
            as_of: AS_OF.into(),
            dataset_sha256: "abc".into()
        })
        .is_err());
    }

    #[test]
    fn rust_issued_work_plan_id_is_present_and_plan_mutations_are_detected() {
        let plan = sample_plan();
        assert!(!plan.work_plan_id.trim().is_empty());
        assert!(work_plan_is_unmodified_and_unexpired(&plan).unwrap());
        let mut tampered = plan.clone();
        tampered.as_of = "2026-10-27T09:00:00.000Z".into();
        assert!(!work_plan_is_unmodified_and_unexpired(&tampered).unwrap());
    }

    #[test]
    fn strict_completion_rejects_untrusted_market_source_labels() {
        let plan = sample_plan();
        let assessments = assessments_for(&plan);
        let mut req = request(plan, assessments);
        req.market_data.source = "test-fixture".into();
        let result = evaluate_completion(req).unwrap();
        assert!(result.reasons.iter().any(|reason| {
            reason.contains("trusted Rust-owned Binance public REST provenance")
        }));
        assert!(!result.allowed);
        assert!(result.next_layer_payload.is_none());
    }

    #[test]
    fn incomplete_candle_is_rejected_even_when_its_open_time_precedes_as_of() {
        let cutoff = DateTime::parse_from_rfc3339("2026-10-27T08:30:00Z")
            .unwrap()
            .timestamp_millis();
        let candles = vec![
            Candle {
                time: cutoff - 120 * 60_000,
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 10.0,
            },
            Candle {
                time: cutoff - 30 * 60_000,
                open: 101.0,
                high: 104.0,
                low: 100.0,
                close: 103.0,
                volume: 12.0,
            },
        ];
        let fingerprint = dataset_sha256(&candles);
        let error = validate_candle_snapshot(&candles, "2026-10-27T08:30:00Z", "1h", &fingerprint)
            .unwrap_err();
        assert!(error.contains("not fully closed"));
    }

    fn unique_persistent_store_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "agentic-strict-analysis-{}.sqlite3",
            Uuid::new_v4()
        ))
    }

    fn remove_persistent_store(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    fn valid_persisted_plan(path: &Path) -> StrictAnalysisWorkPlan {
        create_work_plan_persisted(
            CreateWorkPlanRequest {
                symbol: "BTC-USD".into(),
                timeframe: "1h".into(),
                as_of: AS_OF.into(),
                dataset_sha256: sample_sha(),
            },
            path,
        )
        .unwrap()
    }

    #[test]
    fn durable_audit_chain_binds_plan_consumption_and_blocked_finalization() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        let before = verify_persisted_audit_chain(&path).unwrap();
        assert!(before.valid);
        assert_eq!(
            before.event_count, 3,
            "initialization, schema migration, and issuance must be chained"
        );
        let assessments = assessments_for(&plan);
        let result = evaluate_completion_persisted(request(plan, assessments), &path).unwrap();
        assert!(!result.allowed, "catalog-only concepts must remain blocked");
        assert!(result.next_layer_payload.is_none());
        let follow_up = verify_persisted_audit_chain(&path).unwrap();
        assert!(follow_up.valid);
        assert_eq!(
            follow_up.event_count, 6,
            "initialization, migration, issue, consume, start, and finalize must be chained"
        );
        remove_persistent_store(&path);
    }

    #[test]
    fn audit_chain_rejects_modified_event_and_truncated_tail_fail_closed() {
        let path = unique_persistent_store_path();
        let _plan = valid_persisted_plan(&path);
        {
            let connection = Connection::open(&path).unwrap();
            assert!(
                connection
                    .execute(
                        "UPDATE strict_audit_chain_events SET event_sha256=?1 WHERE seq=2",
                        params![AUDIT_CHAIN_GENESIS]
                    )
                    .is_err(),
                "append-only trigger must reject ordinary UPDATE"
            );
            assert!(verify_persisted_audit_chain(&path).unwrap().valid);
            connection
                .execute_batch("DROP TRIGGER strict_audit_chain_no_update;")
                .unwrap();
            connection
                .execute(
                    "UPDATE strict_audit_chain_events SET event_sha256=?1 WHERE seq=2",
                    params![AUDIT_CHAIN_GENESIS],
                )
                .unwrap();
        }
        assert!(
            verify_persisted_audit_chain(&path).is_err(),
            "event digest mutation must be detected"
        );
        assert!(
            create_work_plan_persisted(
                CreateWorkPlanRequest {
                    symbol: "BTC-USD".into(),
                    timeframe: "1h".into(),
                    as_of: AS_OF.into(),
                    dataset_sha256: sample_sha(),
                },
                &path
            )
            .is_err(),
            "corrupt audit history must block new plans"
        );
        remove_persistent_store(&path);

        let path = unique_persistent_store_path();
        let _plan = valid_persisted_plan(&path);
        {
            let connection = Connection::open(&path).unwrap();
            assert!(connection.execute("DELETE FROM strict_audit_chain_events WHERE seq=(SELECT MAX(seq) FROM strict_audit_chain_events)", []).is_err(), "append-only trigger must reject ordinary DELETE");
            connection
                .execute_batch("DROP TRIGGER strict_audit_chain_no_delete;")
                .unwrap();
            connection.execute("DELETE FROM strict_audit_chain_events WHERE seq=(SELECT MAX(seq) FROM strict_audit_chain_events)", []).unwrap();
        }
        assert!(
            verify_persisted_audit_chain(&path).is_err(),
            "head anchor must detect ordinary tail truncation"
        );
        remove_persistent_store(&path);
    }

    #[test]
    fn schema_v1_store_migration_imports_existing_plan_into_the_chain() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("DROP TABLE strict_audit_chain_events; DROP TABLE strict_audit_chain_head; PRAGMA user_version=1;").unwrap();
        }
        let status = verify_persisted_audit_chain(&path).unwrap();
        assert!(status.valid);
        assert!(status.event_count >= 2);
        assert!(persisted_work_plan_is_unmodified_and_unexpired(&plan, &path).unwrap());
        remove_persistent_store(&path);
    }

    #[test]
    fn persistent_gate_requires_nine_durable_specialist_receipts_and_never_fakes_dispatch() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        let assessments = assessments_for(&plan);
        let gaps = specialist_dispatch_receipt_gaps(&plan, &assessments, &path).unwrap();
        assert_eq!(
            gaps.iter()
                .filter(|gap| gap.contains("Missing durable execution receipt"))
                .count(),
            9
        );
        let result =
            evaluate_completion_persisted(request(plan.clone(), assessments), &path).unwrap();
        assert!(!result.allowed);
        assert!(result.next_layer_payload.is_none());
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("Missing durable execution receipt")));
        assert!(!result.live_execution_authorized);
        remove_persistent_store(&path);
    }

    #[test]
    fn specialist_receipts_bind_exact_category_output_and_are_append_only() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        let assessments = assessments_for(&plan);
        let manifest = trusted_catalog().unwrap();
        for category in &manifest.categories {
            let subset = assessments
                .iter()
                .filter(|assessment| {
                    manifest.entries.iter().any(|entry| {
                        entry.id == assessment.catalog_id && entry.category == *category
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            persist_specialist_dispatch_receipt(&plan, category, &subset, &path).unwrap();
        }
        assert!(specialist_dispatch_receipt_gaps(&plan, &assessments, &path)
            .unwrap()
            .is_empty());
        // The receipts prove matching category outputs were persisted, not that catalog-only methods are implemented.
        let result =
            evaluate_completion_persisted(request(plan.clone(), assessments), &path).unwrap();
        assert!(!result.allowed);
        assert!(result.next_layer_payload.is_none());
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("Unsupported concept blocks downstream decisions")));
        let connection = open_strict_store(&path).unwrap();
        let update = connection.execute("UPDATE strict_specialist_dispatch_receipts SET covered_concept_count=covered_concept_count+1 WHERE work_plan_id=?1", params![plan.work_plan_id]);
        assert!(update.is_err(), "receipt ledger must be append-only");
        assert!(verify_persisted_audit_chain(&path).unwrap().valid);
        connection
            .execute_batch("DROP TRIGGER strict_dispatch_receipts_no_update;")
            .unwrap();
        connection.execute(
            "UPDATE strict_specialist_dispatch_receipts SET task_input_sha256=?1 WHERE work_plan_id=?2 AND agent_id=?3",
            params!["0".repeat(64), plan.work_plan_id, specialist_for_category("SMC & ICT").unwrap()],
        ).unwrap();
        drop(connection);
        assert!(verify_persisted_audit_chain(&path).is_err(), "semantic cross-check must detect receipt tampering even if the row trigger is bypassed");
        remove_persistent_store(&path);
    }

    #[test]
    fn persistent_work_plan_consumption_survives_connection_reopen_and_rejects_replay() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        assert!(persisted_work_plan_is_unmodified_and_unexpired(&plan, &path).unwrap());
        // Each call opens a fresh SQLite connection; the second consume simulates a replay
        // after the original request's database connection/process context is gone.
        assert!(consume_work_plan_once_persisted(&plan, &path).unwrap());
        assert!(!persisted_work_plan_is_unmodified_and_unexpired(&plan, &path).unwrap());
        assert!(!consume_work_plan_once_persisted(&plan, &path).unwrap());
        remove_persistent_store(&path);
    }

    #[test]
    fn persistent_tampering_does_not_consume_the_authentic_plan() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        let mut tampered = plan.clone();
        tampered.symbol = "ETH-USD".into();
        assert!(!consume_work_plan_once_persisted(&tampered, &path).unwrap());
        assert!(persisted_work_plan_is_unmodified_and_unexpired(&plan, &path).unwrap());
        assert!(consume_work_plan_once_persisted(&plan, &path).unwrap());
        remove_persistent_store(&path);
    }

    #[test]
    fn persistent_completion_consumes_even_when_assessment_gate_blocks() {
        let path = unique_persistent_store_path();
        let plan = valid_persisted_plan(&path);
        let req = request(plan.clone(), assessments_for(&plan));
        let first = evaluate_completion_persisted(req.clone(), &path).unwrap();
        assert!(
            !first.allowed,
            "357 catalog-only concepts must keep the gate blocked"
        );
        assert!(first.next_layer_payload.is_none());
        let connection = open_strict_store(&path).unwrap();
        let (status, decision, receipt_digest, has_bundle, has_result): (String, String, String, bool, bool) = connection.query_row(
            "SELECT status, decision, receipt_sha256, assessment_bundle_json IS NOT NULL, result_json IS NOT NULL FROM strict_analysis_attempts WHERE work_plan_id=?1",
            params![plan.work_plan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).unwrap();
        assert_eq!(status, "blocked");
        assert_eq!(decision, "BLOCK");
        assert_eq!(receipt_digest.len(), 64);
        assert!(
            has_bundle && has_result,
            "completed attempt must retain the input assessment bundle and auditable result"
        );
        drop(connection);
        let replay_error = evaluate_completion_persisted(req, &path).unwrap_err();
        assert!(replay_error.contains("unknown, modified, expired, or already consumed"));
        let chain = verify_persisted_audit_chain(&path).unwrap();
        assert!(chain.valid);
        assert_eq!(
            chain.event_count, 6,
            "init, schema migration, issue, consume, start, finalize must be chained"
        );
        remove_persistent_store(&path);
    }

    #[test]
    fn work_plan_is_single_use_and_replay_is_fail_closed() {
        let plan = sample_plan();
        assert!(consume_work_plan_once(&plan).unwrap());
        assert!(
            !consume_work_plan_once(&plan).unwrap(),
            "second consumption must be rejected"
        );
        assert!(
            !work_plan_is_unmodified_and_unexpired(&plan).unwrap(),
            "consumed plans must not be considered reusable"
        );
    }

    #[test]
    fn tampered_plan_cannot_consume_the_original_issued_plan() {
        let plan = sample_plan();
        let mut tampered = plan.clone();
        tampered.symbol = "ETH-USD".into();
        assert!(!consume_work_plan_once(&tampered).unwrap());
        assert!(
            consume_work_plan_once(&plan).unwrap(),
            "tampered request must not burn the authentic plan"
        );
    }

    #[test]
    fn audit_receipt_exists_for_blocked_result_and_binds_all_categories_and_input_bundles() {
        let plan = sample_plan();
        let assessments = assessments_for(&plan);
        let mut req = request(plan.clone(), assessments.clone());
        let result = evaluate_completion(req.clone()).unwrap();
        assert!(!result.allowed);
        assert_eq!(
            result.audit_receipt.schema_version,
            "strict-analysis-audit.v1"
        );
        assert_eq!(result.audit_receipt.work_plan_id, plan.work_plan_id);
        assert_eq!(result.audit_receipt.coverage_assessed, 364);
        assert_eq!(result.audit_receipt.coverage_catalog_only, 357);
        assert_eq!(result.audit_receipt.category_assessment_sha256.len(), 9);
        assert_eq!(result.audit_receipt.assessment_bundle_sha256.len(), 64);
        assert_eq!(result.audit_receipt.reasons_sha256.len(), 64);
        assert_eq!(result.audit_receipt.receipt_sha256.len(), 64);
        assert!(!result.audit_receipt.live_execution_authorized);
        let mut receipt_for_verification = result.audit_receipt.clone();
        let expected_receipt_hash = receipt_for_verification.receipt_sha256.clone();
        receipt_for_verification.receipt_sha256.clear();
        assert_eq!(
            sha256_serializable(&receipt_for_verification).unwrap(),
            expected_receipt_hash
        );

        req.assessments[0]
            .reason
            .push_str(" modified after initial receipt");
        let modified = evaluate_completion(req).unwrap();
        assert_ne!(
            result.audit_receipt.assessment_bundle_sha256,
            modified.audit_receipt.assessment_bundle_sha256
        );
        assert_ne!(
            result.audit_receipt.receipt_sha256,
            modified.audit_receipt.receipt_sha256
        );
    }

    #[test]
    fn audit_receipt_verifier_accepts_original_and_rejects_tampering() {
        let plan = sample_plan();
        let result = evaluate_completion(request(plan.clone(), assessments_for(&plan))).unwrap();
        assert!(verify_audit_receipt(&result.audit_receipt).is_ok());

        let mut tampered = result.audit_receipt.clone();
        tampered.decision = "ALLOW_TO_NEXT_LAYER".into();
        assert!(verify_audit_receipt(&tampered)
            .unwrap_err()
            .contains("self-hash mismatch"));

        let mut live = result.audit_receipt.clone();
        live.live_execution_authorized = true;
        assert!(verify_audit_receipt(&live)
            .unwrap_err()
            .contains("cannot authorize live execution"));

        let mut wrong_version = result.audit_receipt.clone();
        wrong_version.master_list_version = "999.0.0".into();
        assert!(verify_audit_receipt(&wrong_version)
            .unwrap_err()
            .contains("catalog version"));
    }

    #[test]
    fn audit_receipt_verifier_rejects_missing_specialist_digest() {
        let plan = sample_plan();
        let result = evaluate_completion(request(plan.clone(), assessments_for(&plan))).unwrap();
        let mut receipt = result.audit_receipt.clone();
        receipt.category_assessment_sha256.pop_first();
        assert!(verify_audit_receipt(&receipt)
            .unwrap_err()
            .contains("all nine specialist categories"));
    }

    #[test]
    fn audit_assessment_bundle_hash_is_independent_of_input_assessment_order() {
        let plan = sample_plan();
        let assessments = assessments_for(&plan);
        let digest_a = assessment_bundle_sha256(&assessments).unwrap();
        let mut reversed = assessments.clone();
        reversed.reverse();
        let digest_b = assessment_bundle_sha256(&reversed).unwrap();
        assert_eq!(digest_a, digest_b);
    }

    #[test]
    fn full_inventory_still_blocks_handoff_when_catalog_only_concepts_exist() {
        let plan = sample_plan();
        let result = evaluate_completion(request(plan, assessments_for(&sample_plan()))).unwrap();
        assert!(!result.allowed);
        assert_eq!(result.decision, "BLOCK");
        assert_eq!(result.coverage.required, 364);
        assert_eq!(result.coverage.assessed, 364);
        assert_eq!(result.coverage.catalog_only, 357);
        assert_eq!(result.coverage.unsupported, 357);
        assert!(result.next_layer_payload.is_none());
        assert!(!result.live_execution_authorized);
    }

    #[test]
    fn canonical_evidence_refs_must_bind_to_the_market_snapshot() {
        let digest = "a".repeat(64);
        assert!(canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:binance:BTCUSDT/1h/1700000000"),
            &digest
        ));
        assert!(canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{}:binance:BTCUSDT/1h/1700000000", digest.to_ascii_uppercase()),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{}:binance:BTCUSDT/1h/1700000000", "b".repeat(64)),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            "test:arbitrary-claim",
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:agent:assertion"),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:fixture:catalog-abc"),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:binance:../private"),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:binance:BTCUSDT//1h"),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:binance:BTCUSDT/./1h"),
            &digest
        ));
        assert!(!canonical_evidence_ref_matches_snapshot(
            &format!("snapshot:{digest}:binance:bad locator"),
            &digest
        ));
    }

    #[test]
    fn evidence_refs_reject_duplicates_and_noncanonical_whitespace() {
        let digest = "a".repeat(64);
        let reference = format!("snapshot:{digest}:binance:BTCUSDT/1h/1700000000");
        assert!(has_auditable_refs(std::slice::from_ref(&reference), &digest));
        assert!(!has_auditable_refs(&[reference.clone(), reference.clone()], &digest));
        assert!(!has_auditable_refs(&[format!(" {reference}")], &digest));
    }

    #[test]
    fn evidence_refs_enforce_count_and_length_limits() {
        let digest = "a".repeat(64);
        let reference = format!("snapshot:{digest}:binance:BTCUSDT/1h/1700000000");
        assert!(!has_auditable_refs(
            &vec![reference.clone(); MAX_EVIDENCE_REFS_PER_ASSESSMENT + 1],
            &digest
        ));
        let long_reference = format!("snapshot:{digest}:binance:{}", "x".repeat(MAX_EVIDENCE_REF_LENGTH));
        assert!(!has_auditable_refs(&[long_reference], &digest));
    }

    #[test]
    fn snapshot_sha256_hex_case_does_not_change_digest_identity() {
        let plan = sample_plan();
        let mut req = request(plan.clone(), assessments_for(&plan));
        req.market_data.dataset_sha256 = req.market_data.dataset_sha256.to_ascii_uppercase();
        for assessment in &mut req.assessments {
            if assessment.status == "evaluated" {
                assessment.data_snapshot_id = assessment.data_snapshot_id.to_ascii_uppercase();
            }
        }
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed, "catalog-only concepts must still block handoff");
        assert!(!result.reasons.iter().any(|reason| reason.contains("fingerprint does not match the work-plan snapshot")));
        assert!(!result.reasons.iter().any(|reason| reason.contains("Assessment data snapshot does not match")));
        assert!(result.next_layer_payload.is_none());
    }

    #[test]
    fn snapshot_mismatch_and_agent_spoofing_fail_closed() {
        let plan = sample_plan();
        let mut assessments = assessments_for(&plan);
        assessments[0].agent_id = "agent-09-risk-management-evaluation".into();
        let mut req = request(plan, assessments);
        req.market_data.dataset_sha256 =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("fingerprint does not match")));
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("identity/contract/agent routing mismatch")));
        assert!(result.next_layer_payload.is_none());
    }

    #[test]
    fn strict_snapshot_rejects_non_positive_candle_timestamps() {
        let plan = sample_plan();
        let mut req = request(plan.clone(), assessments_for(&plan));
        req.market_data.candles[0].time = 0;
        req.market_data.dataset_sha256 = dataset_sha256(&req.market_data.candles);
        for assessment in &mut req.assessments {
            if assessment.status == "evaluated" {
                assessment.data_snapshot_id = req.market_data.dataset_sha256.clone();
            }
        }
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed);
        assert!(result.reasons.iter().any(|reason| reason.contains("timestamps must be positive and strictly increasing")));
        assert!(result.next_layer_payload.is_none());
    }

    #[test]
    fn snapshot_hash_and_future_candle_are_verified_by_rust_not_trusted_from_the_caller() {
        let plan = sample_plan();
        let mut req = request(plan, assessments_for(&sample_plan()));
        req.market_data.candles[0].close = 101.5;
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed);
        assert!(result.reasons.iter().any(
            |reason| reason.contains("fingerprint does not match the supplied OHLCV snapshot")
        ));

        let plan = sample_plan();
        let mut req = request(plan, assessments_for(&sample_plan()));
        req.market_data.candles[2].time = DateTime::parse_from_rfc3339(AS_OF)
            .unwrap()
            .timestamp_millis()
            + 1;
        req.market_data.dataset_sha256 = dataset_sha256(&req.market_data.candles);
        for assessment in &mut req.assessments {
            if assessment.status == "evaluated" {
                assessment.data_snapshot_id = req.market_data.dataset_sha256.clone();
            }
        }
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("after the asOf cutoff")));
    }

    #[test]
    fn missing_duplicate_unknown_and_not_applicable_without_refs_are_blocked() {
        let plan = sample_plan();
        let mut assessments = assessments_for(&plan);
        assessments.pop();
        assessments.push(StrictAssessment {
            catalog_id: "unknown concept".into(),
            ..StrictAssessment::default()
        });
        assessments.push(assessments[0].clone());
        let mut req = request(plan, assessments);
        req.assessments[1].status = "not_applicable".into();
        req.assessments[1].reason = "not applicable for this context".into();
        req.assessments[1].evidence_refs.clear();
        let result = evaluate_completion(req).unwrap();
        assert!(!result.allowed);
        assert!(result.coverage.duplicate > 0);
        assert!(result.coverage.malformed > 0);
        assert!(result
            .reasons
            .iter()
            .any(|reason| reason.contains("requires auditable contextual evidence")));
        assert!(result.next_layer_payload.is_none());
    }
}
