default:
  @just --choose

insta:
    cargo nextest r
    cargo insta test --review --unreferenced delete

test:
    cargo nextest r

miri:
    cargo miri nextest r -j12
