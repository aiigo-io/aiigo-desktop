use crate::compute::types::{ComputeConfig, ComputeConfigResponse};
use std::env;


/// Read compute config from canonical `AIIGO_COMPUTE_*` env vars.
/// Falls back to `VITE_AIIGO_COMPUTE_*` with a warning (dev-only bridge).
pub fn load_compute_config_response() -> ComputeConfigResponse {
    let mut missing: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    let chain_id = read_u64_env(
        "AIIGO_COMPUTE_CHAIN_ID",
        "VITE_AIIGO_COMPUTE_CHAIN_ID",
        &mut missing,
        &mut warnings,
    );

    let rpc_url = read_str_env(
        "AIIGO_COMPUTE_RPC_URL",
        "VITE_AIIGO_COMPUTE_RPC_URL",
        &mut missing,
        &mut warnings,
    );

    let task_marketplace_address = read_str_env(
        "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
        "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
        &mut missing,
        &mut warnings,
    );

    let node_registry_address = read_str_env(
        "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
        "VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
        &mut missing,
        &mut warnings,
    );

    let escrow_manager_address = read_str_env(
        "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
        "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
        &mut missing,
        &mut warnings,
    );

    let task_deploy_block = read_u64_env(
        "AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
        "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
        &mut missing,
        &mut warnings,
    );

    let node_deploy_block = read_u64_env(
        "AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
        "VITE_AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
        &mut missing,
        &mut warnings,
    );

    let escrow_deploy_block = read_u64_env(
        "AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
        "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
        &mut missing,
        &mut warnings,
    );

    let confirmation_depth = read_u64_env(
        "AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        "VITE_AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        &mut missing,
        &mut warnings,
    );

    let bootstrap_start_block = match (task_deploy_block, node_deploy_block, escrow_deploy_block) {
        (Some(t), Some(n), Some(e)) => Some(t.min(n).min(e)),
        _ => None,
    };

    let chain_name = chain_id.map(|id| match id {
        1 => "Ethereum Mainnet".to_string(),
        11155111 => "Ethereum Sepolia".to_string(),
        _ => format!("Chain {}", id),
    });

    // PoW verifier address is optional — does not affect core marketplace is_configured.
    let pow_verifier_address = read_str_env_optional(
        "AIIGO_COMPUTE_POW_VERIFIER_ADDRESS",
        "VITE_AIIGO_COMPUTE_POW_VERIFIER_ADDRESS",
        &mut warnings,
    );
    let is_pow_configured = pow_verifier_address.is_some();

    ComputeConfigResponse {
        chain_id,
        chain_name,
        rpc_url,
        task_marketplace_address,
        node_registry_address,
        escrow_manager_address,
        task_marketplace_deploy_block: task_deploy_block,
        node_registry_deploy_block: node_deploy_block,
        escrow_manager_deploy_block: escrow_deploy_block,
        confirmation_depth,  // already Option<u64>
        bootstrap_start_block,
        is_configured: missing.is_empty(),
        missing,
        warnings,
        pow_verifier_address,
        is_pow_configured,
    }
}

/// Attempt to parse a fully-validated `ComputeConfig` from env vars.
/// Returns `Err` with list of missing fields when any required field is absent.
pub fn load_compute_config() -> Result<ComputeConfig, Vec<String>> {
    let resp = load_compute_config_response();
    if !resp.missing.is_empty() {
        return Err(resp.missing);
    }
    Ok(ComputeConfig {
        chain_id: resp.chain_id.unwrap(),
        rpc_url: resp.rpc_url.unwrap(),
        task_marketplace_address: resp.task_marketplace_address.unwrap(),
        node_registry_address: resp.node_registry_address.unwrap(),
        escrow_manager_address: resp.escrow_manager_address.unwrap(),
        task_marketplace_deploy_block: resp.task_marketplace_deploy_block.unwrap(),
        node_registry_deploy_block: resp.node_registry_deploy_block.unwrap(),
        escrow_manager_deploy_block: resp.escrow_manager_deploy_block.unwrap(),
        confirmation_depth: resp.confirmation_depth.unwrap(), // safe: missing.is_empty() check above
    })
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn read_str_env(
    canon: &str,
    vite_fallback: &str,
    missing: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    if let Ok(v) = env::var(canon) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    if let Ok(v) = env::var(vite_fallback) {
        if !v.trim().is_empty() {
            warnings.push(format!(
                "Using VITE_ fallback for {}; set {} for production",
                canon, canon
            ));
            return Some(v.trim().to_string());
        }
    }
    missing.push(canon.to_string());
    None
}

fn read_u64_env(
    canon: &str,
    vite_fallback: &str,
    missing: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Option<u64> {
    if let Ok(v) = env::var(canon) {
        if let Ok(n) = v.trim().parse::<u64>() {
            return Some(n);
        }
    }
    if let Ok(v) = env::var(vite_fallback) {
        if let Ok(n) = v.trim().parse::<u64>() {
            warnings.push(format!(
                "Using VITE_ fallback for {}; set {} for production",
                canon, canon
            ));
            return Some(n);
        }
    }
    missing.push(canon.to_string());
    None
}

/// Like `read_str_env` but does NOT push to `missing` when absent — used for
/// optional config fields that don't affect `is_configured`.
fn read_str_env_optional(
    canon: &str,
    vite_fallback: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    if let Ok(v) = env::var(canon) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    if let Ok(v) = env::var(vite_fallback) {
        if !v.trim().is_empty() {
            warnings.push(format!(
                "Using VITE_ fallback for {}; set {} for production",
                canon, canon
            ));
            return Some(v.trim().to_string());
        }
    }
    None
}

/// Load only the ProofOfWorkVerifier contract address.
/// Returns `Err` with a descriptive message when the env var is absent.
/// Does NOT affect the core marketplace `is_configured` flag.
pub fn load_pow_verifier_address() -> Result<String, String> {
    const CANON: &str = "AIIGO_COMPUTE_POW_VERIFIER_ADDRESS";
    const VITE: &str = "VITE_AIIGO_COMPUTE_POW_VERIFIER_ADDRESS";

    if let Ok(v) = env::var(CANON) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Ok(v);
        }
    }
    if let Ok(v) = env::var(VITE) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Ok(v);
        }
    }
    Err(format!(
        "pow_verifier_unconfigured: {} is not set; set it to the deployed ProofOfWorkVerifier address",
        CANON
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Env-var tests mutate global process state.  Serialise them so they don't
    // race each other when `cargo test` runs them in parallel threads.
    static ENV_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn missing_all_env_vars_returns_all_missing() {
        let _guard = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Unset all relevant keys so this test is deterministic.
        for key in &[
            "AIIGO_COMPUTE_CHAIN_ID",
            "AIIGO_COMPUTE_RPC_URL",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_CONFIRMATION_DEPTH",
            "VITE_AIIGO_COMPUTE_CHAIN_ID",
            "VITE_AIIGO_COMPUTE_RPC_URL",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }

        let resp = load_compute_config_response();
        assert!(!resp.is_configured);
        assert!(!resp.missing.is_empty());
        assert!(resp.missing.contains(&"AIIGO_COMPUTE_CHAIN_ID".to_string()));
        assert!(resp.missing.contains(&"AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK".to_string()));
        assert!(resp.missing.contains(&"AIIGO_COMPUTE_CONFIRMATION_DEPTH".to_string()));
        assert_eq!(resp.bootstrap_start_block, None);
    }

    #[test]
    fn all_env_vars_present_is_configured() {
        let _guard = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("AIIGO_COMPUTE_CHAIN_ID", "11155111");
        std::env::set_var("AIIGO_COMPUTE_RPC_URL", "https://rpc.example.com");
        std::env::set_var(
            "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "0x1111111111111111111111111111111111111111",
        );
        std::env::set_var(
            "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "0x2222222222222222222222222222222222222222",
        );
        std::env::set_var(
            "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "0x3333333333333333333333333333333333333333",
        );
        std::env::set_var("AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK", "1000");
        std::env::set_var("AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK", "900");
        std::env::set_var("AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK", "800");
        std::env::set_var("AIIGO_COMPUTE_CONFIRMATION_DEPTH", "12");

        for key in &[
            "VITE_AIIGO_COMPUTE_CHAIN_ID",
            "VITE_AIIGO_COMPUTE_RPC_URL",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }

        let resp = load_compute_config_response();
        assert!(resp.is_configured, "missing: {:?}", resp.missing);
        assert_eq!(resp.chain_id, Some(11155111));
        // bootstrap = min(1000, 900, 800) = 800
        assert_eq!(resp.bootstrap_start_block, Some(800));
        assert!(resp.warnings.is_empty());

        // cleanup
        for key in &[
            "AIIGO_COMPUTE_CHAIN_ID",
            "AIIGO_COMPUTE_RPC_URL",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn vite_fallback_produces_warning() {
        let _guard = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Fully clear canonical keys first.
        for key in &[
            "AIIGO_COMPUTE_CHAIN_ID",
            "AIIGO_COMPUTE_RPC_URL",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }

        std::env::set_var("VITE_AIIGO_COMPUTE_CHAIN_ID", "11155111");
        std::env::set_var("VITE_AIIGO_COMPUTE_RPC_URL", "https://rpc.example.com");
        std::env::set_var(
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "0x1111111111111111111111111111111111111111",
        );
        std::env::set_var(
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "0x2222222222222222222222222222222222222222",
        );
        std::env::set_var(
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "0x3333333333333333333333333333333333333333",
        );
        std::env::set_var("VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK", "1000");
        std::env::set_var("VITE_AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK", "900");
        std::env::set_var("VITE_AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK", "800");
        std::env::set_var("VITE_AIIGO_COMPUTE_CONFIRMATION_DEPTH", "12");

        let resp = load_compute_config_response();
        assert!(resp.is_configured);
        assert!(!resp.warnings.is_empty(), "Expected warnings for VITE_ fallback");

        // cleanup
        for key in &[
            "VITE_AIIGO_COMPUTE_CHAIN_ID",
            "VITE_AIIGO_COMPUTE_RPC_URL",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS",
            "VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "VITE_AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }
    }

    // ── Task 2: PoW verifier config ──────────────────────────────────────────

    /// Missing POW verifier env var → load_pow_verifier_address returns an error
    /// containing the expected var name.  Does NOT affect is_configured.
    #[test]
    fn pow_verifier_missing_reports_unconfigured() {
        let _guard = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("AIIGO_COMPUTE_POW_VERIFIER_ADDRESS");
        std::env::remove_var("VITE_AIIGO_COMPUTE_POW_VERIFIER_ADDRESS");

        let result = load_pow_verifier_address();
        assert!(result.is_err(), "must return Err when env var is absent");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("pow_verifier_unconfigured") || msg.contains("AIIGO_COMPUTE_POW_VERIFIER_ADDRESS"),
            "error message must name the missing var, got: {}",
            msg
        );
    }

    /// POW verifier absent must not block the core marketplace is_configured flag.
    #[test]
    fn pow_verifier_absent_does_not_affect_is_configured() {
        let _guard = ENV_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("AIIGO_COMPUTE_POW_VERIFIER_ADDRESS");
        std::env::remove_var("VITE_AIIGO_COMPUTE_POW_VERIFIER_ADDRESS");

        // Set all core marketplace vars
        std::env::set_var("AIIGO_COMPUTE_CHAIN_ID", "11155111");
        std::env::set_var("AIIGO_COMPUTE_RPC_URL", "https://rpc.example.com");
        std::env::set_var("AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS", "0x1111111111111111111111111111111111111111");
        std::env::set_var("AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS", "0x2222222222222222222222222222222222222222");
        std::env::set_var("AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS", "0x3333333333333333333333333333333333333333");
        std::env::set_var("AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK", "100");
        std::env::set_var("AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK", "100");
        std::env::set_var("AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK", "100");
        std::env::set_var("AIIGO_COMPUTE_CONFIRMATION_DEPTH", "12");

        let resp = load_compute_config_response();
        assert!(resp.is_configured, "core marketplace must be configured; missing: {:?}", resp.missing);
        assert!(!resp.is_pow_configured, "POW not set → is_pow_configured must be false");
        assert!(resp.pow_verifier_address.is_none());

        // cleanup
        for key in &[
            "AIIGO_COMPUTE_CHAIN_ID", "AIIGO_COMPUTE_RPC_URL",
            "AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS", "AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS",
            "AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS", "AIIGO_COMPUTE_TASK_MARKETPLACE_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_NODE_REGISTRY_DEPLOY_BLOCK", "AIIGO_COMPUTE_ESCROW_MANAGER_DEPLOY_BLOCK",
            "AIIGO_COMPUTE_CONFIRMATION_DEPTH",
        ] {
            std::env::remove_var(key);
        }
    }
}
