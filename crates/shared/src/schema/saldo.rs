use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Saldo {
    Table,
    SaldoId,
    UserId,
    TotalBalance,
    WithdrawAmount,
    WithdrawTime,
    CreatedAt,
    UpdatedAt,
}
