default:
  @just --list

# insta reviews
insta:
    cargo insta review


# run benchmarks with criterion
[working-directory: 'froql/']
bench:
    cargo bench

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

# I run this in my pre-push hook
pre-push:
    cargo test
    @just book-test


pre-commit := trim('
#!/usr/bin/env -S bash
just pre-commit
')
pre-push := trim('
#!/usr/bin/env -S bash
just pre-push
')

# sets up git hooks
setup-hooks:
    echo "{{pre-commit}}" > .git/hooks/pre-commit
    echo "{{pre-push}}"> .git/hooks/pre-push
    chmod +x .git/hooks/pre-commit
    chmod +x .git/hooks/pre-push

