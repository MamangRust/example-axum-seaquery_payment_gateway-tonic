use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "src/gen";

    fs::create_dir_all(out_dir)?;

    tonic_build::configure()
        .build_server(true)
        .out_dir(out_dir)
        .compile_protos(
            &[
                "../../proto/user.proto",
                "../../proto/auth.proto",
                "../../proto/saldo.proto",
                "../../proto/topup.proto",
                "../../proto/transfer.proto",
                "../../proto/withdraw.proto",
            ],
            &["../../proto"],
        )?;

    println!("cargo:rerun-if-changed=../../proto");

    Ok(())
}
