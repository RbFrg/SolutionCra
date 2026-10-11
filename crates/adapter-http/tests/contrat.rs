//! C1 : le document OpenAPI est figé par un snapshot ; toute modification
//! d'API apparaît dans le diff de la pull request.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn chaque_operation_a_un_identifiant_unique() {
    // Deux operationId identiques mélangent les types générés côté TypeScript.
    let document = serde_json::to_value(adapter_http::openapi()).unwrap();
    let mut ids: Vec<String> = document["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|methodes| methodes.as_object().unwrap().values())
        .filter_map(|op| op["operationId"].as_str().map(str::to_owned))
        .collect();
    let total = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), total);
}

#[test]
fn openapi_correspond_au_snapshot() {
    let document = serde_json::to_value(adapter_http::openapi()).unwrap();
    insta::assert_json_snapshot!("openapi", document);
}
