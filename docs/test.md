# Testing

Almost none of this emulators tests run with `cargo test` because of the checks for the main thread present within it, and the fact that `cargo test` spawns a new thread for every test.

If you need to run tests use `cargo nextest`, which gives tests their own processes.