use futures_util::StreamExt;
use select::document::Document;
use select::predicate::Name;
use std::time::Duration;

const USER_AGENT: &str = "bmm-description-fetcher";
pub const DEFAULT_FETCH_TIMEOUT_SECS: u16 = 8;

#[derive(thiserror::Error, Debug)]
pub enum FetchDescriptionError {
    #[error("couldn't set up http client: {0}")]
    CouldntBuildHttpClient(reqwest::Error),
    #[error("couldn't reach \"{0}\": {1}")]
    CouldntFetchPage(String, reqwest::Error),
    #[error("\"{0}\" returned a non-success status: {1}")]
    NonSuccessStatus(String, reqwest::StatusCode),
    #[error("couldn't read response body for \"{0}\": {1}")]
    CouldntReadBody(String, reqwest::Error),
}

/// Metadata bmm was able to scrape out of a page's `<head>`.
#[derive(Debug, Clone, Default)]
pub struct PageMetadata {
    // Parsed and covered by tests below, but no longer read outside this
    // module now that the title-fallback behavior has been removed - kept
    // around since it's cheap to extract and may be useful again later.
    #[allow(dead_code)]
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Fetches `uri` and pulls out whatever title/description metadata is
/// present in its HTML. Used by both `bmm fetch <URI>` (CLI) and the TUI's
/// Alt+F ("fetch description into the Title field") shortcut in the
/// add/edit bookmark screen.
pub async fn fetch_page_metadata(
    uri: &str,
    timeout_secs: u16,
) -> Result<PageMetadata, FetchDescriptionError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1) as u64))
        .user_agent(USER_AGENT)
        .build()
        .map_err(FetchDescriptionError::CouldntBuildHttpClient)?;

    let response = client
        .get(uri)
        .send()
        .await
        .map_err(|e| FetchDescriptionError::CouldntFetchPage(uri.to_string(), e))?;

    if !response.status().is_success() {
        return Err(FetchDescriptionError::NonSuccessStatus(
            uri.to_string(),
            response.status(),
        ));
    }

    let head_html = read_head_only(response, uri).await?;

    Ok(parse_page_metadata(&head_html))
}

/// Title/description metadata always lives inside a page's `<head>`, so
/// there's no need to wait for (and download) the rest of the page - the
/// body can be images, scripts, huge amounts of markup, etc. This streams
/// the response and stops as soon as it has seen a closing `</head>` tag,
/// which is what makes `bmm fetch`/Alt+F feel fast even on heavy pages.
///
/// As a safety net for pages with a missing/malformed `</head>` (or no
/// `<head>` at all), reading also stops once `MAX_HEAD_BYTES` have come in,
/// so a pathological page can't stall the fetch until the timeout hits.
const MAX_HEAD_BYTES: usize = 256 * 1024;

async fn read_head_only(
    response: reqwest::Response,
    uri: &str,
) -> Result<String, FetchDescriptionError> {
    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| FetchDescriptionError::CouldntReadBody(uri.to_string(), e))?;
        buf.extend_from_slice(&chunk);

        if buf.len() >= MAX_HEAD_BYTES || contains_ignore_case(&buf, b"</head") {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Case-insensitive byte search, used to spot `</head` as soon as it shows
/// up in whatever's been streamed in so far.
fn contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

/// Pulls `<title>` and a description (preferring `<meta name="description">`,
/// falling back to `og:description`, then `twitter:description`) out of a
/// page's HTML.
pub fn parse_page_metadata(html: &str) -> PageMetadata {
    let document = Document::from(html);

    let title = document
        .find(Name("title"))
        .next()
        .map(|n| n.text().trim().to_string())
        .filter(|t| !t.is_empty());

    let mut description: Option<String> = None;
    let mut og_description: Option<String> = None;
    let mut twitter_description: Option<String> = None;

    for node in document.find(Name("meta")) {
        let content = node
            .attr("content")
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty());

        let content = match content {
            Some(c) => c,
            None => continue,
        };

        let name_attr = node.attr("name").map(|s| s.to_ascii_lowercase());
        let property_attr = node.attr("property").map(|s| s.to_ascii_lowercase());

        match (name_attr.as_deref(), property_attr.as_deref()) {
            (Some("description"), _) if description.is_none() => {
                description = Some(content);
            }
            (_, Some("og:description")) if og_description.is_none() => {
                og_description = Some(content);
            }
            (Some("twitter:description"), _) if twitter_description.is_none() => {
                twitter_description = Some(content);
            }
            _ => {}
        }
    }

    let description = description.or(og_description).or(twitter_description);

    PageMetadata { title, description }
}

/// Implements `bmm fetch <URI>`: fetches the page's metadata and prints its
/// description (the same text a search engine typically shows under a
/// link). If the page has no description, this prints nothing at all -
/// there's no fallback to the page title and no "not found" message.
pub async fn fetch_description(uri: &str, timeout_secs: u16) -> Result<(), FetchDescriptionError> {
    let uri = crate::domain::normalize_uri_scheme(uri.to_string());

    let metadata = fetch_page_metadata(&uri, timeout_secs).await?;

    if let Some(description) = &metadata.description {
        println!("{description}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_meta_description_works() {
        // GIVEN
        let html = r#"
<html>
<head>
<title>Example Domain</title>
<meta name="description" content="This domain is for use in illustrative examples.">
</head>
<body></body>
</html>
"#;

        // WHEN
        let metadata = parse_page_metadata(html);

        // THEN
        assert_eq!(metadata.title.as_deref(), Some("Example Domain"));
        assert_eq!(
            metadata.description.as_deref(),
            Some("This domain is for use in illustrative examples.")
        );
    }

    #[test]
    fn falls_back_to_og_description_when_meta_description_missing() {
        // GIVEN
        let html = r#"
<html>
<head>
<title>Example Domain</title>
<meta property="og:description" content="An OG description.">
</head>
<body></body>
</html>
"#;

        // WHEN
        let metadata = parse_page_metadata(html);

        // THEN
        assert_eq!(metadata.description.as_deref(), Some("An OG description."));
    }

    #[test]
    fn returns_none_when_no_description_present() {
        // GIVEN
        let html = r#"
<html>
<head>
<title>Example Domain</title>
</head>
<body></body>
</html>
"#;

        // WHEN
        let metadata = parse_page_metadata(html);

        // THEN
        assert_eq!(metadata.title.as_deref(), Some("Example Domain"));
        assert!(metadata.description.is_none());
    }
}
