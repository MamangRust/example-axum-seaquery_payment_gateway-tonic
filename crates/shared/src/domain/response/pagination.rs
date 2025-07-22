use genproto::api::Pagination as ProtoPagination;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Pagination {
    pub page: i32,
    pub page_size: i32,
    pub total_items: i64,
    pub total_pages: i32,
}

impl From<ProtoPagination> for Pagination {
    fn from(value: ProtoPagination) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            total_items: value.total_items,
            total_pages: value.total_pages,
        }
    }
}

impl From<Pagination> for ProtoPagination {
    fn from(value: Pagination) -> Self {
        Self {
            page: value.page,
            page_size: value.page_size,
            total_items: value.total_items,
            total_pages: value.total_pages,
        }
    }
}
