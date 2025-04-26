default:
  @just --list

# insta reviews
insta:
    cargo insta review

# run tests with nextest
test:
    cargo nextest r

# run tests under miri
miri:
    cargo miri nextest r -j12

# checks that everything is well formatted
check-format:
    cargo fmt --check

check-book:
    cargo build
    mdbook test docs/book -L target/debug/deps/
