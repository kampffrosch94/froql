default:
  @just --choose

insta:
    cargo insta review

test:
    cargo nextest r
    @just insta

miri:
    cargo miri nextest r -j12
