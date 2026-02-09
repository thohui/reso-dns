#[cfg(test)]
mod tests {
    use super::super::pagination::{PagedQuery, PagedResponse};
    use serde::Serialize;

    #[derive(Serialize, Debug, PartialEq)]
    struct TestItem {
        id: usize,
        name: String,
    }

    #[test]
    fn test_paged_query_default_skip() {
        let query = PagedQuery {
            skip: None,
            top: Some(10),
        };
        assert_eq!(query.skip(), 0);
        assert_eq!(query.top(), 10);
    }

    #[test]
    fn test_paged_query_default_top() {
        let query = PagedQuery {
            skip: Some(5),
            top: None,
        };
        assert_eq!(query.skip(), 5);
        assert_eq!(query.top(), 25);
    }

    #[test]
    fn test_paged_query_custom_values() {
        let query = PagedQuery {
            skip: Some(100),
            top: Some(50),
        };
        assert_eq!(query.skip(), 100);
        assert_eq!(query.top(), 50);
    }

    #[test]
    fn test_paged_query_all_none() {
        let query = PagedQuery {
            skip: None,
            top: None,
        };
        assert_eq!(query.skip(), 0);
        assert_eq!(query.top(), 25);
    }

    #[test]
    fn test_paged_response_no_more_pages() {
        let items = vec![
            TestItem { id: 1, name: "Item 1".to_string() },
            TestItem { id: 2, name: "Item 2".to_string() },
        ];
        let response = PagedResponse::new(items, 2, 10, 0);

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 2);
        assert_eq!(response.top, 10);
        assert_eq!(response.skip, 0);
        assert_eq!(response.next_offset, 2);
        assert!(!response.has_more);
    }

    #[test]
    fn test_paged_response_has_more() {
        let items = vec![
            TestItem { id: 1, name: "Item 1".to_string() },
            TestItem { id: 2, name: "Item 2".to_string() },
        ];
        let response = PagedResponse::new(items, 10, 2, 0);

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 10);
        assert_eq!(response.top, 2);
        assert_eq!(response.skip, 0);
        assert_eq!(response.next_offset, 2);
        assert!(response.has_more);
    }

    #[test]
    fn test_paged_response_middle_page() {
        let items = vec![
            TestItem { id: 3, name: "Item 3".to_string() },
            TestItem { id: 4, name: "Item 4".to_string() },
        ];
        let response = PagedResponse::new(items, 10, 2, 2);

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 10);
        assert_eq!(response.top, 2);
        assert_eq!(response.skip, 2);
        assert_eq!(response.next_offset, 4);
        assert!(response.has_more);
    }

    #[test]
    fn test_paged_response_last_page() {
        let items = vec![
            TestItem { id: 9, name: "Item 9".to_string() },
            TestItem { id: 10, name: "Item 10".to_string() },
        ];
        let response = PagedResponse::new(items, 10, 2, 8);

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 10);
        assert_eq!(response.skip, 8);
        assert_eq!(response.next_offset, 10);
        assert!(!response.has_more);
    }

    #[test]
    fn test_paged_response_empty() {
        let items: Vec<TestItem> = vec![];
        let response = PagedResponse::new(items, 0, 10, 0);

        assert_eq!(response.items.len(), 0);
        assert_eq!(response.total, 0);
        assert_eq!(response.next_offset, 0);
        assert!(!response.has_more);
    }

    #[test]
    fn test_paged_response_serialization() {
        let items = vec![TestItem { id: 1, name: "Test".to_string() }];
        let response = PagedResponse::new(items, 1, 10, 0);

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total"], 1);
        assert_eq!(json["top"], 10);
        assert_eq!(json["skip"], 0);
        assert_eq!(json["has_more"], false);
        assert_eq!(json["next_offset"], 1);
        assert!(json["items"].is_array());
    }

    #[test]
    fn test_paged_response_partial_last_page() {
        let items = vec![TestItem { id: 10, name: "Item 10".to_string() }];
        let response = PagedResponse::new(items, 10, 3, 9);

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.total, 10);
        assert_eq!(response.skip, 9);
        assert_eq!(response.next_offset, 10);
        assert!(!response.has_more);
    }
}