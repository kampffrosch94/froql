default:
  @just --choose

insta:
    cargo insta test --review --unreferenced delete

test:
    cargo nextest r

miri:
    cargo miri nextest r -j12
