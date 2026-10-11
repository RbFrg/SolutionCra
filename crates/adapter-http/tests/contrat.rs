//! C1 : le document OpenAPI est figé par un snapshot ; toute modification
//! d'API apparaît dans le diff de la pull request.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn openapi_correspond_au_snapshot() {
    let document = serde_json::to_value(adapter_http::openapi()).unwrap();
    insta::assert_json_snapshot!("openapi", document);
}
