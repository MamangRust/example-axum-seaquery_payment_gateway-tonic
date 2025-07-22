use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Transfers {
    Table,
    TransferId,
    TransferFrom,
    TransferTo,
    TransferAmount,
    TransferTime,
    CreatedAt,
    UpdatedAt,
}
