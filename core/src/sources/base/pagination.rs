//! Generic pagination support for API responses
//!
//! Provides a trait-based abstraction for paginating through API results,
//! supporting different pagination styles (token-based, cursor-based, etc.).

use crate::error::Result;
use std::future::Future;

/// Trait for API responses that support pagination
pub trait Paginated {
    /// Get the next page token/cursor (None = no more pages)
    fn next_page_token(&self) -> Option<String>;

    /// Check if there are more pages
    /// Default implementation checks if next_page_token exists
    fn has_more(&self) -> bool {
        self.next_page_token().is_some()
    }
}

/// Helper for paginating through API responses
///
/// # Example
///
/// ```rust,ignore
/// let paginator = paginate(|page_token| async move {
///     client.fetch_messages(page_token).await
/// });
///
/// let all_messages = paginator
///     .collect_all(|response| response.messages)
///     .await?;
/// ```
pub struct Paginator<F, Fut, T>
where
    F: Fn(Option<String>) -> Fut,
    Fut: Future<Output = Result<T>>,
    T: Paginated,
{
    fetch_fn: F,
    current_token: Option<String>,
    exhausted: bool,
}

impl<F, Fut, T> Paginator<F, Fut, T>
where
    F: Fn(Option<String>) -> Fut,
    Fut: Future<Output = Result<T>>,
    T: Paginated,
{
    /// Create a new paginator with a fetch function
    pub fn new(fetch_fn: F) -> Self {
        Self {
            fetch_fn,
            current_token: None,
            exhausted: false,
        }
    }

    /// Fetch the next page
    ///
    /// Returns `Ok(Some(response))` if there's a page to fetch,
    /// `Ok(None)` if pagination is exhausted,
    /// or `Err` if the fetch fails.
    pub async fn next_page(&mut self) -> Result<Option<T>> {
        if self.exhausted {
            return Ok(None);
        }

        let response = (self.fetch_fn)(self.current_token.clone()).await?;

        if let Some(token) = response.next_page_token() {
            self.current_token = Some(token);
        } else {
            self.exhausted = true;
        }

        Ok(Some(response))
    }

    /// Fetch all pages and collect items using an extractor function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let all_messages = paginator
    ///     .collect_all(|response| {
    ///         response.messages.clone().unwrap_or_default()
    ///     })
    ///     .await?;
    /// ```
    pub async fn collect_all<I, E>(mut self, extract_items: E) -> Result<Vec<I>>
    where
        E: Fn(&T) -> Vec<I>,
    {
        let mut all_items = Vec::new();

        while let Some(response) = self.next_page().await? {
            all_items.extend(extract_items(&response));
        }

        Ok(all_items)
    }

    /// Fetch all pages and process each page with a callback
    ///
    /// Useful when you want to process each page individually
    /// rather than collecting all items in memory.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut count = 0;
    /// paginator
    ///     .for_each(|response| async {
    ///         count += response.messages.len();
    ///         Ok(())
    ///     })
    ///     .await?;
    /// ```
    pub async fn for_each<F2, Fut2>(mut self, mut process_fn: F2) -> Result<()>
    where
        F2: FnMut(T) -> Fut2,
        Fut2: Future<Output = Result<()>>,
    {
        while let Some(response) = self.next_page().await? {
            process_fn(response).await?;
        }

        Ok(())
    }
}

/// Helper function to create a paginator
///
/// # Example
///
/// ```rust,ignore
/// let all_messages = paginate(|page_token| async move {
///     client.list_messages(page_token).await
/// })
/// .collect_all(|response| response.messages.clone().unwrap_or_default())
/// .await?;
/// ```
pub fn paginate<F, Fut, T>(fetch_fn: F) -> Paginator<F, Fut, T>
where
    F: Fn(Option<String>) -> Fut,
    Fut: Future<Output = Result<T>>,
    T: Paginated,
{
    Paginator::new(fetch_fn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestResponse {
        items: Vec<String>,
        next_token: Option<String>,
    }

    impl Paginated for TestResponse {
        fn next_page_token(&self) -> Option<String> {
            self.next_token.clone()
        }
    }

    #[tokio::test]
    async fn test_paginator_single_page() {
        let paginator = paginate(|_token: Option<String>| async {
            Ok(TestResponse {
                items: vec!["item1".to_string(), "item2".to_string()],
                next_token: None,
            })
        });

        let all_items = paginator
            .collect_all(|response| response.items.clone())
            .await
            .unwrap();

        assert_eq!(all_items.len(), 2);
        assert_eq!(all_items[0], "item1");
        assert_eq!(all_items[1], "item2");
    }

    #[tokio::test]
    async fn test_paginator_multiple_pages() {
        let paginator = paginate(|token: Option<String>| async move {
            match token.as_deref() {
                None => Ok(TestResponse {
                    items: vec!["page1_item1".to_string(), "page1_item2".to_string()],
                    next_token: Some("token1".to_string()),
                }),
                Some("token1") => Ok(TestResponse {
                    items: vec!["page2_item1".to_string(), "page2_item2".to_string()],
                    next_token: Some("token2".to_string()),
                }),
                Some("token2") => Ok(TestResponse {
                    items: vec!["page3_item1".to_string()],
                    next_token: None,
                }),
                _ => panic!("Unexpected token"),
            }
        });

        let all_items = paginator
            .collect_all(|response| response.items.clone())
            .await
            .unwrap();

        assert_eq!(all_items.len(), 5);
        assert_eq!(all_items[0], "page1_item1");
        assert_eq!(all_items[4], "page3_item1");
    }

    #[tokio::test]
    async fn test_paginator_for_each() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let page_count = Arc::new(Mutex::new(0));
        let total_items = Arc::new(Mutex::new(0));

        let paginator = paginate(|token: Option<String>| async move {
            match token.as_deref() {
                None => Ok(TestResponse {
                    items: vec!["item1".to_string(), "item2".to_string()],
                    next_token: Some("token1".to_string()),
                }),
                Some("token1") => Ok(TestResponse {
                    items: vec!["item3".to_string()],
                    next_token: None,
                }),
                _ => panic!("Unexpected token"),
            }
        });

        let page_count_clone = page_count.clone();
        let total_items_clone = total_items.clone();

        paginator
            .for_each(|response| {
                let items_len = response.items.len();
                let page_count = page_count_clone.clone();
                let total_items = total_items_clone.clone();
                async move {
                    *page_count.lock().await += 1;
                    *total_items.lock().await += items_len;
                    Ok(())
                }
            })
            .await
            .unwrap();

        assert_eq!(*page_count.lock().await, 2);
        assert_eq!(*total_items.lock().await, 3);
    }

    #[tokio::test]
    async fn test_has_more_default_impl() {
        let response_with_token = TestResponse {
            items: vec!["item1".to_string()],
            next_token: Some("token".to_string()),
        };

        let response_without_token = TestResponse {
            items: vec!["item1".to_string()],
            next_token: None,
        };

        assert!(response_with_token.has_more());
        assert!(!response_without_token.has_more());
    }
}
