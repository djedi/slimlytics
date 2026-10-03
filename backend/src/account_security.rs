//! Account security: sign-in sessions (devices), passkeys (WebAuthn), and passkey step-up
//! that marks a session as MFA-verified.
use super::*;
use webauthn_rs::prelude::{
    DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential, Webauthn,
    WebauthnBuilder,
};

/// A WebAuthn ceremony must finish within this many seconds.
const CHALLENGE_TTL_SECONDS: i32 = 300;
const MAX_PASSKEYS: i64 = 10;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/account/sessions", get(list_sessions))
        .route("/api/account/sessions/{session_id}", delete(revoke_session))
        .route("/api/account/passkeys", get(list_passkeys))
        .route(
            "/api/account/passkeys/register/start",
            post(start_registration),
        )
        .route(
            "/api/account/passkeys/register/finish",
            post(finish_registration),
        )
        .route("/api/account/passkeys/{passkey_id}", delete(delete_passkey))
        .route("/api/auth/passkey/start", post(start_login))
        .route("/api/auth/passkey/finish", post(finish_login))
        .route("/api/auth/mfa/start", post(start_step_up))
        .route("/api/auth/mfa/finish", post(finish_step_up))
}

/// The relying party is the public URL: passkeys are bound to its host and origin.
fn webauthn(state: &AppState) -> Result<Webauthn, ApiError> {
    let origin = Url::parse(&state.public_url).map_err(|_| ApiError::Internal)?;
    let rp_id = origin.host_str().ok_or(ApiError::Internal)?.to_owned();
    WebauthnBuilder::new(&rp_id, &origin)
        .and_then(|builder| builder.rp_name("Slimlytics").build())
        .map_err(|_| ApiError::Internal)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeResponse<T: Serialize> {
    challenge_id: Uuid,
    options: T,
}

async fn store_challenge(
    pool: &PgPool,
    kind: &str,
    user: Option<Uuid>,
    session: Option<Uuid>,
    state: &impl Serialize,
) -> Result<Uuid, ApiError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO webauthn_challenges(kind,user_id,session_id,state,expires_at)
         VALUES($1,$2,$3,$4,now()+make_interval(secs=>$5)) RETURNING id",
    )
    .bind(kind)
    .bind(user)
    .bind(session)
    .bind(serde_json::to_value(state).map_err(|_| ApiError::Internal)?)
    .bind(CHALLENGE_TTL_SECONDS as f64)
    .fetch_one(pool)
    .await?)
}

/// Consumes a challenge exactly once. `user`/`session` must match when the ceremony is
/// tied to a signed-in account.
async fn take_challenge<T: serde::de::DeserializeOwned>(
    pool: &PgPool,
    id: Uuid,
    kind: &str,
    user: Option<Uuid>,
    session: Option<Uuid>,
) -> Result<Option<T>, ApiError> {
    let state: Option<Value> = sqlx::query_scalar(
        "DELETE FROM webauthn_challenges
         WHERE id=$1 AND kind=$2 AND expires_at>now()
           AND user_id IS NOT DISTINCT FROM $3 AND session_id IS NOT DISTINCT FROM $4
         RETURNING state",
    )
    .bind(id)
    .bind(kind)
    .bind(user)
    .bind(session)
    .fetch_optional(pool)
    .await?;
    state
        .map(|value| serde_json::from_value(value).map_err(|_| ApiError::Internal))
        .transpose()
}

async fn user_passkeys(pool: &PgPool, user: Uuid) -> Result<Vec<(Uuid, Passkey)>, ApiError> {
    let rows: Vec<(Uuid, Value)> =
        sqlx::query_as("SELECT id,passkey FROM user_passkeys WHERE user_id=$1")
            .bind(user)
            .fetch_all(pool)
            .await?;
    rows.into_iter()
        .map(|(id, value)| {
            serde_json::from_value(value)
                .map(|key| (id, key))
                .map_err(|_| ApiError::Internal)
        })
        .collect()
}

/// Confirms the asserted passkey is still registered to `user` (it may have been removed
/// while the ceremony was pending) and saves its new signature counter. The row lock holds
/// off a concurrent removal until the caller's transaction commits.
async fn record_use(
    tx: &mut Transaction<'_, Postgres>,
    user: Uuid,
    result: &webauthn_rs::prelude::AuthenticationResult,
) -> Result<(), ApiError> {
    let row: Option<(Uuid, Value)> = sqlx::query_as(
        "SELECT id,passkey FROM user_passkeys WHERE user_id=$1 AND credential_id=$2 FOR UPDATE",
    )
    .bind(user)
    .bind(result.cred_id().to_vec())
    .fetch_optional(&mut **tx)
    .await?;
    let (id, value) = row.ok_or(ApiError::Unauthorized)?;
    let mut key: Passkey = serde_json::from_value(value).map_err(|_| ApiError::Internal)?;
    key.update_credential(result);
    sqlx::query("UPDATE user_passkeys SET passkey=$2,last_used_at=now() WHERE id=$1")
        .bind(id)
        .bind(serde_json::to_value(&key).map_err(|_| ApiError::Internal)?)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Re-checks the signed-in person's password, throttled per account like sign-in so a
/// stolen session cannot be used to guess it.
async fn password_matches(state: &AppState, user: Uuid, password: &str) -> Result<bool, ApiError> {
    if !state.login_limiter.check(&format!("reauth:{user}")) {
        return Err(ApiError::RateLimited);
    }
    if password.is_empty() || password.len() > 1024 {
        return Ok(false);
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id=$1")
        .bind(user)
        .fetch_one(&state.pool)
        .await?;
    Ok(verify_password(password, &hash).unwrap_or(false))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Reauth {
    current_password: Option<String>,
}

// ---- sessions -------------------------------------------------------------------------

async fn list_sessions(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<Vec<Value>>, ApiError> {
    type Row = (
        Uuid,
        String,
        Option<String>,
        DateTime<Utc>,
        DateTime<Utc>,
        DateTime<Utc>,
        bool,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id,auth_method,user_agent,created_at,last_used_at,expires_at,
                coalesce(mfa_verified_at > now()-make_interval(hours=>$2), false)
         FROM user_sessions
         WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>now()
         ORDER BY last_used_at DESC",
    )
    .bind(signed_in.user_id)
    .bind(MFA_WINDOW_HOURS as i32)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| {
                json!({
                    "id": row.0, "authMethod": row.1, "userAgent": row.2, "createdAt": row.3,
                    "lastUsedAt": row.4, "expiresAt": row.5, "mfaVerified": row.6,
                    "current": Some(row.0) == signed_in.session_id,
                })
            })
            .collect(),
    ))
}

async fn revoke_session(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(session): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let done = sqlx::query(
        "UPDATE user_sessions SET revoked_at=now() WHERE id=$1 AND user_id=$2 AND revoked_at IS NULL",
    )
    .bind(session)
    .bind(signed_in.user_id)
    .execute(&state.pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- passkey management ---------------------------------------------------------------

async fn list_passkeys(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<Vec<Value>>, ApiError> {
    type PasskeyRow = (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>);
    let rows: Vec<PasskeyRow> = sqlx::query_as(
        "SELECT id,name,created_at,last_used_at FROM user_passkeys WHERE user_id=$1 ORDER BY created_at",
    )
    .bind(signed_in.user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| json!({"id": row.0, "name": row.1, "createdAt": row.2, "lastUsedAt": row.3}))
            .collect(),
    ))
}

/// Adding the first passkey needs the current password; adding more needs a session that
/// already proved a passkey, so a stolen password alone can never plant a new one.
async fn start_registration(
    State(state): State<AppState>,
    signed_in: SignedIn,
    body: Option<Json<Reauth>>,
) -> Result<Json<ChallengeResponse<Value>>, ApiError> {
    let Json(reauth) = body.unwrap_or_default();
    let user = signed_in.user_id;
    let keys = user_passkeys(&state.pool, user).await?;
    if keys.is_empty() {
        let password = reauth.current_password.unwrap_or_default();
        if !password_matches(&state, user, &password).await? {
            return Err(ApiError::Forbidden);
        }
    } else if !session_mfa_verified(&state.pool, signed_in.session_id).await? {
        return Err(ApiError::MfaRequired);
    }
    if keys.len() as i64 >= MAX_PASSKEYS {
        return Err(ApiError::BadRequest(format!(
            "an account can have at most {MAX_PASSKEYS} passkeys"
        )));
    }
    let email: String = sqlx::query_scalar("SELECT email FROM users WHERE id=$1")
        .bind(user)
        .fetch_one(&state.pool)
        .await?;
    let exclude = keys.iter().map(|(_, key)| key.cred_id().clone()).collect();
    let (options, registration) = webauthn(&state)?
        .start_passkey_registration(user, &email, &email, Some(exclude))
        .map_err(|_| ApiError::Internal)?;
    // Require a discoverable credential: sign-in is usernameless, so a passkey the
    // authenticator cannot offer unprompted would be unusable there.
    let mut options = serde_json::to_value(options).map_err(|_| ApiError::Internal)?;
    if let Some(selection) = options
        .pointer_mut("/publicKey/authenticatorSelection")
        .and_then(Value::as_object_mut)
    {
        selection.insert("residentKey".into(), json!("required"));
        selection.insert("requireResidentKey".into(), json!(true));
    }
    let challenge_id = store_challenge(
        &state.pool,
        "register",
        Some(user),
        signed_in.session_id,
        &registration,
    )
    .await?;
    Ok(Json(ChallengeResponse {
        challenge_id,
        options,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishRegistration {
    challenge_id: Uuid,
    name: String,
    credential: RegisterPublicKeyCredential,
}

async fn finish_registration(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Json(input): Json<FinishRegistration>,
) -> Result<impl IntoResponse, ApiError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ApiError::BadRequest(
            "passkey name must contain between 1 and 100 characters".into(),
        ));
    }
    let registration: PasskeyRegistration = take_challenge(
        &state.pool,
        input.challenge_id,
        "register",
        Some(signed_in.user_id),
        signed_in.session_id,
    )
    .await?
    .ok_or_else(|| ApiError::BadRequest("passkey setup expired; try again".into()))?;
    let passkey = webauthn(&state)?
        .finish_passkey_registration(&input.credential, &registration)
        .map_err(|_| ApiError::BadRequest("the passkey could not be verified".into()))?;
    // Re-check enrollment rules now: another ceremony may have saved a passkey since this
    // one started. The account row lock serializes concurrent enrollments.
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT 1 FROM users WHERE id=$1 FOR UPDATE")
        .bind(signed_in.user_id)
        .execute(&mut *tx)
        .await?;
    // The session may have been revoked (e.g. by a passkey reset) while this ceremony ran.
    let live: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_sessions
         WHERE id=$1 AND revoked_at IS NULL AND expires_at>now())",
    )
    .bind(signed_in.session_id)
    .fetch_one(&mut *tx)
    .await?;
    if !live {
        return Err(ApiError::Unauthorized);
    }
    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM user_passkeys WHERE user_id=$1")
        .bind(signed_in.user_id)
        .fetch_one(&mut *tx)
        .await?;
    if existing >= MAX_PASSKEYS {
        return Err(ApiError::BadRequest(format!(
            "an account can have at most {MAX_PASSKEYS} passkeys"
        )));
    }
    if existing > 0 && !session_mfa_verified(&state.pool, signed_in.session_id).await? {
        return Err(ApiError::MfaRequired);
    }
    let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO user_passkeys(user_id,name,credential_id,passkey) VALUES($1,$2,$3,$4)
         RETURNING id,created_at",
    )
    .bind(signed_in.user_id)
    .bind(name)
    .bind(passkey.cred_id().to_vec())
    .bind(serde_json::to_value(&passkey).map_err(|_| ApiError::Internal)?)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_conflict)?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"id": row.0, "name": name, "createdAt": row.1, "lastUsedAt": null})),
    ))
}

/// Removing a passkey needs a passkey-verified session. Accepting the password here would
/// let a thief delete every passkey and then enroll their own with the password alone.
async fn delete_passkey(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Path(passkey): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if !session_mfa_verified(&state.pool, signed_in.session_id).await? {
        return Err(ApiError::MfaRequired);
    }
    let done = sqlx::query("DELETE FROM user_passkeys WHERE id=$1 AND user_id=$2")
        .bind(passkey)
        .bind(signed_in.user_id)
        .execute(&state.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- passkey sign-in ------------------------------------------------------------------

/// Usernameless sign-in: the browser offers whichever passkey the person picks.
async fn start_login(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<ChallengeResponse<webauthn_rs::prelude::RequestChallengeResponse>>, ApiError> {
    let ip = client_ip(&headers, peer.ip(), state.trust_proxy);
    if !state.passkey_limiter.check(&ip.to_string()) {
        return Err(ApiError::RateLimited);
    }
    let (mut options, authentication) = webauthn(&state)?
        .start_discoverable_authentication()
        .map_err(|_| ApiError::Internal)?;
    // The sign-in page asks on a button press rather than through autofill.
    options.mediation = None;
    let challenge_id = store_challenge(&state.pool, "login", None, None, &authentication).await?;
    Ok(Json(ChallengeResponse {
        challenge_id,
        options,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishAuthentication {
    challenge_id: Uuid,
    credential: PublicKeyCredential,
}

async fn finish_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FinishAuthentication>,
) -> Result<Json<SessionTokens>, ApiError> {
    let authentication: DiscoverableAuthentication =
        take_challenge(&state.pool, input.challenge_id, "login", None, None)
            .await?
            .ok_or(ApiError::Unauthorized)?;
    let webauthn = webauthn(&state)?;
    let (user, _) = webauthn
        .identify_discoverable_authentication(&input.credential)
        .map_err(|_| ApiError::Unauthorized)?;
    let keys = user_passkeys(&state.pool, user).await?;
    let discoverable: Vec<DiscoverableKey> = keys
        .iter()
        .map(|(_, key)| DiscoverableKey::from(key))
        .collect();
    let result = webauthn
        .finish_discoverable_authentication(&input.credential, authentication, &discoverable)
        .map_err(|_| ApiError::Unauthorized)?;
    // One transaction, account row first, then the passkey row: a concurrent disable or
    // passkey reset either blocks this sign-in or revokes the session it creates.
    let mut tx = state.pool.begin().await?;
    lock_enabled_user(&mut tx, user, true).await?;
    record_use(&mut tx, user, &result).await?;
    let tokens = create_session_in(&mut tx, &state, user, "passkey", &headers).await?;
    tx.commit().await?;
    Ok(Json(tokens))
}

// ---- step-up (MFA) --------------------------------------------------------------------

/// Proves a passkey from an existing session, e.g. after a password sign-in.
async fn start_step_up(
    State(state): State<AppState>,
    signed_in: SignedIn,
) -> Result<Json<ChallengeResponse<webauthn_rs::prelude::RequestChallengeResponse>>, ApiError> {
    let session = signed_in.session_id.ok_or(ApiError::Unauthorized)?;
    let keys = user_passkeys(&state.pool, signed_in.user_id).await?;
    if keys.is_empty() {
        return Err(ApiError::BadRequest(
            "add a passkey to your account first".into(),
        ));
    }
    let credentials: Vec<Passkey> = keys.into_iter().map(|(_, key)| key).collect();
    let (options, authentication) = webauthn(&state)?
        .start_passkey_authentication(&credentials)
        .map_err(|_| ApiError::Internal)?;
    let challenge_id = store_challenge(
        &state.pool,
        "step_up",
        Some(signed_in.user_id),
        Some(session),
        &authentication,
    )
    .await?;
    Ok(Json(ChallengeResponse {
        challenge_id,
        options,
    }))
}

async fn finish_step_up(
    State(state): State<AppState>,
    signed_in: SignedIn,
    Json(input): Json<FinishAuthentication>,
) -> Result<StatusCode, ApiError> {
    let session = signed_in.session_id.ok_or(ApiError::Unauthorized)?;
    let authentication: PasskeyAuthentication = take_challenge(
        &state.pool,
        input.challenge_id,
        "step_up",
        Some(signed_in.user_id),
        Some(session),
    )
    .await?
    .ok_or(ApiError::Unauthorized)?;
    let result = webauthn(&state)?
        .finish_passkey_authentication(&input.credential, &authentication)
        .map_err(|_| ApiError::Unauthorized)?;
    let mut tx = state.pool.begin().await?;
    record_use(&mut tx, signed_in.user_id, &result).await?;
    sqlx::query("UPDATE user_sessions SET mfa_verified_at=now() WHERE id=$1")
        .bind(session)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
