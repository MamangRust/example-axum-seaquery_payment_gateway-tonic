use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Users {
    Table,
    UserId,
    Firstname,
    Lastname,
    Email,
    Password,
    NocTransfer,
    CreatedAt,
    UpdatedAt,
}
