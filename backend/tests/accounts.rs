//! Long-lived sessions, passkeys, and the MFA-gated admin portal.
//! Requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database.
use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};
use slimlytics_backend::{app, prune_auth_state, AppState};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;
use webauthn_authenticator_rs::{softpasskey::SoftPasskey, WebauthnAuthenticator};
use webauthn_rs::prelude::{
    Base64UrlSafeData, CreationChallengeResponse, PublicKeyCredential, RequestChallengeResponse,
};

const PASSWORD: &str = "correct horse battery staple";
const ORIGIN: &str = "http://localhost:8080";

async fn setup() -> (PgPool, Router) {
    let pool = PgPoolOptions::new()
        .connect(&std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required"))
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState::new(
        pool.clone(),
        "01234567890123456789012345678901".into(),
        vec![42; 32],
    ));
    (pool, router)
}

async fn call(
    router: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("user-agent", "accounts-test");
    if let Some(token) = bearer {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let mut request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
        None => request.body(Body::empty()).unwrap(),
    };
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([198, 51, 100, 7], 40000))));
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

fn email() -> String {
    format!("acct-{}@example.com", Uuid::new_v4().simple())
}

/// Registers an account and returns (user id, access token, refresh token).
async fn register(router: &Router, email: &str) -> (Uuid, String, String) {
    let (status, body) = call(
        router,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({"email": email, "password": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let token = body["token"].as_str().unwrap().to_owned();
    let refresh = body["refreshToken"].as_str().unwrap().to_owned();
    let (_, me) = call(router, "GET", "/api/auth/me", Some(&token), None).await;
    (me["id"].as_str().unwrap().parse().unwrap(), token, refresh)
}

fn authenticator() -> WebauthnAuthenticator<SoftPasskey> {
    WebauthnAuthenticator::new(SoftPasskey::new(true))
}

/// Adds a passkey through the API and returns its credential id.
async fn add_passkey(
    router: &Router,
    token: &str,
    auth: &mut WebauthnAuthenticator<SoftPasskey>,
    body: Value,
) -> Vec<u8> {
    let (status, start) = call(
        router,
        "POST",
        "/api/account/passkeys/register/start",
        Some(token),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{start}");
    let selection = &start["options"]["publicKey"]["authenticatorSelection"];
    assert_eq!(
        selection["residentKey"], "required",
        "usernameless sign-in needs it"
    );
    assert_eq!(selection["requireResidentKey"], true);
    // The soft authenticator cannot store resident keys; real ones (and the browser E2E with
    // a CDP virtual authenticator) honour the requirement. The server does not depend on it.
    let mut soft = start["options"].clone();
    soft["publicKey"]["authenticatorSelection"]["requireResidentKey"] = json!(false);
    soft["publicKey"]["authenticatorSelection"]["residentKey"] = json!("discouraged");
    let options: CreationChallengeResponse = serde_json::from_value(soft).unwrap();
    let credential = auth
        .do_registration(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    let raw_id = credential.raw_id.to_vec();
    let (status, finished) = call(
        router,
        "POST",
        "/api/account/passkeys/register/finish",
        Some(token),
        Some(json!({
            "challengeId": start["challengeId"],
            "name": "Test key",
            "credential": credential,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{finished}");
    assert_eq!(finished["name"], "Test key");
    raw_id
}

/// Signs a server challenge. Discoverable (usernameless) requests carry no credential list,
/// so the soft authenticator is pointed at the known key and the user handle a real
/// resident key would return is attached; neither is covered by the signature.
fn sign(
    auth: &mut WebauthnAuthenticator<SoftPasskey>,
    options: Value,
    credential_id: &[u8],
    user: Option<Uuid>,
) -> PublicKeyCredential {
    let mut options = options;
    let allowed = &mut options["publicKey"]["allowCredentials"];
    if allowed.as_array().is_none_or(Vec::is_empty) {
        *allowed = json!([{"type": "public-key", "id": URL_SAFE_NO_PAD.encode(credential_id)}]);
    }
    let options: RequestChallengeResponse = serde_json::from_value(options).unwrap();
    let mut credential = auth
        .do_authentication(Url::parse(ORIGIN).unwrap(), options)
        .unwrap();
    if let Some(user) = user {
        credential.response.user_handle = Some(Base64UrlSafeData::from(user.as_bytes().to_vec()));
    }
    credential
}

async fn step_up(
    router: &Router,
    token: &str,
    auth: &mut WebauthnAuthenticator<SoftPasskey>,
    credential_id: &[u8],
) -> StatusCode {
    let (status, start) = call(router, "POST", "/api/auth/mfa/start", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{start}");
    let credential = sign(auth, start["options"].clone(), credential_id, None);
    call(
        router,
        "POST",
        "/api/auth/mfa/finish",
        Some(token),
        Some(json!({"challengeId": start["challengeId"], "credential": credential})),
    )
    .await
    .0
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn refresh_tokens_keep_people_signed_in_and_rotate() {
    let (pool, router) = setup().await;
    let address = email();
    let (_, _, _) = register(&router, &address).await;

    let (status, login) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": address, "password": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(login["expiresIn"].as_i64().unwrap() > 0);
    let first_refresh = login["refreshToken"].as_str().unwrap().to_owned();
    assert!(first_refresh.starts_with("slrt_"));

    let (status, refreshed) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": first_refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{refreshed}");
    let access = refreshed["token"].as_str().unwrap().to_owned();
    let second_refresh = refreshed["refreshToken"].as_str().unwrap().to_owned();
    assert_ne!(second_refresh, first_refresh);
    let (status, me) = call(&router, "GET", "/api/auth/me", Some(&access), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["isAdmin"], false);
    assert_eq!(me["mfaVerified"], false);

    // A second tab racing the same rotated token is refused but does not end the session.
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": first_refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&access), None).await;
    assert_eq!(status, StatusCode::OK);

    // Replay well after rotation means the token leaked: the whole session is revoked.
    sqlx::query("UPDATE user_session_used_tokens SET used_at=now()-interval '5 minutes' WHERE token_hash=sha256($1::text::bytea)")
        .bind(&first_refresh)
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": first_refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&access), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": second_refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Expired sessions cannot be refreshed.
    let (_, login) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": address, "password": PASSWORD})),
    )
    .await;
    let refresh = login["refreshToken"].as_str().unwrap();
    sqlx::query("UPDATE user_sessions SET expires_at=now()-interval '1 second' WHERE refresh_token_hash=sha256($1::text::bytea)")
        .bind(refresh)
        .execute(&pool)
        .await
        .unwrap();
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(prune_auth_state(&pool).await.is_ok());
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn sessions_can_be_listed_and_signed_out() {
    let (_, router) = setup().await;
    let address = email();
    let (_, token, _) = register(&router, &address).await;
    let (_, other) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": address, "password": PASSWORD})),
    )
    .await;
    let other_token = other["token"].as_str().unwrap();

    let (status, sessions) =
        call(&router, "GET", "/api/account/sessions", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    let sessions = sessions.as_array().unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions.iter().filter(|s| s["current"] == true).count(), 1);
    assert_eq!(sessions[0]["userAgent"], "accounts-test");
    let other_id = sessions.iter().find(|s| s["current"] == false).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let (status, _) = call(
        &router,
        "DELETE",
        &format!("/api/account/sessions/{other_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(other_token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = call(&router, "POST", "/api/auth/logout", Some(&token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn passkeys_register_sign_in_and_step_up() {
    let (_, router) = setup().await;
    let address = email();
    let (user, token, _) = register(&router, &address).await;
    let mut auth = authenticator();

    // The first passkey needs the current password so a stolen session cannot plant one.
    let (status, _) = call(
        &router,
        "POST",
        "/api/account/passkeys/register/start",
        Some(&token),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = call(
        &router,
        "POST",
        "/api/account/passkeys/register/start",
        Some(&token),
        Some(json!({"currentPassword": "not the password"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let credential_id = add_passkey(
        &router,
        &token,
        &mut auth,
        json!({"currentPassword": PASSWORD}),
    )
    .await;

    let (_, passkeys) = call(&router, "GET", "/api/account/passkeys", Some(&token), None).await;
    assert_eq!(passkeys.as_array().unwrap().len(), 1);
    assert!(
        passkeys[0].get("passkey").is_none(),
        "key material must not leak"
    );

    // With a passkey on file, adding another needs a passkey-verified session.
    let (status, body) = call(
        &router,
        "POST",
        "/api/account/passkeys/register/start",
        Some(&token),
        Some(json!({"currentPassword": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "mfa_required");

    // Usernameless sign-in.
    let (status, start) = call(&router, "POST", "/api/auth/passkey/start", None, None).await;
    assert_eq!(status, StatusCode::OK, "{start}");
    let assertion = sign(
        &mut auth,
        start["options"].clone(),
        &credential_id,
        Some(user),
    );
    let (status, login) = call(
        &router,
        "POST",
        "/api/auth/passkey/finish",
        None,
        Some(json!({"challengeId": start["challengeId"], "credential": assertion})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{login}");
    let passkey_token = login["token"].as_str().unwrap();
    assert!(login["refreshToken"].as_str().unwrap().starts_with("slrt_"));
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(passkey_token), None).await;
    assert_eq!(me["id"], user.to_string());
    assert_eq!(me["mfaVerified"], true);

    // Challenges are single use.
    let replay = sign(
        &mut auth,
        start["options"].clone(),
        &credential_id,
        Some(user),
    );
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/passkey/finish",
        None,
        Some(json!({"challengeId": start["challengeId"], "credential": replay})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // A password session steps up to MFA with the passkey.
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(me["mfaVerified"], false);
    assert_eq!(
        step_up(&router, &token, &mut auth, &credential_id).await,
        StatusCode::NO_CONTENT
    );
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(me["mfaVerified"], true);
    assert_eq!(me["passkeyCount"], 1);

    // Another account's passkey cannot satisfy this account's challenge.
    let (_, stranger_token, _) = register(&router, &email()).await;
    let mut stranger = authenticator();
    let stranger_key = add_passkey(
        &router,
        &stranger_token,
        &mut stranger,
        json!({"currentPassword": PASSWORD}),
    )
    .await;
    let (_, start) = call(&router, "POST", "/api/auth/passkey/start", None, None).await;
    let forged = sign(
        &mut stranger,
        start["options"].clone(),
        &stranger_key,
        Some(user),
    );
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/passkey/finish",
        None,
        Some(json!({"challengeId": start["challengeId"], "credential": forged})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Removing a passkey needs a passkey check: a password alone would let a thief swap
    // the victim's passkeys for their own.
    let id = passkeys[0]["id"].as_str().unwrap().to_owned();
    let (_, other_login) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": address, "password": PASSWORD})),
    )
    .await;
    let password_only = other_login["token"].as_str().unwrap().to_owned();
    let (status, body) = call(
        &router,
        "DELETE",
        &format!("/api/account/passkeys/{id}"),
        Some(&password_only),
        Some(json!({"currentPassword": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "mfa_required");

    // A step-up challenge only verifies the session that requested it.
    let (_, start) = call(&router, "POST", "/api/auth/mfa/start", Some(&token), None).await;
    let assertion = sign(&mut auth, start["options"].clone(), &credential_id, None);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/mfa/finish",
        Some(&password_only),
        Some(json!({"challengeId": start["challengeId"], "credential": assertion})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(&password_only), None).await;
    assert_eq!(me["mfaVerified"], false);
    // A step-up started before a passkey is removed cannot finish with that passkey.
    let (_, pending) = call(
        &router,
        "POST",
        "/api/auth/mfa/start",
        Some(&password_only),
        None,
    )
    .await;
    let (status, _) = call(
        &router,
        "DELETE",
        &format!("/api/account/passkeys/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let stale = sign(&mut auth, pending["options"].clone(), &credential_id, None);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/mfa/finish",
        Some(&password_only),
        Some(json!({"challengeId": pending["challengeId"], "credential": stale})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(&password_only), None).await;
    assert_eq!(me["mfaVerified"], false);
    let (_, passkeys) = call(&router, "GET", "/api/account/passkeys", Some(&token), None).await;
    assert!(passkeys.as_array().unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn admin_portal_requires_admin_role_and_passkey_mfa() {
    let (pool, router) = setup().await;
    let admin_email = email();
    let (admin, admin_token, _) = register(&router, &admin_email).await;
    let target_email = email();
    let (target, target_token, target_refresh) = register(&router, &target_email).await;

    // Ordinary accounts are refused.
    let (status, _) = call(&router, "GET", "/api/admin/users", Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    sqlx::query("UPDATE users SET is_admin=true WHERE id=$1")
        .bind(admin)
        .execute(&pool)
        .await
        .unwrap();
    let (_, me) = call(&router, "GET", "/api/auth/me", Some(&admin_token), None).await;
    assert_eq!(me["isAdmin"], true);

    // Admins without a passkey-verified session are refused with a specific code.
    let (status, body) = call(&router, "GET", "/api/admin/users", Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "mfa_required");

    let mut auth = authenticator();
    let key = add_passkey(
        &router,
        &admin_token,
        &mut auth,
        json!({"currentPassword": PASSWORD}),
    )
    .await;
    assert_eq!(
        step_up(&router, &admin_token, &mut auth, &key).await,
        StatusCode::NO_CONTENT
    );

    // API tokens never reach the admin portal, even for an admin.
    let (_, api_token) = call(
        &router,
        "POST",
        "/api/account/tokens",
        Some(&admin_token),
        Some(json!({"name": "cli", "scopes": ["sites:read"]})),
    )
    .await;
    let (status, _) = call(
        &router,
        "GET",
        "/api/admin/users",
        api_token["token"].as_str(),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, overview) = call(
        &router,
        "GET",
        "/api/admin/overview",
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{overview}");
    assert!(overview["users"].as_i64().unwrap() >= 2);
    assert!(overview["admins"].as_i64().unwrap() >= 1);

    let (status, list) = call(
        &router,
        "GET",
        &format!("/api/admin/users?q={}", &target_email[..20]),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    assert_eq!(list["total"], 1);
    assert_eq!(list["users"][0]["email"], target_email);
    assert!(list["users"][0].get("passwordHash").is_none());

    let (status, detail) = call(
        &router,
        "GET",
        &format!("/api/admin/users/{target}"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["user"]["email"], target_email);
    assert_eq!(detail["sessions"].as_array().unwrap().len(), 1);

    // Admins cannot lock themselves out or act on other admins.
    let (status, _) = call(
        &router,
        "POST",
        &format!("/api/admin/users/{admin}/disable"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Disabling ends every session and blocks sign-in until re-enabled.
    let (status, _) = call(
        &router,
        "POST",
        &format!("/api/admin/users/{target}/disable"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&target_token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": target_refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": target_email, "password": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &router,
        "POST",
        &format!("/api/admin/users/{target}/enable"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({"email": target_email, "password": PASSWORD})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        &router,
        "POST",
        &format!("/api/admin/users/{target}/revoke-sessions"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Deletion needs the account's email typed back.
    let (status, _) = call(
        &router,
        "DELETE",
        &format!("/api/admin/users/{target}"),
        Some(&admin_token),
        Some(json!({"confirmEmail": "wrong@example.com"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &router,
        "DELETE",
        &format!("/api/admin/users/{target}"),
        Some(&admin_token),
        Some(json!({"confirmEmail": target_email.to_uppercase()})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &router,
        "GET",
        &format!("/api/admin/users/{target}"),
        Some(&admin_token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, audit) = call(&router, "GET", "/api/admin/audit", Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::OK);
    let actions: Vec<&str> = audit
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["targetEmail"] == target_email)
        .map(|entry| entry["action"].as_str().unwrap())
        .collect();
    for action in [
        "user.disable",
        "user.enable",
        "user.revoke_sessions",
        "user.delete",
    ] {
        assert!(actions.contains(&action), "missing {action} in {actions:?}");
    }

    // MFA verification ages out.
    sqlx::query(
        "UPDATE user_sessions SET mfa_verified_at=now()-interval '13 hours' WHERE user_id=$1",
    )
    .bind(admin)
    .execute(&pool)
    .await
    .unwrap();
    let (status, body) = call(&router, "GET", "/api/admin/users", Some(&admin_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "mfa_required");
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn replaying_any_rotated_refresh_token_revokes_the_session() {
    let (pool, router) = setup().await;
    let (_, _, r0) = register(&router, &email()).await;
    let mut current = r0.clone();
    let mut access = String::new();
    for _ in 0..3 {
        let (status, body) = call(
            &router,
            "POST",
            "/api/auth/refresh",
            None,
            Some(json!({"refreshToken": current})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        current = body["refreshToken"].as_str().unwrap().to_owned();
        access = body["token"].as_str().unwrap().to_owned();
    }
    // R0 was rotated out three generations ago; age the history past the grace window.
    sqlx::query(
        "UPDATE user_session_used_tokens SET used_at=now()-interval '5 minutes'
         WHERE token_hash=sha256($1::text::bytea)",
    )
    .bind(&r0)
    .execute(&pool)
    .await
    .unwrap();
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": r0})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&access), None).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the descendant session is revoked"
    );
}

#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn logout_with_only_the_refresh_token_ends_the_session() {
    let (_, router) = setup().await;
    let (_, token, refresh) = register(&router, &email()).await;
    // An expired access token cannot authenticate, so the refresh token proves the session.
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/logout",
        Some("expired.jwt.value"),
        Some(json!({"refreshToken": refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &router,
        "POST",
        "/api/auth/refresh",
        None,
        Some(json!({"refreshToken": refresh})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(&router, "GET", "/api/auth/me", Some(&token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
