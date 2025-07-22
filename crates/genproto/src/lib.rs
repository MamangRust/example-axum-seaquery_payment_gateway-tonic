pub mod api {
    include!("gen/api.rs");
}

pub mod auth {
    include!("gen/auth.rs");
}

pub mod user {
    include!("gen/user.rs");
}

pub mod saldo {
    include!("gen/saldo.rs");
}

pub mod topup {
    include!("gen/topup.rs");
}

pub mod transfer {
    include!("gen/transfer.rs");
}

pub mod withdraw {
    include!("gen/withdraw.rs");
}
