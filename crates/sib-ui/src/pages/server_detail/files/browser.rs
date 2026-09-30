use std::collections::{HashMap, HashSet};

use sib_core::QueryRequest;
use sib_modules::files::{self, Listing};

const SEARCH_ERROR_KEY: &str = "search";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    List(String),
    Read(String),
    Search(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub pattern: String,
    pub listing: Listing,
}

#[derive(Debug, Default)]
pub struct FileBrowser {
    search: Option<SearchResult>,
    listings: HashMap<String, Listing>,
    contents: HashMap<String, String>,
    errors: HashMap<String, String>,
    pending: HashMap<u64, Pending>,
    loading: HashSet<String>,
}

impl FileBrowser {
    pub fn listing(&self, path: &str) -> Option<&Listing> {
        self.listings.get(path)
    }

    pub fn content(&self, path: &str) -> Option<&String> {
        self.contents.get(path)
    }

    pub fn search(&self) -> Option<&SearchResult> {
        self.search.as_ref()
    }

    pub fn is_searching(&self) -> bool {
        self.pending
            .values()
            .any(|p| matches!(p, Pending::Search(_)))
    }

    pub fn clear_search(&mut self) {
        self.search = None;
        self.errors.remove(SEARCH_ERROR_KEY);
    }

    pub fn search_error(&self) -> Option<&String> {
        self.errors.get(SEARCH_ERROR_KEY)
    }

    pub fn error(&self, path: &str) -> Option<&String> {
        self.errors.get(path)
    }

    pub fn is_loading(&self, path: &str) -> bool {
        self.loading.contains(path)
    }

    pub fn has_pending(&self, token: u64) -> bool {
        self.pending.contains_key(&token)
    }

    pub fn start(&mut self, token: u64, request: &QueryRequest) {
        let pending = match request.kind.as_str() {
            files::QUERY_LIST => Pending::List(request.target.clone()),
            files::QUERY_READ => Pending::Read(request.target.clone()),
            files::QUERY_SEARCH => {
                self.errors.remove(SEARCH_ERROR_KEY);
                self.pending
                    .insert(token, Pending::Search(request.target.clone()));
                return;
            }
            _ => return,
        };
        self.loading.insert(request.target.clone());
        self.errors.remove(&request.target);
        self.pending.insert(token, pending);
    }

    pub fn accept(&mut self, token: u64, result: Result<String, String>) {
        let Some(pending) = self.pending.remove(&token) else {
            return;
        };
        match pending {
            Pending::Search(pattern) => self.accept_search(pattern, result),
            Pending::List(path) => {
                self.loading.remove(&path);
                let parsed = result.and_then(|text| {
                    files::parse_listing(&text).map_err(|error| error.to_string())
                });
                match parsed {
                    Ok(listing) => {
                        self.listings.insert(path, listing);
                    }
                    Err(error) => {
                        self.errors.insert(path, error);
                    }
                }
            }
            Pending::Read(path) => {
                self.loading.remove(&path);
                match result {
                    Ok(text) => {
                        self.contents.insert(path, text);
                    }
                    Err(error) => {
                        self.errors.insert(path, error);
                    }
                }
            }
        }
    }

    fn accept_search(&mut self, pattern: String, result: Result<String, String>) {
        match result.and_then(|text| files::parse_listing(&text).map_err(|e| e.to_string())) {
            Ok(listing) => self.search = Some(SearchResult { pattern, listing }),
            Err(error) => {
                self.errors.insert(SEARCH_ERROR_KEY.to_owned(), error);
            }
        }
    }

    pub fn forget_content(&mut self, path: &str) {
        self.contents.remove(path);
    }

    pub fn forget_subtree(&mut self, path: &str) {
        let prefix = format!("{}/", path.trim_end_matches('/'));
        self.listings
            .retain(|key, _| key != path && !key.starts_with(&prefix));
        self.contents
            .retain(|key, _| key != path && !key.starts_with(&prefix));
        self.errors
            .retain(|key, _| key != path && !key.starts_with(&prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_list_parses_and_clears_loading() {
        let mut browser = FileBrowser::default();
        browser.start(1, &QueryRequest::new(files::QUERY_LIST, "/"));
        assert!(browser.is_loading("/"));
        browser.accept(
            1,
            Ok("d\td\t755\troot\troot\t0\t1789480000.0\t\tetc\n".to_owned()),
        );
        assert!(!browser.is_loading("/"));
        assert_eq!(browser.listing("/").map(|l| l.entries.len()), Some(1));
    }

    #[test]
    fn accept_error_records_message_for_path() {
        let mut browser = FileBrowser::default();
        browser.start(2, &QueryRequest::new(files::QUERY_READ, "/etc/shadow"));
        browser.accept(2, Err("нет доступа".to_owned()));
        assert_eq!(
            browser.error("/etc/shadow").map(String::as_str),
            Some("нет доступа")
        );
    }

    #[test]
    fn unknown_token_is_ignored() {
        let mut browser = FileBrowser::default();
        browser.accept(9, Ok(String::new()));
        assert!(browser.listing("/").is_none());
    }

    #[test]
    fn search_result_is_parsed_and_cleared() {
        let mut browser = FileBrowser::default();
        browser.start(3, &QueryRequest::new(files::QUERY_SEARCH, "hosts"));
        assert!(browser.is_searching());
        browser.accept(
            3,
            Ok("f\tf\t644\troot\troot\t200\t1789480000.0\t\t/etc/hosts\n".to_owned()),
        );
        assert!(!browser.is_searching());
        let result = browser.search().expect("search");
        assert_eq!(result.pattern, "hosts");
        assert_eq!(result.listing.entries[0].name, "/etc/hosts");
        browser.clear_search();
        assert!(browser.search().is_none());
    }

    #[test]
    fn forget_subtree_drops_nested_paths_only() {
        let mut browser = FileBrowser::default();
        browser.listings.insert(
            "/srv".into(),
            Listing {
                entries: vec![],
                is_truncated: false,
            },
        );
        browser.listings.insert(
            "/srv/app".into(),
            Listing {
                entries: vec![],
                is_truncated: false,
            },
        );
        browser.listings.insert(
            "/srvx".into(),
            Listing {
                entries: vec![],
                is_truncated: false,
            },
        );
        browser.forget_subtree("/srv");
        assert!(browser.listing("/srv").is_none());
        assert!(browser.listing("/srv/app").is_none());
        assert!(browser.listing("/srvx").is_some());
    }
}
