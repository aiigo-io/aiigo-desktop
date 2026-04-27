use super::types::{PasswordAuthState, PasswordKdfParams, SecurityError};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::random;
use subtle::ConstantTimeEq;

const PASSWORD_HASH_LEN: usize = 32;
const PASSWORD_SALT_LEN: usize = 16;
const DEFAULT_MEMORY_COST_KIB: u32 = 19_456;
const DEFAULT_ITERATIONS: u32 = 2;
const DEFAULT_PARALLELISM: u32 = 1;

fn default_kdf_params() -> PasswordKdfParams {
    PasswordKdfParams {
        memory_cost_kib: DEFAULT_MEMORY_COST_KIB,
        iterations: DEFAULT_ITERATIONS,
        parallelism: DEFAULT_PARALLELISM,
    }
}

fn build_argon2(kdf_params: &PasswordKdfParams) -> Result<Argon2<'static>, SecurityError> {
    let params = Params::new(
        kdf_params.memory_cost_kib,
        kdf_params.iterations,
        kdf_params.parallelism,
        Some(PASSWORD_HASH_LEN),
    )
    .map_err(|_| SecurityError::OperationNotAllowed)?;

    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn validate_password(password: &str) -> Result<(), SecurityError> {
    if password.trim().is_empty() {
        return Err(SecurityError::PolicyDenied);
    }

    Ok(())
}

pub fn hash_password(password: &str) -> Result<PasswordAuthState, SecurityError> {
    validate_password(password)?;

    let kdf_params = default_kdf_params();
    let argon2 = build_argon2(&kdf_params)?;
    let salt_bytes = random::<[u8; PASSWORD_SALT_LEN]>();
    let mut hash_bytes = [0u8; PASSWORD_HASH_LEN];

    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut hash_bytes)
        .map_err(|_| SecurityError::OperationNotAllowed)?;

    Ok(PasswordAuthState {
        password_hash: hex::encode(hash_bytes),
        password_salt: hex::encode(salt_bytes),
        kdf_params,
    })
}

pub fn verify_password(
    password: &str,
    auth_state: &PasswordAuthState,
) -> Result<bool, SecurityError> {
    validate_password(password)?;

    let salt_bytes =
        hex::decode(&auth_state.password_salt).map_err(|_| SecurityError::OperationNotAllowed)?;
    let expected_hash =
        hex::decode(&auth_state.password_hash).map_err(|_| SecurityError::OperationNotAllowed)?;

    if expected_hash.len() != PASSWORD_HASH_LEN {
        return Err(SecurityError::OperationNotAllowed);
    }

    let argon2 = build_argon2(&auth_state.kdf_params)?;
    let mut actual_hash = vec![0u8; expected_hash.len()];

    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut actual_hash)
        .map_err(|_| SecurityError::OperationNotAllowed)?;

    Ok(actual_hash.ct_eq(&expected_hash).into())
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};
    use crate::wallet::security::types::SecurityError;

    #[test]
    fn hash_and_verify_password_round_trip() {
        let auth_state = hash_password("correct horse battery staple").unwrap();

        assert!(verify_password("correct horse battery staple", &auth_state).unwrap());
        assert!(!verify_password("wrong password", &auth_state).unwrap());
    }

    #[test]
    fn empty_password_is_rejected() {
        assert_eq!(hash_password("   "), Err(SecurityError::PolicyDenied));
    }
}
