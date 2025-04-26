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
format-check:
    cargo fmt --check

# runs code blocks in the book as tests
[working-directory: 'docs/book_test']
book-test:
    cargo test
    mdbook build ../book/

[working-directory: 'docs/book/']
book-serve:
    #!/usr/bin/env -S bash
    mdbook serve &
    xdg-open http://localhost:3000
    wait

# I run this in my pre-commit hook
pre-commit: format-check

pre-push:
    cargo test
    @just book-test
