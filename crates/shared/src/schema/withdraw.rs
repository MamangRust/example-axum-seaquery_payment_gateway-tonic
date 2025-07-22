use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Withdraws {
    Table,
    WithdrawId,
    UserId,
    WithdrawAmount,
    WithdrawTime,
    CreatedAt,
    UpdatedAt,
}
