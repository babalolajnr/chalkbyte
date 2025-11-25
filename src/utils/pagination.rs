use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => s.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMeta {
    pub total: i64,
    pub limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub limit: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub offset: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub page: Option<i64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: Some(10),
            offset: Some(0),
            page: Some(1),
        }
    }
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).max(1).min(100)
    }

    pub fn offset(&self) -> i64 {
        // If page is provided, calculate offset from page
        if let Some(page) = self.page {
            let page = page.max(1);
            let limit = self.limit();
            (page - 1) * limit
        } else {
            self.offset.unwrap_or(0).max(0)
        }
    }

    pub fn page(&self) -> Option<i64> {
        self.page.map(|p| p.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(0));
    }

    #[test]
    fn test_pagination_params_limit_method_default() {
        let params = PaginationParams::default();
        assert_eq!(params.limit(), 10);
    }

    #[test]
    fn test_pagination_params_offset_method_default() {
        let params = PaginationParams::default();
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_custom_values() {
        let params = PaginationParams {
            limit: Some(20),
            offset: Some(40),
            page: None,
        };
        assert_eq!(params.limit(), 20);
        assert_eq!(params.offset(), 40);
    }

    #[test]
    fn test_pagination_params_limit_min_boundary() {
        let params = PaginationParams {
            limit: Some(0),
            offset: Some(0),
            page: None,
        };
        assert_eq!(params.limit(), 1);
    }

    #[test]
    fn test_pagination_params_limit_max_boundary() {
        let params = PaginationParams {
            limit: Some(150),
            offset: Some(0),
            page: None,
        };
        assert_eq!(params.limit(), 100);
    }

    #[test]
    fn test_pagination_params_limit_negative() {
        let params = PaginationParams {
            limit: Some(-10),
            offset: Some(0),
            page: None,
        };
        assert_eq!(params.limit(), 1);
    }

    #[test]
    fn test_pagination_params_offset_negative() {
        let params = PaginationParams {
            limit: Some(10),
            offset: Some(-5),
            page: None,
        };
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_none_values() {
        let params = PaginationParams {
            limit: None,
            offset: None,
            page: None,
        };
        assert_eq!(params.limit(), 10);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_limit_exact_max() {
        let params = PaginationParams {
            limit: Some(100),
            offset: Some(0),
            page: None,
        };
        assert_eq!(params.limit(), 100);
    }

    #[test]
    fn test_pagination_params_limit_exact_min() {
        let params = PaginationParams {
            limit: Some(1),
            offset: Some(0),
            page: None,
        };
        assert_eq!(params.limit(), 1);
    }

    #[test]
    fn test_pagination_params_large_offset() {
        let params = PaginationParams {
            limit: Some(10),
            offset: Some(1000),
            page: None,
        };
        assert_eq!(params.offset(), 1000);
    }

    #[test]
    fn test_pagination_meta_single_page() {
        let meta = PaginationMeta {
            total: 5,
            limit: 10,
            offset: Some(0),
            page: Some(1),
            has_more: false,
        };
        assert_eq!(meta.total, 5);
        assert_eq!(meta.limit, 10);
        assert_eq!(meta.offset, Some(0));
        assert!(!meta.has_more);
    }

    #[test]
    fn test_pagination_meta_multiple_pages() {
        let meta = PaginationMeta {
            total: 100,
            limit: 10,
            offset: Some(20),
            page: Some(3),
            has_more: true,
        };
        assert_eq!(meta.total, 100);
        assert_eq!(meta.limit, 10);
        assert_eq!(meta.offset, Some(20));
        assert!(meta.has_more);
    }

    #[test]
    fn test_pagination_meta_has_more_true() {
        let meta = PaginationMeta {
            total: 50,
            limit: 10,
            offset: Some(0),
            page: Some(1),
            has_more: true,
        };
        assert!(meta.has_more);
    }

    #[test]
    fn test_pagination_meta_has_more_false() {
        let meta = PaginationMeta {
            total: 10,
            limit: 10,
            offset: Some(0),
            page: Some(1),
            has_more: false,
        };
        assert!(!meta.has_more);
    }

    #[test]
    fn test_pagination_meta_serialize() {
        let meta = PaginationMeta {
            total: 100,
            limit: 20,
            offset: Some(40),
            page: Some(3),
            has_more: true,
        };
        let serialized = serde_json::to_string(&meta).unwrap();
        assert!(serialized.contains(r#""total":100"#));
        assert!(serialized.contains(r#""limit":20"#));
        assert!(serialized.contains(r#""offset":40"#));
        assert!(serialized.contains(r#""has_more":true"#));
    }

    #[test]
    fn test_pagination_params_deserialize_with_values() {
        let json = r#"{"limit":"25","offset":"50"}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit(), 25);
        assert_eq!(params.offset(), 50);
    }

    #[test]
    fn test_pagination_params_deserialize_empty_strings() {
        let json = r#"{"limit":"","offset":""}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit(), 10);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_deserialize_missing_fields() {
        let json = r#"{}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit(), 10);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_deserialize_only_limit() {
        let json = r#"{"limit":"30"}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit(), 30);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_deserialize_only_offset() {
        let json = r#"{"offset":"15"}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit(), 10);
        assert_eq!(params.offset(), 15);
    }

    #[test]
    fn test_pagination_params_limit_boundary_cases() {
        let test_cases = vec![
            (Some(1), 1),
            (Some(50), 50),
            (Some(100), 100),
            (Some(101), 100),
            (Some(0), 1),
            (Some(-1), 1),
        ];

        for (input, expected) in test_cases {
            let params = PaginationParams {
                limit: input,
                offset: Some(0),
                page: None,
            };
            assert_eq!(params.limit(), expected);
        }
    }

    #[test]
    fn test_pagination_params_offset_boundary_cases() {
        let test_cases = vec![
            (Some(0), 0),
            (Some(10), 10),
            (Some(100), 100),
            (Some(-1), 0),
            (Some(-100), 0),
        ];

        for (input, expected) in test_cases {
            let params = PaginationParams {
                limit: Some(10),
                offset: input,
                page: None,
            };
            assert_eq!(params.offset(), expected);
        }
    }

    #[test]
    fn test_pagination_meta_zero_total() {
        let meta = PaginationMeta {
            total: 0,
            limit: 10,
            offset: Some(0),
            page: Some(1),
            has_more: false,
        };
        assert_eq!(meta.total, 0);
        assert!(!meta.has_more);
    }

    #[test]
    fn test_pagination_meta_large_total() {
        let meta = PaginationMeta {
            total: 1000000,
            limit: 100,
            offset: Some(5000),
            page: Some(51),
            has_more: true,
        };
        assert_eq!(meta.total, 1000000);
        assert_eq!(meta.limit, 100);
        assert_eq!(meta.offset, Some(5000));
    }
}
