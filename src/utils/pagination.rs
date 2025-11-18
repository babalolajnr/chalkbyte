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
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub limit: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_i64")]
    pub offset: Option<i64>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: Some(10),
            offset: Some(0),
        }
    }
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(10).max(1).min(100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
