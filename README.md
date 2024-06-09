Four Buttons
============

A machine with four light up buttons in a pretty box.  It reminds you to do certain tasks, like water the plants, on a set schedule.  Also an excuse to learn Rust.

## Pre-requisites

* docker or [podman](https://podman.io/docs/installation).
* `cargo install cross` - for cross compilation

## Usage

* Run locally with `USE_FAKE_RPI=1 RUST_LOG=debug cargo run`.
* Release with `./release.sh`.
* Autoformat code with `cargo fmt`.

## Pretty pictures

![Picture of the finished machine](images/complete.png)

## Circuits

A circuit diagram showing a single LED and button, in reality there are 4 of each.
![Circuit Diagram](images/circuit-2.png)

The stripboarded elements (transistor & resistor):
![Stripboard diagram](images/circuit-1.png)

## First time Raspberry Pi Configuration

Automatically connect to the wifi on startup:
1. `nmcli device wifi connect "<SSID>" password "<password>"`
2. `nmcli connection up "<SSID>"`
3. `nmcli connection mod "<SSID>" connection.autoconnect yes`
(To see if the wifi is connected: `nmcli connection show`)

Manually reserved 192.168.0.94 on the router.

## TODO

Some things I could do?  But who has the time.
* Tidy up my half-baked DIY actor framework!
 * Actually test it, write some tests
 * Cleanly shutdown the other actors when one dies
 * Handle sigterm and shutdown cleanly
* Test the core notification logic now it isn't all tied up with threads.
* Devcontainer with pinned rust version, clippy version, etc
* Store scheduler state in DB.
* Create a proper config class / don't store all the secrets in separate text files

---

Bugs:

Interesting test failures - in tmp DB handling?

Can reliably trigger with `while :; do cargo test || break; done`.

```
fourbuttons git:(tests) ✗ cargo test
    Blocking waiting for file lock on build directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.57s
     Running unittests src/main.rs (target/debug/deps/fourbuttons-e7e4eb79bff57945)

running 23 tests
test appdb::tests::load_app_state_after_no_saves ... ok
test actor::control_actor::tests::test_take_pills_activity ... ok
test appdb::tests::save_and_load_populated_app_state ... ok
test db::tests::fail_on_invalid_migration ... ok
test db::tests::fail_on_unknown_migration ... FAILED
test appdb::tests::save_and_load_empty_app_state ... ok
test email::tests::send_an_email ... ignored
test schedule::tests::daily_next_day ... ok
test schedule::tests::daily_next_day_week_boundary ... ok
test schedule::tests::daily_next_week ... ok
test schedule::tests::daily_same_day ... ok
test schedule::tests::daily_same_week ... ok
test schedule::tests::weekly_just_after_schedule_time ... ok
test schedule::tests::weekly_just_before_schedule_time ... ok
test schedule::tests::weekly_large_gap ... ok
test schedule::tests::weekly_two_days_after ... ok
test schedule::tests::weekly_two_days_before ... ok
test scheduler::tests::outside_of_grace_period ... ok
test scheduler::tests::regular_ticks ... ok
test scheduler::tests::within_grace_period ... ok
test db::tests::upgrade_from_one_migration ... ok
test db::tests::upgrade_from_empty_db ... ok
test db::tests::upgrade_from_zero_migrations ... ok

failures:

---- db::tests::fail_on_unknown_migration stdout ----
thread 'db::tests::fail_on_unknown_migration' panicked at src/db.rs:289:13:
assertion failed: table_exists(&conn, "t2")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    db::tests::fail_on_unknown_migration

test result: FAILED. 21 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.36s

error: test failed, to rerun pass `--bin fourbuttons`
➜  fourbuttons git:(tests) ✗ cargo test
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.19s
     Running unittests src/main.rs (target/debug/deps/fourbuttons-e7e4eb79bff57945)

running 23 tests
test appdb::tests::load_app_state_after_no_saves ... ok
test actor::control_actor::tests::test_take_pills_activity ... ok
test appdb::tests::save_and_load_empty_app_state ... ok
test appdb::tests::save_and_load_populated_app_state ... ok
test db::tests::fail_on_unknown_migration ... FAILED
test db::tests::fail_on_invalid_migration ... ok
test email::tests::send_an_email ... ignored
test schedule::tests::daily_next_day ... ok
test schedule::tests::daily_next_day_week_boundary ... ok
test schedule::tests::daily_next_week ... ok
test schedule::tests::daily_same_day ... ok
test schedule::tests::daily_same_week ... ok
test schedule::tests::weekly_just_after_schedule_time ... ok
test schedule::tests::weekly_just_before_schedule_time ... ok
test schedule::tests::weekly_large_gap ... ok
test schedule::tests::weekly_two_days_after ... ok
test schedule::tests::weekly_two_days_before ... ok
test scheduler::tests::outside_of_grace_period ... ok
test scheduler::tests::regular_ticks ... ok
test scheduler::tests::within_grace_period ... ok
test db::tests::upgrade_from_empty_db ... ok
test db::tests::upgrade_from_zero_migrations ... ok
test db::tests::upgrade_from_one_migration ... FAILED

failures:

---- db::tests::fail_on_unknown_migration stdout ----
thread 'db::tests::fail_on_unknown_migration' panicked at src/db.rs:289:13:
assertion failed: table_exists(&conn, "t2")
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- db::tests::upgrade_from_one_migration stdout ----
thread 'db::tests::upgrade_from_one_migration' panicked at src/db.rs:263:39:
called `Result::unwrap()` on an `Err` value: migration ID 002 in database doesn't appear in migration history


failures:
    db::tests::fail_on_unknown_migration
    db::tests::upgrade_from_one_migration

test result: FAILED. 20 passed; 2 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.34s

error: test failed, to rerun pass `--bin fourbuttons`
```