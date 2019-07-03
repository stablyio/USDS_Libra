Hack the libra
----

```bash

# run cargo
cargo run -p libra_swarm -- -s

# create first account
account create

# mint libra coin
account mint 0 1000
query account_state 0

# issue a etoken to first account
hack issue 0
query account_state 0
hack account_state 0

# mint etoken
hack mint 0 1000
hack account_state 0

# create second account
account create
account mint 1 1000

# init second etoken account
hack init 1
hack account_state 1

# sell and buy etoken
hack sell 0 1000 1000
hack account_state 0
hack buy 1 0
hack account_state 0
hack account_state 1

# execute a script directly to create an account

hack execute 0 ${replace me with libra project path}/language/functional_tests/tests/testsuite/move_getting_started_examples/create_account_script.mvir 0dfda76385ef8812a4bec0b3fbb1997c1336d78411ae3059c0ad9ba14ac1b2d7 500

```
