use reqwest;
use std::time::Duration;

// Test 1: Try creating a client and doing a fetch from the RUNTIME static.
#[test]
fn test_runtime_can_fetch_via_static() {
    eeman::RUNTIME.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client build");

        let resp = client
            .get("https://api.alquran.cloud/v1/surah")
            .send()
            .await
            .expect("send should succeed");

        assert!(resp.status().is_success(), "HTTP {}", resp.status());
    });
}

// Test 2: spawn + mpsc pattern (mimics the actual app flow)
#[test]
fn test_runtime_spawn_pattern() {
    let (tx, rx) = std::sync::mpsc::channel::<Result<u16, String>>();

    eeman::RUNTIME.spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(e) => { let _ = tx.send(Err(e.to_string())); return; }
        };
        match client
            .get("https://api.alquran.cloud/v1/surah")
            .send()
            .await
        {
            Ok(resp) => { let _ = tx.send(Ok(resp.status().as_u16())); }
            Err(e) => { let _ = tx.send(Err(e.to_string())); }
        }
    });

    let status = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("timed out waiting for response");

    let status = status.expect("fetch should succeed");
    assert_eq!(status, 200, "Expected 200 OK, got {}", status);
}
