//! EDINET API v2 client (documents.json + XBRL ZIP download) and helpers.
//!
//! This is the only scraping path after the migration: the search-form
//! scraper and browser_service are gone. docID discovery goes through the
//! documents.json listing API; XBRL artifacts come from the documents/{docID}
//! download API. Both run over plain HTTPS via reqwest (no browser).

pub mod client;
pub mod historical;
pub mod resolver;
pub mod storage;
