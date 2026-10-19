//! Password hashing with argon2, bounded so a burst of sign-ins cannot exhaust memory.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use super::{AccountError, MIN_PASSWORD};

/// Each argon2 run takes about 19 MiB. Per-IP limits do not bound how many IPs sign in at once,
/// so this does: at most 4 hashes run, the rest wait. A waiter whose client leaves is dropped
/// from the queue with its request, so the queue holds only live sign-ins.
static HASHING: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

pub(super) async fn hash_password(password: &str) -> Result<String, AccountError> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(AccountError::PasswordTooShort);
    }
    let pw = password.to_owned();
    let slot = HASHING.acquire().await.map_err(anyhow::Error::from)?;
    let hashed = tokio::task::spawn_blocking(move || {
        // The permit lives in the blocking task: a dropped request cannot free it mid-hash.
        let _slot = slot;
        Argon2::default()
            .hash_password(pw.as_bytes())
            .map(|h| h.to_string())
    })
    .await
    .map_err(anyhow::Error::from)?
    .map_err(|e| anyhow::anyhow!("hash failed: {e}"))?;
    Ok(hashed)
}

/// An argon2 hash, at the default cost, of 24 random bytes nobody kept. A sign-in with no
/// password to check runs against it, so it takes as long as a real one: the answer's timing
/// does not tell which emails have accounts.
const NO_ACCOUNT: &str = "$argon2id$v=19$m=19456,t=2,p=1$TkbtZai7kbGkr09VcoBdXA$dpaolgd3dJjqNnz616Z7SolFqFI5byhCWBC5vuSQy4Y";

/// Spends one password check and refuses: for an unknown, disabled or not yet activated account.
pub(super) async fn refuse_like_a_check(password: &str) -> bool {
    let _ = verify_password(password, NO_ACCOUNT).await;
    false
}

/// False for a wrong password. A stored hash that does not parse, or a hashing task that dies,
/// also refuses the sign-in, but is logged: those are faults, not wrong passwords.
pub(super) async fn verify_password(password: &str, stored: &str) -> bool {
    verify_with(&HASHING, password, stored).await
}

async fn verify_with(slots: &'static tokio::sync::Semaphore, password: &str, stored: &str) -> bool {
    let (pw, stored) = (password.to_owned(), stored.to_owned());
    let Ok(slot) = slots.acquire().await else {
        tracing::error!("password check unavailable: the hashing limit is closed");
        return false;
    };
    let checked = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        match PasswordHash::new(&stored) {
            Ok(h) => Ok(Argon2::default().verify_password(pw.as_bytes(), &h).is_ok()),
            Err(e) => Err(e.to_string()),
        }
    })
    .await;
    match checked {
        Ok(Ok(ok)) => ok,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "a stored password hash does not parse");
            false
        }
        Err(e) => {
            tracing::error!(error = %e, "the password check task failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_dropped_check_keeps_its_hash_slot_until_the_hash_ends() {
        // Its own semaphore: the shared one would let any other hashing test change the counts.
        static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
        let stored = hash_password("correct-horse-battery").await.unwrap();
        // The first poll takes the slot and starts the hash; the 1 ms timeout then drops the
        // future while the hash, far slower than 1 ms, still runs.
        let dropped = tokio::time::timeout(
            std::time::Duration::from_millis(1),
            verify_with(&SLOTS, "wrong-password", &stored),
        )
        .await;
        assert!(dropped.is_err());
        assert!(
            SLOTS.available_permits() < 4,
            "the running hash still holds its slot"
        );
        for _ in 0..200 {
            if SLOTS.available_permits() == 4 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(SLOTS.available_permits(), 4, "and frees it when done");
    }
}
