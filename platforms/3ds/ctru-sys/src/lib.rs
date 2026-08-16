// Empty local stub crate. Its sole purpose is to make the `ctru_sys` lib unit
// appear in the build graph so cargo-3ds's debuginfo probe succeeds (see
// Cargo.toml). Nothing in rabuka_3ds references this crate; it is never linked
// into the binary.
#![no_std]
