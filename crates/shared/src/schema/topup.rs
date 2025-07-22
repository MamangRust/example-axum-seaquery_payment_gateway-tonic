use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Topups {
    Table,
    TopupId,
    UserId,
    TopupNo,
    TopupAmount,
    TopupMethod,
    TopupTime,
    CreatedAt,
    UpdatedAt,
}
