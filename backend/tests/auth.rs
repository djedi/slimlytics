use slimlytics_backend::auth::{
    generate_api_token, generate_refresh_token, hash_api_token, hash_password, issue_session_token,
    issue_token, verify_password, verify_token,
};
use uuid::Uuid;

#[test]
fn password_hashes_are_argon2_and_verify() {
    let hash = hash_password("correct horse battery staple").unwrap();
    assert!(hash.starts_with("$argon2"));
    assert!(verify_password("correct horse battery staple", &hash).unwrap());
    assert!(!verify_password("wrong", &hash).unwrap());
}

#[test]
fn jwt_round_trip_rejects_wrong_secret() {
    let id = Uuid::new_v4();
    let token = issue_token(id, "secret", 60).unwrap();
    let claims = verify_token(&token, "secret").unwrap();
    assert_eq!(claims.sub, id);
    assert_eq!(claims.exp - claims.iat, 60);
    assert!(verify_token(&token, "other").is_err());
}

#[test]
fn personal_access_tokens_are_random_and_hashable() {
    let first = generate_api_token();
    let second = generate_api_token();
    assert!(first.starts_with("slyt_"));
    assert_eq!(first.len(), 48);
    assert_ne!(first, second);
    assert_eq!(hash_api_token(&first), hash_api_token(&first));
    assert_ne!(hash_api_token(&first), hash_api_token(&second));
    assert_eq!(hash_api_token(&first).len(), 32);
}

#[test]
fn session_tokens_carry_the_session_id() {
    let user = Uuid::new_v4();
    let session = Uuid::new_v4();
    let token = issue_session_token(user, session, "secret", 60).unwrap();
    let claims = verify_token(&token, "secret").unwrap();
    assert_eq!(claims.sub, user);
    assert_eq!(claims.sid, Some(session));
    let legacy = verify_token(&issue_token(user, "secret", 60).unwrap(), "secret").unwrap();
    assert_eq!(legacy.sid, None);
}

#[test]
fn refresh_tokens_are_random_and_distinct_from_api_tokens() {
    let first = generate_refresh_token();
    assert!(first.starts_with("slrt_"));
    assert_eq!(first.len(), 48);
    assert_ne!(first, generate_refresh_token());
}
