USDS the libra
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
usds issue 0
query account_state 0
usds account_state 0

# mint etoken
usds mint 0 1000
usds account_state 0

# create second account
account create
account mint 1 1000

# init second etoken account
usds init 1
usds account_state 1

# sell and buy etoken
usds sell 0 1000 1000
usds account_state 0
usds buy 1 0
usds account_state 0
usds account_state 1

# execute a script directly to create an account

usds execute 0 ${replace me with libra project path}/language/functional_tests/tests/testsuite/move_getting_started_examples/create_account_script.mvir 0dfda76385ef8812a4bec0b3fbb1997c1336d78411ae3059c0ad9ba14ac1b2d7 500

```
