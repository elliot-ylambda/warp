use super::*;

#[test]
fn authorize_url_contains_required_params() {
    let pkce = PkceParams::generate();
    let url = authorize_url(&pkce);

    assert!(url.starts_with("https://auth.x.ai/oauth2/authorize?"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains(&format!("client_id={CLIENT_ID}")));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("scope=openid"));
    assert!(url.contains("plan=generic"));
    assert!(url.contains("referrer=warp"));
    // The redirect URI must be percent-encoded and match the registered value.
    assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A56121%2Fcallback"));
    // The CSRF state and PKCE challenge are echoed into the URL verbatim
    // (both are URL-safe base64, so no percent-encoding is applied).
    assert!(url.contains(&format!("state={}", pkce.state)));
    assert!(url.contains(&format!("code_challenge={}", pkce.challenge)));
}

#[test]
fn token_response_parses_minimal_and_full() {
    let minimal: TokenResponse =
        serde_json::from_str(r#"{"access_token":"abc"}"#).expect("minimal response should parse");
    assert_eq!(minimal.access_token, "abc");
    assert!(minimal.refresh_token.is_none());
    assert!(minimal.expires_in.is_none());

    // Unconsumed response fields (token_type, scope) are ignored by serde.
    let full: TokenResponse = serde_json::from_str(
        r#"{"access_token":"a","refresh_token":"r","token_type":"Bearer","expires_in":3600,"scope":"api:access"}"#,
    )
    .expect("full response should parse");
    assert_eq!(full.access_token, "a");
    assert_eq!(full.refresh_token.as_deref(), Some("r"));
    assert_eq!(full.expires_in, Some(3600));
}

#[test]
fn connection_closed_without_data_is_ignored() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind should succeed");
    let addr = listener.local_addr().expect("local addr should resolve");

    // Simulates a browser preconnect socket: opened and closed with no request.
    drop(TcpStream::connect(addr).expect("connect should succeed"));

    let (stream, _) = listener.accept().expect("accept should succeed");
    let result = handle_callback_connection(stream).expect("empty connection should not error");
    assert!(result.is_none());
}

#[test]
fn callback_request_split_across_writes_is_parsed() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind should succeed");
    let addr = listener.local_addr().expect("local addr should resolve");

    let client = std::thread::spawn(move || {
        let mut client = TcpStream::connect(addr).expect("connect should succeed");
        client
            .write_all(b"GET /callback?code=abc&st")
            .expect("first write should succeed");
        client.flush().expect("flush should succeed");
        std::thread::sleep(Duration::from_millis(50));
        client
            .write_all(b"ate=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .expect("second write should succeed");
        client.flush().expect("flush should succeed");
        let mut response = String::new();
        let _ = client.read_to_string(&mut response);
        response
    });

    let (stream, _) = listener.accept().expect("accept should succeed");
    let data = handle_callback_connection(stream)
        .expect("split request should not error")
        .expect("split request should parse as a callback");
    assert_eq!(data.code, "abc");
    assert_eq!(data.state, "xyz");

    let response = client.join().expect("client thread should not panic");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
}
