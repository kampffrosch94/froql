# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Add
- `World::defer_closure(f)` for deferring an arbitrary operation until `World::process()`
- `World::view_deferred(entity)`: Wraps an existing Entity in an `EntityViewDeferred`.
- `Bookkeeping::ensure_alive_generation(entity)`: like `ensure_alive(..)` but also has a preset generation
### Fix
- entities had invalid IDs when created in defered mode after another entity was forced alive

## [0.1.0] - 2025-05-08
- first public release of froql
