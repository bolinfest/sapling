// @generated by autocargo

use std::env;
use std::fs;
use std::path::Path;
use thrift_compiler::Config;
use thrift_compiler::GenContext;
const CRATEMAP: &str = "\
blame mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
bonsai mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
bssm mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
changeset_info mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
content mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
data mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
deleted_manifest mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
derivation_queue crate //eden/mononoke/repo_attributes/repo_derivation_queues/if:derivation_queue_thrift-rust
fastlog mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
fsnodes mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
id mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
path mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
raw_bundle2 mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
redaction mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
sharded_map mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
skeleton_manifest mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
test_manifest mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
time mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
unodes mononoke_types_serialization //eden/mononoke/mononoke_types/serialization:mononoke_types_serialization-rust
";
#[rustfmt::skip]
fn main() {
    println!("cargo:rerun-if-changed=thrift_build.rs");
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR env not provided");
    let cratemap_path = Path::new(&out_dir).join("cratemap");
    fs::write(cratemap_path, CRATEMAP).expect("Failed to write cratemap");
    Config::from_env(GenContext::Clients)
        .expect("Failed to instantiate thrift_compiler::Config")
        .base_path("../../../../../..")
        .types_crate("derivation_queue_thrift__types")
        .clients_crate("derivation_queue_thrift__clients")
        .run(["../derivation_queue.thrift"])
        .expect("Failed while running thrift compilation");
}
