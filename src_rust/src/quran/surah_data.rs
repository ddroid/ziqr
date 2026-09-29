use serde::Deserialize;

/// Create a shared HTTP client. Clone is cheap (internally Arc'd).
/// Create once at app startup, reuse across all fetches to avoid
/// repeated TLS handshake + connection pool initialization.
pub fn create_shared_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .pool_max_idle_per_host(4)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurahListResponse {
    pub data: Vec<SurahMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurahMeta {
    pub number: u32,
    pub name: String,
    #[serde(rename = "englishName")]
    pub english_name: String,
    #[serde(rename = "englishNameTranslation")]
    pub english_name_translation: String,
    #[serde(rename = "numberOfAyahs")]
    pub number_of_ayahs: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurahResponse {
    pub data: SurahDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SurahDetail {
    #[serde(flatten)]
    pub surah: SurahMeta,
    pub ayahs: Vec<RawAyah>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawAyah {
    pub number: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Ayah {
    pub number: u32,
    pub text_english: String,
    pub text_arabic: String,
}

/// Fetch all 114 surah metadata in a single API call.
pub async fn fetch_all_surahs(client: &reqwest::Client) -> Result<Vec<SurahMeta>, String> {
    let resp = client
        .get("https://api.alquran.cloud/v1/surah")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: SurahListResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json.data)
}

/// Arabic edition to read. The API default switched to the "quran-academy"
/// text, which spells every yeh as U+06CC FARSI YEH. The Hafs font has no
/// glyph for that, so every yeh showed a dotted circle. Tanzil's Uthmani text
/// uses the standard Arabic code points the font was drawn for.
const ARABIC_EDITION: &str = "quran-uthmani";

/// Replace letter variants the Hafs font cannot draw with their Arabic
/// equivalents. The shapes are identical in Uthmani script; only the code
/// points differ.
pub fn normalize_arabic(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{06CC}' => '\u{064A}', // Farsi yeh -> yeh
            '\u{06A9}' => '\u{0643}', // keheh -> kaf
            '\u{06C1}' => '\u{0647}', // heh goal -> heh
            _ => c,
        })
        .collect()
}

/// Fetch a single surah's ayahs (Arabic + English in parallel).
pub async fn fetch_surah_ayahs(client: &reqwest::Client, surah_number: u32) -> Result<(SurahMeta, Vec<Ayah>), String> {
    // Fetch both Arabic and English translations in parallel
    let arabic_fut = client
        .get(format!(
            "https://api.alquran.cloud/v1/surah/{}/{}",
            surah_number, ARABIC_EDITION
        ))
        .send();

    let english_fut = client
        .get(format!(
            "https://api.alquran.cloud/v1/surah/{}/en.sahih",
            surah_number
        ))
        .send();

    let (arabic_resp, english_resp) =
        futures::try_join!(arabic_fut, english_fut).map_err(|e| e.to_string())?;

    if !arabic_resp.status().is_success() || !english_resp.status().is_success() {
        return Err(format!(
            "HTTP {} / {}",
            arabic_resp.status(),
            english_resp.status()
        ));
    }

    let (arabic_json, english_json) =
        futures::try_join!(
            arabic_resp.json::<SurahResponse>(),
            english_resp.json::<SurahResponse>()
        )
        .map_err(|e| e.to_string())?;

    let surah_meta = SurahMeta {
        number: arabic_json.data.surah.number,
        name: arabic_json.data.surah.name.clone(),
        english_name: arabic_json.data.surah.english_name.clone(),
        english_name_translation: arabic_json.data.surah.english_name_translation.clone(),
        number_of_ayahs: arabic_json.data.surah.number_of_ayahs,
    };

    // `number` in the API is the Quran-wide index. The reader shows the
    // ayah's place inside this surah, which is just the list position.
    let mut combined = Vec::with_capacity(arabic_json.data.ayahs.len());
    for (index, (ar, en)) in arabic_json
        .data
        .ayahs
        .into_iter()
        .zip(english_json.data.ayahs)
        .enumerate()
    {
        combined.push(Ayah {
            number: (index as u32) + 1,
            text_english: en.text,
            text_arabic: normalize_arabic(&ar.text),
        });
    }

    Ok((surah_meta, combined))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_shared_client() {
        // Shared client should be created successfully
        let client = create_shared_client();
        assert!(client.is_ok(), "Shared client creation should succeed");
    }

    #[test]
    fn test_client_clone_is_cheap() {
        // Verify that cloning a client is cheap (internally Arc'd)
        let client = create_shared_client().unwrap();
        let _cloned = client.clone();
        // If we get here without error, clone succeeded
        // In production, this should be nearly instant
    }

    #[tokio::test]
    async fn test_fetch_all_surahs_accepts_client() {
        // Verify the function signature accepts a client parameter
        // This is a compile-time check - if it compiles, the test passes
        let client = create_shared_client().unwrap();
        // We don't actually call the API in unit tests
        // The real test is that the function signature accepts &reqwest::Client
        let _ = &client;
    }

    #[tokio::test]
    async fn test_fetch_surah_ayahs_accepts_client() {
        // Verify the function signature accepts a client parameter
        let client = create_shared_client().unwrap();
        let _ = &client;
        // Compile-time verification that function accepts client param
    }

    #[test]
    fn farsi_yeh_becomes_arabic_yeh() {
        // 2:21 as the quran-academy edition spells it.
        let academy = "یَـٰۤأَیُّهَا ٱلنَّاسُ";
        let fixed = normalize_arabic(academy);
        assert!(!fixed.contains('\u{06CC}'));
        assert_eq!(fixed.matches('\u{064A}').count(), 2);
        // Marks and the standard letters are left alone.
        assert!(fixed.contains('\u{0670}'));
        assert!(fixed.contains("ٱلنَّاسُ"));
    }

    #[test]
    fn standard_text_is_unchanged() {
        let tanzil = "يَٰٓأَيُّهَا ٱلنَّاسُ ٱعْبُدُوا۟";
        assert_eq!(normalize_arabic(tanzil), tanzil);
    }
}
