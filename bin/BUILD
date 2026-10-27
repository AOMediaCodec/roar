# Standalone CLI utilities for Rust Open Audio Renderer (ROAR).

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

rust_binary(
    name = "generate_binaural_filters",
    srcs = ["generate_binaural_filters.rs"],
    edition = "2024",
    deps = [
        "@aom-roar//src:rice_block_decoder",
    ],
)

rust_test(
    name = "generate_binaural_filters_test",
    srcs = ["generate_binaural_filters.rs"],
    edition = "2024",
    deps = [
        "@aom-roar//src:rice_block_decoder",
        "@crates//:googletest",
    ],
)
